<!--
  BranchCard.svelte - Card display for a tracked branch

  Shows branch name, commit stack, and session controls.
  Commits are displayed newest-first with the HEAD commit having a "Continue" option.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    GitBranch,
    GitCommit,
    Eye,
    Plus,
    Trash2,
    Loader2,
    Clock,
    Play,
    MessageSquare,
  } from 'lucide-svelte';
  import type { Branch, CommitInfo, BranchSession } from './services/branch';
  import * as branchService from './services/branch';
  import SessionViewerModal from './SessionViewerModal.svelte';

  interface Props {
    branch: Branch;
    /** Increment to force a data refresh */
    refreshKey?: number;
    onNewSession?: () => void;
    onViewDiff?: () => void;
    onDelete?: () => void;
  }

  let { branch, refreshKey = 0, onNewSession, onViewDiff, onDelete }: Props = $props();

  // State
  let commits = $state<CommitInfo[]>([]);
  let runningSession = $state<BranchSession | null>(null);
  let loading = $state(true);

  // Session viewer modal state
  let showSessionViewer = $state(false);
  let viewingSession = $state<BranchSession | null>(null);

  // Load commits and running session on mount
  onMount(async () => {
    await loadData();
  });

  // Reload when refreshKey changes
  $effect(() => {
    if (refreshKey > 0) {
      loadData();
    }
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
    if (runningSession?.aiSessionId) {
      viewingSession = runningSession;
      showSessionViewer = true;
    }
  }

  function closeSessionViewer() {
    showSessionViewer = false;
    viewingSession = null;
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
      <!-- Commits list (with running session as skeleton at top) -->
      {#if commits.length > 0 || runningSession}
        <div class="commits-list">
          <!-- Running session as skeleton commit -->
          {#if runningSession}
            <button class="commit-row skeleton-commit" onclick={handleWatchSession}>
              <div class="commit-marker">
                <Loader2 size={12} class="spinner skeleton-spinner" />
                {#if commits.length > 0}
                  <div class="commit-line"></div>
                {/if}
              </div>
              <div class="commit-info">
                <span class="commit-subject skeleton-subject">{runningSession.prompt}</span>
                <div class="commit-meta">
                  <span class="commit-sha skeleton-sha">generating...</span>
                </div>
              </div>
              <div class="watch-button">
                <MessageSquare size={12} />
                Watch
              </div>
            </button>
          {/if}

          <!-- Real commits -->
          {#each commits as commit, index (commit.sha)}
            <div class="commit-row" class:is-head={index === 0 && !runningSession}>
              <div class="commit-marker">
                {#if index === 0 && !runningSession}
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

<!-- Session viewer modal -->
{#if showSessionViewer && viewingSession?.aiSessionId}
  <SessionViewerModal
    sessionId={viewingSession.aiSessionId}
    title={viewingSession.prompt}
    onClose={closeSessionViewer}
  />
{/if}

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

  /* Skeleton commit (running session) */
  .skeleton-commit {
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    width: 100%;
    border-radius: 6px;
    margin: -4px;
    padding: 10px;
    transition: background-color 0.15s ease;
  }

  .skeleton-commit:hover {
    background-color: var(--bg-hover);
  }

  :global(.skeleton-spinner) {
    color: var(--ui-accent);
  }

  .skeleton-subject {
    color: var(--text-muted);
    font-style: italic;
  }

  .skeleton-sha {
    background: linear-gradient(
      90deg,
      var(--bg-hover) 25%,
      var(--bg-primary) 50%,
      var(--bg-hover) 75%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
    border-radius: 4px;
    padding: 0 4px;
  }

  .watch-button {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    background-color: transparent;
    border: 1px solid var(--border-muted);
    border-radius: 4px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .skeleton-commit:hover .watch-button {
    border-color: var(--ui-accent);
    color: var(--ui-accent);
  }

  @keyframes shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
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
