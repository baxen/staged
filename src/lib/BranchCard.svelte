<!--
  BranchCard.svelte - Card display for a tracked branch

  Shows branch name, commit stack, and session controls.
  Commits are displayed newest-first with the HEAD commit having a "Continue" option.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { GitBranch, GitCommit, Eye, Plus, Trash2, Loader2, Clock, Play } from 'lucide-svelte';
  import type { Branch, CommitInfo, BranchSession } from './services/branch';
  import * as branchService from './services/branch';

  interface Props {
    branch: Branch;
    onNewSession?: () => void;
    onViewDiff?: () => void;
    onDelete?: () => void;
  }

  let { branch, onNewSession, onViewDiff, onDelete }: Props = $props();

  // State
  let commits = $state<CommitInfo[]>([]);
  let runningSession = $state<BranchSession | null>(null);
  let loading = $state(true);

  // Load commits and running session
  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    try {
      const [commitsResult, sessionResult] = await Promise.all([
        branchService.getBranchCommits(branch.id),
        branchService.getRunningSession(branch.id),
      ]);
      commits = commitsResult;
      runningSession = sessionResult;
    } catch (e) {
      console.error('Failed to load branch data:', e);
    } finally {
      loading = false;
    }
  }

  // Format relative time
  function formatRelativeTime(timestamp: number): string {
    const date = new Date(timestamp * 1000); // Unix timestamp is in seconds
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  }

  function handleDelete(e: MouseEvent) {
    e.stopPropagation();
    onDelete?.();
  }

  function handleContinue(commit: CommitInfo) {
    // TODO: Implement continue session (amend flow)
    console.log('Continue session for commit:', commit.sha);
  }

  function handleWatchSession() {
    // TODO: Open session viewer
    console.log('Watch running session');
  }
</script>

<div class="branch-card">
  <div class="card-header">
    <div class="branch-info">
      <GitBranch size={16} class="branch-icon" />
      <span class="branch-name">{branch.branchName}</span>
    </div>
    <div class="header-actions">
      <button class="action-button" onclick={onViewDiff} title="View diff">
        <Eye size={14} />
      </button>
      <button class="action-button delete-button" onclick={handleDelete} title="Delete branch">
        <Trash2 size={14} />
      </button>
    </div>
  </div>

  <div class="card-content">
    {#if loading}
      <div class="loading">
        <Loader2 size={14} class="spinner" />
        <span>Loading...</span>
      </div>
    {:else}
      <!-- Running session indicator -->
      {#if runningSession}
        <button class="running-session" onclick={handleWatchSession}>
          <div class="session-indicator">
            <Loader2 size={12} class="spinner" />
            <span class="session-label">Session running...</span>
          </div>
          <p class="session-prompt">{runningSession.prompt}</p>
        </button>
      {/if}

      <!-- Commits list -->
      {#if commits.length > 0}
        <div class="commits-list">
          {#each commits as commit, index (commit.sha)}
            <div class="commit-row" class:is-head={index === 0}>
              <div class="commit-marker">
                {#if index === 0}
                  <div class="head-marker"></div>
                {:else}
                  <div class="commit-dot"></div>
                {/if}
                {#if index < commits.length - 1}
                  <div class="commit-line"></div>
                {/if}
              </div>
              <div class="commit-info">
                <span class="commit-subject">{commit.subject}</span>
                <div class="commit-meta">
                  <span class="commit-sha">{commit.shortSha}</span>
                  <span class="commit-time">{formatRelativeTime(commit.timestamp)}</span>
                </div>
              </div>
              {#if index === 0 && !runningSession}
                <button class="continue-button" onclick={() => handleContinue(commit)}>
                  <Play size={12} />
                  Continue
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {:else if !runningSession}
        <p class="no-commits">No commits yet</p>
      {/if}
    {/if}
  </div>

  <div class="card-footer">
    <button class="new-session-button" onclick={onNewSession} disabled={!!runningSession}>
      <Plus size={14} />
      New Session
    </button>
  </div>
</div>

<style>
  .branch-card {
    display: flex;
    flex-direction: column;
    background-color: var(--bg-primary);
    border-radius: 8px;
    overflow: hidden;
  }

  /* Header */
  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .branch-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  :global(.branch-icon) {
    color: var(--status-renamed);
  }

  .branch-name {
    font-size: var(--size-md);
    font-weight: 500;
    color: var(--text-primary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .branch-card:hover .header-actions {
    opacity: 1;
  }

  .action-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-faint);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-button:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .action-button.delete-button:hover {
    background-color: var(--ui-danger-bg);
    color: var(--ui-danger);
  }

  /* Content */
  .card-content {
    padding: 12px 16px;
    min-height: 60px;
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: var(--size-sm);
  }

  /* Running session */
  .running-session {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    padding: 10px 12px;
    margin-bottom: 12px;
    background-color: var(--bg-hover);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.15s ease;
  }

  .running-session:hover {
    border-color: var(--ui-accent);
  }

  .session-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--ui-accent);
    font-size: var(--size-xs);
    font-weight: 500;
  }

  .session-label {
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .session-prompt {
    margin: 0;
    font-size: var(--size-sm);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Commits list */
  .commits-list {
    display: flex;
    flex-direction: column;
  }

  .commit-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 6px 0;
  }

  .commit-marker {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 12px;
    padding-top: 4px;
  }

  .head-marker {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background-color: var(--ui-accent);
    box-shadow:
      0 0 0 2px var(--bg-primary),
      0 0 0 3px var(--ui-accent);
  }

  .commit-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--border-emphasis);
  }

  .commit-line {
    flex: 1;
    width: 2px;
    min-height: 20px;
    background-color: var(--border-subtle);
    margin-top: 4px;
  }

  .commit-info {
    flex: 1;
    min-width: 0;
  }

  .commit-subject {
    display: block;
    font-size: var(--size-sm);
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .commit-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
  }

  .commit-sha {
    font-size: var(--size-xs);
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    color: var(--text-faint);
  }

  .commit-time {
    font-size: var(--size-xs);
    color: var(--text-faint);
  }

  .continue-button {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    background-color: transparent;
    border: 1px solid var(--border-muted);
    border-radius: 4px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    cursor: pointer;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .continue-button:hover {
    border-color: var(--ui-accent);
    color: var(--ui-accent);
    background-color: var(--bg-hover);
  }

  .no-commits {
    margin: 0;
    font-size: var(--size-sm);
    color: var(--text-faint);
    font-style: italic;
  }

  /* Footer */
  .card-footer {
    display: flex;
    justify-content: flex-end;
    padding: 12px 16px;
    border-top: 1px solid var(--border-subtle);
  }

  .new-session-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background-color: transparent;
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    color: var(--text-muted);
    font-size: var(--size-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .new-session-button:hover:not(:disabled) {
    border-color: var(--ui-accent);
    color: var(--ui-accent);
    background-color: var(--bg-hover);
  }

  .new-session-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Spinner animation */
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
