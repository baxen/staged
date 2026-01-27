/**
 * Plan Store - manages the planning artifact independently from agent conversations.
 *
 * A plan is a first-class entity that represents work the user wants to accomplish.
 * It has its own lifecycle separate from the agent sessions used to create/modify it.
 *
 * State machine:
 *   null -> drafting -> ready <-> refining
 *                         |
 *                         v
 *                    implementing -> complete
 */

import type { AgentId } from '../AgentSelector.svelte';

// =============================================================================
// Types
// =============================================================================

/** Plan status - represents the current phase of the plan lifecycle */
export type PlanStatus = 'drafting' | 'ready' | 'refining' | 'implementing' | 'complete';

/** A plan artifact */
export interface Plan {
  /** The markdown content of the plan */
  content: string;
  /** Current status in the lifecycle */
  status: PlanStatus;
  /** Original user description that started this plan */
  description: string;
  /** Agent used to create/refine the plan */
  planningAgent: AgentId;
  /** Agent selected for implementation (may differ from planning agent) */
  implementAgent: AgentId;
  /** Timestamp when plan was created */
  createdAt: number;
  /** Timestamp of last update */
  updatedAt: number;
}

/** State for the plan store */
export interface PlanState {
  /** The current plan, or null if no plan exists */
  plan: Plan | null;
  /** Error message if something went wrong */
  error: string | null;
}

// =============================================================================
// Factory Function
// =============================================================================

/**
 * Create a new isolated plan state instance.
 * Used by the tab system to create per-tab state.
 */
export function createPlanState(): PlanState {
  return {
    plan: null,
    error: null,
  };
}

// =============================================================================
// Reactive State (Singleton)
// =============================================================================

/**
 * Module-level singleton state.
 * Gets synced to/from the active tab's planState.
 */
export const planState = $state<PlanState>(createPlanState());

// =============================================================================
// Actions
// =============================================================================

/**
 * Start drafting a new plan.
 * Called when user submits their description.
 */
export function startDrafting(description: string, agent: AgentId): void {
  planState.plan = {
    content: '',
    status: 'drafting',
    description,
    planningAgent: agent,
    implementAgent: agent, // Default to same agent
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
  planState.error = null;
}

/**
 * Update the plan content (called as agent streams response).
 */
export function updatePlanContent(content: string): void {
  if (planState.plan) {
    planState.plan.content = content;
    planState.plan.updatedAt = Date.now();
  }
}

/**
 * Mark the plan as ready (drafting complete).
 */
export function markPlanReady(): void {
  if (planState.plan && planState.plan.status === 'drafting') {
    planState.plan.status = 'ready';
    planState.plan.updatedAt = Date.now();
  }
}

/**
 * Start refining the plan with feedback.
 */
export function startRefining(): void {
  if (planState.plan && planState.plan.status === 'ready') {
    planState.plan.status = 'refining';
    planState.plan.updatedAt = Date.now();
  }
}

/**
 * Mark refining as complete, back to ready.
 */
export function markRefiningComplete(): void {
  if (planState.plan && planState.plan.status === 'refining') {
    planState.plan.status = 'ready';
    planState.plan.updatedAt = Date.now();
  }
}

/**
 * Set the agent to use for implementation.
 */
export function setImplementAgent(agent: AgentId): void {
  if (planState.plan) {
    planState.plan.implementAgent = agent;
  }
}

/**
 * Start implementing the plan.
 */
export function startImplementing(): void {
  if (planState.plan && planState.plan.status === 'ready') {
    planState.plan.status = 'implementing';
    planState.plan.updatedAt = Date.now();
  }
}

/**
 * Mark implementation as complete.
 */
export function markImplementationComplete(): void {
  if (planState.plan && planState.plan.status === 'implementing') {
    planState.plan.status = 'complete';
    planState.plan.updatedAt = Date.now();
  }
}

/**
 * Set an error on the plan.
 */
export function setPlanError(error: string): void {
  planState.error = error;
  // If we were in a transitional state, revert to ready
  if (planState.plan) {
    if (planState.plan.status === 'drafting') {
      // Failed during initial draft - clear the plan
      planState.plan = null;
    } else if (planState.plan.status === 'refining' || planState.plan.status === 'implementing') {
      // Failed during refine/implement - revert to ready
      planState.plan.status = 'ready';
    }
  }
}

/**
 * Clear the error.
 */
export function clearPlanError(): void {
  planState.error = null;
}

/**
 * Clear the plan entirely (start over).
 */
export function clearPlan(): void {
  planState.plan = null;
  planState.error = null;
}

/**
 * Sync singleton state from a tab's plan state.
 * Called when switching tabs.
 */
export function syncFromTab(tabPlanState: PlanState): void {
  planState.plan = tabPlanState.plan;
  planState.error = tabPlanState.error;
}

/**
 * Sync singleton state to a tab's plan state.
 * Called before switching away from a tab.
 */
export function syncToTab(tabPlanState: PlanState): void {
  tabPlanState.plan = planState.plan;
  tabPlanState.error = planState.error;
}

// =============================================================================
// Derived Helpers
// =============================================================================

/**
 * Check if we're in a loading state (agent is working on the plan).
 */
export function isLoading(): boolean {
  const status = planState.plan?.status;
  return status === 'drafting' || status === 'refining' || status === 'implementing';
}

/**
 * Get a user-friendly status message.
 */
export function getStatusMessage(): { title: string; subtitle: string } | null {
  if (!planState.plan) return null;

  switch (planState.plan.status) {
    case 'drafting':
      return { title: 'Planning your work...', subtitle: 'Analyzing the codebase' };
    case 'refining':
      return { title: 'Updating the plan...', subtitle: 'Incorporating your feedback' };
    case 'implementing':
      return { title: 'Implementing the plan...', subtitle: 'Making code changes' };
    default:
      return null;
  }
}
