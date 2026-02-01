<!--
  ArtifactCard.svelte - Card display for an artifact

  Shows artifact title, type indicator, preview, and metadata.
  Supports selection and contextual actions.
-->
<script lang="ts">
  import { FileText, GitCommit, Clock, Trash2, Maximize2 } from 'lucide-svelte';
  import type { Artifact } from './types';

  interface Props {
    artifact: Artifact;
    selected?: boolean;
    onSelect?: () => void;
    onOpenDetail?: () => void;
    onDelete?: () => void;
  }

  let { artifact, selected = false, onSelect, onOpenDetail, onDelete }: Props = $props();

  let isMarkdown = $derived(artifact.data.type === 'markdown');
  let isCommit = $derived(artifact.data.type === 'commit');

  // Get preview content for markdown artifacts
  let preview = $derived.by(() => {
    if (artifact.data.type !== 'markdown') return '';
    const content = artifact.data.content;
    // Strip markdown headers and get first meaningful line
    const lines = content.split('\n').filter((line) => {
      const trimmed = line.trim();
      return trimmed && !trimmed.startsWith('#');
    });
    const firstLine = lines[0] || '';
    // Truncate if too long
    return firstLine.length > 120 ? firstLine.slice(0, 120) + '...' : firstLine;
  });

  // Get commit info for commit artifacts
  let commitInfo = $derived.by(() => {
    if (artifact.data.type !== 'commit') return null;
    return {
      repo: artifact.data.repo,
      branch: artifact.data.branch,
      sha: artifact.data.commitSha.slice(0, 7),
    };
  });

  function formatRelativeTime(dateStr: string): string {
    const date = new Date(dateStr);
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
    if (confirm(`Delete "${artifact.title}"?`)) {
      onDelete?.();
    }
  }

  function handleOpenDetail(e: MouseEvent) {
    e.stopPropagation();
    onOpenDetail?.();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="artifact-card"
  class:selected
  class:markdown={isMarkdown}
  class:commit={isCommit}
  onclick={onSelect}
  role="button"
  tabindex="0"
>
  <div class="card-header">
    <div class="type-icon" class:markdown={isMarkdown} class:commit={isCommit}>
      {#if isMarkdown}
        <FileText size={14} />
      {:else}
        <GitCommit size={14} />
      {/if}
    </div>
    <h3 class="title">{artifact.title}</h3>
    <div class="header-actions">
      <button class="action-button expand-button" onclick={handleOpenDetail} title="Open">
        <Maximize2 size={14} />
      </button>
      <button class="action-button delete-button" onclick={handleDelete} title="Delete">
        <Trash2 size={14} />
      </button>
    </div>
  </div>

  <div class="card-content">
    {#if isMarkdown && preview}
      <p class="preview">{preview}</p>
    {:else if commitInfo}
      <div class="commit-info">
        <span class="repo">{commitInfo.repo}</span>
        <span class="separator">/</span>
        <span class="branch">{commitInfo.branch}</span>
        <span class="sha">{commitInfo.sha}</span>
      </div>
    {/if}
  </div>

  <div class="card-footer">
    <div class="timestamp">
      <Clock size={12} />
      <span>{formatRelativeTime(artifact.updatedAt)}</span>
    </div>
  </div>
</div>

<style>
  .artifact-card {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 16px;
    background-color: var(--bg-primary);
    border-radius: 8px;
    cursor: pointer;
    transition: transform 0.15s ease;
    text-align: left;
    min-height: 120px;
  }

  .artifact-card:hover {
    transform: translateY(-2px);
  }

  .artifact-card.selected {
    box-shadow: inset 0 0 0 1px var(--ui-accent);
  }

  /* Header */
  .card-header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-bottom: 12px;
  }

  .type-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    flex-shrink: 0;
  }

  .type-icon.markdown {
    background-color: rgba(88, 166, 255, 0.15);
    color: var(--text-accent);
  }

  .type-icon.commit {
    background-color: rgba(63, 185, 80, 0.15);
    color: var(--status-added);
  }

  .title {
    flex: 1;
    font-size: var(--size-md);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
    line-height: 1.4;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .artifact-card:hover .header-actions {
    opacity: 1;
  }

  .action-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
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
    flex: 1;
    margin-bottom: 12px;
  }

  .preview {
    font-size: var(--size-sm);
    color: var(--text-muted);
    margin: 0;
    line-height: 1.5;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .commit-info {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--size-sm);
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
  }

  .repo {
    color: var(--text-muted);
  }

  .separator {
    color: var(--text-faint);
  }

  .branch {
    color: var(--status-renamed);
  }

  .sha {
    color: var(--text-faint);
    margin-left: 8px;
  }

  /* Footer */
  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .timestamp {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--size-xs);
    color: var(--text-faint);
  }
</style>
