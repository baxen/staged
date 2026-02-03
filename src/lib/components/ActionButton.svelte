<script lang="ts">
  import {
    Play,
    Square,
    MessageSquare,
    ChevronDown,
    Loader2,
    Download,
    Trash2,
    Hammer,
    Rocket,
    FlaskConical,
    Sparkles,
    MoreHorizontal,
  } from 'lucide-svelte';
  import type {
    RepoAction,
    ActionCategory,
    ActionExecution,
    ExecutionStatus,
  } from '../stores/actionsState.svelte';
  import { CATEGORY_LABELS } from '../stores/actionsState.svelte';

  /** Icons for each category */
  const CATEGORY_ICONS: Record<ActionCategory, typeof Download> = {
    setup: Download,
    clean: Trash2,
    build: Hammer,
    run: Rocket,
    test: FlaskConical,
    lint: Sparkles,
    other: MoreHorizontal,
  };

  interface Props {
    category: ActionCategory;
    actions: RepoAction[];
    currentAction: RepoAction | null;
    execution: ActionExecution | null;
    onRun: (action: RepoAction) => void;
    onStop: (executionId: string) => void;
    onChat: (actionName: string, output: string) => void;
  }

  let {
    category,
    actions,
    currentAction,
    execution,
    onRun,
    onStop,
    onChat,
  }: Props = $props();

  let dropdownOpen = $state(false);
  let dropdownRef: HTMLDivElement | null = $state(null);

  // Derive execution status
  let status = $derived<ExecutionStatus>(execution?.status ?? 'idle');
  let isRunning = $derived(status === 'running');
  let hasOutput = $derived((execution?.output?.length ?? 0) > 0);

  // Get the appropriate icon component for this category
  let CategoryIcon = $derived(CATEGORY_ICONS[category]);

  function handleMainClick(e: MouseEvent) {
    e.stopPropagation();

    if (!currentAction) return;

    onRun(currentAction);
  }

  function handleDropdownToggle(e: MouseEvent) {
    e.stopPropagation();

    // Toggle the dropdown
    dropdownOpen = !dropdownOpen;
  }

  function handleActionSelect(action: RepoAction) {
    onRun(action);
    // Keep dropdown open to show output
  }

  function handleStopClick() {
    if (execution) {
      // Find the execution ID from the executions map
      // The execution object itself doesn't have the ID, so we pass the action ID
      const executionId = `${execution.actionId}-${execution.startedAt}`;
      onStop(executionId);
    }
  }

  function handleRunAgainClick() {
    if (currentAction) {
      onRun(currentAction);
    }
  }

  function handleChatClick() {
    if (currentAction && execution) {
      onChat(currentAction.name, execution.output);
    }
  }

  // Close dropdown when clicking outside
  function handleClickOutside(event: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
      dropdownOpen = false;
    }
  }

  $effect(() => {
    if (dropdownOpen) {
      document.addEventListener('click', handleClickOutside);
    }
    return () => {
      document.removeEventListener('click', handleClickOutside);
    };
  });

  // Scroll output to bottom when it changes
  let outputRef: HTMLPreElement | null = $state(null);
  $effect(() => {
    if (outputRef && execution?.output) {
      outputRef.scrollTop = outputRef.scrollHeight;
    }
  });
</script>

<div class="action-button-container" bind:this={dropdownRef}>
  <div
    class="action-btn-group"
    class:running={isRunning}
    class:disabled={!currentAction}
  >
    <button
      class="action-main"
      onclick={handleMainClick}
      disabled={!currentAction}
      title={currentAction?.description ?? CATEGORY_LABELS[category]}
    >
      <span class="action-icon">
        {#if isRunning}
          <Loader2 size={12} class="spinning" />
        {:else}
          <svelte:component this={CategoryIcon} size={12} />
        {/if}
      </span>
      <span class="action-label">{currentAction?.name ?? CATEGORY_LABELS[category]}</span>
    </button>
    {#if actions.length > 1}
      <button
        class="action-dropdown-trigger"
        onclick={handleDropdownToggle}
        disabled={!currentAction}
        title="More actions"
      >
        <ChevronDown size={10} />
      </button>
    {/if}
  </div>

  {#if dropdownOpen}
    <div class="dropdown">
      <!-- Output section (if any output exists or running) -->
      {#if hasOutput || isRunning}
        <div class="output-section">
          <pre class="output" bind:this={outputRef}>{execution?.output ?? ''}</pre>
          <div class="output-actions">
            {#if isRunning}
              <button class="output-btn stop-btn" onclick={handleStopClick}>
                <Square size={12} />
                <span>Stop</span>
              </button>
            {:else}
              <button class="output-btn run-again-btn" onclick={handleRunAgainClick}>
                <Play size={12} />
                <span>Run</span>
              </button>
            {/if}
            {#if hasOutput}
              <button class="output-btn chat-btn" onclick={handleChatClick}>
                <MessageSquare size={12} />
                <span>Chat</span>
              </button>
            {/if}
          </div>
        </div>
        {#if actions.length > 1}
          <div class="dropdown-divider"></div>
        {/if}
      {/if}

      <!-- Actions list -->
      {#if actions.length > 1}
        <div class="actions-list">
          {#each actions as action}
            <button
              class="action-item"
              class:active={action.id === currentAction?.id}
              onclick={() => handleActionSelect(action)}
              title={action.description}
            >
              <span class="action-item-name">{action.name}</span>
              <span class="action-item-command">{action.command}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .action-button-container {
    position: relative;
  }

  .action-btn-group {
    display: flex;
    align-items: center;
    height: 24px;
    background: var(--bg-primary);
    border-radius: 6px;
    color: var(--text-muted);
    font-size: var(--size-xs);
  }

  .action-btn-group.disabled {
    opacity: 0.5;
  }

  .action-btn-group.running {
    color: var(--text-primary);
  }

  .action-btn-group.running .action-icon {
    color: var(--text-link);
  }

  .action-main {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    height: 100%;
    background: none;
    border: none;
    border-radius: 6px;
    color: inherit;
    font-size: inherit;
    cursor: pointer;
    transition: background-color 0.1s, color 0.1s;
  }

  .action-main:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .action-main:disabled {
    cursor: not-allowed;
  }

  .action-dropdown-trigger {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px 6px;
    height: 100%;
    background: none;
    border: none;
    border-left: 1px solid var(--border-subtle);
    border-radius: 0 6px 6px 0;
    color: var(--text-faint);
    cursor: pointer;
    transition: background-color 0.1s, color 0.1s;
  }

  .action-dropdown-trigger:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .action-dropdown-trigger:disabled {
    cursor: not-allowed;
  }

  .action-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 12px;
    height: 12px;
    line-height: 0;
  }

  .action-icon :global(svg) {
    display: block;
    flex-shrink: 0;
  }

  .action-icon :global(.spinning) {
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

  .action-label {
    max-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    min-width: 280px;
    max-width: 400px;
    background: var(--bg-chrome);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 1000;
    overflow: hidden;
  }

  .output-section {
    display: flex;
    flex-direction: column;
  }

  .output {
    margin: 0;
    padding: 8px;
    max-height: 200px;
    overflow-y: auto;
    font-family: 'SF Mono', 'Monaco', 'Menlo', monospace;
    font-size: 11px;
    line-height: 1.4;
    background: var(--bg-primary);
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-all;
  }

  .output:empty::before {
    content: 'Waiting for output...';
    color: var(--text-muted);
    font-style: italic;
  }

  .output-actions {
    display: flex;
    gap: 4px;
    padding: 6px 8px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-subtle);
  }

  .output-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    transition:
      background-color 0.1s,
      color 0.1s;
  }

  .output-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .output-btn.stop-btn:hover {
    color: var(--status-deleted);
    border-color: var(--status-deleted);
  }

  .output-btn.run-again-btn:hover {
    color: var(--status-added);
    border-color: var(--status-added);
  }

  .output-btn.chat-btn:hover {
    color: var(--ui-accent);
    border-color: var(--ui-accent);
  }

  .dropdown-divider {
    height: 1px;
    background: var(--border-subtle);
  }

  .actions-list {
    padding: 4px 0;
    max-height: 200px;
    overflow-y: auto;
  }

  .action-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: 6px 12px;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background-color 0.1s;
  }

  .action-item:hover {
    background: var(--bg-hover);
  }

  .action-item.active {
    background: var(--bg-primary);
  }

  .action-item-name {
    font-size: var(--size-sm);
    color: var(--text-primary);
    font-weight: 500;
  }

  .action-item-command {
    font-size: 10px;
    color: var(--text-muted);
    font-family: 'SF Mono', 'Monaco', 'Menlo', monospace;
  }
</style>
