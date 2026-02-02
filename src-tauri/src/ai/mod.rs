//! AI integration via ACP (Agent Client Protocol).
//!
//! This module provides a single integration point for running AI agents.
//! All AI generation in Staged goes through this module, enabling consistent
//! inspection and debugging of AI interactions.
//!
//! ## Architecture
//!
//! - `client.rs` - Core ACP client implementation (agent discovery, session management)
//! - `legacy/` - Reference code for diff analysis prompts (not wired up, kept for reference)
//!
//! ## Usage
//!
//! ```ignore
//! use crate::ai::{find_acp_agent, run_acp_prompt};
//!
//! let agent = find_acp_agent().ok_or("No AI agent found")?;
//! let response = run_acp_prompt(&agent, &working_dir, "Your prompt here").await?;
//! ```

mod client;
mod legacy; // Reference code only - not exposed or used

// Re-export core ACP client functionality
pub use client::{
    discover_acp_providers, find_acp_agent, find_acp_agent_by_id, run_acp_prompt,
    run_acp_prompt_streaming, run_acp_prompt_with_session, AcpAgent, AcpPromptResult,
    AcpProviderInfo, FinalizedMessage, SessionCompleteEvent, SessionErrorEvent, ToolCallSummary,
};
