import { invoke } from '@tauri-apps/api/core';
import type { GitStatus, FileDiff, CommitResult } from '../types';

export async function getGitStatus(path?: string): Promise<GitStatus> {
  return invoke<GitStatus>('get_git_status', { path: path ?? null });
}

export async function openRepository(path: string): Promise<GitStatus> {
  return invoke<GitStatus>('open_repository', { path });
}

/**
 * Get diff for a file between two refs.
 * This is the primary diff function for the review model.
 *
 * @param base - Base ref (branch name, SHA, "HEAD", etc.)
 * @param head - Head ref (same as base, or "@" for working tree)
 * @param filePath - Path to file relative to repo root
 * @param repoPath - Optional path to repository
 */
export async function getRefDiff(
  base: string,
  head: string,
  filePath: string,
  repoPath?: string
): Promise<FileDiff> {
  return invoke<FileDiff>('get_ref_diff', {
    repoPath: repoPath ?? null,
    base,
    head,
    filePath,
  });
}

// Staging operations

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

export async function stageAll(repoPath?: string): Promise<void> {
  return invoke('stage_all', {
    repoPath: repoPath ?? null,
  });
}

export async function unstageAll(repoPath?: string): Promise<void> {
  return invoke('unstage_all', {
    repoPath: repoPath ?? null,
  });
}

// Commit operations

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

// Line-level operations

import type { SourceLines } from '../types';

export async function discardLines(
  filePath: string,
  sourceLines: SourceLines,
  staged: boolean,
  repoPath?: string
): Promise<void> {
  return invoke('discard_lines', {
    repoPath: repoPath ?? null,
    filePath,
    oldStart: sourceLines.old_start,
    oldEnd: sourceLines.old_end,
    newStart: sourceLines.new_start,
    newEnd: sourceLines.new_end,
    staged,
  });
}
