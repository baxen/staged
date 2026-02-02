//! Chat session persistence using SQLite.
//!
//! Stores finalized chat sessions, messages, and tool calls.
//! The SessionManager handles live state; this handles history.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

// =============================================================================
// Types
// =============================================================================

/// A chat session (conversation with an AI agent)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub id: String,
    pub working_dir: String,
    pub agent_id: String,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A message in a chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: i64,
    pub session_id: String,
    pub role: MessageRole,
    /// For user messages: plain text
    /// For assistant messages: JSON array of ContentSegment
    pub content: String,
    pub created_at: i64,
}

/// A segment of assistant content (text or tool call), stored in order
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentSegment {
    Text {
        text: String,
    },
    ToolCall {
        id: String,
        title: String,
        status: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        locations: Vec<String>,
    },
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "user" => MessageRole::User,
            _ => MessageRole::Assistant,
        }
    }
}

/// Full session with messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionFull {
    pub session: ChatSession,
    pub messages: Vec<ChatMessage>,
}

// =============================================================================
// Error type
// =============================================================================

#[derive(Debug)]
pub struct ChatStoreError(pub String);

impl ChatStoreError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl std::fmt::Display for ChatStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ChatStoreError {}

impl From<rusqlite::Error> for ChatStoreError {
    fn from(e: rusqlite::Error) -> Self {
        ChatStoreError(e.to_string())
    }
}

impl From<serde_json::Error> for ChatStoreError {
    fn from(e: serde_json::Error) -> Self {
        ChatStoreError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ChatStoreError>;

// =============================================================================
// ChatStore
// =============================================================================

/// SQLite-backed storage for chat sessions
pub struct ChatStore {
    conn: Mutex<Connection>,
}

impl ChatStore {
    /// Open or create the chat database at the given path
    pub fn open(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ChatStoreError::new(format!("Cannot create directory: {}", e)))?;
        }

        let conn = Connection::open(&db_path)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            -- Chat sessions table
            CREATE TABLE IF NOT EXISTS chat_sessions (
                id TEXT PRIMARY KEY,
                working_dir TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                title TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            -- Chat messages table
            CREATE TABLE IF NOT EXISTS chat_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            -- Tool calls table
            CREATE TABLE IF NOT EXISTS chat_tool_calls (
                id TEXT PRIMARY KEY,
                message_id INTEGER NOT NULL REFERENCES chat_messages(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                locations_json TEXT NOT NULL DEFAULT '[]'
            );

            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_chat_tool_calls_message ON chat_tool_calls(message_id);
            CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated ON chat_sessions(updated_at DESC);

            PRAGMA foreign_keys = ON;
            "#,
        )?;

        Ok(())
    }

    // =========================================================================
    // Session operations
    // =========================================================================

    /// Create a new chat session
    pub fn create_session(&self, session: &ChatSession) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chat_sessions (id, working_dir, agent_id, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &session.id,
                &session.working_dir,
                &session.agent_id,
                &session.title,
                session.created_at,
                session.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Result<Option<ChatSession>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, working_dir, agent_id, title, created_at, updated_at
             FROM chat_sessions WHERE id = ?1",
            params![id],
            |row| {
                Ok(ChatSession {
                    id: row.get(0)?,
                    working_dir: row.get(1)?,
                    agent_id: row.get(2)?,
                    title: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    /// List all sessions, ordered by most recently updated
    pub fn list_sessions(&self) -> Result<Vec<ChatSession>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, working_dir, agent_id, title, created_at, updated_at
             FROM chat_sessions ORDER BY updated_at DESC",
        )?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(ChatSession {
                    id: row.get(0)?,
                    working_dir: row.get(1)?,
                    agent_id: row.get(2)?,
                    title: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    /// List sessions for a specific working directory
    pub fn list_sessions_for_dir(&self, working_dir: &str) -> Result<Vec<ChatSession>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, working_dir, agent_id, title, created_at, updated_at
             FROM chat_sessions WHERE working_dir = ?1 ORDER BY updated_at DESC",
        )?;
        let sessions = stmt
            .query_map(params![working_dir], |row| {
                Ok(ChatSession {
                    id: row.get(0)?,
                    working_dir: row.get(1)?,
                    agent_id: row.get(2)?,
                    title: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(sessions)
    }

    /// Update session title
    pub fn update_session_title(&self, id: &str, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = now_timestamp();
        conn.execute(
            "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, now, id],
        )?;
        Ok(())
    }

    /// Touch session (update updated_at)
    pub fn touch_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = now_timestamp();
        conn.execute(
            "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Delete a session and all its messages
    pub fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM chat_sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    // =========================================================================
    // Message operations
    // =========================================================================

    /// Add a message to a session, returns the message ID
    pub fn add_message(&self, session_id: &str, role: MessageRole, content: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = now_timestamp();

        conn.execute(
            "INSERT INTO chat_messages (session_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role.as_str(), content, now],
        )?;

        let message_id = conn.last_insert_rowid();

        // Update session's updated_at
        conn.execute(
            "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, session_id],
        )?;

        Ok(message_id)
    }

    /// Get all messages for a session
    pub fn get_messages(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, created_at
             FROM chat_messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;
        let messages = stmt
            .query_map(params![session_id], |row| {
                let role_str: String = row.get(2)?;
                Ok(ChatMessage {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: MessageRole::parse(&role_str),
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(messages)
    }

    // =========================================================================
    // Tool call operations
    // =========================================================================
    // Combined operations
    // =========================================================================

    /// Get full session with all messages
    /// Assistant message content is JSON array of ContentSegment
    pub fn get_session_full(&self, id: &str) -> Result<Option<ChatSessionFull>> {
        let session = match self.get_session(id)? {
            Some(s) => s,
            None => return Ok(None),
        };

        let messages = self.get_messages(id)?;
        Ok(Some(ChatSessionFull { session, messages }))
    }

    /// Add an assistant turn with ordered segments (text + tool calls interleaved)
    pub fn add_assistant_turn(&self, session_id: &str, segments: &[ContentSegment]) -> Result<i64> {
        // Serialize segments to JSON for storage
        let content = serde_json::to_string(segments)?;
        self.add_message(session_id, MessageRole::Assistant, &content)
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Get current timestamp in milliseconds
fn now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Generate a unique session ID
pub fn generate_session_id() -> String {
    let timestamp = now_timestamp();
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("chat_{:x}_{:x}", timestamp, count)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_get_session() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ChatStore::open(db_path).unwrap();

        let now = now_timestamp();
        let session = ChatSession {
            id: "test-session".to_string(),
            working_dir: "/tmp/repo".to_string(),
            agent_id: "goose".to_string(),
            title: Some("Test Session".to_string()),
            created_at: now,
            updated_at: now,
        };

        store.create_session(&session).unwrap();
        let retrieved = store.get_session("test-session").unwrap().unwrap();

        assert_eq!(retrieved.id, "test-session");
        assert_eq!(retrieved.title, Some("Test Session".to_string()));
    }

    #[test]
    fn test_add_and_get_messages() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ChatStore::open(db_path).unwrap();

        let now = now_timestamp();
        let session = ChatSession {
            id: "test-session".to_string(),
            working_dir: "/tmp/repo".to_string(),
            agent_id: "goose".to_string(),
            title: None,
            created_at: now,
            updated_at: now,
        };
        store.create_session(&session).unwrap();

        store
            .add_message("test-session", MessageRole::User, "Hello")
            .unwrap();
        store
            .add_message("test-session", MessageRole::Assistant, "Hi there!")
            .unwrap();

        let messages = store.get_messages("test-session").unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_assistant_segments() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ChatStore::open(db_path).unwrap();

        let now = now_timestamp();
        let session = ChatSession {
            id: "test-session".to_string(),
            working_dir: "/tmp/repo".to_string(),
            agent_id: "goose".to_string(),
            title: None,
            created_at: now,
            updated_at: now,
        };
        store.create_session(&session).unwrap();

        // Interleaved segments: text, tool, text
        let segments = vec![
            ContentSegment::Text {
                text: "Let me read that file.".to_string(),
            },
            ContentSegment::ToolCall {
                id: "tc1".to_string(),
                title: "Read file".to_string(),
                status: "completed".to_string(),
                locations: vec!["src/main.rs".to_string()],
            },
            ContentSegment::Text {
                text: "Here's what I found.".to_string(),
            },
        ];

        store
            .add_message("test-session", MessageRole::User, "Read main.rs")
            .unwrap();
        store.add_assistant_turn("test-session", &segments).unwrap();

        let full = store.get_session_full("test-session").unwrap().unwrap();
        assert_eq!(full.messages.len(), 2);

        // Parse the assistant message content as segments
        let loaded_segments: Vec<ContentSegment> =
            serde_json::from_str(&full.messages[1].content).unwrap();
        assert_eq!(loaded_segments.len(), 3);

        // Verify order is preserved
        assert!(
            matches!(&loaded_segments[0], ContentSegment::Text { text } if text == "Let me read that file.")
        );
        assert!(
            matches!(&loaded_segments[1], ContentSegment::ToolCall { title, .. } if title == "Read file")
        );
        assert!(
            matches!(&loaded_segments[2], ContentSegment::Text { text } if text == "Here's what I found.")
        );
    }

    #[test]
    fn test_delete_session_cascades() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ChatStore::open(db_path).unwrap();

        let now = now_timestamp();
        let session = ChatSession {
            id: "test-session".to_string(),
            working_dir: "/tmp/repo".to_string(),
            agent_id: "goose".to_string(),
            title: None,
            created_at: now,
            updated_at: now,
        };
        store.create_session(&session).unwrap();
        store
            .add_message("test-session", MessageRole::User, "Hello")
            .unwrap();

        store.delete_session("test-session").unwrap();

        let retrieved = store.get_session("test-session").unwrap();
        assert!(retrieved.is_none());

        // Messages should be deleted too (cascade)
        let messages = store.get_messages("test-session").unwrap();
        assert!(messages.is_empty());
    }
}
