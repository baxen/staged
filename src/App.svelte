<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import DiffViewer from './lib/DiffViewer.svelte';
  import TopBar from './lib/TopBar.svelte';
  import { getRepoInfo, getRefs } from './lib/services/git';
  import type { GitRef, DiffSpec } from './lib/types';
  import {
    subscribeToFileChanges,
    startWatching,
    stopWatching,
    type Unsubscribe,
  } from './lib/services/statusEvents';
  import {
    preferences,
    loadSavedSize,
    loadSavedSyntaxTheme,
    handlePreferenceKeydown,
  } from './lib/stores/preferences.svelte';
  import {
    WORKDIR,
    diffSelection,
    selectDiffSpec,
    selectCustomDiff,
    initDiffSelection,
    setDefaultBranch,
  } from './lib/stores/diffSelection.svelte';
  import {
    diffState,
    getCurrentDiff,
    loadDiffs,
    refreshDiffs,
    selectFile,
    resetState,
  } from './lib/stores/diffState.svelte';
  import { loadComments, setCurrentPath } from './lib/stores/comments.svelte';

  // UI State
  let sidebarRef: Sidebar | null = $state(null);
  let unsubscribe: Unsubscribe | null = null;
  let repoName = $state('Loading...');

  // Diff Loading
  async function loadAllDiffs() {
    await loadDiffs(diffSelection.spec.base, diffSelection.spec.head);
    await loadComments(diffSelection.spec.base, diffSelection.spec.head);
    sidebarRef?.setDiffs(diffState.diffs);
  }

  // Update comments store when selected file changes
  $effect(() => {
    const path = currentDiff?.after?.path ?? currentDiff?.before?.path ?? null;
    setCurrentPath(path);
  });

  async function handleFilesChanged() {
    if (diffSelection.spec.head !== WORKDIR) return;
    // Use refreshDiffs to avoid loading flicker - keeps content visible during fetch
    await refreshDiffs(diffSelection.spec.base, diffSelection.spec.head);
    // Reload comments - they may have changed after a commit
    await loadComments(diffSelection.spec.base, diffSelection.spec.head);
    sidebarRef?.setDiffs(diffState.diffs);
  }

  // Preset selection
  async function handleDiffSelect(spec: DiffSpec) {
    resetState();
    await selectDiffSpec(spec);
    await loadAllDiffs();
  }

  // Custom diff selection
  async function handleCustomDiff(base: string, head: string) {
    resetState();
    await selectCustomDiff(base, head);
    await loadAllDiffs();
  }

  // Repo selection (placeholder for now)
  function handleRepoSelect() {
    // TODO: Implement repo selection
    console.log('Repo select clicked');
  }

  /**
   * Detect the default branch (main, master, etc.) from available refs.
   */
  function detectDefaultBranch(refs: GitRef[]): string {
    const branchNames = refs.filter((r) => r.ref_type === 'branch').map((r) => r.name);

    // Check common default branch names in order of preference
    const candidates = ['main', 'master', 'develop', 'trunk'];
    for (const name of candidates) {
      if (branchNames.includes(name)) {
        return name;
      }
    }

    // Fallback to first branch, or 'main' if no branches
    return branchNames[0] ?? 'main';
  }

  /**
   * Extract repo name from path (last directory component)
   */
  function extractRepoName(repoPath: string): string {
    // Remove trailing slash if present
    const cleanPath = repoPath.replace(/\/$/, '');
    // Get last component
    const parts = cleanPath.split('/');
    return parts[parts.length - 1] || 'Repository';
  }

  let currentDiff = $derived(getCurrentDiff());

  // Lifecycle
  onMount(() => {
    loadSavedSize();
    window.addEventListener('keydown', handlePreferenceKeydown);

    (async () => {
      await loadSavedSyntaxTheme();

      // Load refs for autocomplete and detect default branch
      try {
        const refs = await getRefs();
        const defaultBranch = detectDefaultBranch(refs);
        setDefaultBranch(defaultBranch);
      } catch (e) {
        console.error('Failed to load refs:', e);
      }

      await initDiffSelection();
      await loadAllDiffs();

      try {
        const info = await getRepoInfo();
        if (info?.repo_path) {
          repoName = extractRepoName(info.repo_path);
          await startWatching(info.repo_path);
        }
      } catch (e) {
        console.error('Failed to start watcher:', e);
      }

      unsubscribe = await subscribeToFileChanges(handleFilesChanged);
    })();
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handlePreferenceKeydown);
    unsubscribe?.();
    stopWatching().catch(() => {});
  });
</script>

<main>
  <TopBar
    {repoName}
    onDiffSelect={handleDiffSelect}
    onCustomDiff={handleCustomDiff}
    onRepoSelect={handleRepoSelect}
  />

  <div class="app-container">
    <section class="main-content">
      {#if diffState.loading}
        <div class="loading-state">
          <p>Loading...</p>
        </div>
      {:else if diffState.error}
        <div class="error-state">
          <p>Error loading diff:</p>
          <p class="error-message">{diffState.error}</p>
        </div>
      {:else}
        <DiffViewer
          diff={currentDiff}
          diffBase={diffSelection.spec.base}
          diffHead={diffSelection.spec.head}
          sizeBase={preferences.sizeBase}
          syntaxThemeVersion={preferences.syntaxThemeVersion}
        />
      {/if}
    </section>
    <aside class="sidebar">
      <Sidebar
        bind:this={sidebarRef}
        onFileSelect={selectFile}
        selectedFile={diffState.selectedFile}
        diffBase={diffSelection.spec.base}
        diffHead={diffSelection.spec.head}
      />
    </aside>
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background-color: var(--bg-chrome);
    color: var(--text-primary);
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background-color: var(--bg-chrome);
  }

  .app-container {
    display: flex;
    flex: 1;
    overflow: hidden;
    padding: 0 8px 8px 8px;
    gap: 8px;
  }

  .sidebar {
    width: 260px;
    min-width: 180px;
    background-color: transparent;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: var(--size-lg);
  }

  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--status-deleted);
    font-size: var(--size-lg);
  }

  .error-message {
    font-family: monospace;
    font-size: var(--size-sm);
    color: var(--text-muted);
    margin-top: 8px;
  }
</style>
