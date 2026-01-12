<script lang="ts">
  import { X } from 'lucide-svelte';
  import {
    prSettings,
    updateRefreshInterval,
    updatePRLimit,
    toggleAutoRefresh,
    resetPRSettings,
  } from './stores/prSettings.svelte';
  import { restartAutoRefresh } from './stores/prState.svelte';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  // Local state for inputs (in user-friendly units)
  let refreshMinutes = $state(prSettings.refreshInterval / 60000);
  let limit = $state(prSettings.prLimit);
  let autoRefresh = $state(prSettings.autoRefreshEnabled);

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onClose();
    }
  }

  function handleRefreshIntervalChange() {
    updateRefreshInterval(refreshMinutes * 60000);
    restartAutoRefresh(); // Restart polling with new interval
  }

  function handlePRLimitChange() {
    updatePRLimit(limit);
  }

  function handleAutoRefreshToggle() {
    toggleAutoRefresh();
    autoRefresh = prSettings.autoRefreshEnabled;
    restartAutoRefresh(); // Restart or stop polling based on new state
  }

  function handleReset() {
    resetPRSettings();
    // Update local state to reflect reset
    refreshMinutes = prSettings.refreshInterval / 60000;
    limit = prSettings.prLimit;
    autoRefresh = prSettings.autoRefreshEnabled;
    restartAutoRefresh();
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
      <h2>Pull Request Settings</h2>
      <button class="icon-btn" onclick={onClose}>
        <X size={16} />
      </button>
    </header>

    <div class="modal-body">
      <div class="setting-group">
        <label class="checkbox-label">
          <input type="checkbox" checked={autoRefresh} onchange={handleAutoRefreshToggle} />
          <span>Auto-refresh pull requests</span>
        </label>
        <p class="setting-description">
          Automatically fetch the latest PR list at the configured interval
        </p>
      </div>

      <div class="setting-group">
        <label class="setting-label">
          <span>Refresh interval</span>
          <span class="setting-value">{refreshMinutes} min</span>
        </label>
        <input
          type="range"
          min="1"
          max="10"
          bind:value={refreshMinutes}
          onchange={handleRefreshIntervalChange}
          disabled={!autoRefresh}
          class="slider"
        />
        <p class="setting-description">How often to check for new pull requests (1-10 minutes)</p>
      </div>

      <div class="setting-group">
        <label class="setting-label">
          <span>PR fetch limit</span>
          <span class="setting-value">{limit}</span>
        </label>
        <input
          type="number"
          min="10"
          max="100"
          bind:value={limit}
          onchange={handlePRLimitChange}
          class="number-input"
        />
        <p class="setting-description">Maximum number of PRs to fetch (10-100)</p>
      </div>
    </div>

    <div class="modal-footer">
      <button class="secondary-btn" onclick={handleReset}>Reset to Defaults</button>
      <button class="primary-btn" onclick={onClose}>Done</button>
    </div>
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
    width: 480px;
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
    flex-shrink: 0;
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
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: var(--size-sm);
    color: var(--text-primary);
  }

  .checkbox-label input[type='checkbox'] {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }

  .setting-label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--size-sm);
    color: var(--text-primary);
    font-weight: 500;
  }

  .setting-value {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    color: var(--text-muted);
    font-weight: normal;
  }

  .setting-description {
    margin: 0;
    font-size: var(--size-xs);
    color: var(--text-faint);
    line-height: 1.4;
  }

  .slider {
    width: 100%;
    height: 4px;
    border-radius: 2px;
    background: var(--bg-hover);
    outline: none;
    cursor: pointer;
  }

  .slider:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .number-input {
    padding: 8px 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--size-sm);
    transition:
      border-color 0.1s,
      background-color 0.1s;
  }

  .number-input:focus {
    outline: none;
    border-color: var(--border-emphasis);
    background-color: var(--bg-hover);
  }

  .modal-footer {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .secondary-btn {
    padding: 8px 16px;
    background: var(--bg-primary);
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    font-size: var(--size-sm);
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color 0.1s,
      color 0.1s;
  }

  .secondary-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .primary-btn {
    padding: 8px 16px;
    background: var(--ui-accent);
    border: none;
    border-radius: 6px;
    color: var(--bg-chrome);
    font-size: var(--size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.1s;
  }

  .primary-btn:hover {
    opacity: 0.9;
  }
</style>
