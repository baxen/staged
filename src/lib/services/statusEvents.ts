/**
 * File watcher event subscription service.
 *
 * Listens for backend file change events and forwards them to callbacks.
 * The frontend decides what to refresh when notified.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

/** Callback for file change notifications */
export type FilesChangedCallback = () => void;

/** Cleanup function returned by subscribe */
export type Unsubscribe = () => void;

// Active listener
let filesChangedUnlisten: UnlistenFn | null = null;

/**
 * Subscribe to file change events from the backend.
 *
 * @param onFilesChanged - Called whenever files in the repo change
 * @returns Cleanup function to unsubscribe
 */
export async function subscribeToFileChanges(
  onFilesChanged: FilesChangedCallback
): Promise<Unsubscribe> {
  // Clean up any existing listener first
  await unsubscribeAll();

  // Listen for file change notifications
  filesChangedUnlisten = await listen('files-changed', () => {
    onFilesChanged();
  });

  return unsubscribeAll;
}

/**
 * Unsubscribe from file change events.
 */
async function unsubscribeAll(): Promise<void> {
  if (filesChangedUnlisten) {
    filesChangedUnlisten();
    filesChangedUnlisten = null;
  }
}

/**
 * Start watching a repository for changes.
 * The backend will emit 'files-changed' events when files change.
 */
export async function startWatching(repoPath: string): Promise<void> {
  await invoke('start_watching', { repoPath });
}

/**
 * Stop watching the current repository.
 */
export async function stopWatching(): Promise<void> {
  await invoke('stop_watching');
}
