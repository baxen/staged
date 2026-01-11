/**
 * Pull Request State Store
 *
 * Manages PR list data and auto-refresh polling.
 * Loads PRs from GitHub API and provides polling functionality.
 */

import { checkGitHubAuth, listPullRequests } from '../services/git';
import { repoState } from './repoState.svelte';
import type { PullRequest, GitHubAuthStatus } from '../types';

// =============================================================================
// Reactive State
// =============================================================================

/**
 * PR state object.
 * Use this directly in components - it's reactive!
 */
export const prState = $state({
  pullRequests: [] as PullRequest[],
  loading: false,
  error: null as string | null,
  lastFetched: null as Date | null,
  authStatus: null as GitHubAuthStatus | null,
});

// =============================================================================
// PR Loading
// =============================================================================

/**
 * Load pull requests from GitHub API.
 * Checks authentication first, then fetches PRs.
 */
export async function loadPRs(forceRefresh = false): Promise<void> {
  prState.loading = true;
  prState.error = null;

  try {
    // Check auth first
    prState.authStatus = await checkGitHubAuth();

    if (!prState.authStatus.authenticated) {
      prState.loading = false;
      return;
    }

    // Fetch PRs using the current repo path
    prState.pullRequests = await listPullRequests(
      repoState.currentPath ?? undefined,
      forceRefresh
    );
    prState.lastFetched = new Date();
  } catch (e) {
    prState.error = e instanceof Error ? e.message : String(e);
  } finally {
    prState.loading = false;
  }
}

// =============================================================================
// Polling
// =============================================================================

// Polling interval ID
let intervalId: number | null = null;

/**
 * Start auto-refresh polling.
 * Fetches PRs at the configured interval if auto-refresh is enabled.
 */
export function startAutoRefresh(): void {
  // Import prSettings dynamically to avoid circular dependency
  import('./prSettings.svelte').then(({ prSettings }) => {
    stopAutoRefresh(); // Clear any existing interval

    if (!prSettings.autoRefreshEnabled) {
      return;
    }

    // Set up interval to fetch PRs
    intervalId = window.setInterval(() => {
      // Only fetch if PR tab is active (check viewState)
      import('./viewState.svelte').then(({ viewState }) => {
        if (viewState.activeTab === 'pull-requests') {
          loadPRs(true); // Force refresh to bypass cache
        }
      });
    }, prSettings.refreshInterval);
  });
}

/**
 * Stop auto-refresh polling.
 */
export function stopAutoRefresh(): void {
  if (intervalId !== null) {
    clearInterval(intervalId);
    intervalId = null;
  }
}

/**
 * Restart auto-refresh with new settings.
 * Call this when settings change (e.g., refresh interval updated).
 */
export function restartAutoRefresh(): void {
  stopAutoRefresh();
  startAutoRefresh();
}
