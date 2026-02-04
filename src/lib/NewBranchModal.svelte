<!--
  NewBranchModal.svelte - Create a new branch with worktree

  Three-step flow:
  1. Pick a repository (with search)
  2. Select base branch (local and remote branches)
  3. Enter branch name

  The branch is created with an isolated worktree.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    X,
    Folder,
    GitBranch,
    ChevronRight,
    Search,
    Loader2,
    ArrowLeft,
    Globe,
  } from 'lucide-svelte';
  import type { Branch, BranchRef } from './services/branch';
  import * as branchService from './services/branch';
  import { listDirectory, getHomeDir, searchDirectories, type DirEntry } from './services/files';

  interface Props {
    onCreated: (branch: Branch) => void;
    onClose: () => void;
  }

  let { onCreated, onClose }: Props = $props();

  // State
  type Step = 'repo' | 'base' | 'name';
  let step = $state<Step>('repo');
  let selectedRepo = $state<string | null>(null);
  let selectedBaseBranch = $state<string | null>(null);
  let branchName = $state('');
  let creating = $state(false);
  let error = $state<string | null>(null);

  // Repo picker state
  let query = $state('');
  let currentDir = $state('');
  let homeDir = $state('');
  let entries = $state<DirEntry[]>([]);
  let searchResults = $state<DirEntry[]>([]);
  let loading = $state(false);
  let searching = $state(false);
  let selectedIndex = $state(0);
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  // Base branch picker state
  let baseBranchQuery = $state('');
  let gitBranches = $state<BranchRef[]>([]);
  let loadingBranches = $state(false);
  let baseBranchSelectedIndex = $state(0);

  let inputEl: HTMLInputElement | null = $state(null);
  let baseBranchInputEl: HTMLInputElement | null = $state(null);
  let branchInputEl: HTMLInputElement | null = $state(null);

  let isSearching = $derived(query.length >= 2);

  // Filter branches by query
  let filteredBranches = $derived.by(() => {
    if (!baseBranchQuery) return gitBranches;
    const q = baseBranchQuery.toLowerCase();
    return gitBranches.filter((b) => b.name.toLowerCase().includes(q));
  });

  // Initialize
  onMount(async () => {
    const dir = await getHomeDir();
    homeDir = dir;
    currentDir = dir;
  });

  // Focus appropriate input
  $effect(() => {
    if (step === 'repo' && inputEl) {
      inputEl.focus();
    } else if (step === 'base' && baseBranchInputEl) {
      baseBranchInputEl.focus();
    } else if (step === 'name' && branchInputEl) {
      branchInputEl.focus();
    }
  });

  // Load directory when currentDir changes
  $effect(() => {
    if (currentDir && !isSearching) {
      loadDirectory(currentDir);
    }
  });

  // Debounced search
  $effect(() => {
    if (searchTimeout) clearTimeout(searchTimeout);

    if (!query || query.length < 2) {
      searchResults = [];
      searching = false;
      return;
    }

    searching = true;
    searchTimeout = setTimeout(async () => {
      try {
        const depth = currentDir === homeDir ? 4 : 3;
        const results = await searchDirectories(currentDir, query, depth, 20);
        searchResults = results;
        selectedIndex = 0;
      } catch (e) {
        console.error('Search failed:', e);
        searchResults = [];
      } finally {
        searching = false;
      }
    }, 150);
  });

  async function loadDirectory(path: string) {
    loading = true;
    try {
      const allEntries = await listDirectory(path);
      entries = allEntries.filter((e) => e.isDir);
      selectedIndex = 0;
    } catch (e) {
      entries = [];
    } finally {
      loading = false;
    }
  }

  async function loadGitBranches(repoPath: string) {
    loadingBranches = true;
    try {
      const [branches, defaultBranch] = await Promise.all([
        branchService.listGitBranches(repoPath),
        branchService.detectDefaultBranch(repoPath),
      ]);
      gitBranches = branches;
      selectedBaseBranch = defaultBranch;
      // Find the index of the default branch
      const defaultIndex = branches.findIndex((b) => b.name === defaultBranch);
      baseBranchSelectedIndex = defaultIndex >= 0 ? defaultIndex : 0;
    } catch (e) {
      console.error('Failed to load branches:', e);
      gitBranches = [];
    } finally {
      loadingBranches = false;
    }
  }

  // Get display items based on mode
  let displayItems = $derived.by(() => {
    if (isSearching) {
      return searchResults;
    }
    return entries;
  });

  function selectRepo(path: string) {
    selectedRepo = path;
    step = 'base';
    error = null;
    loadGitBranches(path);
  }

  function selectBaseBranch(branchName: string) {
    selectedBaseBranch = branchName;
    step = 'name';
    error = null;
  }

  function handleEntryClick(entry: DirEntry) {
    if (entry.isRepo) {
      selectRepo(entry.path);
    } else {
      // Navigate into directory
      currentDir = entry.path;
      query = '';
    }
  }

  function goBack() {
    if (step === 'name') {
      step = 'base';
      branchName = '';
      error = null;
    } else if (step === 'base') {
      step = 'repo';
      selectedRepo = null;
      selectedBaseBranch = null;
      gitBranches = [];
      baseBranchQuery = '';
      error = null;
    }
  }

  function goUp() {
    if (currentDir && currentDir !== '/') {
      const parent = currentDir.split('/').slice(0, -1).join('/') || '/';
      currentDir = parent;
      query = '';
    }
  }

  function goHome() {
    if (homeDir) {
      currentDir = homeDir;
      query = '';
    }
  }

  async function handleCreate() {
    if (!selectedRepo || !branchName.trim()) return;

    creating = true;
    error = null;

    try {
      const branch = await branchService.createBranch(
        selectedRepo,
        branchName.trim(),
        selectedBaseBranch ?? undefined
      );
      onCreated(branch);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      creating = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      if (step === 'name' || step === 'base') {
        goBack();
      } else {
        onClose();
      }
      return;
    }

    if (step === 'repo') {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, displayItems.length - 1);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
      } else if (e.key === 'Enter' && displayItems.length > 0) {
        e.preventDefault();
        handleEntryClick(displayItems[selectedIndex]);
      } else if (e.key === 'Backspace' && !query) {
        e.preventDefault();
        goUp();
      }
    } else if (step === 'base') {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        baseBranchSelectedIndex = Math.min(
          baseBranchSelectedIndex + 1,
          filteredBranches.length - 1
        );
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        baseBranchSelectedIndex = Math.max(baseBranchSelectedIndex - 1, 0);
      } else if (e.key === 'Enter' && filteredBranches.length > 0) {
        e.preventDefault();
        selectBaseBranch(filteredBranches[baseBranchSelectedIndex].name);
      }
    } else if (step === 'name') {
      if (e.key === 'Enter' && branchName.trim()) {
        e.preventDefault();
        handleCreate();
      }
    }
  }

  // Extract repo name from path
  function repoName(path: string): string {
    const parts = path.split('/');
    return parts[parts.length - 1] || path;
  }
</script>

<div class="modal-backdrop" role="button" tabindex="-1" onclick={onClose} onkeydown={handleKeydown}>
  <div
    class="modal"
    role="dialog"
    tabindex="-1"
    onkeydown={() => {}}
    onclick={(e) => e.stopPropagation()}
  >
    <div class="modal-header">
      {#if step === 'name' || step === 'base'}
        <button class="back-button" onclick={goBack}>
          <ArrowLeft size={16} />
        </button>
      {/if}
      <h2>
        {#if step === 'repo'}
          Select Repository
        {:else if step === 'base'}
          Select Base Branch
        {:else}
          New Branch
        {/if}
      </h2>
      <button class="close-button" onclick={onClose}>
        <X size={18} />
      </button>
    </div>

    {#if step === 'repo'}
      <!-- Repository picker -->
      <div class="search-container">
        <Search size={16} class="search-icon" />
        <input
          bind:this={inputEl}
          bind:value={query}
          type="text"
          placeholder="Search repositories..."
          class="search-input"
        />
        {#if searching}
          <Loader2 size={16} class="spinner" />
        {/if}
      </div>

      <div class="breadcrumb">
        <button class="breadcrumb-home" onclick={goHome}>~</button>
        {#if currentDir && currentDir !== homeDir}
          <ChevronRight size={12} />
          <span class="breadcrumb-path">
            {currentDir.replace(homeDir, '').replace(/^\//, '')}
          </span>
        {/if}
      </div>

      <div class="entries-list">
        {#if loading && !isSearching}
          <div class="loading">
            <Loader2 size={16} class="spinner" />
            <span>Loading...</span>
          </div>
        {:else if displayItems.length === 0}
          <div class="empty">
            {isSearching ? 'No repositories found' : 'No folders'}
          </div>
        {:else}
          {#each displayItems as entry, index (entry.path)}
            <button
              class="entry"
              class:selected={index === selectedIndex}
              class:repo={entry.isRepo}
              onclick={() => handleEntryClick(entry)}
            >
              {#if entry.isRepo}
                <GitBranch size={16} class="entry-icon repo-icon" />
              {:else}
                <Folder size={16} class="entry-icon" />
              {/if}
              <span class="entry-name">{entry.name}</span>
              {#if !entry.isRepo}
                <ChevronRight size={14} class="entry-chevron" />
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    {:else if step === 'base'}
      <!-- Base branch picker -->
      <div class="search-container">
        <Search size={16} class="search-icon" />
        <input
          bind:this={baseBranchInputEl}
          bind:value={baseBranchQuery}
          type="text"
          placeholder="Filter branches..."
          class="search-input"
        />
      </div>

      <div class="selected-repo-bar">
        <GitBranch size={14} />
        <span>{repoName(selectedRepo ?? '')}</span>
      </div>

      <div class="entries-list">
        {#if loadingBranches}
          <div class="loading">
            <Loader2 size={16} class="spinner" />
            <span>Loading branches...</span>
          </div>
        {:else if filteredBranches.length === 0}
          <div class="empty">
            {baseBranchQuery ? 'No matching branches' : 'No branches found'}
          </div>
        {:else}
          {#each filteredBranches as branch, index (branch.name)}
            <button
              class="entry"
              class:selected={index === baseBranchSelectedIndex}
              class:default-branch={branch.name === selectedBaseBranch}
              onclick={() => selectBaseBranch(branch.name)}
            >
              {#if branch.isRemote}
                <Globe size={16} class="entry-icon remote-icon" />
              {:else}
                <GitBranch size={16} class="entry-icon local-icon" />
              {/if}
              <span class="entry-name">{branch.name}</span>
              {#if branch.name === selectedBaseBranch}
                <span class="default-badge">default</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    {:else}
      <!-- Branch name input -->
      <div class="name-step">
        <div class="selected-info">
          <div class="info-row">
            <GitBranch size={14} />
            <span class="info-label">Repository:</span>
            <span class="info-value">{repoName(selectedRepo ?? '')}</span>
          </div>
          <div class="info-row">
            <Globe size={14} />
            <span class="info-label">Base:</span>
            <span class="info-value">{selectedBaseBranch}</span>
          </div>
        </div>

        <div class="input-group">
          <label for="branch-name">Branch name</label>
          <input
            bind:this={branchInputEl}
            bind:value={branchName}
            id="branch-name"
            type="text"
            placeholder="feature/my-feature"
            class="branch-input"
          />
        </div>

        {#if error}
          <p class="error">{error}</p>
        {/if}

        <div class="actions">
          <button class="cancel-button" onclick={goBack}>Cancel</button>
          <button
            class="create-button"
            onclick={handleCreate}
            disabled={!branchName.trim() || creating}
          >
            {#if creating}
              <Loader2 size={14} class="spinner" />
            {/if}
            Create Branch
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 10vh;
    z-index: 1000;
  }

  .modal {
    width: 500px;
    max-width: 90vw;
    max-height: 70vh;
    background-color: var(--bg-primary);
    border-radius: 12px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-header h2 {
    flex: 1;
    margin: 0;
    font-size: var(--size-md);
    font-weight: 500;
    color: var(--text-primary);
  }

  .back-button,
  .close-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
  }

  .back-button:hover,
  .close-button:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Search */
  .search-container {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  :global(.search-icon) {
    color: var(--text-faint);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    padding: 8px 0;
    background: transparent;
    border: none;
    outline: none;
    font-size: var(--size-md);
    color: var(--text-primary);
  }

  .search-input::placeholder {
    color: var(--text-faint);
  }

  /* Breadcrumb */
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 16px;
    font-size: var(--size-sm);
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-subtle);
  }

  .breadcrumb-home {
    padding: 2px 6px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    font-family: 'SF Mono', 'Menlo', monospace;
    cursor: pointer;
  }

  .breadcrumb-home:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .breadcrumb-path {
    font-family: 'SF Mono', 'Menlo', monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Entries list */
  .entries-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .loading,
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    color: var(--text-muted);
    font-size: var(--size-sm);
  }

  .entry {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--size-sm);
    text-align: left;
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .entry:hover,
  .entry.selected {
    background-color: var(--bg-hover);
  }

  .entry.repo {
    color: var(--ui-accent);
  }

  :global(.entry-icon) {
    color: var(--text-faint);
    flex-shrink: 0;
  }

  :global(.repo-icon) {
    color: var(--status-renamed);
  }

  .entry-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.entry-chevron) {
    color: var(--text-faint);
    flex-shrink: 0;
  }

  /* Base branch picker */
  .selected-repo-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    font-size: var(--size-sm);
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-subtle);
  }

  .selected-repo-bar :global(svg) {
    color: var(--status-renamed);
  }

  :global(.remote-icon) {
    color: var(--text-muted) !important;
  }

  :global(.local-icon) {
    color: var(--status-renamed) !important;
  }

  .default-badge {
    padding: 2px 6px;
    background-color: var(--ui-accent);
    border-radius: 4px;
    font-size: 10px;
    font-weight: 500;
    color: var(--bg-deepest);
    text-transform: uppercase;
  }

  .entry.default-branch {
    background-color: var(--bg-hover);
  }

  /* Name step */
  .name-step {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .selected-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    background-color: var(--bg-hover);
    border-radius: 6px;
  }

  .info-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--size-sm);
  }

  .info-row :global(svg) {
    color: var(--text-faint);
    flex-shrink: 0;
  }

  .info-label {
    color: var(--text-muted);
  }

  .info-value {
    color: var(--text-primary);
    font-family: 'SF Mono', 'Menlo', monospace;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-group label {
    font-size: var(--size-sm);
    color: var(--text-muted);
  }

  .branch-input {
    padding: 10px 12px;
    background-color: var(--bg-primary);
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    font-size: var(--size-md);
    color: var(--text-primary);
    outline: none;
    transition: border-color 0.15s;
  }

  .branch-input:focus {
    border-color: var(--ui-accent);
  }

  .branch-input::placeholder {
    color: var(--text-faint);
  }

  .error {
    margin: 0;
    padding: 8px 12px;
    background-color: var(--ui-danger-bg);
    border-radius: 6px;
    font-size: var(--size-sm);
    color: var(--ui-danger);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }

  .cancel-button {
    padding: 8px 16px;
    background: transparent;
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    color: var(--text-muted);
    font-size: var(--size-sm);
    cursor: pointer;
    transition: all 0.15s;
  }

  .cancel-button:hover {
    border-color: var(--border-emphasis);
    color: var(--text-primary);
  }

  .create-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background-color: var(--ui-accent);
    border: none;
    border-radius: 6px;
    color: var(--bg-deepest);
    font-size: var(--size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.15s;
  }

  .create-button:hover:not(:disabled) {
    background-color: var(--ui-accent-hover);
  }

  .create-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Spinner */
  :global(.spinner) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
