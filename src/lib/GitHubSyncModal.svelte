<script lang="ts">
  import {
    X,
    AlertCircle,
    Check,
    Upload,
    Download,
    ExternalLink,
    RefreshCw,
    Copy,
  } from 'lucide-svelte';
  import { syncReviewToGitHub, importPrComments } from './services/git';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import type { Comment, DiffSpec } from './types';

  interface Props {
    prNumber: number;
    spec: DiffSpec;
    repoPath: string | null;
    comments: Comment[];
    onClose: () => void;
    onCommentsImported?: () => void;
  }

  let { prNumber, spec, repoPath, comments, onClose, onCommentsImported }: Props = $props();

  // State
  let syncing = $state(false);
  let importing = $state(false);
  let error = $state<string | null>(null);
  let syncedUrl = $state<string | null>(null);
  let importedCount = $state<number | null>(null);
  let copied = $state(false);

  let commentCount = $derived(comments.length);

  async function handleSync() {
    if (commentCount === 0) return;

    syncing = true;
    error = null;

    try {
      const result = await syncReviewToGitHub(prNumber, spec, repoPath ?? undefined);
      syncedUrl = result.review_url;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      syncing = false;
    }
  }

  async function handleImport() {
    importing = true;
    error = null;
    importedCount = null;

    try {
      const count = await importPrComments(prNumber, spec, repoPath ?? undefined);
      importedCount = count;
      if (onCommentsImported) {
        onCommentsImported();
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      importing = false;
    }
  }

  async function copyUrl() {
    if (syncedUrl) {
      await writeText(syncedUrl);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onClose();
      event.preventDefault();
    } else if (event.key === 'Enter' && event.metaKey && !syncing && !syncedUrl && !importing && !importedCount) {
      handleSync();
      event.preventDefault();
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="modal-backdrop"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  onclick={handleBackdropClick}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
>
  <div class="modal">
    <header class="modal-header">
      <h2>Sync Comments with GitHub</h2>
      <button class="icon-btn" onclick={onClose}>
        <X size={16} />
      </button>
    </header>

    <div class="modal-body">
      {#if importedCount !== null}
        <!-- Import success state -->
        <div class="success-state">
          <Check size={32} />
          <h3>Comments Imported</h3>
          <p>
            {importedCount} comment{importedCount === 1 ? '' : 's'} from PR #{prNumber} {importedCount === 1 ? 'has' : 'have'} been imported to your local review.
          </p>
          <p class="hint">You can now view and respond to these comments locally.</p>
        </div>
      {:else if syncedUrl}
        <!-- Success state -->
        <div class="success-state">
          <Check size={32} />
          <h3>Review Created</h3>
          <p>
            Your {commentCount} comment{commentCount === 1 ? '' : 's'} have been synced to a pending review
            on GitHub.
          </p>
          <div class="url-box">
            <a href={syncedUrl} target="_blank" rel="noopener noreferrer" class="url-link">
              <ExternalLink size={12} />
              <span>{syncedUrl}</span>
            </a>
            <button class="copy-url-btn" onclick={copyUrl} title="Copy URL">
              {#if copied}
                <Check size={14} />
              {:else}
                <Copy size={14} />
              {/if}
            </button>
          </div>
          <p class="hint">Submit or discard the review on GitHub to finalize.</p>
        </div>
      {:else if commentCount === 0}
        <!-- No comments state -->
        <div class="empty-state">
          <AlertCircle size={32} />
          <h3>No Comments</h3>
          <p>Add some comments to your review before syncing to GitHub.</p>
        </div>
      {:else}
        <!-- Ready to sync state -->
        <div class="sync-content">
          <div class="pr-info">
            <span class="pr-label">PR #{prNumber}</span>
          </div>

          <div class="sync-actions">
            <div class="sync-action">
              <div class="action-header">
                <Download size={18} />
                <h3>Import from PR</h3>
              </div>
              <p class="action-description">
                Pull review comments from the GitHub PR into your local review.
              </p>
            </div>

            {#if commentCount > 0}
              <div class="sync-action">
                <div class="action-header">
                  <Upload size={18} />
                  <h3>Push to PR</h3>
                </div>
                <p class="action-description">
                  {commentCount} local comment{commentCount === 1 ? '' : 's'} will be synced to a pending review on GitHub.
                </p>
              </div>
            {/if}
          </div>

          {#if error}
            <div class="error">
              <AlertCircle size={14} />
              <span>{error}</span>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <footer class="modal-footer">
      <button class="btn btn-secondary" onclick={onClose}>
        {syncedUrl || importedCount !== null ? 'Done' : 'Cancel'}
      </button>
      {#if !syncedUrl && importedCount === null}
        <button class="btn btn-primary" onclick={handleImport} disabled={importing}>
          {#if importing}
            <RefreshCw size={14} class="spinner" />
            <span>Importing...</span>
          {:else}
            <Download size={14} />
            <span>Import Comments</span>
          {/if}
        </button>
        {#if commentCount > 0}
          <button class="btn btn-primary" onclick={handleSync} disabled={syncing}>
            {#if syncing}
              <RefreshCw size={14} class="spinner" />
              <span>Pushing...</span>
            {:else}
              <Upload size={14} />
              <span>Push Comments</span>
            {/if}
          </button>
        {/if}
      {/if}
    </footer>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--shadow-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-chrome);
    border-radius: 12px;
    box-shadow: var(--shadow-elevated);
    width: 400px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-header h2 {
    margin: 0;
    font-size: var(--size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      color 0.1s,
      background-color 0.1s;
  }

  .icon-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .modal-body {
    padding: 20px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  /* Success state */
  .success-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 12px;
  }

  .success-state :global(svg:first-child) {
    color: var(--status-added);
  }

  .success-state h3 {
    margin: 0;
    font-size: var(--size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .success-state p {
    margin: 0;
    font-size: var(--size-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .url-box {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-primary);
    border-radius: 6px;
    max-width: 100%;
    overflow: hidden;
  }

  .url-link {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--ui-accent);
    font-size: var(--size-xs);
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .url-link:hover {
    text-decoration: underline;
  }

  .url-link :global(svg) {
    flex-shrink: 0;
  }

  .copy-url-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition:
      color 0.1s,
      background-color 0.1s;
  }

  .copy-url-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .hint {
    font-size: var(--size-xs) !important;
    color: var(--text-faint) !important;
  }

  /* Empty state */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 12px;
    padding: 16px;
  }

  .empty-state :global(svg) {
    color: var(--ui-warning);
  }

  .empty-state h3 {
    margin: 0;
    font-size: var(--size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .empty-state p {
    margin: 0;
    font-size: var(--size-sm);
    color: var(--text-muted);
  }

  /* Sync content */
  .sync-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .pr-info {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: var(--bg-primary);
    border-radius: 6px;
  }

  .pr-label {
    font-weight: 600;
    color: var(--ui-accent);
    font-size: var(--size-sm);
  }

  .sync-actions {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .sync-action {
    padding: 12px;
    background: var(--bg-primary);
    border-radius: 6px;
    border: 1px solid var(--border-subtle);
  }

  .action-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .action-header h3 {
    margin: 0;
    font-size: var(--size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .action-header :global(svg) {
    color: var(--ui-accent);
  }

  .action-description {
    margin: 0;
    font-size: var(--size-xs);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .error {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: color-mix(in srgb, var(--status-deleted) 10%, transparent);
    border-radius: 6px;
    font-size: var(--size-sm);
    color: var(--status-deleted);
  }

  /* Buttons */
  .btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: var(--size-sm);
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color 0.1s,
      opacity 0.1s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--border-subtle);
  }

  .btn-primary {
    background: var(--ui-accent);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--ui-accent) 85%, black);
  }

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
