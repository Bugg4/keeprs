//! KeePass database operations wrapper.

use crate::models::{Attachment, Entry, Group};
use anyhow::{Context, Result};
use keepass::{Database, DatabaseKey};
use std::path::Path;

/// Wrapper around the KeePass database for easier operations.
#[derive(Clone)]
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
            is_recycle_bin: self.is_recycle_bin(&kg.uuid.to_string()),
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
        // Atomic save: write to temp file then rename
        let mut temp_path = self.path.clone();
        if let Some(ext) = temp_path.extension() {
            let mut ext = ext.to_os_string();
            ext.push(".tmp");
            temp_path.set_extension(ext);
        } else {
            temp_path.set_extension("tmp");
        }

        {
            let mut file = std::fs::File::create(&temp_path)
                .with_context(|| format!("Failed to create temp database file: {}", temp_path.display()))?;

            self.db
                .save(&mut file, self.key.clone())
                .with_context(|| "Failed to save database to temp file")?;
            
            // Ensure data is flushed to disk
            file.sync_all().context("Failed to sync temp database file")?;
        }

        std::fs::rename(&temp_path, &self.path)
            .with_context(|| format!("Failed to replace database file: {}", self.path.display()))?;

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

    /// Add a new entry to the database under a specific group.
    pub fn add_entry(&mut self, parent_group_uuid: &str, entry: &Entry) -> Result<String> {
        let mut new_entry = keepass::db::Entry::new();
        
        // Map fields
        new_entry.fields.insert("Title".to_string(), keepass::db::Value::Unprotected(entry.title.clone()));
        new_entry.fields.insert("UserName".to_string(), keepass::db::Value::Unprotected(entry.username.clone()));
        new_entry.fields.insert("Password".to_string(), keepass::db::Value::Protected(entry.password.as_bytes().into()));
        new_entry.fields.insert("URL".to_string(), keepass::db::Value::Unprotected(entry.url.clone()));
        new_entry.fields.insert("Notes".to_string(), keepass::db::Value::Unprotected(entry.notes.clone()));
        
        for (k, v) in &entry.custom_fields {
             new_entry.fields.insert(k.clone(), keepass::db::Value::Unprotected(v.clone()));
        }

        let uuid = new_entry.uuid.to_string();

        if Self::add_node_recursive(&mut self.db.root, parent_group_uuid, keepass::db::Node::Entry(new_entry)) {
            Ok(uuid)
        } else {
            anyhow::bail!("Parent group with UUID {} not found", parent_group_uuid)
        }
    }

    /// Add a new group to the database under a specific group.
    pub fn add_group(&mut self, parent_group_uuid: &str, group: &Group) -> Result<String> {
        let new_group = keepass::db::Group::new(&group.name);
        let uuid = new_group.uuid.to_string();
        
        if Self::add_node_recursive(&mut self.db.root, parent_group_uuid, keepass::db::Node::Group(new_group)) {
             Ok(uuid)
        } else {
             anyhow::bail!("Parent group with UUID {} not found", parent_group_uuid)
        }
    }

    pub fn get_recycle_bin_uuid(&self) -> Option<String> {
        self.db.meta.recyclebin_uuid.as_ref().map(|u| u.to_string())
    }

    pub fn is_recycle_bin(&self, uuid: &str) -> bool {
        if let Some(ref bin_uuid) = self.db.meta.recyclebin_uuid {
            bin_uuid.to_string() == uuid
        } else {
            false
        }
    }

    pub fn is_inside_recycle_bin(&self, uuid: &str) -> bool {
        if let Some(ref bin_uuid) = self.db.meta.recyclebin_uuid {
            let bin_uuid_str = bin_uuid.to_string();
            if bin_uuid_str == uuid {
                return true;
            }
            Self::is_descendant_of(&self.db.root, uuid, &bin_uuid_str)
        } else {
            false
        }
    }

    fn is_descendant_of(group: &keepass::db::Group, target_uuid: &str, ancestor_uuid: &str) -> bool {
        // If this group is the ancestor, then any target found inside is a descendant
        let is_ancestor = group.uuid.to_string() == ancestor_uuid;
        
        if is_ancestor {
             // If we found the ancestor, we just need to find the target inside it
             return Self::contains_node(group, target_uuid);
        }

        // Otherwise recurse
        for child in &group.children {
            if let keepass::db::Node::Group(g) = child {
                if Self::is_descendant_of(g, target_uuid, ancestor_uuid) {
                    return true;
                }
            }
        }
        false
    }

    fn contains_node(group: &keepass::db::Group, target_uuid: &str) -> bool {
        if group.uuid.to_string() == target_uuid {
             return true; 
        }
        for node in &group.children {
            match node {
                keepass::db::Node::Group(g) => {
                    if Self::contains_node(g, target_uuid) {
                        return true;
                    }
                }
                keepass::db::Node::Entry(e) => {
                    if e.uuid.to_string() == target_uuid {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn get_or_create_recycle_bin(&mut self) -> Result<String> {
        if let Some(ref uuid) = self.db.meta.recyclebin_uuid {
            // verify it exists... assuming yes for now for simplicity
            return Ok(uuid.to_string());
        }

        // Create new group
        let mut group = keepass::db::Group::new("Recycle Bin");
        // Set icon to trash (43 is trash in standard KeePass)
        group.icon_id = Some(43); 
        let uuid = group.uuid;
        
        self.db.root.children.push(keepass::db::Node::Group(group));
        self.db.meta.recyclebin_uuid = Some(uuid);
        self.db.meta.recyclebin_enabled = Some(true);
        
        Ok(uuid.to_string())
    }

    fn recycle_node(&mut self, uuid: &str, is_group: bool) -> Result<()> {
        let recycle_bin_uuid = self.get_or_create_recycle_bin()?;
        
        // Prevent recycling the recycle bin itself
        if recycle_bin_uuid == uuid {
            anyhow::bail!("Cannot delete the Recycle Bin itself");
        }

        // Remove the node
        if let Some(node) = Self::delete_node_recursive(&mut self.db.root, uuid, is_group) {
             // Add to recycle bin
             if Self::add_node_recursive(&mut self.db.root, &recycle_bin_uuid, node) {
                 Ok(())
             } else {
                 // Should not happen if get_or_create works
                 anyhow::bail!("Failed to move node to Recycle Bin")
             }
        } else {
             anyhow::bail!("Node with UUID {} not found", uuid)
        }
    }

    pub fn empty_recycle_bin(&mut self) -> Result<()> {
        if let Some(ref uuid) = self.db.meta.recyclebin_uuid {
             // Find the group and clear children
             let uuid_str = uuid.to_string();
             if let Some(node) = Self::find_node_recursive_mut(&mut self.db.root, &uuid_str) {
                 if let keepass::db::Node::Group(g) = node {
                     g.children.clear();
                 }
             }
        }
        Ok(())
    }
    
    // Improved helper to find ANY node by UUID to allow manipulation
    fn find_node_recursive_mut<'a>(group: &'a mut keepass::db::Group, target_uuid: &str) -> Option<&'a mut keepass::db::Node> {
         for node in &mut group.children {
             let is_match = if let keepass::db::Node::Group(g) = &*node {
                 g.uuid.to_string() == target_uuid
             } else { false };
             
             if is_match {
                 return Some(node);
             }

             if let keepass::db::Node::Group(g) = node {
                 if let Some(found) = Self::find_node_recursive_mut(g, target_uuid) {
                     return Some(found);
                 }
             }
         }
         None
    }

    fn add_node_recursive(group: &mut keepass::db::Group, target_uuid: &str, new_node: keepass::db::Node) -> bool {
        if group.uuid.to_string() == target_uuid {
            group.children.push(new_node);
            return true;
        }
        
        for node in &mut group.children {
            if let keepass::db::Node::Group(g) = node {
                if Self::add_node_recursive(g, target_uuid, new_node.clone()) {
                    return true;
                }
            }
        }
        
        false
    }



    pub fn delete_entry(&mut self, uuid: &str) -> Result<()> {
        self.recycle_node(uuid, false)
    }

    pub fn delete_group(&mut self, uuid: &str) -> Result<()> {
        self.recycle_node(uuid, true)
    }

    pub fn delete_entry_permanently(&mut self, uuid: &str) -> Result<()> {
        if Self::delete_node_recursive(&mut self.db.root, uuid, false).is_some() {
            Ok(())
        } else {
             anyhow::bail!("Entry with UUID {} not found", uuid)
        }
    }

    pub fn delete_group_permanently(&mut self, uuid: &str) -> Result<()> {
         if Self::delete_node_recursive(&mut self.db.root, uuid, true).is_some() {
            Ok(())
        } else {
             anyhow::bail!("Group with UUID {} not found", uuid)
        }
    }
    
    fn delete_node_recursive(group: &mut keepass::db::Group, target_uuid: &str, is_group: bool) -> Option<keepass::db::Node> {
         if is_group {
             if let Some(pos) = group.children.iter().position(|node| {
                 if let keepass::db::Node::Group(g) = node {
                     g.uuid.to_string() == target_uuid
                 } else {
                     false
                 }
             }) {
                 return Some(group.children.remove(pos));
             }
         } else {
             if let Some(pos) = group.children.iter().position(|node| {
                 if let keepass::db::Node::Entry(e) = node {
                     e.uuid.to_string() == target_uuid
                 } else {
                     false
                 }
             }) {
                 return Some(group.children.remove(pos));
             }
         }

         for node in &mut group.children {
             if let keepass::db::Node::Group(g) = node {
                 if let Some(removed_node) = Self::delete_node_recursive(g, target_uuid, is_group) {
                     return Some(removed_node);
                 }
             }
         }
         
         None
    }
}
