<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { AlertCircle } from 'lucide-svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import DiffViewer from './lib/DiffViewer.svelte';
  import EmptyState from './lib/EmptyState.svelte';
  import TopBar from './lib/TopBar.svelte';
  import PRListView from './lib/PRListView.svelte';
  import ErrorModal from './lib/ErrorModal.svelte';
  import { listRefs, hasUncommittedChanges, checkoutPRBranch, fetchPR } from './lib/services/git';
  import { DiffSpec, inferRefType } from './lib/types';
  import type { DiffSpec as DiffSpecType, PullRequest } from './lib/types';
  import { initWatcher, watchRepo, type Unsubscribe } from './lib/services/statusEvents';
  import {
    preferences,
    loadSavedSize,
    loadSavedSyntaxTheme,
    registerPreferenceShortcuts,
  } from './lib/stores/preferences.svelte';
  import { loadPRSettings } from './lib/stores/prSettings.svelte';
  import { switchTab } from './lib/stores/viewState.svelte';
  import {
    diffSelection,
    selectPreset,
    selectCustomDiff,
    resetDiffSelection,
    setDefaultBranch,
    type DiffPreset,
  } from './lib/stores/diffSelection.svelte';
  import {
    diffState,
    getCurrentDiff,
    loadFiles,
    refreshFiles,
    selectFile,
    resetState,
  } from './lib/stores/diffState.svelte';
  import { loadComments, setCurrentPath, clearComments } from './lib/stores/comments.svelte';
  import { repoState, initRepoState, setCurrentRepo } from './lib/stores/repoState.svelte';
  import { viewState } from './lib/stores/viewState.svelte';

  // UI State (note: watcher is handled by initWatcher/watchRepo, no unsubscribe needed)
  let errorMessage = $state<string | null>(null);

  // Load files and comments for current spec
  async function loadAll() {
    const repoPath = repoState.currentPath ?? undefined;
    await loadFiles(diffSelection.spec, repoPath);
    await loadComments(diffSelection.spec, repoPath);
  }

  // Update comments store when selected file changes
  $effect(() => {
    const diff = getCurrentDiff();
    const path = diff?.after?.path ?? diff?.before?.path ?? null;
    setCurrentPath(path);
  });

  async function handleFilesChanged() {
    // Only refresh if viewing working tree
    if (diffSelection.spec.head.type !== 'WorkingTree') return;

    await refreshFiles(diffSelection.spec, repoState.currentPath ?? undefined);
    // Reload comments - they may have changed after a commit
    await loadComments(diffSelection.spec);
  }

  // Preset selection
  async function handlePresetSelect(preset: DiffPreset) {
    resetState();
    selectPreset(preset);
    await loadAll();
  }

  // Custom diff selection (from DiffSelectorModal or PRSelectorModal)
  async function handleCustomDiff(spec: DiffSpecType, label?: string, prNumber?: number) {
    resetState();
    selectCustomDiff(spec, label, prNumber);
    await loadAll();
  }

  // Repo change - reload everything
  async function handleRepoChange() {
    resetState();
    clearComments();

    if (repoState.currentPath) {
      watchRepo(repoState.currentPath);

      // Load refs and detect default branch for new repo
      try {
        const refs = await listRefs(repoState.currentPath);
        const defaultBranch = detectDefaultBranch(refs);
        setDefaultBranch(defaultBranch);
        // Mark repo as valid since we got refs
        setCurrentRepo(repoState.currentPath);
      } catch (e) {
        // Repo doesn't exist or isn't a git repo - show friendly error
        const errorMsg = e instanceof Error ? e.message : String(e);
        if (errorMsg.includes('No such file or directory')) {
          diffState.error = `Repository not found: ${repoState.currentPath}`;
        } else if (errorMsg.includes('not a git repository')) {
          diffState.error = `Not a git repository: ${repoState.currentPath}`;
        } else {
          diffState.error = errorMsg;
        }
        diffState.loading = false;
        console.error('Failed to load refs:', e);
        return;
      }

      // Reset diff selection to "Uncommitted" and load
      resetDiffSelection();
      await loadAll();
    }
  }

  /**
   * Detect the default branch (main, master, etc.) from available refs.
   */
  function detectDefaultBranch(refs: string[]): string {
    // Filter to likely branch names (not remotes, not tags)
    const branchNames = refs.filter((r) => inferRefType(r) === 'branch');

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

  // PR Checkout Flow
  async function handlePRCheckout(event: CustomEvent<{ pr: PullRequest }>) {
    const { pr } = event.detail;

    try {
      // Check for uncommitted changes
      const hasChanges = await hasUncommittedChanges(repoState.currentPath ?? undefined);
      if (hasChanges) {
        errorMessage = 'Cannot checkout PR: you have uncommitted changes. Commit or stash them first.';
        return;
      }

      // Checkout PR
      const branchName = await checkoutPRBranch(pr.number, pr.base_ref, repoState.currentPath ?? undefined);

      // Switch to diff view and refresh
      switchTab('diff');
      await handleRepoChange();

      console.log(`Checked out ${branchName}`);
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  async function handlePRView(event: CustomEvent<{ pr: PullRequest }>) {
    const { pr } = event.detail;

    try {
      // Fetch PR and get DiffSpec
      const spec = await fetchPR(pr.base_ref, pr.number, repoState.currentPath ?? undefined);
      await handleCustomDiff(spec, `PR #${pr.number}`, pr.number);
      switchTab('diff');
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    }
  }

  // Show empty state when we have a repo, finished loading, no error, but no diffs
  let showEmptyState = $derived(
    repoState.currentPath &&
      !diffState.loading &&
      !diffState.error &&
      diffState.files.length === 0
  );

  // Lifecycle
  onMount(() => {
    loadSavedSize();
    loadPRSettings(); // Load PR settings from localStorage
    registerPreferenceShortcuts();

    (async () => {
      await loadSavedSyntaxTheme();

      // Initialize repo state (loads recent repos, tries current directory)
      const hasRepo = await initRepoState();

      if (hasRepo && repoState.currentPath) {
        // Initialize watcher
        watchRepo(repoState.currentPath);

        // Load refs for autocomplete and detect default branch
        try {
          const refs = await listRefs(repoState.currentPath);
          const defaultBranch = detectDefaultBranch(refs);
          setDefaultBranch(defaultBranch);
        } catch (e) {
          console.error('Failed to load refs:', e);
        }

        resetDiffSelection();
        await loadAll();
      }
    })();
  });

  onDestroy(() => {
    // Watcher cleanup is handled automatically by the watcher system
  });
</script>

<main>
  <TopBar
    onPresetSelect={handlePresetSelect}
    onCustomDiff={handleCustomDiff}
    onRepoChange={handleRepoChange}
  />

  {#if viewState.activeTab === 'diff'}
    <div class="app-container">
      {#if !repoState.currentPath || showEmptyState}
        <!-- Full-width empty state -->
        <section class="main-content full-width">
          <EmptyState />
        </section>
      {:else}
        <section class="main-content">
          {#if diffState.loading}
            <div class="loading-state">
              <p>Loading...</p>
            </div>
          {:else if diffState.error}
            <div class="error-state">
              <AlertCircle size={24} />
              <p>Error loading diff:</p>
              <p class="error-message">{diffState.error}</p>
            </div>
          {:else}
            <DiffViewer
              diff={getCurrentDiff()}
              sizeBase={preferences.sizeBase}
              syntaxThemeVersion={preferences.syntaxThemeVersion}
            />
          {/if}
        </section>
        <aside class="sidebar">
          <Sidebar
            files={diffState.files}
            onFileSelect={selectFile}
            selectedFile={diffState.selectedFile}
          />
        </aside>
      {/if}
    </div>
  {:else if viewState.activeTab === 'pull-requests'}
    <div class="pr-view-container">
      <PRListView on:checkout={handlePRCheckout} on:view={handlePRView} />
    </div>
  {/if}

  {#if errorMessage}
    <ErrorModal message={errorMessage} onClose={() => (errorMessage = null)} />
  {/if}
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

  .full-width {
    width: 100%;
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
    gap: 8px;
  }

  .error-message {
    font-family: monospace;
    font-size: var(--size-sm);
    color: var(--text-muted);
  }

  .pr-view-container {
    flex: 1;
    overflow: hidden;
    padding: 0 8px 8px 8px;
    display: flex;
    flex-direction: column;
  }
</style>
