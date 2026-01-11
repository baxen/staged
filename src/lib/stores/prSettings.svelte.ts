/**
 * Pull Request Settings Store
 *
 * Manages persistent PR-related preferences (localStorage-backed).
 * Handles refresh interval, PR limit, and auto-refresh toggle.
 */

// =============================================================================
// Constants
// =============================================================================

const PR_SETTINGS_KEY = 'staged-pr-settings';

const REFRESH_INTERVAL_DEFAULT = 120000; // 2 minutes in ms
const REFRESH_INTERVAL_MIN = 60000; // 1 minute
const REFRESH_INTERVAL_MAX = 600000; // 10 minutes

const PR_LIMIT_DEFAULT = 50;
const PR_LIMIT_MIN = 10;
const PR_LIMIT_MAX = 100;

// =============================================================================
// Types
// =============================================================================

interface PRSettingsData {
  refreshInterval: number;
  prLimit: number;
  autoRefreshEnabled: boolean;
}

// =============================================================================
// Reactive State
// =============================================================================

/**
 * PR settings state object.
 * Use this directly in components - it's reactive!
 */
export const prSettings = $state({
  refreshInterval: REFRESH_INTERVAL_DEFAULT,
  prLimit: PR_LIMIT_DEFAULT,
  autoRefreshEnabled: true,
});

// =============================================================================
// Persistence
// =============================================================================

/**
 * Save current PR settings to localStorage.
 */
function savePRSettings(): void {
  const data: PRSettingsData = {
    refreshInterval: prSettings.refreshInterval,
    prLimit: prSettings.prLimit,
    autoRefreshEnabled: prSettings.autoRefreshEnabled,
  };
  localStorage.setItem(PR_SETTINGS_KEY, JSON.stringify(data));
}

/**
 * Load saved PR settings from localStorage and apply them.
 */
export function loadPRSettings(): void {
  const saved = localStorage.getItem(PR_SETTINGS_KEY);
  if (!saved) return;

  try {
    const data: PRSettingsData = JSON.parse(saved);

    // Validate and apply refresh interval
    if (
      typeof data.refreshInterval === 'number' &&
      data.refreshInterval >= REFRESH_INTERVAL_MIN &&
      data.refreshInterval <= REFRESH_INTERVAL_MAX
    ) {
      prSettings.refreshInterval = data.refreshInterval;
    }

    // Validate and apply PR limit
    if (
      typeof data.prLimit === 'number' &&
      data.prLimit >= PR_LIMIT_MIN &&
      data.prLimit <= PR_LIMIT_MAX
    ) {
      prSettings.prLimit = data.prLimit;
    }

    // Apply auto-refresh toggle
    if (typeof data.autoRefreshEnabled === 'boolean') {
      prSettings.autoRefreshEnabled = data.autoRefreshEnabled;
    }
  } catch (e) {
    console.warn('Failed to load PR settings:', e);
  }
}

// =============================================================================
// Actions
// =============================================================================

/**
 * Update the refresh interval (in milliseconds).
 * Validates and clamps to min/max range.
 */
export function updateRefreshInterval(ms: number): void {
  prSettings.refreshInterval = Math.max(
    REFRESH_INTERVAL_MIN,
    Math.min(REFRESH_INTERVAL_MAX, ms)
  );
  savePRSettings();
}

/**
 * Update the PR fetch limit.
 * Validates and clamps to min/max range.
 */
export function updatePRLimit(limit: number): void {
  prSettings.prLimit = Math.max(PR_LIMIT_MIN, Math.min(PR_LIMIT_MAX, Math.floor(limit)));
  savePRSettings();
}

/**
 * Toggle auto-refresh on/off.
 */
export function toggleAutoRefresh(): void {
  prSettings.autoRefreshEnabled = !prSettings.autoRefreshEnabled;
  savePRSettings();
}

/**
 * Reset all PR settings to defaults.
 */
export function resetPRSettings(): void {
  prSettings.refreshInterval = REFRESH_INTERVAL_DEFAULT;
  prSettings.prLimit = PR_LIMIT_DEFAULT;
  prSettings.autoRefreshEnabled = true;
  savePRSettings();
}
