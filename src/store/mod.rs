use rusqlite::Connection;
use std::path::Path;

use crate::error::{ZapError, ZapResult};

/// Local SQLite storage for drafts and user preferences.
pub struct LocalStore {
    conn: Connection,
}

impl LocalStore {
    /// Open (or create) a SQLite database at the given path.
    pub fn open(db_path: &Path) -> ZapResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)
            .map_err(|e| ZapError::Database(e.to_string()))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS drafts (
                room_id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| ZapError::Database(e.to_string()))?;

        Ok(Self { conn })
    }

    /// Open an in-memory SQLite database (useful for testing).
    pub fn open_in_memory() -> ZapResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ZapError::Database(e.to_string()))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS drafts (
                room_id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| ZapError::Database(e.to_string()))?;

        Ok(Self { conn })
    }

    /// Save a message draft for the given room, replacing any existing draft.
    pub fn save_draft(&self, room_id: &str, content: &str) -> ZapResult<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO drafts (room_id, content, updated_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![room_id, content, now],
            )
            .map_err(|e| ZapError::Database(e.to_string()))?;
        Ok(())
    }

    /// Load a message draft for the given room, if one exists.
    pub fn load_draft(&self, room_id: &str) -> ZapResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT content FROM drafts WHERE room_id = ?1")
            .map_err(|e| ZapError::Database(e.to_string()))?;

        let result = stmt.query_row(rusqlite::params![room_id], |row| row.get(0));
        match result {
            Ok(content) => Ok(Some(content)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ZapError::Database(e.to_string())),
        }
    }

    /// Save a user preference key-value pair.
    pub fn save_preference(&self, key: &str, value: &str) -> ZapResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO preferences (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .map_err(|e| ZapError::Database(e.to_string()))?;
        Ok(())
    }

    /// Load a user preference by key, if it exists.
    pub fn load_preference(&self, key: &str) -> ZapResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM preferences WHERE key = ?1")
            .map_err(|e| ZapError::Database(e.to_string()))?;

        let result = stmt.query_row(rusqlite::params![key], |row| row.get(0));
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ZapError::Database(e.to_string())),
        }
    }

    /// Delete a draft for the given room.
    pub fn delete_draft(&self, room_id: &str) -> ZapResult<()> {
        self.conn
            .execute(
                "DELETE FROM drafts WHERE room_id = ?1",
                rusqlite::params![room_id],
            )
            .map_err(|e| ZapError::Database(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let store = LocalStore::open_in_memory();
        assert!(store.is_ok());
    }

    #[test]
    fn test_open_with_file_path() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let store = LocalStore::open(&db_path);
        assert!(store.is_ok());
        assert!(db_path.exists());
    }

    #[test]
    fn test_open_creates_parent_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("subdir").join("nested").join("test.db");
        let store = LocalStore::open(&db_path);
        assert!(store.is_ok());
        assert!(db_path.exists());
    }

    #[test]
    fn test_save_and_load_draft() {
        let store = LocalStore::open_in_memory().unwrap();
        store
            .save_draft("!room1:example.com", "Hello, world!")
            .unwrap();
        let draft = store.load_draft("!room1:example.com").unwrap();
        assert_eq!(draft, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_load_missing_draft_returns_none() {
        let store = LocalStore::open_in_memory().unwrap();
        let draft = store.load_draft("!nonexistent:example.com").unwrap();
        assert_eq!(draft, None);
    }

    #[test]
    fn test_overwrite_draft() {
        let store = LocalStore::open_in_memory().unwrap();
        store
            .save_draft("!room1:example.com", "First draft")
            .unwrap();
        store
            .save_draft("!room1:example.com", "Updated draft")
            .unwrap();
        let draft = store.load_draft("!room1:example.com").unwrap();
        assert_eq!(draft, Some("Updated draft".to_string()));
    }

    #[test]
    fn test_delete_draft() {
        let store = LocalStore::open_in_memory().unwrap();
        store
            .save_draft("!room1:example.com", "To be deleted")
            .unwrap();
        store.delete_draft("!room1:example.com").unwrap();
        let draft = store.load_draft("!room1:example.com").unwrap();
        assert_eq!(draft, None);
    }

    #[test]
    fn test_delete_nonexistent_draft() {
        let store = LocalStore::open_in_memory().unwrap();
        // Deleting a draft that does not exist should not error.
        let result = store.delete_draft("!nonexistent:example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_and_load_preference() {
        let store = LocalStore::open_in_memory().unwrap();
        store.save_preference("theme", "dark").unwrap();
        let pref = store.load_preference("theme").unwrap();
        assert_eq!(pref, Some("dark".to_string()));
    }

    #[test]
    fn test_load_missing_preference_returns_none() {
        let store = LocalStore::open_in_memory().unwrap();
        let pref = store.load_preference("nonexistent_key").unwrap();
        assert_eq!(pref, None);
    }

    #[test]
    fn test_overwrite_preference() {
        let store = LocalStore::open_in_memory().unwrap();
        store.save_preference("theme", "light").unwrap();
        store.save_preference("theme", "dark").unwrap();
        let pref = store.load_preference("theme").unwrap();
        assert_eq!(pref, Some("dark".to_string()));
    }

    #[test]
    fn test_multiple_drafts_independent() {
        let store = LocalStore::open_in_memory().unwrap();
        store
            .save_draft("!room1:example.com", "Draft for room 1")
            .unwrap();
        store
            .save_draft("!room2:example.com", "Draft for room 2")
            .unwrap();

        let draft1 = store.load_draft("!room1:example.com").unwrap();
        let draft2 = store.load_draft("!room2:example.com").unwrap();
        assert_eq!(draft1, Some("Draft for room 1".to_string()));
        assert_eq!(draft2, Some("Draft for room 2".to_string()));
    }

    #[test]
    fn test_multiple_preferences_independent() {
        let store = LocalStore::open_in_memory().unwrap();
        store.save_preference("theme", "dark").unwrap();
        store.save_preference("font_size", "14").unwrap();

        let theme = store.load_preference("theme").unwrap();
        let font_size = store.load_preference("font_size").unwrap();
        assert_eq!(theme, Some("dark".to_string()));
        assert_eq!(font_size, Some("14".to_string()));
    }

    #[test]
    fn test_empty_draft_content() {
        let store = LocalStore::open_in_memory().unwrap();
        store.save_draft("!room1:example.com", "").unwrap();
        let draft = store.load_draft("!room1:example.com").unwrap();
        assert_eq!(draft, Some("".to_string()));
    }
}
