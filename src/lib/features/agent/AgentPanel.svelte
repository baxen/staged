<!--
  AgentPanel.svelte - AI agent chat interface
  
  Provides a simple chat input for asking questions about the current diff/changeset.
  Maintains session state for multi-turn conversations.
-->
<script lang="ts">
  import { Send, Bot, Loader2, ChevronDown } from 'lucide-svelte';
  import { sendAgentPrompt } from '../../services/ai';
  import { agentState, type AcpProvider } from '../../stores/agent.svelte';
  import type { FileDiffSummary } from '../../types';

  import { onMount } from 'svelte';

  interface Props {
    /** Repository path for AI agent */
    repoPath?: string | null;
    /** File summaries from the current diff */
    files?: FileDiffSummary[];
    /** Currently selected file path */
    selectedFile?: string | null;
  }

  let { repoPath = null, files = [], selectedFile = null }: Props = $props();

  let showProviderDropdown = $state(false);

  const providers: { id: AcpProvider; label: string }[] = [
    { id: 'goose', label: 'Goose' },
    { id: 'claude', label: 'Claude Code' },
  ];

  function selectProvider(provider: AcpProvider) {
    agentState.provider = provider;
    showProviderDropdown = false;
    // Reset session when switching providers
    agentState.sessionId = null;
    agentState.response = '';
  }

  function toggleProviderDropdown() {
    showProviderDropdown = !showProviderDropdown;
  }

  // Close dropdown when clicking outside
  onMount(() => {
    function handleClickOutside(event: MouseEvent) {
      const target = event.target as HTMLElement;
      if (showProviderDropdown && !target.closest('.provider-picker')) {
        showProviderDropdown = false;
      }
    }
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  });

  /**
   * Get the primary path for a file summary.
   */
  function getFilePath(summary: FileDiffSummary): string {
    return summary.after ?? summary.before ?? '';
  }

  /**
   * Build context-aware prompt with file information.
   */
  function buildPromptWithContext(userPrompt: string, isNewSession: boolean): string {
    let context = '';

    // For new sessions, include changeset overview (up to 5 files)
    if (isNewSession && files.length > 0) {
      const fileNames = files.slice(0, 5).map((f) => getFilePath(f));
      const moreCount = files.length > 5 ? ` (+${files.length - 5} more)` : '';
      context += `[Changeset: ${fileNames.join(', ')}${moreCount}]\n`;
    }

    // Always include current file context
    if (selectedFile) {
      context += `[Viewing: ${selectedFile}]\n`;
    }

    return context ? context + '\n' + userPrompt : userPrompt;
  }

  /**
   * Send prompt to AI agent.
   */
  async function handleSubmit() {
    const userPrompt = agentState.input.trim();
    if (!userPrompt || agentState.loading) return;

    agentState.loading = true;
    agentState.error = '';
    agentState.response = '';
    const inputToSend = agentState.input;
    agentState.input = '';

    try {
      const isNewSession = !agentState.sessionId;
      const promptWithContext = buildPromptWithContext(inputToSend, isNewSession);
      const result = await sendAgentPrompt(repoPath, promptWithContext, agentState.sessionId);
      agentState.response = result.response;
      agentState.sessionId = result.sessionId;
    } catch (e) {
      agentState.error = e instanceof Error ? e.message : String(e);
    } finally {
      agentState.loading = false;
    }
  }

  /**
   * Handle Enter key in input.
   */
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSubmit();
    }
  }
</script>

<div class="agent-section">
  <div class="agent-top">
    {#if agentState.error}
      <div class="agent-error">
        {agentState.error}
      </div>
    {/if}
    {#if agentState.loading || agentState.response}
      <div class="agent-response">
        <div class="agent-response-header">
          <Bot size={12} />
          <span>Agent</span>
        </div>
        <div class="agent-response-content" class:loading={agentState.loading}>
          {#if agentState.loading}
            <Loader2 size={14} class="spinning" /> Thinking...
          {:else}
            {agentState.response}
          {/if}
        </div>
      </div>
    {/if}
  </div>
  <div class="agent-bottom">
    <div class="agent-input-wrapper">
      <textarea
        class="agent-input"
        placeholder="Ask the agent..."
        bind:value={agentState.input}
        onkeydown={handleKeydown}
        disabled={agentState.loading}
        rows="3"
      ></textarea>
      <div class="agent-input-actions">
        <div class="provider-picker">
          <button
            class="provider-btn"
            onclick={toggleProviderDropdown}
            disabled={agentState.loading}
            title="Select AI provider"
          >
            <span class="provider-label">{providers.find((p) => p.id === agentState.provider)?.label}</span>
            <ChevronDown size={12} />
          </button>
          {#if showProviderDropdown}
            <div class="provider-dropdown">
              {#each providers as provider (provider.id)}
                <button
                  class="provider-option"
                  class:selected={agentState.provider === provider.id}
                  onclick={() => selectProvider(provider.id)}
                >
                  {provider.label}
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <button
          class="agent-send-btn"
          onclick={handleSubmit}
          disabled={agentState.loading || !agentState.input.trim()}
          title="Send to agent"
        >
          {#if agentState.loading}
            <Loader2 size={14} class="spinning" />
          {:else}
            <Send size={14} />
          {/if}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .agent-section {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    padding: 0 12px;
  }

  .agent-top {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .agent-bottom {
    flex-shrink: 0;
    padding: 12px 0;
  }

  .agent-input-wrapper {
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    border: 1px solid var(--border-muted);
    border-radius: 8px;
    padding: 10px 12px 8px;
    transition: border-color 0.1s;
  }

  .agent-input-wrapper:focus-within {
    border-color: var(--text-accent);
  }

  .agent-input {
    flex: 1;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: inherit;
    padding: 0;
    outline: none;
    min-width: 0;
    resize: none;
    line-height: 1.4;
  }

  .agent-input::placeholder {
    color: var(--text-faint);
  }

  .agent-input:disabled {
    opacity: 0.6;
  }

  .agent-input-actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
  }

  /* Provider picker */
  .provider-picker {
    position: relative;
  }

  .provider-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: none;
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    font-family: inherit;
    cursor: pointer;
    transition:
      background-color 0.1s,
      border-color 0.1s;
  }

  .provider-btn:hover:not(:disabled) {
    background-color: var(--bg-hover);
    border-color: var(--border-muted);
  }

  .provider-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .provider-label {
    white-space: nowrap;
  }

  .provider-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    margin-bottom: 4px;
    background: var(--bg-primary);
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    overflow: hidden;
    z-index: 100;
    min-width: 120px;
  }

  .provider-option {
    display: block;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .provider-option:hover {
    background-color: var(--bg-hover);
  }

  .provider-option.selected {
    background-color: var(--bg-primary);
    color: var(--text-accent);
  }

  .agent-send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      background-color 0.1s,
      color 0.1s;
    flex-shrink: 0;
    margin-left: auto;
  }

  .agent-send-btn:hover:not(:disabled) {
    background-color: var(--bg-hover);
    color: var(--text-accent);
  }

  .agent-send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .agent-send-btn :global(.spinning) {
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

  .agent-error {
    margin-bottom: 8px;
    padding: 8px;
    background: var(--ui-danger-bg);
    border-radius: 4px;
    color: var(--ui-danger);
    font-size: var(--size-xs);
    word-break: break-word;
  }

  .agent-response {
    margin-bottom: 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    overflow: hidden;
  }

  .agent-response-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: var(--bg-hover);
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-muted);
    font-size: var(--size-xs);
    font-weight: 500;
  }

  .agent-response-content {
    padding: 10px;
    font-size: var(--size-sm);
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow-y: auto;
  }

  .agent-response-content.loading {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-style: italic;
  }
</style>
