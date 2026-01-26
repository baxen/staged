<!--
  AgentChat.svelte - Chat interface for Goose agent
  
  Displays conversation history with markdown rendering and handles message input.
  User messages appear as bubbles on the right, agent responses on the left.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import { Bot, Send, Loader2, Terminal } from 'lucide-svelte';
  import { agentState, sendMessage as sendAgentMessage } from './stores/agent.svelte';
  import { repoState } from './stores/repoState.svelte';
  import { renderMarkdown } from './services/markdown';

  let chatInput = $state('');
  let chatMessagesEl: HTMLDivElement | undefined = $state();

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

  async function handleChatSubmit(e: Event) {
    e.preventDefault();
    if (!chatInput.trim() || agentState.isStreaming) return;
    if (!repoState.currentPath) {
      console.error('No repository path available');
      return;
    }

    const message = chatInput;
    chatInput = '';
    await sendAgentMessage(message, repoState.currentPath);
  }

  function handleChatKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleChatSubmit(e);
    }
  }
</script>

<div class="agent-section">
  <div class="agent-messages" bind:this={chatMessagesEl}>
    {#if agentState.messages.length === 0}
      <div class="agent-empty">
        <Bot size={20} />
        <span>Ask Goose to help with this code</span>
      </div>
    {:else}
      {#each agentState.messages as message (message.id)}
        <div class="agent-message" class:user={message.role === 'user'}>
          <div class="message-content">
            {@html renderMarkdown(message.content)}
          </div>
        </div>
      {/each}
      {#if agentState.currentToolCall}
        <div class="agent-tool-call">
          <Terminal size={12} />
          <span class="tool-name">{agentState.currentToolCall.name}</span>
          {#if agentState.currentToolCall.status === 'running'}
            <Loader2 size={12} class="spinning" />
          {/if}
        </div>
      {/if}
      {#if agentState.isStreaming && !agentState.currentToolCall}
        <div class="agent-thinking">
          <Loader2 size={12} class="spinning" />
          <span>Thinking...</span>
        </div>
      {/if}
    {/if}
  </div>
  <form class="agent-input-form" onsubmit={handleChatSubmit}>
    <input
      type="text"
      class="agent-input"
      placeholder="Ask Goose..."
      bind:value={chatInput}
      onkeydown={handleChatKeydown}
      disabled={agentState.isStreaming}
    />
    <button
      type="submit"
      class="agent-send-btn"
      disabled={agentState.isStreaming || !chatInput.trim()}
      title="Send message"
    >
      <Send size={14} />
    </button>
  </form>
</div>

<style>
  .agent-section {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    margin: 0 8px 8px;
    border-radius: 6px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
  }

  .agent-messages {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .agent-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    color: var(--text-faint);
    font-size: var(--size-sm);
    text-align: center;
    padding: 16px;
  }

  .agent-message {
    padding: 6px 0;
    font-size: var(--size-sm);
    line-height: 1.4;
    max-width: 100%;
    color: var(--text-primary);
    align-self: flex-start;
  }

  .agent-message.user {
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--ui-accent);
    color: white;
    align-self: flex-end;
    max-width: 90%;
  }

  .message-content {
    word-break: break-word;
  }

  /* Markdown styles for agent messages */
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

  .agent-message.user .message-content :global(code) {
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

  .agent-message.user .message-content :global(a) {
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

  .agent-tool-call {
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

  .agent-thinking {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    color: var(--text-muted);
    font-size: var(--size-xs);
  }

  .agent-input-form {
    display: flex;
    gap: 4px;
    padding: 8px;
    border-top: 1px solid var(--border-subtle);
  }

  .agent-input {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: inherit;
  }

  .agent-input:focus {
    outline: none;
    border-color: var(--ui-accent);
  }

  .agent-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .agent-input::placeholder {
    color: var(--text-faint);
  }

  .agent-send-btn {
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

  .agent-send-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .agent-send-btn:disabled {
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
