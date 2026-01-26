//! Agent system for Staged
//!
//! Provides AI agent integration via ACP (Agent Client Protocol).
//! Currently supports Goose.

pub mod acp_client;
pub mod manager;
pub mod registry;
pub mod types;

pub use manager::{AgentManager, ConnectionCommand, ConnectionManager};
pub use types::*;
