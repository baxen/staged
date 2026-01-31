# AI Integration Architecture

## Overview

Staged integrates AI features through the **Agent Client Protocol (ACP)**, supporting both Goose and Claude Code agents. The architecture spans frontend UI, Tauri backend, and external AI agents connected via stdio-based JSON-RPC communication.

---

## Table of Contents

1. [AI-Related API Endpoints](#1-ai-related-api-endpoints)
2. [Frontend Components](#2-frontend-components)
3. [Communication Protocol](#3-communication-protocol)
4. [State Management](#4-state-management)
5. [AI Service Clients](#5-ai-service-clients)
6. [Request/Response Flow](#6-requestresponse-flow)
7. [Artifacts & Sessions](#7-artifacts--sessions)
8. [Key Architectural Patterns](#8-key-architectural-patterns)
9. [Integration Points Summary](#9-integration-points-summary)

---

## 1. AI-Related API Endpoints

All AI functionality is exposed through Tauri command handlers in `src-tauri/src/lib.rs`.

### Core AI Commands

**Agent Discovery & Availability** (`lib.rs:512-538`)
```rust
discover_acp_providers() -> Vec<AcpProviderInfo>
  // Returns list of available agents (Goose, Claude)
  // Searches login shell, direct commands, common paths

check_ai_available() -> Result<String>
  // Returns name of available agent or error
  // Fast check before running analysis
```

**Diff Analysis** (`lib.rs:540-555`)
```rust
analyze_diff(repoPath: String, spec: DiffSpec) -> Result<ChangesetAnalysis>
  // Main entry point: analyzes entire diff with AI
  // Returns changeset summary + per-file annotations
  // Handles tiered prompt strategy for large changesets
```

**Agent Chat Sessions** (`lib.rs:557-585`)
```rust
send_agent_prompt(
  repoPath: String,
  prompt: String,
  sessionId?: String,
  provider?: String
) -> Result<AgentPromptResponse>
  // Send prompt to AI agent with optional session resumption
  // Returns: { response: string, sessionId: string }
  // Session continuity enables multi-turn conversations
```

**AI Analysis Persistence** (`lib.rs:592-717`)
```rust
// Save/load analysis results to SQLite
save_changeset_summary(repoPath, spec, summary)
get_changeset_summary(repoPath, spec) -> Option<ChangesetSummary>

save_file_analysis(repoPath, spec, filePath, result)
get_file_analysis(repoPath, spec, filePath) -> Option<SmartDiffResult>
get_all_file_analyses(repoPath, spec) -> Vec<(String, SmartDiffResult)>

delete_all_analyses(repoPath, spec)

// Convert AI annotations to persistent comments
save_ai_comments(repoPath, spec, annotations)
  // Only saves warnings & suggestions as comments
  // Explanations & context remain as blur overlays
```

---

## 2. Frontend Components

### AI Service Layer (`src/lib/services/ai.ts`)

Thin wrapper over Tauri invoke commands:

```typescript
export async function checkAiAvailable(): Promise<string>
export async function analyzeDiff(repoPath: string, spec: DiffSpec): Promise<ChangesetAnalysis>
export async function discoverAcpProviders(): Promise<AcpProviderInfo[]>
export async function sendAgentPrompt(
  repoPath: string,
  prompt: string,
  sessionId?: string,
  provider?: string
): Promise<AgentPromptResponse>

// Persistence functions
export async function saveChangesetSummary(...)
export async function getAllFileAnalyses(...)
export async function saveAiComments(...)
```

**Key structures:**
```typescript
interface AcpProviderInfo {
  id: string;      // 'goose' or 'claude'
  label: string;   // Display name
}

interface AgentPromptResponse {
  response: string;
  sessionId: string;  // For session resumption
}

interface ChangesetAnalysis {
  summary: string;
  key_changes: string[];
  concerns: string[];
  file_annotations: Record<string, SmartDiffAnnotation[]>;
}
```

### Smart Diff Store (`src/lib/stores/smartDiff.svelte.ts`)

State management for AI diff analysis:

```typescript
interface SmartDiffState {
  results: Map<string, SmartDiffResult>    // Per-file annotations
  loading: boolean                          // Analysis in progress
  aiAvailable: boolean | null              // Cached availability
  aiToolName: string | null                // 'goose' or 'claude'
  showAnnotations: boolean                 // Global toggle
  changesetSummary: ChangesetSummary | null
  annotationsRevealed: boolean             // Reveal via 'A' key
}

// Key functions
runAnalysis(repoPath, spec)               // Main analysis trigger
loadAnalysisFromDb(repoPath, spec)        // Load cached results
deleteAnalysis(repoPath, spec)            // Clear from DB
getFileResult(filePath)                   // Get file's annotations
setAnnotationsRevealed(revealed)          // Toggle via keyboard
```

### Agent Chat State (`src/lib/stores/agent.svelte.ts`)

Per-tab chat session management:

```typescript
class AgentState {
  input: string                      // Current user input
  response: string                   // AI response
  loading: boolean                   // Waiting for response
  error: string                      // Error message
  sessionId: string | null           // Current session ID (for continuity)
  provider: AcpProvider              // 'goose' or 'claude'
  artifacts: Artifact[]              // Saved responses (plans, summaries)
  task: string | null                // Original task for context in follow-ups
}

// Global provider discovery
agentGlobalState: {
  availableProviders: AcpProviderInfo[]
  providersLoaded: boolean
}

// Factory functions
createAgentState()                    // Create new tab session
generateArtifactId()                  // UUID generation
```

### AgentPanel Component (`src/lib/features/agent/AgentPanel.svelte`)

Interactive chat UI (1,319 lines):

**Features:**
- Auto-resizing textarea input
- Provider selector (Goose vs Claude)
- Session management (new, resume, discard)
- Artifact viewing and deletion
- Context inclusion controls:
  - File list (current changeset)
  - Viewing file context
  - Previous artifacts selection
  - Review comments inclusion

**Prompt building with context tags:**
```
[Changeset: file1.rs, file2.rs (+2 more)]
[Viewing: src/main.rs]
[Reference artifacts: ...]
[Code Comments from Review: ...]
[Original task: ...]
```

**Key handlers:**
```typescript
handleSubmit()                   // Send prompt, manage session
buildPromptWithContext()         // Add context for new/follow-up sessions
saveAsArtifact()                // Convert response to artifact
discardResponse()               // Clear response, end session
```

### Annotation Overlay Components

**AnnotationOverlay.svelte** - Blur overlay on annotated code:
- Shows AI commentary on specific lines
- Category-based colors (explanation, warning, suggestion, context)
- Revealed via 'A' key
- Backdrop blur effect

**BeforeAnnotationOverlay.svelte** - Similar for "before" content

**SmartDiffModal.svelte** - Changeset summary modal:
- Shows changeset summary
- Actions: Save as artifact, Refresh, Discard
- Converts summary to markdown artifact format

---

## 3. Communication Protocol

### ACP (Agent Client Protocol)

**No WebSocket implementation.** Instead: JSON-RPC 2.0 over stdin/stdout.

### Backend Implementation (`src-tauri/src/ai/acp_client.rs`, 484 lines)

**Key structures:**
```rust
enum AcpAgent {
  Goose(PathBuf),
  Claude(PathBuf),
}

struct AcpPromptResult {
  response: String,
  session_id: String,  // For resuming
}

async fn run_acp_prompt_with_session(
  agent: &AcpAgent,
  working_dir: &Path,
  prompt: &str,
  session_id: Option<&str>,
) -> Result<AcpPromptResult, String>
```

**Session flow:**
```
1. Spawn agent process in ACP mode
2. Initialize ACP connection with version negotiation
3. Load existing session or create new
4. Send prompt request
5. Collect AgentMessageChunk notifications in real-time
6. Kill process on completion
```

**Response collection:**
```rust
struct StagedAcpClient {
  collector: Arc<ResponseCollector>,
}

async fn session_notification(&self, notification: SessionNotification) {
  // Accumulate text chunks in collector.accumulated_content
  if let SessionUpdate::AgentMessageChunk(chunk) = &notification.update {
    if let AcpContentBlock::Text(text) = &chunk.content {
      accumulated.push_str(&text.text);  // Stream to UI
    }
  }
}
```

**Threading model:**
- ACP runs in dedicated blocking thread with LocalSet (handles !Send futures)
- Prevents blocking main Tauri async runtime
- Called from `run_acp_prompt_with_session()` async function

**Command-line invocation:**
```rust
impl AcpAgent {
  pub fn acp_args(&self) -> Vec<&str> {
    match self {
      // Goose: enable builtin tools
      AcpAgent::Goose(_) => vec!["acp", "--with-builtin", "developer,extensionmanager"],
      // Claude: runs in ACP mode by default
      AcpAgent::Claude(_) => vec![],
    }
  }
}
```

---

## 4. State Management

### Three-Tier Architecture

```
Global State (agentGlobalState)
  ├── availableProviders: AcpProviderInfo[]
  └── providersLoaded: boolean

Tab State (AgentState - per tab)
  ├── input, response, loading, error
  ├── sessionId (for continuity)
  ├── provider ('goose' | 'claude')
  └── artifacts: Artifact[]

View State (smartDiffState)
  ├── results: Map<filePath, SmartDiffResult>
  ├── changesetSummary
  ├── aiAvailable, loading
  └── annotationsRevealed
```

### Session Continuity Model

**Database persistence (SQLite via `src-tauri/src/review/mod.rs`):**

```rust
pub struct Artifact {
  pub id: String,
  pub title: String,
  pub content: String,
  pub created_at: String,
}

// Stored per DiffId in SQLite
```

**Session resumption flow:**
1. User opens new tab with same diff
2. `AgentPanel.onMount()` calls `loadArtifactsFromDb(spec)`
3. Artifacts loaded and displayed
4. User can select artifacts for context in next chat
5. Send prompt to agent with `sessionId`:
   - Agent loads existing session from its database
   - Continues conversation in that context
   - Returns same `sessionId` for further resumption

---

## 5. AI Service Clients

### ACP Client (`src-tauri/src/ai/acp_client.rs`)

**Agent discovery:**

```rust
pub fn discover_acp_providers() -> Vec<AcpProviderInfo>
  // Searches for: goose, claude-code-acp
  // Strategies: login shell, direct command, common paths

pub fn find_acp_agent() -> Option<AcpAgent>
  // Prefers Goose, falls back to Claude

pub fn find_acp_agent_by_id(provider_id: &str) -> Option<AcpAgent>
  // Get specific agent

fn find_agent(cmd: &str) -> Option<AcpAgent>
  // Multi-strategy search:
  //   1. Login shell `which` (for GUI app PATH)
  //   2. Direct command execution
  //   3. Common installation paths
```

### Diff Analysis Runner (`src-tauri/src/ai/runner.rs`)

**Complete analysis orchestration:**

```rust
pub async fn analyze_diff(
  repo_path: &Path,
  spec: &DiffSpec,
) -> Result<ChangesetAnalysis, String> {
  // 1. Find AI agent (fail fast)
  // 2. List files in diff
  // 3. Load unified diffs for each file
  // 4. Load "after" content (if small < 1000 lines)
  // 5. Build prompt with tiered strategy
  // 6. Run via ACP (acp_client)
  // 7. Parse JSON response
  // 8. Return ChangesetAnalysis with file annotations
}
```

---

## 6. Request/Response Flow

### Complete Flow: Smart Diff Analysis

```
Frontend: TopBar.svelte - User clicks AI button
    ↓
Frontend: handleAiAnalysis()
    ├─ checkAi() → invoke('check_ai_available')
    └─ runChangesetAnalysis()
        └─ runAnalysis(repoPath, spec)
            └─ invoke('analyze_diff', { repoPath, spec })
                ↓
Backend: Tauri command handler analyze_diff()
    ├─ ai::analyze_diff(&path, &spec).await
    │   ├─ find_ai_tool() → finds Goose/Claude
    │   ├─ git::list_diff_files()
    │   ├─ For each file:
    │   │   ├─ git::get_unified_diff()
    │   │   └─ load_after_content_if_small()
    │   ├─ build_prompt_with_strategy()
    │   │   ├─ Tier 1: diff + full content (< 10k lines)
    │   │   └─ Tier 2: diff only (fallback)
    │   └─ run_acp_prompt(&agent, repo_path, &prompt).await
    │       ├─ Spawn agent process in ACP mode
    │       ├─ Initialize ACP connection
    │       ├─ Send PromptRequest
    │       └─ Collect AgentMessageChunk notifications
    │           └─ Accumulate text in ResponseCollector
    │   └─ parse_response() → JSON deserialize
    │       └─ ChangesetAnalysis {
    │           summary, key_changes, concerns,
    │           file_annotations: { filePath: [...] }
    │         }
    └─ Return to frontend
                ↓
Frontend: Receive ChangesetAnalysis
    ├─ Store in smartDiffState
    ├─ saveChangesetSummary() → DB
    ├─ For each file: saveFileAnalysis() → DB
    ├─ saveAiComments() → Converts warnings/suggestions to Comments
    └─ createArtifactFromSummary() → saveArtifact()
```

### Agent Chat Request/Response

```
Frontend: AgentPanel - User types message
    ↓
User presses Enter
    ├─ buildPromptWithContext(userPrompt, isNewSession)
    │   ├─ Add [Changeset: ...]
    │   ├─ Add [Viewing: ...]
    │   ├─ If new: add STAGED_SYSTEM_CONTEXT
    │   ├─ If new: add [Reference artifacts: ...]
    │   ├─ If new: add [Code Comments from Review: ...]
    │   └─ If follow-up: add [Original task: ...]
    └─ sendAgentPrompt(repoPath, promptWithContext, sessionId?, provider)
        └─ invoke('send_agent_prompt', ...)
                ↓
Backend: send_agent_prompt()
    ├─ find_acp_agent_by_id(provider)
    └─ run_acp_prompt_with_session(...)
        ├─ If session_id: LoadSessionRequest
        │   └─ If fails: create new session
        └─ If no session_id: NewSessionRequest
            ├─ system_context = STAGED_SYSTEM_CONTEXT
            └─ full_prompt = system + user
        ├─ Send PromptRequest
        ├─ Collect response stream
        └─ Return { response, sessionId }
                ↓
Frontend: Receive AgentPromptResponse
    ├─ agentState.response = result.response
    ├─ agentState.sessionId = result.sessionId
    ├─ If first: agentState.task = userPrompt
    └─ Display (markdown + sanitized)
```

---

## 7. Artifacts & Sessions

### Artifact Model

**Definition (`stores/agent.svelte.ts` & `review/mod.rs`):**

```typescript
interface Artifact {
  id: string;              // UUID
  title: string;           // Auto-generated from content
  content: string;         // Markdown document
  createdAt: string;       // ISO timestamp
}
```

**Sources of artifacts:**
1. Agent responses: User saves response as artifact
2. AI analysis: SmartDiffModal converts changeset summary
3. Manual creation: Via agent UI

### Artifact Persistence

**Database layer (SQLite):**
```rust
pub fn save_artifact(id: &DiffId, artifact: &Artifact)
pub fn get_artifacts(id: &DiffId) -> Vec<Artifact>
pub fn delete_artifact(artifact_id: &str)
```

**Per-DiffId storage:**
- All artifacts for a diff stored together
- Survive app restart
- Loaded when switching diffs

### Session Management

**Architecture:**
- **Agent database**: Goose/Claude store sessions in their own SQLite
- **Staged database**: Stores artifacts (generated documents)

**Session lifecycle:**

```
New Session:
  1. No sessionId in state
  2. Backend: NewSessionRequest creates session
  3. Backend: Prepend STAGED_SYSTEM_CONTEXT
  4. Agent stores session with all history
  5. Return new sessionId

Resume Session:
  1. sessionId present in state
  2. Backend: LoadSessionRequest
  3. Agent replays message history
  4. Backend: Clear accumulated content (old messages)
  5. Append new prompt
  6. Return same sessionId

Session Continuity:
  1. Context includes [Original task: ...]
  2. User-selected artifacts included
  3. Comments included if selected
  4. Agent maintains coherence across turns
```

### System Context Guide

**`STAGED_SYSTEM_CONTEXT` (`acp_client.rs:24-37`):**

```
[System Context for Staged - Code Review Assistant]

You are helping with code review in Staged, a diff viewer application.
Your role is to help users understand, plan changes to, and research code.

Output Guidelines:
- When asked to create a PLAN: structured markdown with objectives and tasks
- When asked to do RESEARCH: research document with findings and recommendations
- When answering QUESTIONS: concise, focused on code changes

Context tags like [Changeset: ...], [Viewing: ...], [Original task: ...]
provide information about what they're looking at.
```

---

## 8. Key Architectural Patterns

### Tiered Prompt Strategy

**File-level threshold (1000 lines):**
- Small files: Include unified diff + full content
- Large files: Include unified diff only

**Changeset-level threshold (10,000 lines):**
- Tier 1: Diff + content for small files
- Tier 2: Diff only for all files (fallback)

**Implementation (`ai/runner.rs`):**
```rust
async fn build_prompt_with_strategy(
  files: &[DiffFile],
  repo_path: &Path,
) -> Result<String, String> {
  let total_lines: usize = files.iter()
    .map(|f| f.after_content.as_ref().map_or(0, |c| c.lines().count()))
    .sum();

  if total_lines < 10_000 {
    // Tier 1: Include full context
  } else {
    // Tier 2: Diff only
  }
}
```

### Error Handling

**Context window detection:**
```rust
fn detect_context_error(output: &str) -> Option<String> {
  let patterns = [
    "context limit reached",
    "prompt is too long",
    "exceeds maximum context",
  ];
  // ... pattern matching
  Some("Changeset too large for AI analysis".to_string())
}
```

### Per-File Result Storage

**SmartDiffResult structure:**
```typescript
interface SmartDiffResult {
  overview: string;                           // (unused)
  annotations: SmartDiffAnnotation[];
}

interface SmartDiffAnnotation {
  id: string;
  file_path?: string;
  before_span?: { start: number, end: number }; // 0-indexed
  before_description?: string;
  after_span?: { start: number, end: number };  // 0-indexed
  content: string;                            // AI commentary
  category: 'explanation' | 'warning' | 'suggestion' | 'context';
}
```

**Benefits:**
- Fast loading of specific file annotations
- Database persistence per DiffId
- Frontend displays without loading entire changeset

### Comment Conversion

**Only warnings & suggestions become persistent comments:**

```rust
// lib.rs:685-690
let is_actionable = matches!(
  ann.category,
  AnnotationCategory::Warning | AnnotationCategory::Suggestion
);

if !is_actionable {
  continue;  // Explanations/context stay as blur overlays
}
```

**Creates Comment with:**
- `author: CommentAuthor::Ai`
- `category: format!("{:?}", ann.category).to_lowercase()`
- `created_at: chrono::Utc::now().to_rfc3339()`

---

## 9. Integration Points Summary

| Component | Purpose | Technology | Location |
|-----------|---------|-----------|----------|
| `acp_client.rs` | Agent discovery & ACP protocol | JSON-RPC over stdio | `src-tauri/src/ai/` |
| `runner.rs` | Orchestrate analysis | Async Rust, libgit2 | `src-tauri/src/ai/` |
| `prompt.rs` | Tiered prompt building | String templates | `src-tauri/src/ai/` |
| `services/ai.ts` | Frontend API layer | Tauri invoke | `src/lib/services/` |
| `smartDiff.svelte.ts` | AI analysis state | Svelte $state | `src/lib/stores/` |
| `agent.svelte.ts` | Chat session state | Svelte $state | `src/lib/stores/` |
| `AgentPanel.svelte` | Chat UI | Svelte, marked, DOMPurify | `src/lib/features/agent/` |
| `AnnotationOverlay.svelte` | Display annotations | Svelte, CSS blur | `src/lib/` |
| `review/mod.rs` | Persistence (SQLite) | rusqlite | `src-tauri/src/review/` |

### Data Flow Summary

```
┌─────────────────────────────────────────────────┐
│              Frontend (Svelte)                  │
│  ┌───────────┐  ┌──────────┐  ┌──────────────┐ │
│  │ AgentPanel│  │ TopBar   │  │ Annotations  │ │
│  └─────┬─────┘  └────┬─────┘  └──────┬───────┘ │
│        │             │                │         │
│  ┌─────▼─────────────▼────────────────▼───────┐ │
│  │      State (agent + smartDiff stores)      │ │
│  └─────┬──────────────────────────────────────┘ │
│        │                                         │
│  ┌─────▼──────────────┐                         │
│  │  services/ai.ts    │                         │
│  └─────┬──────────────┘                         │
└────────┼─────────────────────────────────────────┘
         │ Tauri invoke
┌────────▼─────────────────────────────────────────┐
│          Backend (Rust/Tauri)                    │
│  ┌──────────────────────────────────────────┐   │
│  │  Tauri Command Handlers (lib.rs)        │   │
│  └──┬────────────────────────────────┬──────┘   │
│     │                                │           │
│  ┌──▼────────────┐            ┌──────▼─────┐    │
│  │  ai/runner.rs │            │ review/    │    │
│  │  (analysis)   │            │ (SQLite)   │    │
│  └──┬────────────┘            └────────────┘    │
│     │                                            │
│  ┌──▼──────────────┐                            │
│  │  ai/acp_client  │                            │
│  │  (ACP protocol) │                            │
│  └──┬──────────────┘                            │
└─────┼───────────────────────────────────────────┘
      │ stdio (JSON-RPC)
┌─────▼──────────────────────────────────────────┐
│      External AI Agents (Goose/Claude)         │
└────────────────────────────────────────────────┘
```

This architecture enables:
- Real-time AI analysis with prompt tiering
- Multi-turn conversations with session continuity
- Selective context inclusion (artifacts, comments, files)
- Persistent storage of analysis results
- Flexible UI scaling from small to large changesets
