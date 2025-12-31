<!--
  Sidebar.svelte - File list with review workflow
  
  Shows files changed in the current diff (base..head).
  Files needing review appear above the divider.
  Reviewed files appear below the divider.
  Review state comes from the review storage, not git index.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    MessageSquare,
    CircleFadingArrowUp,
    CircleFadingPlus,
    CircleArrowUp,
    CirclePlus,
    CircleMinus,
    CircleX,
    Check,
    RotateCcw,
  } from 'lucide-svelte';
  import { getReview, markReviewed, unmarkReviewed } from './services/review';
  import type { FileDiff, Review } from './types';
  import { getFilePath } from './diffUtils';

  interface FileEntry {
    path: string;
    status: 'added' | 'deleted' | 'modified' | 'renamed';
    isReviewed: boolean;
    commentCount: number;
  }

  interface Props {
    /** Called when user selects a file to view */
    onFileSelect?: (path: string) => void;
    /** Currently selected file path */
    selectedFile?: string | null;
    /** Base ref for the diff (controlled by parent) */
    diffBase?: string;
    /** Head ref for the diff (controlled by parent) */
    diffHead?: string;
  }

  let { onFileSelect, selectedFile = null, diffBase = 'HEAD', diffHead = '@' }: Props = $props();

  let diffs: FileDiff[] = $state([]);
  let review: Review | null = $state(null);
  let loading = $state(true);

  // Is this viewing the working tree?
  let isWorkingTree = $derived(diffHead === '@');

  /**
   * Determine file status from a FileDiff.
   */
  function getFileStatus(diff: FileDiff): 'added' | 'deleted' | 'modified' | 'renamed' {
    if (diff.before === null) return 'added';
    if (diff.after === null) return 'deleted';
    if (diff.before.path !== diff.after.path) return 'renamed';
    return 'modified';
  }

  /**
   * Build file list from diffs with review state.
   */
  function buildFileList(fileDiffs: FileDiff[], reviewData: Review | null): FileEntry[] {
    const reviewedSet = new Set(reviewData?.reviewed || []);

    // Count comments per file
    const commentCounts = new Map<string, number>();
    for (const comment of reviewData?.comments || []) {
      commentCounts.set(comment.path, (commentCounts.get(comment.path) || 0) + 1);
    }

    return fileDiffs.map((diff) => {
      const path = getFilePath(diff) || '';
      return {
        path,
        status: getFileStatus(diff),
        isReviewed: reviewedSet.has(path),
        commentCount: commentCounts.get(path) || 0,
      };
    });
  }

  /**
   * Set diffs from external source (App.svelte).
   */
  export function setDiffs(newDiffs: FileDiff[]) {
    diffs = newDiffs;
    loading = false;
  }

  let files = $derived(buildFileList(diffs, review));
  let needsReview = $derived(files.filter((f) => !f.isReviewed));
  let reviewed = $derived(files.filter((f) => f.isReviewed));

  onMount(() => {
    loadReview();
  });

  // Reload review when diff spec changes
  $effect(() => {
    // Track diffBase and diffHead to trigger reload
    const _ = diffBase + diffHead;
    loadReview();
  });

  async function loadReview() {
    try {
      review = await getReview(diffBase, diffHead);
    } catch (e) {
      console.error('Failed to load review:', e);
      review = null;
    }
  }

  function selectFile(file: FileEntry) {
    onFileSelect?.(file.path);
  }

  async function toggleReviewed(event: MouseEvent, file: FileEntry) {
    event.stopPropagation();
    try {
      if (file.isReviewed) {
        await unmarkReviewed(diffBase, diffHead, file.path);
      } else {
        await markReviewed(diffBase, diffHead, file.path);
      }
      // Reload review state
      review = await getReview(diffBase, diffHead);
    } catch (e) {
      console.error('Failed to toggle reviewed:', e);
    }
  }

  function getFileName(path: string): string {
    return path.split('/').pop() || path;
  }

  function getFileDir(path: string): string {
    const parts = path.split('/');
    if (parts.length > 1) {
      return parts.slice(0, -1).join('/') + '/';
    }
    return '';
  }
</script>

<div class="sidebar-content">
  {#if loading}
    <div class="loading">Loading...</div>
  {:else if files.length === 0}
    <div class="empty-state">
      <p>No changes</p>
      {#if isWorkingTree}
        <p class="empty-hint">Working tree is clean</p>
      {:else}
        <p class="empty-hint">No differences between refs</p>
      {/if}
    </div>
  {:else}
    <div class="file-list">
      <!-- Needs Review section -->
      {#if needsReview.length > 0}
        <ul class="file-section">
          {#each needsReview as file (file.path)}
            <li
              class="file-item"
              class:selected={selectedFile === file.path}
              onclick={() => selectFile(file)}
              tabindex="0"
              role="button"
            >
              <!-- Status icon - clickable to toggle reviewed -->
              <button
                class="status-icon"
                onclick={(e) => toggleReviewed(e, file)}
                title="Mark as reviewed"
              >
                <!-- Default icon (hidden on hover) -->
                <span class="icon-default">
                  {#if file.status === 'added'}
                    {#if isWorkingTree}
                      <CircleFadingPlus size={16} />
                    {:else}
                      <CirclePlus size={16} />
                    {/if}
                  {:else if file.status === 'deleted'}
                    {#if isWorkingTree}
                      <CircleX size={16} />
                    {:else}
                      <CircleMinus size={16} />
                    {/if}
                  {:else if isWorkingTree}
                    <CircleFadingArrowUp size={16} />
                  {:else}
                    <CircleArrowUp size={16} />
                  {/if}
                </span>
                <!-- Hover icon (checkmark for "mark as reviewed") -->
                <span class="icon-hover">
                  <Check size={16} />
                </span>
              </button>

              <!-- File path -->
              <span class="file-path">
                <span class="file-dir">{getFileDir(file.path)}</span>
                <span class="file-name">{getFileName(file.path)}</span>
              </span>

              <!-- Comment indicator -->
              {#if file.commentCount > 0}
                <span class="comment-indicator">
                  <MessageSquare size={12} />
                  <span class="comment-count">{file.commentCount}</span>
                </span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}

      <!-- Divider with REVIEWED label -->
      {#if reviewed.length > 0}
        <div class="section-divider">
          <span class="divider-label">REVIEWED</span>
        </div>
      {/if}

      <!-- Reviewed section -->
      {#if reviewed.length > 0}
        <ul class="file-section reviewed-section">
          {#each reviewed as file (file.path)}
            <li
              class="file-item"
              class:selected={selectedFile === file.path}
              onclick={() => selectFile(file)}
              tabindex="0"
              role="button"
            >
              <!-- Status icon - clickable to toggle reviewed -->
              <button
                class="status-icon"
                onclick={(e) => toggleReviewed(e, file)}
                title="Mark as needs review"
              >
                <!-- Default icon (hidden on hover) -->
                <span class="icon-default">
                  {#if file.status === 'added'}
                    {#if isWorkingTree}
                      <CircleFadingPlus size={16} />
                    {:else}
                      <CirclePlus size={16} />
                    {/if}
                  {:else if file.status === 'deleted'}
                    {#if isWorkingTree}
                      <CircleX size={16} />
                    {:else}
                      <CircleMinus size={16} />
                    {/if}
                  {:else if isWorkingTree}
                    <CircleFadingArrowUp size={16} />
                  {:else}
                    <CircleArrowUp size={16} />
                  {/if}
                </span>
                <!-- Hover icon (rotate for "unmark as reviewed") -->
                <span class="icon-hover icon-hover-unreview">
                  <RotateCcw size={16} />
                </span>
              </button>

              <span class="file-path">
                <span class="file-dir">{getFileDir(file.path)}</span>
                <span class="file-name">{getFileName(file.path)}</span>
              </span>

              {#if file.commentCount > 0}
                <span class="comment-indicator">
                  <MessageSquare size={12} />
                  <span class="comment-count">{file.commentCount}</span>
                </span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>

<style>
  .sidebar-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .loading,
  .empty-state {
    padding: 20px 16px;
    text-align: center;
    color: var(--text-muted);
  }

  .empty-state p {
    margin: 0;
  }

  .empty-hint {
    font-size: var(--size-sm);
    margin-top: 4px !important;
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .file-section {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  /* Divider with REVIEWED label */
  .section-divider {
    display: flex;
    align-items: center;
    margin: 8px 12px;
    gap: 8px;
  }

  .section-divider::before,
  .section-divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border-subtle);
  }

  .divider-label {
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 0.5px;
    color: var(--text-faint);
    text-transform: uppercase;
  }

  .reviewed-section {
    opacity: 0.7;
  }

  .file-item {
    display: flex;
    align-items: center;
    padding: 3px 8px;
    font-size: var(--size-md);
    gap: 6px;
    cursor: pointer;
    position: relative;
    border-radius: 6px;
    margin: 0 4px;
    transition:
      background-color 0.1s,
      box-shadow 0.1s;
  }

  .file-item:hover {
    background-color: var(--bg-hover);
  }

  .file-item.selected {
    background-color: var(--bg-primary);
  }

  /* Status icon as button */
  .status-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: none;
    border: none;
    padding: 2px;
    margin: -2px;
    cursor: pointer;
    color: var(--text-muted);
    border-radius: 3px;
    transition:
      color 0.1s,
      background-color 0.1s;
  }

  .status-icon:hover {
    background-color: var(--bg-hover);
    color: var(--status-added);
  }

  /* Icon swap on hover */
  .icon-default,
  .icon-hover {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon-hover {
    display: none;
  }

  .status-icon:hover .icon-default {
    display: none;
  }

  .status-icon:hover .icon-hover {
    display: flex;
  }

  /* Unreview hover icon uses muted color instead of green */
  .status-icon:hover .icon-hover-unreview {
    color: var(--text-muted);
  }

  .file-path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    direction: rtl;
    text-align: left;
  }

  .file-dir {
    color: var(--text-muted);
  }

  .file-name {
    color: var(--text-primary);
  }

  /* Comment indicator */
  .comment-indicator {
    display: flex;
    align-items: center;
    gap: 2px;
    color: var(--text-muted);
    font-size: 10px;
    flex-shrink: 0;
  }

  .comment-count {
    font-family: monospace;
  }
</style>
