//! Repository action discovery and execution.
//!
//! Uses AI to discover available commands/scripts in a repository and
//! provides execution with streaming output.

mod executor;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub use executor::{run_action, stop_action, ActionExecutionHandle, RUNNING_ACTIONS};

/// Categories for repository actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionCategory {
    Build,
    Clean,
    Setup,
    Run,
    Test,
    Lint,
    Other,
}

/// A discovered action in a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoAction {
    pub id: String,
    pub name: String,
    pub command: String,
    pub category: ActionCategory,
    /// Priority 1-5, higher = more important/commonly used
    pub priority: u8,
    pub description: String,
}

/// Configuration files to scan for actions.
const CONFIG_FILES: &[&str] = &[
    "package.json",
    "Makefile",
    "Cargo.toml",
    "pyproject.toml",
    "docker-compose.yml",
    "docker-compose.yaml",
    "justfile",
    "Taskfile.yml",
    "Taskfile.yaml",
    "deno.json",
    "bun.lockb", // indicates bun project
];

/// Read relevant config files from the repository for action discovery.
fn read_config_files(repo_path: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();

    for filename in CONFIG_FILES {
        let path = repo_path.join(filename);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                // Limit content size to avoid huge prompts
                let truncated = if content.len() > 10000 {
                    format!("{}...[truncated]", &content[..10000])
                } else {
                    content
                };
                files.push((filename.to_string(), truncated));
            }
        }
    }

    files
}

/// Build the AI prompt for action discovery.
fn build_discovery_prompt(config_files: &[(String, String)]) -> String {
    let mut prompt = String::from(
        r#"Analyze this repository and discover available actions/commands that a developer would commonly run.

For each action found, provide:
- id: A unique kebab-case identifier (e.g., "build-dev", "test-unit")
- name: Human-readable name (e.g., "Build Dev", "Run Unit Tests")
- command: The shell command to execute (e.g., "just build", "cargo test")
- category: One of: build, clean, setup, run, test, lint, other
- priority: 1-5 where 5 = most commonly used/important
- description: Brief description of what this action does

Categories:
- build: Compilation, bundling, building artifacts
- clean: Cleanup commands (removing build artifacts, caches)
- setup: Installation, initialization (npm install, pip install)
- run: Start/serve commands (dev servers, running the app)
- test: Running tests
- lint: Code quality, formatting, linting
- other: Anything else

IMPORTANT rules:
- Deduplication: If the same action is available from multiple sources (e.g., both `pnpm run build` and `just build`), include ONLY the one from the centralized command tool.
- Trust centralized command tools in this order of preference: just > make > task > package manager scripts (npm/pnpm/yarn/bun)
- The centralized tools often wrap the underlying commands with additional setup, so prefer them.
- Only include package manager scripts if there's no equivalent in a centralized tool.
- SKIP any deploy/release commands that target non-dev environments (production, staging, etc.). Only include deploy commands if they explicitly target local or dev environments.

Look for actions in these files:
"#,
    );

    for (filename, content) in config_files {
        prompt.push_str(&format!("\n=== {} ===\n{}\n", filename, content));
    }

    prompt.push_str(
        r#"

Return ONLY a JSON array with the discovered actions. No other text.
Example (assuming justfile exists with these recipes):
[
  {"id": "dev", "name": "Start Dev Server", "command": "just dev", "category": "run", "priority": 5, "description": "Start the development server with hot reload"},
  {"id": "build", "name": "Build", "command": "just build", "category": "build", "priority": 4, "description": "Build the production bundle"},
  {"id": "test", "name": "Run Tests", "command": "just test", "category": "test", "priority": 4, "description": "Run the test suite"}
]
"#,
    );

    prompt
}

/// Parse the AI response into a list of actions.
fn parse_actions_response(response: &str) -> Result<Vec<RepoAction>, String> {
    // Try to extract JSON from the response (may be wrapped in markdown code blocks)
    let json_str = extract_json(response);

    serde_json::from_str(json_str).map_err(|e| {
        log::error!("Failed to parse actions response: {}", e);
        log::error!("Response was: {}", response);
        format!("Failed to parse AI response: {}", e)
    })
}

/// Extract JSON from a response that may contain markdown code blocks.
fn extract_json(response: &str) -> &str {
    let response = response.trim();

    // Strategy 1: Check for ```json ... ``` pattern
    if let Some(start) = response.find("```json") {
        let after_fence = &response[start + 7..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim();
        }
    }

    // Strategy 2: Check for ``` ... ``` pattern (no language)
    if let Some(start) = response.find("```") {
        let after_fence = &response[start + 3..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim();
        }
    }

    // Strategy 3: Find JSON array directly
    if let Some(start) = response.find('[') {
        if let Some(end) = response.rfind(']') {
            return &response[start..=end];
        }
    }

    response
}

/// Discover actions in a repository using AI.
pub async fn discover_actions(repo_path: &Path) -> Result<Vec<RepoAction>, String> {
    // Read config files
    let config_files = read_config_files(repo_path);

    if config_files.is_empty() {
        log::info!("No config files found in {:?}", repo_path);
        return Ok(Vec::new());
    }

    log::info!(
        "Found {} config files: {:?}",
        config_files.len(),
        config_files.iter().map(|(f, _)| f).collect::<Vec<_>>()
    );

    // Build prompt
    let prompt = build_discovery_prompt(&config_files);

    // Find an ACP agent
    let agent = crate::ai::find_acp_agent()
        .ok_or_else(|| "No AI agent found. Install Goose or Claude Code.".to_string())?;

    // Call AI (no session needed - one-shot query)
    let result = crate::ai::run_acp_prompt(&agent, repo_path, &prompt).await?;

    // Parse response
    let actions = parse_actions_response(&result)?;

    log::info!("Discovered {} actions", actions.len());

    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_actions_response_plain_json() {
        let response = r#"[{"id": "dev", "name": "Dev", "command": "npm run dev", "category": "run", "priority": 5, "description": "Start dev server"}]"#;
        let actions = parse_actions_response(response).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, "dev");
        assert_eq!(actions[0].category, ActionCategory::Run);
    }

    #[test]
    fn test_parse_actions_response_code_block() {
        let response = r#"Here are the actions:
```json
[{"id": "build", "name": "Build", "command": "cargo build", "category": "build", "priority": 4, "description": "Build the project"}]
```
"#;
        let actions = parse_actions_response(response).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, "build");
    }

    #[test]
    fn test_extract_json_with_text_around() {
        let response = "Sure, here are the actions:\n[{\"id\": \"test\"}]\nThat's all!";
        let json = extract_json(response);
        assert_eq!(json, r#"[{"id": "test"}]"#);
    }
}
