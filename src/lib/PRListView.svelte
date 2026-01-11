<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { RefreshCw, Settings, AlertCircle, GitPullRequest, ExternalLink } from 'lucide-svelte';
  import { prState, loadPRs, startAutoRefresh, stopAutoRefresh } from './stores/prState.svelte';
  import PRListItem from './PRListItem.svelte';
  import PRSettingsModal from './PRSettingsModal.svelte';

  // Show settings modal
  let showSettingsModal = $state(false);

  // Load PRs on mount
  onMount(() => {
    loadPRs(false);
    startAutoRefresh(); // Start polling if enabled
  });

  onDestroy(() => {
    stopAutoRefresh();
  });

  function handleRefresh() {
    loadPRs(true);
  }

  function formatLastUpdated(date: Date | null): string {
    if (!date) return '';
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);

    if (minutes < 1) return 'Just now';
    if (minutes === 1) return '1 minute ago';
    if (minutes < 60) return `${minutes} minutes ago`;

    const hours = Math.floor(minutes / 60);
    if (hours === 1) return '1 hour ago';
    if (hours < 24) return `${hours} hours ago`;

    const days = Math.floor(hours / 24);
    if (days === 1) return '1 day ago';
    return `${days} days ago`;
  }
</script>

<div class="pr-list-view">
  <div class="pr-list-header">
    <h2>Pull Requests</h2>
    <div class="header-actions">
      {#if prState.lastFetched}
        <span class="last-updated">Updated {formatLastUpdated(prState.lastFetched)}</span>
      {/if}
      <button class="action-btn" onclick={handleRefresh} disabled={prState.loading} title="Refresh PR list">
        <span class:spinner={prState.loading}>
          <RefreshCw size={14} />
        </span>
        Refresh
      </button>
      <button class="action-btn" onclick={() => showSettingsModal = true} title="PR Settings">
        <Settings size={14} />
      </button>
    </div>
  </div>

  <div class="pr-list-body">
    {#if prState.loading && prState.pullRequests.length === 0}
      <div class="loading-state">
        <span class="spinner">
          <RefreshCw size={24} />
        </span>
        <span>Loading pull requests...</span>
      </div>
    {:else if !prState.authStatus?.authenticated}
      <div class="auth-required">
        <AlertCircle size={32} />
        <h3>GitHub CLI Required</h3>
        <p>To view pull requests, you need to authenticate with the GitHub CLI.</p>

        {#if prState.authStatus?.setup_hint}
          <div class="setup-hint">
            <code>{prState.authStatus.setup_hint}</code>
          </div>
        {/if}

        <div class="setup-steps">
          <p><strong>Setup:</strong></p>
          <ol>
            <li>Install GitHub CLI: <code>brew install gh</code></li>
            <li>Authenticate: <code>gh auth login</code></li>
            <li>Restart Staged and try again</li>
          </ol>
        </div>

        <a
          href="https://cli.github.com/"
          target="_blank"
          rel="noopener noreferrer"
          class="docs-link"
        >
          <ExternalLink size={12} />
          GitHub CLI Documentation
        </a>
      </div>
    {:else if prState.error}
      <div class="error-state">
        <AlertCircle size={24} />
        <span>{prState.error}</span>
        <button class="retry-btn" onclick={() => loadPRs(true)}>Try Again</button>
      </div>
    {:else if prState.pullRequests.length === 0}
      <div class="empty-state">
        <GitPullRequest size={32} />
        <span>No open pull requests</span>
      </div>
    {:else}
      <div class="pr-list">
        {#each prState.pullRequests as pr (pr.number)}
          <PRListItem {pr} on:checkout on:view />
        {/each}
      </div>
    {/if}
  </div>
</div>

{#if showSettingsModal}
  <PRSettingsModal onClose={() => showSettingsModal = false} />
{/if}

<style>
  .pr-list-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-chrome);
  }

  .pr-list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .pr-list-header h2 {
    margin: 0;
    font-size: var(--size-lg);
    font-weight: 600;
    color: var(--text-primary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .last-updated {
    font-size: var(--size-xs);
    color: var(--text-faint);
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-primary);
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    cursor: pointer;
    transition:
      background-color 0.1s,
      color 0.1s;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spinner {
    display: inline-flex;
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

  .pr-list-body {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  /* Loading state */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    flex: 1;
    color: var(--text-muted);
  }

  /* Auth required state */
  .auth-required {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 48px 32px;
    text-align: center;
    color: var(--text-muted);
  }

  .auth-required :global(svg) {
    color: var(--ui-warning);
    margin-bottom: 12px;
  }

  .auth-required h3 {
    margin: 0 0 8px;
    font-size: var(--size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .auth-required p {
    margin: 0 0 16px;
    font-size: var(--size-sm);
    line-height: 1.5;
  }

  .setup-hint {
    width: 100%;
    max-width: 400px;
    padding: 10px 12px;
    background: var(--bg-primary);
    border-radius: 6px;
    margin-bottom: 16px;
  }

  .setup-hint code {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: var(--size-xs);
    color: var(--ui-warning);
  }

  .setup-steps {
    text-align: left;
    width: 100%;
    max-width: 400px;
    padding: 12px 16px;
    background: var(--bg-primary);
    border-radius: 6px;
    margin-bottom: 16px;
  }

  .setup-steps p {
    margin: 0 0 8px;
    font-size: var(--size-sm);
  }

  .setup-steps ol {
    margin: 0;
    padding-left: 20px;
    font-size: var(--size-sm);
  }

  .setup-steps li {
    margin-bottom: 4px;
  }

  .setup-steps code {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: var(--size-xs);
    background: var(--bg-hover);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .docs-link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ui-accent);
    font-size: var(--size-sm);
    text-decoration: none;
  }

  .docs-link:hover {
    text-decoration: underline;
  }

  /* Error state */
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    flex: 1;
    color: var(--ui-danger);
    text-align: center;
    padding: 32px;
  }

  .retry-btn {
    padding: 8px 16px;
    background: var(--bg-hover);
    border: none;
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--size-sm);
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .retry-btn:hover {
    background: var(--border-subtle);
  }

  /* Empty state */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    flex: 1;
    color: var(--text-muted);
  }

  /* PR list */
  .pr-list {
    flex: 1;
  }
</style>
