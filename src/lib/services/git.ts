import { invoke } from '@tauri-apps/api/core';
import type { GitStatus, FileDiff } from '../types';

export async function getGitStatus(path?: string): Promise<GitStatus> {
  return invoke<GitStatus>('get_git_status', { path: path ?? null });
}

export async function openRepository(path: string): Promise<GitStatus> {
  return invoke<GitStatus>('open_repository', { path });
}

export async function getFileDiff(
  filePath: string,
  staged: boolean,
  repoPath?: string
): Promise<FileDiff> {
  return invoke<FileDiff>('get_file_diff', {
    repoPath: repoPath ?? null,
    filePath,
    staged,
  });
}

export async function getUntrackedFileDiff(filePath: string, repoPath?: string): Promise<FileDiff> {
  return invoke<FileDiff>('get_untracked_file_diff', {
    repoPath: repoPath ?? null,
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
