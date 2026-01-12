/**
 * Diff Selection Store
 *
 * Manages the current diff specification (base..head) and presets.
 *
 * Rebuildable: This module owns diff selection state. The rest of the app
 * imports the reactive state directly - no subscriptions needed.
 */

import { DiffSpec } from '../types';

// =============================================================================
// Presets
// =============================================================================

/** Preset diff specifications */
export interface DiffPreset {
  spec: DiffSpec;
  label: string;
}

/**
 * Preset store - wrapped in object because Svelte doesn't allow exporting
 * reassignable $state. Access via `presetStore.presets`.
 */
export const presetStore = $state({
  presets: [
    { spec: DiffSpec.uncommitted(), label: 'Uncommitted' },
    { spec: DiffSpec.uncommitted(), label: 'Branch Changes' }, // Base updated on init
    { spec: DiffSpec.lastCommit(), label: 'Last Commit' },
  ] as DiffPreset[],
});

/** Convenience getter for presets */
export function getPresets(): readonly DiffPreset[] {
  return presetStore.presets;
}

/**
 * Update the "Branch Changes" preset to use the detected default branch.
 * Called during app initialization.
 */
export function setDefaultBranch(branch: string): void {
  presetStore.presets = presetStore.presets.map((preset) => {
    if (preset.label === 'Branch Changes') {
      return {
        ...preset,
        spec: {
          base: { type: 'Rev', value: branch },
          head: { type: 'WorkingTree' },
        },
      };
    }
    return preset;
  });
}

// =============================================================================
// Reactive State
// =============================================================================

/**
 * Diff selection state object.
 * Use this directly in components - it's reactive!
 */
export const diffSelection = $state({
  /** Current diff specification */
  spec: presetStore.presets[0].spec as DiffSpec,
  /** Label for current selection (preset name or custom) */
  label: presetStore.presets[0].label as string,
  /** PR number if this diff is for a GitHub PR */
  prNumber: undefined as number | undefined,
});

// =============================================================================
// Derived State (as getters)
// =============================================================================

/** Whether current spec matches a preset */
export function isPreset(): boolean {
  return presetStore.presets.some(
    (p) =>
      DiffSpec.display(p.spec) === DiffSpec.display(diffSelection.spec) &&
      p.label === diffSelection.label
  );
}

/** Display label - preset name or "base..head" */
export function getDisplayLabel(): string {
  return diffSelection.label;
}

// =============================================================================
// Actions
// =============================================================================

/**
 * Select a preset.
 */
export function selectPreset(preset: DiffPreset): void {
  diffSelection.spec = preset.spec;
  diffSelection.label = preset.label;
}

/**
 * Select a custom diff specification.
 */
export function selectCustomDiff(spec: DiffSpec, label?: string, prNumber?: number): void {
  diffSelection.spec = spec;
  diffSelection.label = label ?? DiffSpec.display(spec);
  diffSelection.prNumber = prNumber;
}

/**
 * Reset diff selection to "Uncommitted" (first preset).
 * Call when switching repositories.
 */
export function resetDiffSelection(): void {
  diffSelection.spec = presetStore.presets[0].spec;
  diffSelection.label = presetStore.presets[0].label;
  diffSelection.prNumber = undefined;
}
