//! Review storage trait and SQLite implementation.

use crate::review::types::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

/// Trait for review storage backends.
pub trait ReviewStore: Send + Sync {
    /// Get a review by diff ID, or None if it doesn't exist.
    fn get(&self, id: &DiffId) -> Result<Option<Review>, StoreError>;

    /// Get or create a review for a diff.
    fn get_or_create(&self, id: &DiffId) -> Result<Review, StoreError>;

    /// Save a review (insert or update).
    fn save(&self, review: &Review) -> Result<(), StoreError>;

    /// Delete a review.
    fn delete(&self, id: &DiffId) -> Result<(), StoreError>;

    /// Add a comment to a review.
    fn add_comment(&self, id: &DiffId, comment: &Comment) -> Result<(), StoreError>;

    /// Delete a comment from a review.
    fn delete_comment(&self, id: &DiffId, comment_id: Uuid) -> Result<(), StoreError>;

    /// Mark a file as reviewed.
    fn mark_reviewed(&self, id: &DiffId, file_path: &str) -> Result<(), StoreError>;

    /// Unmark a file as reviewed.
    fn unmark_reviewed(&self, id: &DiffId, file_path: &str) -> Result<(), StoreError>;

    /// Record an edit.
    fn add_edit(&self, id: &DiffId, edit: &Edit) -> Result<(), StoreError>;
}

/// Storage error type.
#[derive(Debug)]
pub struct StoreError {
    pub message: String,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        StoreError {
            message: err.to_string(),
        }
    }
}

/// SQLite-based review storage.
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Open or create the database at the given path.
    pub fn open(path: PathBuf) -> Result<Self, StoreError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StoreError {
                message: format!("Failed to create data directory: {}", e),
            })?;
        }

        let conn = Connection::open(&path)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Initialize the database schema.
    fn init_schema(&self) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();

        // Check current schema version
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);

        if version < 1 {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS reviews (
                    id TEXT PRIMARY KEY,
                    base TEXT NOT NULL,
                    head TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS reviewed_files (
                    review_id TEXT NOT NULL,
                    file_path TEXT NOT NULL,
                    PRIMARY KEY (review_id, file_path),
                    FOREIGN KEY (review_id) REFERENCES reviews(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS comments (
                    id TEXT PRIMARY KEY,
                    review_id TEXT NOT NULL,
                    file_path TEXT NOT NULL,
                    range_index INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (review_id) REFERENCES reviews(id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS edits (
                    id TEXT PRIMARY KEY,
                    review_id TEXT NOT NULL,
                    file_path TEXT NOT NULL,
                    diff TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (review_id) REFERENCES reviews(id) ON DELETE CASCADE
                );

                PRAGMA user_version = 1;
                "#,
            )?;
        }

        // Future migrations go here:
        // if version < 2 { ... }

        // Enable foreign key enforcement (must be set per-connection)
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        Ok(())
    }

    /// Get the storage ID for a diff.
    fn storage_id(id: &DiffId) -> String {
        id.storage_id()
    }

    /// Load reviewed files for a review.
    fn load_reviewed_files(
        conn: &Connection,
        review_id: &str,
    ) -> Result<HashSet<String>, StoreError> {
        let mut stmt = conn.prepare("SELECT file_path FROM reviewed_files WHERE review_id = ?")?;
        let files = stmt
            .query_map([review_id], |row| row.get(0))?
            .collect::<Result<HashSet<String>, _>>()?;
        Ok(files)
    }

    /// Load comments for a review.
    fn load_comments(conn: &Connection, review_id: &str) -> Result<Vec<Comment>, StoreError> {
        let mut stmt = conn.prepare(
            "SELECT id, file_path, range_index, text, created_at FROM comments WHERE review_id = ?",
        )?;
        let comments = stmt
            .query_map([review_id], |row| {
                let id_str: String = row.get(0)?;
                let range_index: i64 = row.get(2)?;
                let created_str: String = row.get(4)?;
                Ok(Comment {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
                    file_path: row.get(1)?,
                    range_index: range_index as usize,
                    text: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(comments)
    }

    /// Load edits for a review.
    fn load_edits(conn: &Connection, review_id: &str) -> Result<Vec<Edit>, StoreError> {
        let mut stmt =
            conn.prepare("SELECT id, file_path, diff, created_at FROM edits WHERE review_id = ?")?;
        let edits = stmt
            .query_map([review_id], |row| {
                let id_str: String = row.get(0)?;
                let created_str: String = row.get(3)?;
                Ok(Edit {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
                    file_path: row.get(1)?,
                    diff: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(edits)
    }

    /// Ensure a review exists, creating it if necessary. Returns the storage ID.
    fn ensure_review(&self, id: &DiffId) -> Result<String, StoreError> {
        let storage_id = Self::storage_id(id);
        let conn = self.conn.lock().unwrap();

        let exists: bool = conn
            .query_row("SELECT 1 FROM reviews WHERE id = ?", [&storage_id], |_| {
                Ok(true)
            })
            .optional()?
            .unwrap_or(false);

        if !exists {
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO reviews (id, base, head, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                params![storage_id, id.base, id.head, now, now],
            )?;
        }

        Ok(storage_id)
    }

    /// Update the updated_at timestamp for a review.
    fn touch(&self, storage_id: &str) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE reviews SET updated_at = ? WHERE id = ?",
            params![now, storage_id],
        )?;
        Ok(())
    }
}

impl ReviewStore for SqliteStore {
    fn get(&self, id: &DiffId) -> Result<Option<Review>, StoreError> {
        let storage_id = Self::storage_id(id);
        let conn = self.conn.lock().unwrap();

        let review_row: Option<(String, String, String, String)> = conn
            .query_row(
                "SELECT base, head, created_at, updated_at FROM reviews WHERE id = ?",
                [&storage_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;

        let Some((base, head, created_str, updated_str)) = review_row else {
            return Ok(None);
        };

        let reviewed = Self::load_reviewed_files(&conn, &storage_id)?;
        let comments = Self::load_comments(&conn, &storage_id)?;
        let edits = Self::load_edits(&conn, &storage_id)?;

        Ok(Some(Review {
            id: DiffId::new(base, head),
            reviewed,
            comments,
            edits,
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }))
    }

    fn get_or_create(&self, id: &DiffId) -> Result<Review, StoreError> {
        if let Some(review) = self.get(id)? {
            return Ok(review);
        }

        let now = Utc::now();
        let review = Review {
            id: id.clone(),
            reviewed: HashSet::new(),
            comments: Vec::new(),
            edits: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        self.save(&review)?;
        Ok(review)
    }

    fn save(&self, review: &Review) -> Result<(), StoreError> {
        let storage_id = Self::storage_id(&review.id);
        let conn = self.conn.lock().unwrap();

        // Upsert review
        conn.execute(
            r#"
            INSERT INTO reviews (id, base, head, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET updated_at = excluded.updated_at
            "#,
            params![
                storage_id,
                review.id.base,
                review.id.head,
                review.created_at.to_rfc3339(),
                review.updated_at.to_rfc3339(),
            ],
        )?;

        // Replace reviewed files
        conn.execute(
            "DELETE FROM reviewed_files WHERE review_id = ?",
            [&storage_id],
        )?;
        for file_path in &review.reviewed {
            conn.execute(
                "INSERT INTO reviewed_files (review_id, file_path) VALUES (?, ?)",
                params![storage_id, file_path],
            )?;
        }

        // Replace comments
        conn.execute("DELETE FROM comments WHERE review_id = ?", [&storage_id])?;
        for comment in &review.comments {
            conn.execute(
                "INSERT INTO comments (id, review_id, file_path, range_index, text, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    comment.id.to_string(),
                    storage_id,
                    comment.file_path,
                    comment.range_index as i64,
                    comment.text,
                    comment.created_at.to_rfc3339(),
                ],
            )?;
        }

        // Replace edits
        conn.execute("DELETE FROM edits WHERE review_id = ?", [&storage_id])?;
        for edit in &review.edits {
            conn.execute(
                "INSERT INTO edits (id, review_id, file_path, diff, created_at) VALUES (?, ?, ?, ?, ?)",
                params![
                    edit.id.to_string(),
                    storage_id,
                    edit.file_path,
                    edit.diff,
                    edit.created_at.to_rfc3339(),
                ],
            )?;
        }

        Ok(())
    }

    fn delete(&self, id: &DiffId) -> Result<(), StoreError> {
        let storage_id = Self::storage_id(id);
        let conn = self.conn.lock().unwrap();
        // Foreign key cascades handle child tables
        conn.execute("DELETE FROM reviews WHERE id = ?", [&storage_id])?;
        Ok(())
    }

    fn add_comment(&self, id: &DiffId, comment: &Comment) -> Result<(), StoreError> {
        let storage_id = self.ensure_review(id)?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO comments (id, review_id, file_path, range_index, text, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                comment.id.to_string(),
                storage_id,
                comment.file_path,
                comment.range_index as i64,
                comment.text,
                comment.created_at.to_rfc3339(),
            ],
        )?;

        drop(conn);
        self.touch(&storage_id)?;
        Ok(())
    }

    fn delete_comment(&self, id: &DiffId, comment_id: Uuid) -> Result<(), StoreError> {
        let storage_id = Self::storage_id(id);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM comments WHERE review_id = ? AND id = ?",
            params![storage_id, comment_id.to_string()],
        )?;
        drop(conn);
        self.touch(&storage_id)?;
        Ok(())
    }

    fn mark_reviewed(&self, id: &DiffId, file_path: &str) -> Result<(), StoreError> {
        let storage_id = self.ensure_review(id)?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO reviewed_files (review_id, file_path) VALUES (?, ?)",
            params![storage_id, file_path],
        )?;

        drop(conn);
        self.touch(&storage_id)?;
        Ok(())
    }

    fn unmark_reviewed(&self, id: &DiffId, file_path: &str) -> Result<(), StoreError> {
        let storage_id = Self::storage_id(id);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM reviewed_files WHERE review_id = ? AND file_path = ?",
            params![storage_id, file_path],
        )?;
        drop(conn);
        self.touch(&storage_id)?;
        Ok(())
    }

    fn add_edit(&self, id: &DiffId, edit: &Edit) -> Result<(), StoreError> {
        let storage_id = self.ensure_review(id)?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO edits (id, review_id, file_path, diff, created_at) VALUES (?, ?, ?, ?, ?)",
            params![
                edit.id.to_string(),
                storage_id,
                edit.file_path,
                edit.diff,
                edit.created_at.to_rfc3339(),
            ],
        )?;

        drop(conn);
        self.touch(&storage_id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct TestStore {
        store: SqliteStore,
        _dir: TempDir, // Keep the dir alive
    }

    fn test_store() -> TestStore {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let store = SqliteStore::open(path).unwrap();
        TestStore { store, _dir: dir }
    }

    #[test]
    fn test_create_and_get_review() {
        let ts = test_store();
        let store = &ts.store;
        let id = DiffId::new("abc123", "@");

        // Initially no review
        assert!(store.get(&id).unwrap().is_none());

        // Create one
        let review = store.get_or_create(&id).unwrap();
        assert_eq!(review.id, id);
        assert!(review.reviewed.is_empty());
        assert!(review.comments.is_empty());

        // Get it back
        let review2 = store.get(&id).unwrap().unwrap();
        assert_eq!(review2.id, id);
    }

    #[test]
    fn test_mark_reviewed() {
        let ts = test_store();
        let store = &ts.store;
        let id = DiffId::new("abc123", "@");

        store.mark_reviewed(&id, "src/foo.rs").unwrap();
        store.mark_reviewed(&id, "src/bar.rs").unwrap();

        let review = store.get(&id).unwrap().unwrap();
        assert!(review.reviewed.contains("src/foo.rs"));
        assert!(review.reviewed.contains("src/bar.rs"));

        store.unmark_reviewed(&id, "src/foo.rs").unwrap();
        let review = store.get(&id).unwrap().unwrap();
        assert!(!review.reviewed.contains("src/foo.rs"));
        assert!(review.reviewed.contains("src/bar.rs"));
    }

    #[test]
    fn test_comments() {
        let ts = test_store();
        let store = &ts.store;
        let id = DiffId::new("abc123", "@");

        let comment = Comment::new("src/foo.rs", 0, "Fix this bug");
        store.add_comment(&id, &comment).unwrap();

        let review = store.get(&id).unwrap().unwrap();
        assert_eq!(review.comments.len(), 1);
        assert_eq!(review.comments[0].text, "Fix this bug");

        store.delete_comment(&id, comment.id).unwrap();
        let review = store.get(&id).unwrap().unwrap();
        assert!(review.comments.is_empty());
    }

    #[test]
    fn test_edits() {
        let ts = test_store();
        let store = &ts.store;
        let id = DiffId::new("abc123", "@");

        let edit = Edit::new(
            "src/foo.rs",
            "--- a/src/foo.rs\n+++ b/src/foo.rs\n@@ -1 +1 @@\n-old\n+new",
        );
        store.add_edit(&id, &edit).unwrap();

        let review = store.get(&id).unwrap().unwrap();
        assert_eq!(review.edits.len(), 1);
        assert!(review.edits[0].diff.contains("-old"));
    }

    #[test]
    fn test_delete_review() {
        let ts = test_store();
        let store = &ts.store;
        let id = DiffId::new("abc123", "@");

        store.mark_reviewed(&id, "src/foo.rs").unwrap();
        store
            .add_comment(&id, &Comment::new("src/foo.rs", 0, "test"))
            .unwrap();

        store.delete(&id).unwrap();
        assert!(store.get(&id).unwrap().is_none());
    }
}
