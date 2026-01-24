//! Core models and database handling for keeprs.
//!
//! This crate provides shared types and database operations used by both
//! the GUI and TUI frontends.

pub mod database;
pub mod models;

pub use database::KeepassDatabase;
pub use models::{Attachment, Entry, Group, NavigationPath, NavigationStep};
