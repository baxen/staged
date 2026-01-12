<script lang="ts">
  import { GitPullRequest, GitMerge } from 'lucide-svelte';
  import type { PullRequest } from './types';
  import { createEventDispatcher } from 'svelte';

  interface Props {
    pr: PullRequest;
  }

  let { pr }: Props = $props();
  const dispatch = createEventDispatcher();

  function handleCheckout() {
    dispatch('checkout', { pr });
  }

  function handleView() {
    dispatch('view', { pr });
  }

  function formatLineChanges(additions: number, deletions: number): string {
    if (additions === 0 && deletions === 0) return '';
    const parts = [];
    if (additions > 0) parts.push(`+${additions}`);
    if (deletions > 0) parts.push(`-${deletions}`);
    return parts.join(' ');
  }
</script>

<div class="pr-list-item">
  <div class="pr-header">
    <span class="pr-number">#{pr.number}</span>
    <span class="pr-title">{pr.title}</span>
    {#if pr.draft}
      <span class="draft-badge">Draft</span>
    {/if}
  </div>

  <div class="pr-meta">
    <span class="pr-author">@{pr.author}</span>
    <span class="pr-refs">{pr.base_ref} ← {pr.head_ref}</span>
  </div>

  <div class="pr-actions">
    <button class="action-btn secondary" onclick={handleView}>
      <GitPullRequest size={14} />
      View Diff
    </button>
    <button class="action-btn primary" onclick={handleCheckout}>
      <GitMerge size={14} />
      Checkout
    </button>
  </div>
</div>

<style>
  .pr-list-item {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    transition: background-color 0.1s;
  }

  .pr-list-item:hover {
    background-color: var(--bg-hover);
  }

  .pr-list-item:last-child {
    border-bottom: none;
  }

  .pr-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .pr-number {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: var(--size-sm);
    font-weight: 600;
    color: var(--ui-accent);
    flex-shrink: 0;
  }

  .pr-title {
    flex: 1;
    font-size: var(--size-sm);
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .draft-badge {
    padding: 2px 6px;
    background: var(--bg-primary);
    border-radius: 4px;
    font-size: var(--size-xs);
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .pr-meta {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: var(--size-xs);
    color: var(--text-muted);
    flex-wrap: wrap;
  }

  .pr-author {
    color: var(--text-faint);
  }

  .pr-refs {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
  }

  .pr-actions {
    display: flex;
    gap: 8px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: none;
    border-radius: 6px;
    font-size: var(--size-xs);
    cursor: pointer;
    transition:
      background-color 0.1s,
      color 0.1s;
  }

  .action-btn :global(svg) {
    flex-shrink: 0;
  }

  .action-btn.secondary {
    background: var(--bg-primary);
    color: var(--text-muted);
  }

  .action-btn.secondary:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .action-btn.primary {
    background: var(--ui-accent);
    color: var(--bg-chrome);
  }

  .action-btn.primary:hover {
    opacity: 0.9;
  }
</style>
