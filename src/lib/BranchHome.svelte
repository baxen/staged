<!--
  BranchHome.svelte - Branch-based workflow homepage

  Shows all tracked branches grouped by repository, with their commit stacks.
  Each branch has a worktree for isolated development.

  Keyboard shortcuts:
  - Cmd+N: New branch
  - Escape: Close modals
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Plus, Sparkles, GitBranch } from 'lucide-svelte';
  import type { Branch, CommitInfo, BranchSession } from './services/branch';
  import * as branchService from './services/branch';
  import BranchCard from './BranchCard.svelte';
  import NewBranchModal from './NewBranchModal.svelte';
  import NewSessionModal from './NewSessionModal.svelte';

  // State
  let branches = $state<Branch[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Modal state
  let showNewBranchModal = $state(false);
  let showNewSessionModal = $state(false);
  let sessionBranch = $state<Branch | null>(null);

  // Group branches by repo path
  let branchesByRepo = $derived.by(() => {
    const grouped = new Map<string, Branch[]>();
    for (const branch of branches) {
      const existing = grouped.get(branch.repoPath) || [];
      existing.push(branch);
      grouped.set(branch.repoPath, existing);
    }
    return grouped;
  });

  // Extract repo name from path
  function repoName(path: string): string {
    const parts = path.split('/');
    return parts[parts.length - 1] || path;
  }

  // Load branches on mount
  onMount(async () => {
    await loadBranches();
  });

  async function loadBranches() {
    loading = true;
    error = null;
    try {
      branches = await branchService.listBranches();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function handleNewBranch() {
    showNewBranchModal = true;
  }

  async function handleBranchCreated(branch: Branch) {
    branches = [...branches, branch];
    showNewBranchModal = false;
  }

  async function handleDeleteBranch(branchId: string) {
    const branch = branches.find((b) => b.id === branchId);
    if (!branch) return;

    if (!confirm(`Delete branch "${branch.branchName}" and its worktree?`)) return;

    try {
      await branchService.deleteBranch(branchId);
      branches = branches.filter((b) => b.id !== branchId);
    } catch (e) {
      console.error('Failed to delete branch:', e);
    }
  }

  function handleNewSession(branch: Branch) {
    sessionBranch = branch;
    showNewSessionModal = true;
  }

  function handleViewDiff(branch: Branch) {
    // TODO: Open diff viewer with base..branch-tip
    console.log('View diff for branch:', branch.branchName);
  }

  // Keyboard shortcuts
  function handleKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA';

    if (isInput && e.key !== 'Escape') {
      return;
    }

    // Cmd+N - New branch
    if (e.metaKey && e.key === 'n') {
      e.preventDefault();
      handleNewBranch();
      return;
    }

    // Escape - Close modals
    if (e.key === 'Escape') {
      if (showNewBranchModal) {
        e.preventDefault();
        showNewBranchModal = false;
      } else if (showNewSessionModal) {
        e.preventDefault();
        showNewSessionModal = false;
      }
      return;
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="branch-home">
  <div class="content">
    {#if loading}
      <div class="loading-state">
        <p>Loading...</p>
      </div>
    {:else if error}
      <div class="error-state">
        <p>{error}</p>
      </div>
    {:else if branches.length === 0}
      <div class="empty-state">
        <Sparkles size={48} strokeWidth={1} />
        <h2>Welcome to Staged</h2>
        <p>Create a branch to start working</p>
        <button class="create-button" onclick={handleNewBranch}>
          <Plus size={16} />
          New Branch
        </button>
        <span class="shortcut-hint">or press ⌘N</span>
      </div>
    {:else}
      <!-- Branches grouped by repo -->
      <div class="repos-list">
        {#each [...branchesByRepo.entries()] as [repoPath, repoBranches] (repoPath)}
          <div class="repo-section">
            <div class="repo-header">
              <span class="repo-path">{repoPath}</span>
            </div>
            <div class="branches-list">
              {#each repoBranches as branch (branch.id)}
                <BranchCard
                  {branch}
                  onNewSession={() => handleNewSession(branch)}
                  onViewDiff={() => handleViewDiff(branch)}
                  onDelete={() => handleDeleteBranch(branch.id)}
                />
              {/each}
            </div>
          </div>
        {/each}

        <!-- New branch button at the bottom -->
        <div class="new-branch-section">
          <button class="new-branch-button" onclick={handleNewBranch}>
            <Plus size={16} />
            New Branch
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<!-- New branch modal -->
{#if showNewBranchModal}
  <NewBranchModal onCreated={handleBranchCreated} onClose={() => (showNewBranchModal = false)} />
{/if}

<!-- New session modal -->
{#if showNewSessionModal && sessionBranch}
  <NewSessionModal
    branch={sessionBranch}
    onClose={() => {
      showNewSessionModal = false;
      sessionBranch = null;
    }}
  />
{/if}

<style>
  .branch-home {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-chrome);
  }

  .content {
    flex: 1;
    overflow: auto;
    padding: 24px;
  }

  .loading-state,
  .error-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  .error-state {
    color: var(--ui-danger);
  }

  /* Empty state */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    color: var(--text-muted);
  }

  .empty-state h2 {
    font-size: var(--size-xl);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
  }

  .empty-state p {
    margin: 0;
    color: var(--text-muted);
  }

  .create-button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background-color: var(--ui-accent);
    border: none;
    border-radius: 8px;
    color: var(--bg-deepest);
    font-size: var(--size-md);
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .create-button:hover {
    background-color: var(--ui-accent-hover);
  }

  .shortcut-hint {
    font-size: var(--size-sm);
    color: var(--text-faint);
  }

  /* Repos list */
  .repos-list {
    max-width: 800px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .repo-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .repo-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .repo-path {
    font-size: var(--size-sm);
    color: var(--text-muted);
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
  }

  .branches-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* New branch section */
  .new-branch-section {
    display: flex;
    justify-content: center;
    padding-top: 8px;
  }

  .new-branch-button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background-color: transparent;
    border: 1px dashed var(--border-muted);
    border-radius: 8px;
    color: var(--text-muted);
    font-size: var(--size-md);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .new-branch-button:hover {
    border-color: var(--ui-accent);
    color: var(--ui-accent);
    background-color: var(--bg-hover);
  }
</style>
