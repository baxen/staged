/**
 * Window Service
 *
 * Provides window management functions for creating new windows and getting window info.
 */

import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Create a new window.
 * Returns the label of the newly created window.
 */
export async function createWindow(repoPath?: string): Promise<string> {
  return invoke<string>('create_window', { repoPath: repoPath ?? null });
}

/**
 * Get the current window's label.
 */
export async function getWindowLabel(): Promise<string> {
  return invoke<string>('get_window_label');
}

/**
 * Get the repo path from URL query parameters if present.
 * Used when a new window is opened with a specific repo.
 */
export function getRepoFromUrl(): string | null {
  const params = new URLSearchParams(window.location.search);
  const repo = params.get('repo');
  return repo ? decodeURIComponent(repo) : null;
}

/**
 * Get the current window instance.
 */
export { getCurrentWindow };
