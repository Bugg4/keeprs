//! KeePass database operations wrapper.

use crate::models::{Attachment, Entry, Group};
use anyhow::{Context, Result};
use keepass::{Database, DatabaseKey};
use std::path::Path;

/// Wrapper around the KeePass database for easier operations.
pub struct KeepassDatabase {
    db: Database,
    path: std::path::PathBuf,
    key: DatabaseKey,
}

impl KeepassDatabase {
    /// Open and unlock a KeePass database.
    pub fn unlock(path: impl AsRef<Path>, password: &str) -> Result<Self> {
        let path = path.as_ref();

        let key = DatabaseKey::new().with_password(password);

        let db = Database::open(&mut std::fs::File::open(path)?, key.clone())
            .with_context(|| format!("Failed to open database: {}", path.display()))?;

        Ok(Self {
            db,
            path: path.to_path_buf(),
            key,
        })
    }

    /// Get the root group of the database.
    pub fn root_group(&self) -> Group {
        self.convert_group(&self.db.root)
    }

    /// Convert a keepass::Group to our Group model.
    fn convert_group(&self, kg: &keepass::db::Group) -> Group {
        Group {
            uuid: kg.uuid.to_string(),
            name: kg.name.clone(),
            children: kg
                .children
                .iter()
                .filter_map(|node| {
                    if let keepass::db::Node::Group(g) = node {
                        Some(self.convert_group(g))
                    } else {
                        None
                    }
                })
                .collect(),
            entries: kg
                .children
                .iter()
                .filter_map(|node| {
                    if let keepass::db::Node::Entry(e) = node {
                        Some(self.convert_entry(e))
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    /// Convert a keepass::Entry to our Entry model.
    fn convert_entry(&self, ke: &keepass::db::Entry) -> Entry {
        let mut custom_fields = std::collections::HashMap::new();
        let mut attachments = Vec::new();

        for (key, val) in &ke.fields {
            // Skip standard fields that are handled by specific getters
            if matches!(
                key.as_str(),
                "Title" | "UserName" | "Password" | "URL" | "Notes"
            ) {
                continue;
            }

            match val {
                keepass::db::Value::Bytes(bytes) => {
                    attachments.push(Attachment {
                        filename: key.clone(),
                        _mime_type: None,
                        data: bytes.clone(),
                    });
                }
                keepass::db::Value::BinaryRef(ref_id) => {
                    if let Ok(index) = ref_id.parse::<usize>() {
                        if let Some(att) = self.db.header_attachments.get(index) {
                            attachments.push(Attachment {
                                filename: key.clone(),
                                _mime_type: None,
                                data: att.content.clone(),
                            });
                        } else {
                            tracing::warn!("Attachment reference {} not found in header", index);
                        }
                    } else {
                        tracing::warn!("Invalid attachment reference format: {}", ref_id);
                    }
                }
                keepass::db::Value::Unprotected(s) => {
                    custom_fields.insert(key.clone(), s.clone());
                }
                keepass::db::Value::Protected(_) => {
                    if let Some(s) = ke.get(key) {
                        custom_fields.insert(key.clone(), s.to_string());
                    }
                }
            }
        }

        Entry {
            uuid: ke.uuid.to_string(),
            title: ke.get_title().unwrap_or_default().to_string(),
            username: ke.get_username().unwrap_or_default().to_string(),
            password: ke.get_password().unwrap_or_default().to_string(),
            url: ke.get_url().unwrap_or_default().to_string(),
            notes: ke.get("Notes").unwrap_or_default().to_string(),
            custom_fields,
            otp: ke.get_raw_otp_value().map(|s| s.to_string()),
            attachments,
        }
    }

    /// Find an entry by UUID.
    pub fn find_entry(&self, uuid: &str) -> Option<Entry> {
        self.find_entry_in_group(&self.db.root, uuid)
    }

    fn find_entry_in_group(&self, group: &keepass::db::Group, uuid: &str) -> Option<Entry> {
        for node in &group.children {
            match node {
                keepass::db::Node::Entry(e) if e.uuid.to_string() == uuid => {
                    return Some(self.convert_entry(e));
                }
                keepass::db::Node::Group(g) => {
                    if let Some(entry) = self.find_entry_in_group(g, uuid) {
                        return Some(entry);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Save the database to disk.
    pub fn save(&self) -> Result<()> {
        let mut file = std::fs::File::create(&self.path)
            .with_context(|| format!("Failed to create database file: {}", self.path.display()))?;

        self.db
            .save(&mut file, self.key.clone())
            .with_context(|| "Failed to save database")?;

        Ok(())
    }

    /// Get mutable access to the underlying database for modifications.
    pub fn db_mut(&mut self) -> &mut Database {
        &mut self.db
    }

    /// Update an entry in the database.
    pub fn update_entry(&mut self, entry: &Entry) -> Result<()> {
        if Self::update_entry_recursive(&mut self.db.root, entry) {
            Ok(())
        } else {
            anyhow::bail!("Entry with UUID {} not found", entry.uuid)
        }
    }

    fn update_entry_recursive(group: &mut keepass::db::Group, entry: &Entry) -> bool {
        for node in &mut group.children {
            match node {
                keepass::db::Node::Entry(e) => {
                    if e.uuid.to_string() == entry.uuid {
                        // Update standard fields
                        e.fields.insert(
                            "Title".to_string(),
                            keepass::db::Value::Unprotected(entry.title.clone()),
                        );
                        e.fields.insert(
                            "UserName".to_string(),
                            keepass::db::Value::Unprotected(entry.username.clone()),
                        );
                        e.fields.insert(
                            "Password".to_string(),
                            keepass::db::Value::Protected(entry.password.as_bytes().into()),
                        );
                        e.fields.insert(
                            "URL".to_string(),
                            keepass::db::Value::Unprotected(entry.url.clone()),
                        );
                        e.fields.insert(
                            "Notes".to_string(),
                            keepass::db::Value::Unprotected(entry.notes.clone()),
                        );

                        // Update custom fields (simple overwrite for now)
                        for (k, v) in &entry.custom_fields {
                            e.fields
                                .insert(k.clone(), keepass::db::Value::Unprotected(v.clone()));
                        }

                        return true;
                    }
                }
                keepass::db::Node::Group(g) => {
                    if Self::update_entry_recursive(g, entry) {
                        return true;
                    }
                }
            }
        }
        false
    }
}
