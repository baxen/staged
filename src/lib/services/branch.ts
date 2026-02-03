import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Types
// =============================================================================

/** A tracked branch with an associated worktree */
export interface Branch {
  id: string;
  /** Path to the original repository */
  repoPath: string;
  /** Name of the branch (e.g., "feature/auth-flow") */
  branchName: string;
  /** Path to the worktree directory */
  worktreePath: string;
  /** The branch we forked from (for computing diffs) */
  baseBranch: string;
  createdAt: number;
  updatedAt: number;
}

/** Status of a branch session */
export type BranchSessionStatus = 'running' | 'completed' | 'error';

/** A session tied to a branch, producing a commit */
export interface BranchSession {
  id: string;
  branchId: string;
  /** The AI session ID (for watching/resuming) */
  aiSessionId: string | null;
  /** The commit SHA produced by this session (null while running) */
  commitSha: string | null;
  status: BranchSessionStatus;
  /** The user's prompt that started this session */
  prompt: string;
  /** Error message if status is 'error' */
  errorMessage: string | null;
  createdAt: number;
  updatedAt: number;
}

/** Commit info for display */
export interface CommitInfo {
  sha: string;
  shortSha: string;
  subject: string;
  author: string;
  timestamp: number;
}

// =============================================================================
// Branch Operations
// =============================================================================

/**
 * Create a new branch with a worktree.
 * The branch is created from the repo's default branch.
 */
export async function createBranch(repoPath: string, branchName: string): Promise<Branch> {
  return invoke<Branch>('create_branch', { repoPath, branchName });
}

/**
 * Get a branch by ID.
 */
export async function getBranch(branchId: string): Promise<Branch | null> {
  return invoke<Branch | null>('get_branch', { branchId });
}

/**
 * List all branches.
 */
export async function listBranches(): Promise<Branch[]> {
  return invoke<Branch[]>('list_branches');
}

/**
 * List branches for a specific repository.
 */
export async function listBranchesForRepo(repoPath: string): Promise<Branch[]> {
  return invoke<Branch[]>('list_branches_for_repo', { repoPath });
}

/**
 * Delete a branch and its worktree.
 */
export async function deleteBranch(branchId: string): Promise<void> {
  return invoke<void>('delete_branch', { branchId });
}

// =============================================================================
// Commit Operations
// =============================================================================

/**
 * Get commits for a branch since it diverged from base.
 * Returns commits in reverse chronological order (newest first).
 */
export async function getBranchCommits(branchId: string): Promise<CommitInfo[]> {
  return invoke<CommitInfo[]>('get_branch_commits', { branchId });
}

/**
 * Get the HEAD commit SHA for a branch's worktree.
 */
export async function getBranchHead(branchId: string): Promise<string> {
  return invoke<string>('get_branch_head', { branchId });
}

// =============================================================================
// Session Operations
// =============================================================================

/**
 * List all sessions for a branch.
 */
export async function listBranchSessions(branchId: string): Promise<BranchSession[]> {
  return invoke<BranchSession[]>('list_branch_sessions', { branchId });
}

/**
 * Get the session associated with a specific commit.
 */
export async function getSessionForCommit(
  branchId: string,
  commitSha: string
): Promise<BranchSession | null> {
  return invoke<BranchSession | null>('get_session_for_commit', { branchId, commitSha });
}

/**
 * Get the currently running session for a branch (if any).
 */
export async function getRunningSession(branchId: string): Promise<BranchSession | null> {
  return invoke<BranchSession | null>('get_running_session', { branchId });
}

// =============================================================================
// Session Lifecycle
// =============================================================================

/** Response from starting a branch session */
export interface StartBranchSessionResponse {
  branchSessionId: string;
  aiSessionId: string;
}

/**
 * Start a new session on a branch.
 * Creates a branch_session record, starts an AI session, and sends the prompt.
 */
export async function startBranchSession(
  branchId: string,
  prompt: string
): Promise<StartBranchSessionResponse> {
  return invoke<StartBranchSessionResponse>('start_branch_session', { branchId, prompt });
}

/**
 * Mark a branch session as completed with a commit SHA.
 */
export async function completeBranchSession(
  branchSessionId: string,
  commitSha: string
): Promise<void> {
  return invoke<void>('complete_branch_session', { branchSessionId, commitSha });
}

/**
 * Mark a branch session as failed with an error message.
 */
export async function failBranchSession(
  branchSessionId: string,
  errorMessage: string
): Promise<void> {
  return invoke<void>('fail_branch_session', { branchSessionId, errorMessage });
}

/**
 * Cancel a running branch session (deletes the record).
 * Used to recover from stuck sessions.
 */
export async function cancelBranchSession(branchSessionId: string): Promise<void> {
  return invoke<void>('cancel_branch_session', { branchSessionId });
}

/**
 * Recover orphaned sessions for a branch.
 * If there's a "running" session but no live AI session, checks if commits were made
 * and marks the session as completed or errored accordingly.
 * Returns the updated session if one was recovered, null otherwise.
 */
export async function recoverOrphanedSession(branchId: string): Promise<BranchSession | null> {
  return invoke<BranchSession | null>('recover_orphaned_session', { branchId });
}

/**
 * Get a branch session by its AI session ID.
 * Used to look up branch sessions when AI session status changes.
 */
export async function getBranchSessionByAiSession(
  aiSessionId: string
): Promise<BranchSession | null> {
  return invoke<BranchSession | null>('get_branch_session_by_ai_session', { aiSessionId });
}
