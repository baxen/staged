/**
 * Actions State Store
 *
 * Manages repository action discovery and execution state per tab.
 * Actions are discovered using AI and can be run with streaming output.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// =============================================================================
// Type Definitions
// =============================================================================

/** Categories for repository actions */
export type ActionCategory = 'build' | 'clean' | 'setup' | 'run' | 'test' | 'lint' | 'other';

/** All category values in display order */
export const ACTION_CATEGORIES: ActionCategory[] = [
  'setup',
  'clean',
  'build',
  'run',
  'test',
  'lint',
  'other',
];

/** Display names for categories */
export const CATEGORY_LABELS: Record<ActionCategory, string> = {
  setup: 'Install',
  clean: 'Clean',
  build: 'Build',
  run: 'Run',
  test: 'Test',
  lint: 'Lint',
  other: 'Other',
};

/** A discovered action in a repository */
export interface RepoAction {
  id: string;
  name: string;
  command: string;
  category: ActionCategory;
  /** Priority 1-5, higher = more important/commonly used */
  priority: number;
  description: string;
}

/** Status of an action execution */
export type ExecutionStatus = 'idle' | 'running' | 'success' | 'failed';

/** State of an action's execution */
export interface ActionExecution {
  actionId: string;
  status: ExecutionStatus;
  output: string;
  startedAt?: number;
  endedAt?: number;
  exitCode?: number;
}

/** Discovery status */
export type DiscoveryStatus = 'pending' | 'loading' | 'complete' | 'error';

/** Per-tab actions state */
export interface ActionsState {
  /** Discovered actions for this repo */
  actions: RepoAction[];
  /** Current execution state per action */
  executions: Map<string, ActionExecution>;
  /** Last executed action per category */
  lastExecutedByCategory: Map<ActionCategory, string>;
  /** Discovery status */
  discoveryStatus: DiscoveryStatus;
  /** Discovery error message */
  discoveryError?: string;
}

// =============================================================================
// Factory Function
// =============================================================================

/**
 * Create a new isolated actions state instance.
 * Used by the tab system to create per-tab state.
 */
export function createActionsState(): ActionsState {
  return {
    actions: [],
    executions: new Map<string, ActionExecution>(),
    lastExecutedByCategory: new Map<ActionCategory, string>(),
    discoveryStatus: 'pending',
    discoveryError: undefined,
  };
}

// =============================================================================
// Reactive State (Singleton)
// =============================================================================

/**
 * Module-level singleton state (for backwards compatibility).
 */
export const actionsState = $state(createActionsState());

// =============================================================================
// Event Listeners
// =============================================================================

/** Active event listeners */
let outputUnlisten: UnlistenFn | null = null;
let completeUnlisten: UnlistenFn | null = null;

/** Event payload for action output */
interface ActionOutputEvent {
  executionId: string;
  chunk: string;
}

/** Event payload for action completion */
interface ActionCompleteEvent {
  executionId: string;
  exitCode: number | null;
  success: boolean;
}

/**
 * Initialize action event listeners.
 * Call once on app startup.
 */
export async function initActionEvents(): Promise<() => void> {
  // Clean up existing listeners
  if (outputUnlisten) outputUnlisten();
  if (completeUnlisten) completeUnlisten();

  // Listen for output chunks
  outputUnlisten = await listen<ActionOutputEvent>('action-output', ({ payload }) => {
    const execution = actionsState.executions.get(payload.executionId);
    if (execution) {
      // Create new Map to trigger reactivity
      const newExecutions = new Map(actionsState.executions);
      newExecutions.set(payload.executionId, {
        ...execution,
        output: execution.output + payload.chunk,
      });
      actionsState.executions = newExecutions;
    }
  });

  // Listen for completion
  completeUnlisten = await listen<ActionCompleteEvent>('action-complete', ({ payload }) => {
    const execution = actionsState.executions.get(payload.executionId);
    if (execution) {
      const newExecutions = new Map(actionsState.executions);
      newExecutions.set(payload.executionId, {
        ...execution,
        status: payload.success ? 'success' : 'failed',
        exitCode: payload.exitCode ?? undefined,
        endedAt: Date.now(),
      });
      actionsState.executions = newExecutions;
    }
  });

  return () => {
    if (outputUnlisten) {
      outputUnlisten();
      outputUnlisten = null;
    }
    if (completeUnlisten) {
      completeUnlisten();
      completeUnlisten = null;
    }
  };
}

// =============================================================================
// Helpers
// =============================================================================

/**
 * Get actions for a specific category, sorted by priority (highest first).
 */
export function getActionsForCategory(category: ActionCategory): RepoAction[] {
  return actionsState.actions
    .filter((a) => a.category === category)
    .sort((a, b) => b.priority - a.priority);
}

/**
 * Get the current/default action for a category.
 * Returns the last executed action, or the highest priority action.
 */
export function getCurrentActionForCategory(category: ActionCategory): RepoAction | null {
  const actions = getActionsForCategory(category);
  if (actions.length === 0) return null;

  const lastExecutedId = actionsState.lastExecutedByCategory.get(category);
  if (lastExecutedId) {
    const lastExecuted = actions.find((a) => a.id === lastExecutedId);
    if (lastExecuted) return lastExecuted;
  }

  return actions[0]; // Highest priority
}

/**
 * Get the execution state for an action.
 */
export function getExecution(actionId: string): ActionExecution | null {
  return actionsState.executions.get(actionId) ?? null;
}

/**
 * Get categories that have at least one action.
 */
export function getActiveCategories(): ActionCategory[] {
  const categories = new Set<ActionCategory>();
  for (const action of actionsState.actions) {
    categories.add(action.category);
  }
  return ACTION_CATEGORIES.filter((c) => categories.has(c));
}

// =============================================================================
// Actions
// =============================================================================

/**
 * Discover actions in a repository using AI.
 */
export async function discoverActions(repoPath: string): Promise<void> {
  actionsState.discoveryStatus = 'loading';
  actionsState.discoveryError = undefined;

  try {
    const actions = await invoke<RepoAction[]>('discover_actions', { repoPath });
    actionsState.actions = actions;
    actionsState.discoveryStatus = 'complete';
    console.log(`[Actions] Discovered ${actions.length} actions for ${repoPath}`);
  } catch (e) {
    actionsState.discoveryError = e instanceof Error ? e.message : String(e);
    actionsState.discoveryStatus = 'error';
    console.error('[Actions] Discovery failed:', e);
  }
}

/**
 * Run an action.
 * Returns the execution ID.
 */
export async function runAction(action: RepoAction, repoPath: string): Promise<string> {
  const executionId = `${action.id}-${Date.now()}`;

  // Initialize execution state
  const newExecutions = new Map(actionsState.executions);
  newExecutions.set(executionId, {
    actionId: action.id,
    status: 'running',
    output: '',
    startedAt: Date.now(),
  });
  actionsState.executions = newExecutions;

  // Update last executed for this category
  const newLastExecuted = new Map(actionsState.lastExecutedByCategory);
  newLastExecuted.set(action.category, action.id);
  actionsState.lastExecutedByCategory = newLastExecuted;

  try {
    await invoke<string>('run_action', {
      executionId,
      command: action.command,
      repoPath,
    });
  } catch (e) {
    // Update execution state on immediate failure
    const executions = new Map(actionsState.executions);
    const execution = executions.get(executionId);
    if (execution) {
      executions.set(executionId, {
        ...execution,
        status: 'failed',
        output: execution.output + `\nError: ${e instanceof Error ? e.message : String(e)}`,
        endedAt: Date.now(),
      });
      actionsState.executions = executions;
    }
  }

  return executionId;
}

/**
 * Stop a running action.
 */
export async function stopAction(executionId: string): Promise<void> {
  try {
    await invoke('stop_action', { executionId });
  } catch (e) {
    console.error('[Actions] Failed to stop action:', e);
  }
}

/**
 * Get the most recent execution for an action (if any).
 */
export function getLatestExecutionForAction(actionId: string): ActionExecution | null {
  let latest: ActionExecution | null = null;
  let latestTime = 0;

  for (const execution of actionsState.executions.values()) {
    if (execution.actionId === actionId) {
      const time = execution.startedAt ?? 0;
      if (time > latestTime) {
        latestTime = time;
        latest = execution;
      }
    }
  }

  return latest;
}

/**
 * Reset actions state (for tab cleanup).
 */
export function resetActionsState(): void {
  actionsState.actions = [];
  actionsState.executions = new Map();
  actionsState.lastExecutedByCategory = new Map();
  actionsState.discoveryStatus = 'pending';
  actionsState.discoveryError = undefined;
}
