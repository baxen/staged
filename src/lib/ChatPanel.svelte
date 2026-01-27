<!--
  ChatPanel.svelte - Bottom panel chat interface for Goose agent
  
  A VSCode terminal-style bottom panel that displays conversation history
  with markdown rendering and handles message input.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import { Send, Loader2, Terminal, ChevronDown } from 'lucide-svelte';
  import { agentState, sendMessage as sendAgentMessage } from './stores/agent.svelte';
  import { repoState } from './stores/repoState.svelte';
  import { getActiveTab } from './stores/tabState.svelte';
  import { renderMarkdown } from './services/markdown';
  import AgentSelector, { type AgentId } from './AgentSelector.svelte';

  type AgentMode = 'plan' | 'implement' | 'review';

  const modeLabels: Record<AgentMode, string> = {
    plan: 'Plan',
    implement: 'Implement',
    review: 'Review',
  };

  const modeDescriptions: Record<AgentMode, string> = {
    plan: 'Analyze and plan changes without modifying code',
    implement: 'Make code changes to implement features or fixes',
    review: 'Review the current diff and provide feedback',
  };

  let chatInput = $state('');
  let chatMessagesEl: HTMLDivElement | undefined = $state();
  let selectedMode = $state<AgentMode>('plan');
  let selectedAgent = $state<AgentId>('goose');
  let modeDropdownOpen = $state(false);
  let modeDropdownEl: HTMLDivElement | undefined = $state();

  // Show messages area when there are messages
  let hasMessages = $derived(agentState.messages.length > 0);

  // Auto-scroll chat to bottom when messages change
  $effect(() => {
    // Access messages to create dependency
    const _messages = agentState.messages;
    if (chatMessagesEl) {
      tick().then(() => {
        if (chatMessagesEl) {
          chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
        }
      });
    }
  });

  // Close dropdowns when clicking outside
  $effect(() => {
    if (!modeDropdownOpen) return;

    function handleClickOutside(e: MouseEvent) {
      if (modeDropdownEl && !modeDropdownEl.contains(e.target as Node)) {
        modeDropdownOpen = false;
      }
    }

    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  });

  function selectMode(mode: AgentMode) {
    selectedMode = mode;
    modeDropdownOpen = false;
  }

  async function handleChatSubmit(e: Event) {
    e.preventDefault();
    if (!chatInput.trim() || agentState.isStreaming) return;
    if (!repoState.currentPath) {
      console.error('No repository path available');
      return;
    }

    // Get the active tab's agent state for session registration
    const activeTab = getActiveTab();
    const tabAgentState = activeTab?.agentState;

    // Prepend mode context to the message
    const modeContext = `[Mode: ${modeLabels[selectedMode]}] `;
    const message = modeContext + chatInput;
    chatInput = '';
    await sendAgentMessage(message, repoState.currentPath, selectedAgent, tabAgentState);
  }

  function handleChatKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleChatSubmit(e);
    }
  }
</script>

<div class="chat-panel" class:expanded={hasMessages}>
  {#if hasMessages}
    <div class="chat-messages" bind:this={chatMessagesEl}>
      {#each agentState.messages as message (message.id)}
        <div class="chat-message" class:user={message.role === 'user'}>
          <div class="message-content">
            {@html renderMarkdown(message.content)}
          </div>
        </div>
      {/each}
      {#if agentState.currentToolCall}
        <div class="chat-tool-call">
          <Terminal size={12} />
          <span class="tool-name">{agentState.currentToolCall.name}</span>
          {#if agentState.currentToolCall.status === 'running'}
            <Loader2 size={12} class="spinning" />
          {/if}
        </div>
      {/if}
      {#if agentState.isStreaming && !agentState.currentToolCall}
        <div class="chat-thinking">
          <Loader2 size={12} class="spinning" />
          <span>Thinking...</span>
        </div>
      {/if}
    </div>
  {/if}
  <form class="chat-input-form" onsubmit={handleChatSubmit}>
    <div class="input-actions">
      <!-- Agent selector -->
      <AgentSelector
        value={selectedAgent}
        onchange={(agent) => (selectedAgent = agent)}
        disabled={agentState.isStreaming}
      />
      <!-- Mode selector dropdown -->
      <div class="dropdown" bind:this={modeDropdownEl}>
        <button
          type="button"
          class="dropdown-trigger"
          class:open={modeDropdownOpen}
          onclick={() => (modeDropdownOpen = !modeDropdownOpen)}
          title={modeDescriptions[selectedMode]}
        >
          <span class="dropdown-label">{modeLabels[selectedMode]}</span>
          <ChevronDown size={12} />
        </button>
        {#if modeDropdownOpen}
          <div class="dropdown-menu">
            {#each Object.entries(modeLabels) as [mode, label]}
              <button
                type="button"
                class="dropdown-option"
                class:selected={selectedMode === mode}
                onclick={() => selectMode(mode as AgentMode)}
              >
                <span class="dropdown-option-label">{label}</span>
                <span class="dropdown-option-desc">{modeDescriptions[mode as AgentMode]}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
    <input
      type="text"
      class="chat-input"
      placeholder="Ask the agent..."
      bind:value={chatInput}
      onkeydown={handleChatKeydown}
      disabled={agentState.isStreaming}
    />
    <button
      type="submit"
      class="chat-send-btn"
      disabled={agentState.isStreaming || !chatInput.trim()}
      title="Send message"
    >
      <Send size={14} />
    </button>
  </form>
</div>

<style>
  .chat-panel {
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-subtle);
    max-height: 48px;
    transition: max-height 0.2s ease;
  }

  .chat-panel.expanded {
    max-height: 300px;
  }

  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }

  .chat-message {
    padding: 6px 0;
    font-size: var(--size-sm);
    line-height: 1.5;
    max-width: 100%;
    color: var(--text-primary);
    align-self: flex-start;
  }

  .chat-message.user {
    padding: 6px 12px;
    border-radius: 6px;
    background: var(--ui-accent);
    color: white;
    align-self: flex-end;
    max-width: 80%;
  }

  .message-content {
    word-break: break-word;
  }

  /* Markdown styles for chat messages */
  .message-content :global(p) {
    margin: 0 0 0.5em 0;
  }

  .message-content :global(p:last-child) {
    margin-bottom: 0;
  }

  .message-content :global(code) {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 0.9em;
    padding: 0.1em 0.3em;
    border-radius: 3px;
    background: var(--bg-hover);
  }

  .chat-message.user .message-content :global(code) {
    background: rgba(255, 255, 255, 0.2);
  }

  .message-content :global(pre) {
    margin: 0.5em 0;
    padding: 8px;
    border-radius: 4px;
    background: var(--bg-hover);
    overflow-x: auto;
  }

  .message-content :global(pre code) {
    padding: 0;
    background: none;
  }

  .message-content :global(strong) {
    font-weight: 600;
  }

  .message-content :global(a) {
    color: var(--text-accent);
    text-decoration: none;
  }

  .message-content :global(a:hover) {
    text-decoration: underline;
  }

  .chat-message.user .message-content :global(a) {
    color: inherit;
    text-decoration: underline;
  }

  .message-content :global(ul),
  .message-content :global(ol) {
    margin: 0.5em 0;
    padding-left: 1.5em;
  }

  .message-content :global(li) {
    margin: 0.25em 0;
  }

  .message-content :global(h1),
  .message-content :global(h2),
  .message-content :global(h3),
  .message-content :global(h4),
  .message-content :global(h5),
  .message-content :global(h6) {
    margin: 0.5em 0 0.25em 0;
    font-weight: 600;
    line-height: 1.3;
  }

  .message-content :global(h1) {
    font-size: 1.2em;
  }
  .message-content :global(h2) {
    font-size: 1.1em;
  }
  .message-content :global(h3),
  .message-content :global(h4),
  .message-content :global(h5),
  .message-content :global(h6) {
    font-size: 1em;
  }

  .message-content :global(table) {
    border-collapse: collapse;
    margin: 0.5em 0;
    font-size: 0.9em;
    width: auto;
  }

  .message-content :global(th),
  .message-content :global(td) {
    border: 1px solid var(--border-muted);
    padding: 4px 8px;
    text-align: left;
  }

  .message-content :global(th) {
    background: var(--bg-hover);
    font-weight: 600;
  }

  .message-content :global(tr:nth-child(even)) {
    background: var(--bg-hover);
  }

  .chat-message.user .message-content :global(th),
  .chat-message.user .message-content :global(td) {
    border-color: rgba(255, 255, 255, 0.3);
  }

  .chat-message.user .message-content :global(th),
  .chat-message.user .message-content :global(tr:nth-child(even)) {
    background: rgba(255, 255, 255, 0.1);
  }

  .message-content :global(blockquote) {
    margin: 0.5em 0;
    padding: 0.25em 0.75em;
    border-left: 3px solid var(--border-muted);
    color: var(--text-muted);
  }

  .message-content :global(hr) {
    border: none;
    border-top: 1px solid var(--border-subtle);
    margin: 0.75em 0;
  }

  .message-content :global(del) {
    text-decoration: line-through;
    opacity: 0.7;
  }

  .chat-tool-call {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 4px;
    background: var(--bg-hover);
    color: var(--text-muted);
    font-size: var(--size-xs);
  }

  .tool-name {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
  }

  .chat-thinking {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    color: var(--text-muted);
    font-size: var(--size-xs);
  }

  .chat-input-form {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px 12px;
    background: var(--bg-secondary);
  }

  .chat-panel.expanded .chat-input-form {
    padding-bottom: 8px;
    border-top: 1px solid var(--border-subtle);
  }

  .input-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .dropdown {
    position: relative;
  }

  .dropdown-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: var(--bg-primary);
    border: none;
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--size-xs);
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .dropdown-trigger:hover,
  .dropdown-trigger.open {
    background: var(--bg-hover);
  }

  .dropdown-trigger :global(svg) {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .dropdown-trigger :global(svg:last-child) {
    transition: transform 0.15s;
  }

  .dropdown-trigger.open :global(svg:last-child) {
    transform: rotate(180deg);
  }

  .dropdown-label {
    font-weight: 500;
  }

  .dropdown-menu {
    position: absolute;
    bottom: 100%;
    left: 0;
    margin-bottom: 4px;
    min-width: 220px;
    background: var(--bg-chrome);
    border: 1px solid var(--border-muted);
    border-radius: 8px;
    box-shadow: var(--shadow-elevated);
    overflow: hidden;
    z-index: 100;
  }

  .dropdown-option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: none;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    transition: background-color 0.1s;
  }

  .dropdown-option:hover {
    background-color: var(--bg-hover);
  }

  .dropdown-option.selected {
    background-color: var(--bg-primary);
  }

  .dropdown-option-label {
    font-size: var(--size-xs);
    font-weight: 500;
    color: var(--text-primary);
  }

  .dropdown-option-desc {
    font-size: calc(var(--size-xs) - 1px);
    color: var(--text-faint);
  }

  .chat-input {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: inherit;
  }

  .chat-input:focus {
    outline: none;
    border-color: var(--ui-accent);
  }

  .chat-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .chat-input::placeholder {
    color: var(--text-faint);
  }

  .chat-send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px 10px;
    border: none;
    border-radius: 4px;
    background: var(--ui-accent);
    color: white;
    cursor: pointer;
    transition: opacity 0.1s;
  }

  .chat-send-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .chat-send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Spinning animation for loader */
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
