/**
 * View State Store
 *
 * Manages the active tab/view in the application.
 * Currently supports "diff" and "pull-requests" tabs.
 */

export type ViewTab = 'diff' | 'pull-requests';

/**
 * View state object.
 * Use this directly in components - it's reactive!
 */
export const viewState = $state({
  activeTab: 'diff' as ViewTab,
});

/**
 * Switch to a different tab/view.
 */
export function switchTab(tab: ViewTab): void {
  viewState.activeTab = tab;
}
