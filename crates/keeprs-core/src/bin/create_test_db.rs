use keepass::{config::DatabaseConfig, db::Node, db::Value, Database, DatabaseKey};
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Database::new(DatabaseConfig::default());
    db.meta.database_name = Some("Test DB".to_string());
    db.meta.database_description = Some("A test database for Keeprs".to_string());

    // Add a group
    let mut group = keepass::db::Group::new("Test Group");
    
    // Add an entry
    let mut entry = keepass::db::Entry::new();
    entry.fields.insert("Title".to_string(), Value::Unprotected("Test Entry".to_string()));
    entry.fields.insert("UserName".to_string(), Value::Unprotected("user".to_string()));
    entry.fields.insert("Password".to_string(), Value::Protected("pass".as_bytes().into()));
    entry.fields.insert("URL".to_string(), Value::Unprotected("http://example.com".to_string()));
    
    group.children.push(Node::Entry(entry));
    db.root.children.push(Node::Group(group));

    // Recycle bin
    let mut bin = keepass::db::Group::new("Recycle Bin");
    bin.icon_id = Some(43); // Trash icon
    let bin_uuid = bin.uuid;
    db.root.children.push(Node::Group(bin));
    db.meta.recyclebin_uuid = Some(bin_uuid);
    db.meta.recyclebin_enabled = Some(true);

    let key = DatabaseKey::new().with_password("password");
    let mut file = File::create("test_db.kdbx")?;
    db.save(&mut file, key)?;
    
    println!("Created test_db.kdbx with password 'password'");
    Ok(())
}
