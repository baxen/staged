/**
 * File watcher event subscription service.
 *
 * Uses a generation counter to ignore stale events from old repos.
 * All watcher commands are fire-and-forget (no awaiting backend).
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

/** Callback for file change notifications */
export type FilesChangedCallback = () => void;

/** Cleanup function returned by subscribe */
export type Unsubscribe = () => void;

/** Payload from backend files-changed event */
interface FilesChangedPayload {
  watchId: number;
}

// Current watch ID - incremented on each repo switch
let currentWatchId = 0;

// Active listener
let filesChangedUnlisten: UnlistenFn | null = null;

/**
 * Initialize the watcher event listener.
 * Call once on app startup. The callback is invoked when files change
 * in the currently watched repo (stale events are filtered out).
 *
 * @param onFilesChanged - Called when files in the current repo change
 * @returns Cleanup function to unsubscribe
 */
export async function initWatcher(onFilesChanged: FilesChangedCallback): Promise<Unsubscribe> {
  // Clean up any existing listener
  if (filesChangedUnlisten) {
    filesChangedUnlisten();
  }

  filesChangedUnlisten = await listen<FilesChangedPayload>('files-changed', ({ payload }) => {
    // Ignore events from old repos
    if (payload.watchId === currentWatchId) {
      onFilesChanged();
    }
  });

  return () => {
    if (filesChangedUnlisten) {
      filesChangedUnlisten();
      filesChangedUnlisten = null;
    }
  };
}

/**
 * Switch to watching a new repository.
 * Fire-and-forget: returns immediately, actual setup happens on backend thread.
 *
 * @param repoPath - Absolute path to the repository
 */
export function watchRepo(repoPath: string): void {
  currentWatchId++;
  invoke('watch_repo', { repoPath, watchId: currentWatchId });
}

/**
 * Get the current watch ID (for testing/debugging).
 */
export function getWatchId(): number {
  return currentWatchId;
}
