<!--
  ActionOutputModal.svelte - View output from a running or completed action

  Shows streaming output from action execution with ANSI color support.
  Subscribes to action_output and action_status events for real-time updates.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { X, Loader2, AlertCircle, StopCircle, RotateCcw, CheckCircle2 } from 'lucide-svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    /** The action execution ID */
    executionId: string;
    /** The action definition ID */
    actionId: string;
    /** The action name to display */
    actionName: string;
    /** The branch ID this action is running on */
    branchId: string;
    onClose: () => void;
  }

  let { executionId, actionId, actionName, branchId, onClose }: Props = $props();

  // ==========================================================================
  // State
  // ==========================================================================

  let outputLines = $state<Array<{ text: string; stream: 'stdout' | 'stderr' }>>([]);
  let isRunning = $state(true);
  let exitCode = $state<number | null>(null);
  let error = $state<string | null>(null);
  let startedAt = $state<number>(Date.now());
  let completedAt = $state<number | null>(null);

  // Refs
  let outputContainer: HTMLDivElement;
  let unlistenOutput: UnlistenFn | null = null;
  let unlistenStatus: UnlistenFn | null = null;

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  onMount(async () => {
    // Listen for output events
    unlistenOutput = await listen('action_output', (event: any) => {
      const payload = event.payload as {
        executionId: string;
        chunk: string;
        stream: string;
      };

      if (payload.executionId === executionId) {
        outputLines.push({
          text: payload.chunk,
          stream: payload.stream as 'stdout' | 'stderr',
        });
        scrollToBottom();
      }
    });

    // Listen for status events
    unlistenStatus = await listen('action_status', (event: any) => {
      const payload = event.payload as {
        executionId: string;
        branchId: string;
        status: string;
        exitCode: number | null;
        startedAt: number;
        completedAt: number | null;
      };

      if (payload.executionId === executionId) {
        startedAt = payload.startedAt;
        completedAt = payload.completedAt;

        if (
          payload.status === 'completed' ||
          payload.status === 'failed' ||
          payload.status === 'stopped'
        ) {
          isRunning = false;
          exitCode = payload.exitCode;
        }
      }
    });
  });

  onDestroy(() => {
    if (unlistenOutput) unlistenOutput();
    if (unlistenStatus) unlistenStatus();
  });

  // ==========================================================================
  // Actions
  // ==========================================================================

  async function handleStop() {
    try {
      await invoke('stop_branch_action', { executionId });
      isRunning = false;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function handleRestart() {
    try {
      const newExecutionId = await invoke<string>('restart_branch_action', {
        branchId,
        actionId,
      });

      // Reset state for new execution
      outputLines = [];
      isRunning = true;
      exitCode = null;
      error = null;
      startedAt = Date.now();
      completedAt = null;

      // Update to track new execution
      executionId = newExecutionId;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  // ==========================================================================
  // Helpers
  // ==========================================================================

  function scrollToBottom() {
    tick().then(() => {
      if (outputContainer) {
        outputContainer.scrollTop = outputContainer.scrollHeight;
      }
    });
  }

  function formatDuration(): string {
    const end = completedAt || Date.now();
    const duration = Math.floor((end - startedAt) / 1000);

    if (duration < 60) {
      return `${duration}s`;
    }

    const minutes = Math.floor(duration / 60);
    const seconds = duration % 60;
    return `${minutes}m ${seconds}s`;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" role="presentation" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header class="modal-header">
      <div class="header-content">
        <span class="header-title">{actionName}</span>
        {#if isRunning}
          <span class="status-badge running">
            <Loader2 size={12} class="spinning" />
            Running
          </span>
        {:else if exitCode === 0}
          <span class="status-badge success">
            <CheckCircle2 size={12} />
            Completed
          </span>
        {:else if exitCode !== null}
          <span class="status-badge failed">
            <AlertCircle size={12} />
            Failed (exit {exitCode})
          </span>
        {:else}
          <span class="status-badge stopped">
            <StopCircle size={12} />
            Stopped
          </span>
        {/if}
        <span class="duration">{formatDuration()}</span>
      </div>

      <div class="header-actions">
        {#if isRunning}
          <button class="action-btn stop" onclick={handleStop} title="Stop action">
            <StopCircle size={16} />
          </button>
        {:else}
          <button class="action-btn restart" onclick={handleRestart} title="Restart action">
            <RotateCcw size={16} />
          </button>
        {/if}
        <button class="close-btn" onclick={onClose}>
          <X size={18} />
        </button>
      </div>
    </header>

    <div class="modal-content" bind:this={outputContainer}>
      {#if error}
        <div class="error-message">
          <AlertCircle size={16} />
          <span>{error}</span>
        </div>
      {/if}

      {#if outputLines.length === 0 && isRunning}
        <div class="empty-state">
          <Loader2 size={24} class="spinning" />
          <span>Waiting for output...</span>
        </div>
      {:else if outputLines.length === 0}
        <div class="empty-state">
          <span>No output</span>
        </div>
      {:else}
        <div class="output">
          {#each outputLines as line}
            <div class="output-line" class:stderr={line.stream === 'stderr'}>
              {line.text}
            </div>
          {/each}
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
    display: flex;
    flex-direction: column;
    width: 90%;
    max-width: 800px;
    max-height: 80vh;
    background: var(--bg-chrome);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-primary);
    min-width: 0;
    flex: 1;
  }

  .header-title {
    font-size: var(--size-md);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: var(--size-xs);
    font-weight: 500;
    flex-shrink: 0;
  }

  .status-badge.running {
    background: var(--ui-accent);
    color: var(--bg-deepest);
  }

  .status-badge.success {
    background: var(--ui-success);
    color: var(--bg-deepest);
  }

  .status-badge.failed {
    background: var(--ui-danger);
    color: var(--bg-deepest);
  }

  .status-badge.stopped {
    background: var(--text-muted);
    color: var(--bg-deepest);
  }

  .duration {
    font-size: var(--size-xs);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    background: none;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition:
      color 0.1s,
      background-color 0.1s;
  }

  .action-btn.stop {
    color: var(--ui-danger);
  }

  .action-btn.stop:hover {
    background: var(--ui-danger);
    color: var(--bg-chrome);
  }

  .action-btn.restart {
    color: var(--text-muted);
  }

  .action-btn.restart:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

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
    transition:
      color 0.1s,
      background-color 0.1s;
  }

  .close-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .modal-content {
    flex: 1;
    overflow-y: auto;
    background: var(--bg-deepest);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px;
    color: var(--text-muted);
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    margin: 16px;
    background: var(--ui-danger-subtle);
    border: 1px solid var(--ui-danger);
    border-radius: 6px;
    color: var(--ui-danger);
    font-size: var(--size-sm);
  }

  .output {
    padding: 16px;
    font-family: var(--font-mono);
    font-size: var(--size-sm);
    line-height: 1.5;
  }

  .output-line {
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--text-primary);
  }

  .output-line.stderr {
    color: var(--ui-danger);
  }

  :global(.spinning) {
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
