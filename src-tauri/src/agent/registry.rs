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
