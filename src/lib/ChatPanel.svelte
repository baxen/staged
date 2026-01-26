<!--
  ChatPanel.svelte - Bottom panel chat interface for Goose agent
  
  A VSCode terminal-style bottom panel that displays conversation history
  with markdown rendering and handles message input.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import { Bot, Send, Loader2, Terminal } from 'lucide-svelte';
  import { agentState, sendMessage as sendAgentMessage } from './stores/agent.svelte';
  import { repoState } from './stores/repoState.svelte';
  import { renderMarkdown } from './services/markdown';

  let chatInput = $state('');
  let chatMessagesEl: HTMLDivElement | undefined = $state();

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
      <span class="chat-icon">
        <Bot size={14} />
      </span>
      <!-- Future buttons can go here -->
    </div>
    <input
      type="text"
      class="chat-input"
      placeholder="Ask Goose..."
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

  .chat-icon {
    display: flex;
    align-items: center;
    justify-content: center;
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
