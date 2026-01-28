/**
 * Artifacts Store - manages structured outputs from agent sessions
 *
 * Artifacts are the key outputs from agent work:
 * - Plans: Markdown plans created during planning phase
 * - Implementations: Git commits created during implementation phase
 * - Reviews: Code review feedback (future)
 *
 * Each tab maintains its own collection of artifacts.
 */

import type { AgentId } from '../AgentSelector.svelte';

// =============================================================================
// Types
// =============================================================================

/** Base artifact properties */
interface BaseArtifact {
  id: string;
  createdAt: number;
  agent: AgentId;
}

/** Plan artifact - markdown plan created during planning */
export interface PlanArtifact extends BaseArtifact {
  type: 'plan';
  title: string;
  content: string; // markdown
  description: string; // original user request
}

/** Implementation artifact - git commit created during implementation */
export interface ImplementationArtifact extends BaseArtifact {
  type: 'implementation';
  title: string;
  commitHash: string;
  commitMessage: string;
  filesChanged: string[];
  planId?: string; // reference to the plan that was implemented
}

/** Active session - ongoing agent conversation */
export interface SessionArtifact extends BaseArtifact {
  type: 'session';
  title: string;
  status: 'planning' | 'refining' | 'implementing' | 'active';
  sessionId: string;
  description: string; // what the session is working on
}

/** Union type of all artifacts */
export type Artifact = PlanArtifact | ImplementationArtifact | SessionArtifact;

/** State for the artifacts store */
export interface ArtifactsState {
  /** All artifacts for the current tab, newest first */
  artifacts: Artifact[];
}

// =============================================================================
// Factory Function
// =============================================================================

/**
 * Create a new isolated artifacts state instance.
 * Used by the tab system to create per-tab state.
 */
export function createArtifactsState(): ArtifactsState {
  return {
    artifacts: [],
  };
}

// =============================================================================
// Reactive State (Singleton)
// =============================================================================

/**
 * Module-level singleton state.
 * Gets synced to/from the active tab's artifactsState.
 */
export const artifactsState = $state<ArtifactsState>(createArtifactsState());

/**
 * Currently selected artifact ID for viewing in main panel.
 * Special values:
 * - null: No artifact selected (default diff view)
 * - "uncommitted": Show uncommitted changes
 */
let _selectedArtifactId = $state<string | null>(null);

export function getSelectedArtifactId(): string | null {
  return _selectedArtifactId;
}

export const selectedArtifactId = {
  get value(): string | null {
    return _selectedArtifactId;
  }
};

// =============================================================================
// Actions
// =============================================================================

/**
 * Add a plan artifact.
 */
export function addPlanArtifact(
  title: string,
  content: string,
  description: string,
  agent: AgentId
): string {
  const id = crypto.randomUUID();
  const artifact: PlanArtifact = {
    id,
    type: 'plan',
    title,
    content,
    description,
    agent,
    createdAt: Date.now(),
  };
  // Add to beginning (newest first)
  artifactsState.artifacts = [artifact, ...artifactsState.artifacts];
  return id;
}

/**
 * Add an implementation artifact.
 */
export function addImplementationArtifact(
  title: string,
  commitHash: string,
  commitMessage: string,
  filesChanged: string[],
  agent: AgentId,
  planId?: string
): string {
  const id = crypto.randomUUID();
  const artifact: ImplementationArtifact = {
    id,
    type: 'implementation',
    title,
    commitHash,
    commitMessage,
    filesChanged,
    planId,
    agent,
    createdAt: Date.now(),
  };
  // Add to beginning (newest first)
  artifactsState.artifacts = [artifact, ...artifactsState.artifacts];
  return id;
}

/**
 * Remove an artifact by ID.
 */
export function removeArtifact(id: string): void {
  artifactsState.artifacts = artifactsState.artifacts.filter((a) => a.id !== id);
}

/**
 * Clear all artifacts.
 */
export function clearArtifacts(): void {
  artifactsState.artifacts = [];
}

/**
 * Get an artifact by ID.
 */
export function getArtifact(id: string): Artifact | null {
  return artifactsState.artifacts.find((a) => a.id === id) ?? null;
}

/**
 * Sync singleton state from a tab's artifacts state.
 * Called when switching tabs.
 */
export function syncFromTab(tabArtifactsState: ArtifactsState): void {
  artifactsState.artifacts = tabArtifactsState.artifacts;
}

/**
 * Sync singleton state to a tab's artifacts state.
 * Called before switching away from a tab.
 */
export function syncToTab(tabArtifactsState: ArtifactsState): void {
  tabArtifactsState.artifacts = artifactsState.artifacts;
}

/**
 * Select an artifact for viewing.
 */
export function selectArtifact(id: string | null): void {
  _selectedArtifactId = id;
}

/**
 * Check if there are uncommitted changes that should be shown.
 */
export function hasUncommittedChanges(filesCount: number, isWorkingTree: boolean): boolean {
  return isWorkingTree && filesCount > 0;
}

/**
 * Add or update a session artifact.
 * Sessions represent active or ongoing agent work.
 */
export function upsertSessionArtifact(
  sessionId: string,
  title: string,
  status: 'planning' | 'refining' | 'implementing' | 'active',
  description: string,
  agent: AgentId
): string {
  // Check if session already exists
  const existing = artifactsState.artifacts.find(
    (a) => a.type === 'session' && a.sessionId === sessionId
  ) as SessionArtifact | undefined;

  if (existing) {
    // Update existing session
    existing.title = title;
    existing.status = status;
    existing.description = description;
    existing.agent = agent;
    return existing.id;
  }

  // Create new session artifact
  const id = crypto.randomUUID();
  const artifact: SessionArtifact = {
    id,
    type: 'session',
    title,
    status,
    sessionId,
    description,
    agent,
    createdAt: Date.now(),
  };

  // Add to beginning (newest first)
  artifactsState.artifacts = [artifact, ...artifactsState.artifacts];
  return id;
}

/**
 * Remove a session artifact by session ID.
 */
export function removeSessionArtifact(sessionId: string): void {
  artifactsState.artifacts = artifactsState.artifacts.filter(
    (a) => !(a.type === 'session' && a.sessionId === sessionId)
  );
}

/**
 * Get a session artifact by session ID.
 */
export function getSessionArtifact(sessionId: string): SessionArtifact | null {
  const artifact = artifactsState.artifacts.find(
    (a) => a.type === 'session' && a.sessionId === sessionId
  );
  return artifact?.type === 'session' ? artifact : null;
}
