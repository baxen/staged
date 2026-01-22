<!--
  SmartDiffModal.svelte - AI analysis overview modal
  
  Shows the changeset summary (TL;DR of all changes).
  This modal is only opened when results are available - 
  analysis runs in TopBar with loading indicator there.
-->
<script lang="ts">
  import { X, AlertTriangle, Orbit, RefreshCw } from 'lucide-svelte';
  import { smartDiffState } from './stores/smartDiff.svelte';

  interface Props {
    onClose: () => void;
    onRefresh?: () => void;
  }

  let { onClose, onRefresh }: Props = $props();

  // Derived state
  let summary = $derived(smartDiffState.changesetSummary);
  let isLoading = $derived(smartDiffState.loading);

  function handleClose() {
    onClose();
  }

  function handleRefresh() {
    if (onRefresh && !isLoading) {
      onRefresh();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="modal-backdrop" onclick={handleClose}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <header class="modal-header">
      <div class="header-title">
        <Orbit size={16} />
        <h2>AI Analysis</h2>
      </div>
      <div class="header-actions">
        {#if onRefresh}
          <button
            class="refresh-btn"
            class:loading={isLoading}
            onclick={handleRefresh}
            disabled={isLoading}
            title="Re-run analysis"
          >
            <RefreshCw size={14} />
          </button>
        {/if}
        <button class="close-btn" onclick={handleClose} title="Close (Esc)">
          <X size={16} />
        </button>
      </div>
    </header>

    <div class="modal-content">
      {#if summary}
        <div class="summary-section">
          <h3>Summary</h3>
          <p class="summary-text">{summary.summary}</p>
        </div>

        {#if summary.key_changes.length > 0}
          <div class="changes-section">
            <h3>Key Changes</h3>
            <ul class="changes-list">
              {#each summary.key_changes as change}
                <li>{change}</li>
              {/each}
            </ul>
          </div>
        {/if}

        {#if summary.concerns.length > 0}
          <div class="concerns-section">
            <h3>
              <AlertTriangle size={14} />
              Concerns
            </h3>
            <ul class="concerns-list">
              {#each summary.concerns as concern}
                <li>{concern}</li>
              {/each}
            </ul>
          </div>
        {/if}
      {:else}
        <div class="empty-state">
          <p>No analysis available</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-chrome);
    border: 1px solid var(--border-muted);
    border-radius: 12px;
    width: 90%;
    max-width: 560px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-elevated);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
  }

  .header-title h2 {
    margin: 0;
    font-size: var(--size-md);
    font-weight: 600;
  }

  .header-title :global(svg) {
    color: var(--ui-accent);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .refresh-btn,
  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    background: none;
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.1s;
  }

  .refresh-btn:hover:not(:disabled),
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .refresh-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .refresh-btn.loading :global(svg) {
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

  .modal-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  /* Empty state */
  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    color: var(--text-muted);
  }

  /* Summary section */
  .summary-section {
    margin-bottom: 24px;
  }

  .summary-section h3 {
    margin: 0 0 12px;
    font-size: var(--size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .summary-text {
    margin: 0;
    font-size: var(--size-sm);
    line-height: 1.6;
    color: var(--text-secondary);
  }

  /* Changes section */
  .changes-section {
    margin-bottom: 24px;
  }

  .changes-section h3 {
    margin: 0 0 12px;
    font-size: var(--size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .changes-list {
    margin: 0;
    padding-left: 20px;
    font-size: var(--size-sm);
    line-height: 1.6;
    color: var(--text-secondary);
  }

  .changes-list li {
    margin-bottom: 8px;
  }

  .changes-list li:last-child {
    margin-bottom: 0;
  }

  /* Concerns section */
  .concerns-section {
    background: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 16px;
  }

  .concerns-section h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 12px;
    font-size: var(--size-sm);
    font-weight: 600;
    color: var(--status-deleted);
  }

  .concerns-list {
    margin: 0;
    padding-left: 20px;
    font-size: var(--size-sm);
    line-height: 1.6;
    color: var(--text-secondary);
  }

  .concerns-list li {
    margin-bottom: 8px;
  }

  .concerns-list li:last-child {
    margin-bottom: 0;
  }
</style>
