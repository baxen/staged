import { invoke } from '@tauri-apps/api/core';
import type { GitStatus, CommitResult, GitRef, FileDiff } from '../types';

// =============================================================================
// Diff API
// =============================================================================

/**
 * Get the full diff between two refs.
 * Returns all changed files with their content and alignments.
 */
export async function getDiff(base: string, head: string, repoPath?: string): Promise<FileDiff[]> {
  return invoke<FileDiff[]>('get_diff', {
    repoPath: repoPath ?? null,
    base,
    head,
  });
}

/**
 * Get list of refs (branches, tags, special refs) with type info for autocomplete.
 */
export async function getRefs(repoPath?: string): Promise<GitRef[]> {
  return invoke<GitRef[]>('get_refs', {
    repoPath: repoPath ?? null,
  });
}

/**
 * Resolve a ref to its short SHA for display/validation.
 * Returns "working tree" for "@", otherwise returns short SHA.
 */
export async function resolveRef(refStr: string, repoPath?: string): Promise<string> {
  return invoke<string>('resolve_ref', {
    repoPath: repoPath ?? null,
    refStr,
  });
}

// =============================================================================
// Status API (still uses legacy backend)
// =============================================================================

export async function getGitStatus(path?: string): Promise<GitStatus> {
  return invoke<GitStatus>('get_git_status', { path: path ?? null });
}

// =============================================================================
// Staging Operations (still uses legacy backend)
// =============================================================================

export async function stageFile(filePath: string, repoPath?: string): Promise<void> {
  return invoke('stage_file', {
    repoPath: repoPath ?? null,
    filePath,
  });
}

export async function unstageFile(filePath: string, repoPath?: string): Promise<void> {
  return invoke('unstage_file', {
    repoPath: repoPath ?? null,
    filePath,
  });
}

export async function discardFile(filePath: string, repoPath?: string): Promise<void> {
  return invoke('discard_file', {
    repoPath: repoPath ?? null,
    filePath,
  });
}

// =============================================================================
// Commit Operations (still uses legacy backend)
// =============================================================================

export async function getLastCommitMessage(repoPath?: string): Promise<string | null> {
  return invoke<string | null>('get_last_commit_message', {
    repoPath: repoPath ?? null,
  });
}

export async function createCommit(message: string, repoPath?: string): Promise<CommitResult> {
  return invoke<CommitResult>('create_commit', {
    repoPath: repoPath ?? null,
    message,
  });
}

export async function amendCommit(message: string, repoPath?: string): Promise<CommitResult> {
  return invoke<CommitResult>('amend_commit', {
    repoPath: repoPath ?? null,
    message,
  });
}
