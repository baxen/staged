<!--
  ArtifactDetail.svelte - Detail view for a selected artifact

  Shows the full content of a markdown artifact or commit info.
  Shows generating state with placeholder when artifact is being created.
  Supports editing and refinement actions.
-->
<script lang="ts">
  import { X, FileText, GitCommit, Clock, Loader2, AlertCircle } from 'lucide-svelte';
  import type { Artifact } from './types';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';

  interface Props {
    artifact: Artifact;
    onClose?: () => void;
  }

  let { artifact, onClose }: Props = $props();

  let isMarkdown = $derived(artifact.data.type === 'markdown');
  let isCommit = $derived(artifact.data.type === 'commit');
  let isGenerating = $derived(artifact.status === 'generating');
  let isError = $derived(artifact.status === 'error');

  // Render markdown content
  let renderedContent = $derived.by(() => {
    if (artifact.data.type !== 'markdown') return '';
    const rawHtml = marked(artifact.data.content) as string;
    return DOMPurify.sanitize(rawHtml);
  });

  // Get commit info
  let commitInfo = $derived.by(() => {
    if (artifact.data.type !== 'commit') return null;
    return {
      repo: artifact.data.repo,
      branch: artifact.data.branch,
      sha: artifact.data.commitSha,
    };
  });

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  }
</script>

<div class="artifact-detail">
  <header class="detail-header">
    <div class="header-left">
      <div class="type-icon" class:markdown={isMarkdown} class:commit={isCommit}>
        {#if isMarkdown}
          <FileText size={16} />
        {:else}
          <GitCommit size={16} />
        {/if}
      </div>
      <h2 class="title">{artifact.title}</h2>
    </div>
    <button class="close-button" onclick={onClose} title="Close">
      <X size={18} />
    </button>
  </header>

  <div class="detail-meta">
    <span class="meta-item">
      <Clock size={12} />
      Created {formatDate(artifact.createdAt)}
    </span>
    {#if artifact.updatedAt !== artifact.createdAt}
      <span class="meta-item">
        Updated {formatDate(artifact.updatedAt)}
      </span>
    {/if}
  </div>

  <div class="detail-content">
    {#if isGenerating}
      <div class="generating-content">
        <div class="generating-indicator">
          <Loader2 size={24} class="spinner" />
          <span>Generating with AI...</span>
        </div>
        <p class="generating-hint">
          The artifact content will appear here when generation completes.
        </p>
      </div>
    {:else if isError}
      <div class="error-content">
        <div class="error-indicator">
          <AlertCircle size={24} />
          <span>Generation Failed</span>
        </div>
        <p class="error-message">{artifact.errorMessage || 'An unknown error occurred'}</p>
      </div>
    {:else if isMarkdown}
      <div class="markdown-content">
        {@html renderedContent}
      </div>
    {:else if commitInfo}
      <div class="commit-content">
        <div class="commit-field">
          <span class="field-label">Repository</span>
          <span class="field-value">{commitInfo.repo}</span>
        </div>
        <div class="commit-field">
          <span class="field-label">Branch</span>
          <span class="field-value branch">{commitInfo.branch}</span>
        </div>
        <div class="commit-field">
          <span class="field-label">Commit</span>
          <span class="field-value mono">{commitInfo.sha}</span>
        </div>
        <button class="view-diff-button"> View Diff </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .artifact-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-primary);
    border-radius: 8px;
    overflow: hidden;
  }

  /* Header */
  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    background-color: var(--bg-primary);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .type-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
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
    font-size: var(--size-lg);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .close-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .close-button:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Meta */
  .detail-meta {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 20px 12px 20px;
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--size-sm);
    color: var(--text-faint);
  }

  /* Content */
  .detail-content {
    flex: 1;
    overflow: auto;
    padding: 20px;
  }

  /* Generating state */
  .generating-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    color: var(--text-muted);
  }

  .generating-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--text-accent);
    font-size: var(--size-lg);
  }

  .generating-hint {
    font-size: var(--size-sm);
    color: var(--text-faint);
    margin: 0;
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

  /* Error state */
  .error-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
  }

  .error-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--ui-danger);
    font-size: var(--size-lg);
  }

  .error-message {
    font-size: var(--size-sm);
    color: var(--ui-danger);
    margin: 0;
    text-align: center;
    max-width: 400px;
  }

  /* Markdown content */
  .markdown-content {
    font-size: var(--size-md);
    line-height: 1.6;
    color: var(--text-primary);
  }

  .markdown-content :global(h1) {
    font-size: var(--size-xl);
    font-weight: 600;
    margin: 0 0 16px 0;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .markdown-content :global(h2) {
    font-size: var(--size-lg);
    font-weight: 600;
    margin: 24px 0 12px 0;
  }

  .markdown-content :global(h3) {
    font-size: var(--size-md);
    font-weight: 600;
    margin: 20px 0 8px 0;
  }

  .markdown-content :global(p) {
    margin: 0 0 12px 0;
  }

  .markdown-content :global(ul),
  .markdown-content :global(ol) {
    margin: 0 0 12px 0;
    padding-left: 24px;
  }

  .markdown-content :global(li) {
    margin: 4px 0;
  }

  .markdown-content :global(code) {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: var(--size-sm);
    background-color: var(--bg-elevated);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .markdown-content :global(pre) {
    background-color: var(--bg-deepest);
    border-radius: 8px;
    padding: 16px;
    overflow-x: auto;
    margin: 12px 0;
  }

  .markdown-content :global(pre code) {
    background: none;
    padding: 0;
  }

  .markdown-content :global(strong) {
    font-weight: 600;
  }

  .markdown-content :global(a) {
    color: var(--text-accent);
    text-decoration: none;
  }

  .markdown-content :global(a:hover) {
    text-decoration: underline;
  }

  /* Commit content */
  .commit-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .commit-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: var(--size-sm);
    color: var(--text-faint);
  }

  .field-value {
    font-size: var(--size-md);
    color: var(--text-primary);
  }

  .field-value.branch {
    color: var(--status-renamed);
  }

  .field-value.mono {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: var(--size-sm);
  }

  .view-diff-button {
    align-self: flex-start;
    padding: 10px 20px;
    background-color: var(--ui-accent);
    border: none;
    border-radius: 8px;
    color: var(--bg-deepest);
    font-size: var(--size-md);
    font-weight: 500;
    cursor: pointer;
    margin-top: 8px;
    transition: background-color 0.15s ease;
  }

  .view-diff-button:hover {
    background-color: var(--ui-accent-hover);
  }
</style>
