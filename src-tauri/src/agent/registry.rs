//! Agent registry - manages available agents
//!
//! This module provides a registry of known ACP-compatible agents.
//! Currently only Goose is supported.

use std::collections::HashMap;

use crate::agent::types::AgentConfig;

/// Registry of available agents
pub struct AgentRegistry {
    agents: HashMap<String, AgentConfig>,
}

impl AgentRegistry {
    /// Create a new registry with default agents
    pub fn new() -> Self {
        let mut registry = Self {
            agents: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    /// Register the default agents
    fn register_defaults(&mut self) {
        // Goose - Block's AI agent (via ACP)
        self.register(AgentConfig {
            id: "goose".to_string(),
            name: "Goose".to_string(),
            description: "Block's open-source AI developer agent".to_string(),
            command: "goose".to_string(),
            args: vec!["acp".to_string()],
            env: vec![],
            icon: Some("goose".to_string()),
            enabled: true,
            api_key_env_var: None, // Goose manages its own API keys
            version_check_arg: None,
        });

        // Claude Code - Anthropic's AI coding agent (via Zed's ACP adapter)
        // Requires: npm install -g @anthropic-ai/claude-code-acp
        // Requires: Claude CLI auth or ANTHROPIC_API_KEY environment variable
        self.register(AgentConfig {
            id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            description: "Anthropic's AI coding assistant".to_string(),
            command: "claude-code-acp".to_string(),
            args: vec![],
            env: vec![],
            icon: Some("claude".to_string()),
            enabled: true,
            api_key_env_var: Some("ANTHROPIC_API_KEY".to_string()),
            version_check_arg: None,
        });
    }

    /// Register an agent configuration
    pub fn register(&mut self, config: AgentConfig) {
        let id = config.id.clone();
        self.agents.insert(id, config);
    }

    /// Get an agent by ID
    pub fn get(&self, id: &str) -> Option<&AgentConfig> {
        self.agents.get(id)
    }

    /// List all registered agents
    pub fn list(&self) -> Vec<&AgentConfig> {
        self.agents.values().collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
