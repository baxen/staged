<!--
  ArtifactDetailView.svelte - Main view for selected artifacts

  Shows different content based on artifact type:
  - Plan: Markdown content
  - Implementation: Commit diff
  - Uncommitted: Current working tree changes
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { selectedArtifactId, artifactsState, getArtifact } from './stores/artifacts.svelte';
  import { diffState, loadFiles, getCurrentDiff, selectFile } from './stores/diffState.svelte';
  import { diffSelection } from './stores/diffSelection.svelte';
  import { renderMarkdown } from './services/markdown';
  import { listDiffFiles, getFileDiff } from './services/git';
  import { repoState } from './stores/repoState.svelte';
  import DiffViewer from './DiffViewer.svelte';
  import EmptyState from './EmptyState.svelte';
  import { DiffSpec } from './types';
  import type { FileDiff, FileDiffSummary } from './types';
  import { preferences } from './stores/preferences.svelte';
  import { planState } from './stores/plan.svelte';

  interface Props {
    sizeBase?: number;
    syntaxThemeVersion?: number;
    onImplementationStateChange?: (state: {
      files: FileDiffSummary[];
      loading: boolean;
      selectedFile: string | null;
      onFileSelect: (path: string) => void;
    }) => void;
  }

  let { sizeBase, syntaxThemeVersion = 0, onImplementationStateChange }: Props = $props();

  // State for loading implementation diffs
  let implementationFiles = $state<FileDiffSummary[]>([]);
  let implementationDiff = $state<FileDiff | null>(null);
  let implementationLoading = $state(false);
  let implementationError = $state<string | null>(null);
  let selectedImplementationFile = $state<string | null>(null);

  // Load implementation artifact diff
  async function loadImplementationDiff(commitHash: string) {
    implementationLoading = true;
    implementationError = null;
    implementationFiles = [];
    implementationDiff = null;
    selectedImplementationFile = null;

    try {
      const spec: DiffSpec = {
        base: { type: 'Rev', value: `${commitHash}~1` },
        head: { type: 'Rev', value: commitHash },
      };

      const files = await listDiffFiles(spec, repoState.currentPath ?? undefined);
      implementationFiles = files;

      // Select first file if available
      if (files.length > 0) {
        const firstPath = files[0].after ?? files[0].before;
        if (firstPath) {
          await selectImplementationFile(firstPath, spec);
        }
      }
    } catch (e) {
      implementationError = e instanceof Error ? e.message : String(e);
    } finally {
      implementationLoading = false;
    }
  }

  async function selectImplementationFile(path: string, spec: DiffSpec) {
    selectedImplementationFile = path;
    implementationLoading = true;
    try {
      implementationDiff = await getFileDiff(spec, path, repoState.currentPath ?? undefined);
    } catch (e) {
      implementationError = e instanceof Error ? e.message : String(e);
    } finally {
      implementationLoading = false;
    }
  }

  // Get current artifact
  let currentArtifact = $derived.by(() => {
    if (!selectedArtifactId.value) return null;
    if (selectedArtifactId.value === 'uncommitted') return { type: 'uncommitted' as const };
    return getArtifact(selectedArtifactId.value);
  });

  // Load data when artifact changes
  $effect(() => {
    if (currentArtifact && currentArtifact.type === 'implementation') {
      loadImplementationDiff(currentArtifact.commitHash);
    }
  });

  // Notify parent of implementation state changes
  $effect(() => {
    if (currentArtifact?.type === 'implementation' && onImplementationStateChange) {
      onImplementationStateChange({
        files: implementationFiles,
        loading: implementationLoading,
        selectedFile: selectedImplementationFile,
        onFileSelect: (path: string) => {
          if (implementationSpec) {
            selectImplementationFile(path, implementationSpec);
          }
        }
      });
    } else if (onImplementationStateChange && currentArtifact?.type !== 'implementation') {
      // Clear implementation state when not viewing implementation
      onImplementationStateChange({
        files: [],
        loading: false,
        selectedFile: null,
        onFileSelect: () => {}
      });
    }
  });

  // Get current implementation spec for the selected file
  let implementationSpec = $derived.by(() => {
    if (!currentArtifact || currentArtifact.type !== 'implementation') return null;
    return {
      base: { type: 'Rev' as const, value: `${currentArtifact.commitHash}~1` },
      head: { type: 'Rev' as const, value: currentArtifact.commitHash },
    };
  });
</script>

<div class="artifact-detail">
  {#if !currentArtifact}
    <!-- No artifact selected - this should not happen -->
    <div class="empty">
      <p>Select an artifact to view</p>
    </div>
  {:else if currentArtifact.type === 'session'}
    <!-- Session artifact - show planning interface -->
    <EmptyState />
  {:else if currentArtifact.type === 'uncommitted'}
    <!-- Uncommitted changes - show diff view full width -->
    <DiffViewer
      diff={getCurrentDiff()}
      {sizeBase}
      {syntaxThemeVersion}
      loading={diffState.loadingFile !== null}
      isReferenceFile={false}
    />
  {:else if currentArtifact.type === 'plan'}
    <!-- Plan artifact - show markdown -->
    <div class="plan-detail">
      <div class="plan-header">
        <h2>{currentArtifact.title}</h2>
        <p class="plan-description">{currentArtifact.description}</p>
      </div>
      <div class="plan-content">
        {@html renderMarkdown(currentArtifact.content)}
      </div>
    </div>
  {:else if currentArtifact.type === 'implementation'}
    <!-- Implementation artifact - show commit diff full width -->
    {#if implementationLoading && !implementationDiff}
      <div class="loading">Loading diff...</div>
    {:else if implementationError}
      <div class="error">{implementationError}</div>
    {:else if implementationDiff}
      <DiffViewer
        diff={implementationDiff}
        {sizeBase}
        {syntaxThemeVersion}
        loading={implementationLoading}
        isReferenceFile={false}
      />
    {:else}
      <div class="empty">Select a file to view its diff</div>
    {/if}
  {/if}
</div>

<style>
  .artifact-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .plan-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--color-bg);
    border-radius: 4px;
    padding: calc(var(--size-base) * 2);
  }

  .plan-header {
    margin-bottom: calc(var(--size-base) * 2);
  }

  .plan-header h2 {
    font-size: calc(var(--size-base) * 1.5);
    font-weight: 600;
    color: var(--color-fg);
    margin: 0 0 calc(var(--size-base) * 0.5);
  }

  .plan-description {
    font-size: calc(var(--size-base) * 0.95);
    color: var(--color-fg-muted);
    margin: 0;
  }

  .plan-content {
    flex: 1;
    overflow-y: auto;
    font-size: calc(var(--size-base) * 0.95);
    line-height: 1.6;
  }

  .empty,
  .loading,
  .error {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-fg-muted);
  }

  .error {
    color: var(--color-error, #ef4444);
  }

  .session-message {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-fg-muted);
    font-size: calc(var(--size-base) * 0.95);
  }
</style>
