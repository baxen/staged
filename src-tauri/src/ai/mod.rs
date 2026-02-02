//! AI integration via ACP (Agent Client Protocol).
//!
//! This module provides AI chat functionality with persistent sessions.
//!
//! ## Architecture
//!
//! - `chat_store.rs` - SQLite persistence for sessions, messages, tool calls
//! - `session.rs` - SessionManager for live agent connections + streaming
//! - `client.rs` - Core ACP client implementation (agent discovery, protocol)
//! - `legacy/` - Reference code for diff analysis prompts (not wired up)
//!
//! ## Data Flow
//!
//! 1. Frontend calls `create_chat_session` → creates in SQLite + live session
//! 2. Frontend calls `send_chat_prompt` → stores user message, streams response
//! 3. On turn complete → assistant message + tool calls persisted to SQLite
//! 4. Frontend can `get_chat_session` to load full history from SQLite
//!
//! Live sessions (agent connections) are ephemeral. History survives app restart.

pub mod chat_store;
mod client;
mod legacy; // Reference code only - not exposed or used
pub mod session;

// Re-export core ACP client functionality
pub use client::{
    discover_acp_providers, find_acp_agent, find_acp_agent_by_id, run_acp_prompt,
    run_acp_prompt_streaming, run_acp_prompt_with_session, AcpAgent, AcpPromptResult,
    AcpProviderInfo,
};

// Re-export chat store types
pub use chat_store::{
    ChatMessage, ChatSession, ChatSessionFull, ChatStore, ContentSegment, MessageRole,
};

// Re-export session manager types
pub use session::{LiveSessionInfo, SessionManager, SessionStatus, SessionStatusEvent};
