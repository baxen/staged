<!--
  ChatPanel.svelte - AI chat interface panel

  A slide-out panel for chatting with an AI agent via ACP.
  Uses the shared streaming store for persistent streaming state.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { X, Send, Bot, Loader2, AlertCircle } from 'lucide-svelte';
  import {
    checkAiAvailable,
    createChatSession,
    getChatSession,
    sendChatPrompt,
  } from './services/ai';
  import { toDisplayMessage, type DisplayMessage } from './types/streaming';
  import {
    connectToSession,
    disconnectFromSession,
    clearStreamingState,
    type StreamingSessionState,
    type ConnectOptions,
  } from './stores/streamingSession.svelte';
  import StreamingMessages from './StreamingMessages.svelte';

  interface Props {
    repoPath?: string;
    onClose: () => void;
  }

  let { repoPath, onClose }: Props = $props();

  // ==========================================================================
  // State
  // ==========================================================================

  let messages = $state<DisplayMessage[]>([]);
  let inputValue = $state('');
  let isLoading = $state(false);
  let sessionId = $state<string | null>(null);
  let error = $state<string | null>(null);
  let aiAvailable = $state<boolean | null>(null);
  let aiAgentName = $state<string>('');

  // Streaming store connection
  let streamState = $state<StreamingSessionState | null>(null);
  let connectOptions: ConnectOptions | undefined;

  // Refs
  let messagesContainer: HTMLDivElement;
  let inputElement: HTMLTextAreaElement;

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  onMount(async () => {
    try {
      aiAgentName = await checkAiAvailable();
      aiAvailable = true;
    } catch (e) {
      aiAvailable = false;
      error = e instanceof Error ? e.message : String(e);
    }

    inputElement?.focus();
  });

  onDestroy(() => {
    if (sessionId) {
      disconnectFromSession(sessionId, connectOptions);
    }
  });

  // ==========================================================================
  // Streaming store connection (lazily when session is created)
  // ==========================================================================

  function connectStreaming(sid: string) {
    connectOptions = {
      onIdle: () => refreshFromDatabase(),
      onError: (message) => {
        error = message;
        isLoading = false;
      },
    };
    streamState = connectToSession(sid, connectOptions);
  }

  async function refreshFromDatabase() {
    if (!sessionId) return;

    try {
      const session = await getChatSession(sessionId);
      if (session) {
        messages = session.messages.map(toDisplayMessage);
      }
    } catch (e) {
      console.error('Failed to refresh from database:', e);
    }

    clearStreamingState(sessionId);
    isLoading = false;
    scrollToBottom();
  }

  // ==========================================================================
  // Actions
  // ==========================================================================

  async function sendMessage() {
    const content = inputValue.trim();
    if (!content || isLoading || !aiAvailable) return;

    error = null;
    inputValue = '';

    // Create session if needed
    if (!sessionId) {
      try {
        const workingDir = repoPath || '.';
        sessionId = await createChatSession(workingDir);
        connectStreaming(sessionId);
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        return;
      }
    }

    // Add user message to display immediately
    messages = [...messages, { role: 'user', content, segments: [] }];

    // Reset streaming state for new turn
    if (sessionId) {
      clearStreamingState(sessionId);
    }
    isLoading = true;

    await tick();
    scrollToBottom();

    try {
      await sendChatPrompt(sessionId!, content);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      isLoading = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      sendMessage();
    }
    if (event.key === 'Escape') {
      onClose();
    }
  }

  function scrollToBottom() {
    tick().then(() => {
      if (messagesContainer) {
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
      }
    });
  }

  function handleInput(event: Event) {
    const target = event.target as HTMLTextAreaElement;
    target.style.height = 'auto';
    target.style.height = Math.min(target.scrollHeight, 150) + 'px';
  }
</script>

<div class="chat-panel">
  <header class="panel-header">
    <div class="header-title">
      <Bot size={16} />
      <span>AI Chat</span>
      {#if aiAgentName}
        <span class="agent-badge">{aiAgentName}</span>
      {/if}
    </div>
    <button class="close-btn" onclick={onClose}>
      <X size={16} />
    </button>
  </header>

  <div class="messages-container" bind:this={messagesContainer}>
    {#if aiAvailable === false}
      <div class="empty-state error-state">
        <AlertCircle size={24} />
        <p>No AI agent available</p>
        <span class="hint">{error}</span>
      </div>
    {:else if messages.length === 0 && !isLoading}
      <div class="empty-state">
        <Bot size={32} />
        <p>Start a conversation</p>
        <span class="hint">Ask questions about your code or get help with your changes.</span>
      </div>
    {:else}
      <StreamingMessages
        {messages}
        streamingSegments={streamState?.streamingSegments ?? []}
        isActive={isLoading}
      />
    {/if}
  </div>

  {#if error && aiAvailable}
    <div class="error-banner">
      <AlertCircle size={14} />
      <span>{error}</span>
    </div>
  {/if}

  <div class="input-area">
    <textarea
      bind:this={inputElement}
      bind:value={inputValue}
      placeholder={aiAvailable ? 'Ask a question...' : 'AI not available'}
      disabled={!aiAvailable || isLoading}
      onkeydown={handleKeydown}
      oninput={handleInput}
      rows="1"
    ></textarea>
    <button
      class="send-btn"
      onclick={sendMessage}
      disabled={!inputValue.trim() || isLoading || !aiAvailable}
    >
      {#if isLoading}
        <Loader2 size={16} class="spinning" />
      {:else}
        <Send size={16} />
      {/if}
    </button>
  </div>
</div>

<style>
  .chat-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-chrome);
    border-left: 1px solid var(--border-muted);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .header-title :global(svg) {
    color: var(--text-muted);
  }

  .agent-badge {
    font-size: var(--size-xs);
    font-weight: 500;
    color: var(--text-muted);
    background: var(--bg-primary);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .close-btn {
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
      color 0.1s,
      background-color 0.1s;
  }

  .close-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--text-muted);
    text-align: center;
    padding: 32px;
  }

  .empty-state :global(svg) {
    color: var(--text-faint);
    margin-bottom: 12px;
  }

  .empty-state p {
    margin: 0 0 4px 0;
    font-size: var(--size-sm);
    color: var(--text-primary);
  }

  .empty-state .hint {
    font-size: var(--size-xs);
    color: var(--text-faint);
    max-width: 240px;
  }

  .empty-state.error-state :global(svg) {
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

  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    background: var(--ui-danger-bg);
    color: var(--ui-danger);
    font-size: var(--size-xs);
    border-top: 1px solid var(--ui-danger);
  }

  .input-area {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-chrome);
  }

  .input-area textarea {
    flex: 1;
    padding: 10px 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border-muted);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: inherit;
    resize: none;
    min-height: 40px;
    max-height: 150px;
    line-height: 1.4;
  }

  .input-area textarea::placeholder {
    color: var(--text-faint);
  }

  .input-area textarea:focus {
    outline: none;
    border-color: var(--border-emphasis);
  }

  .input-area textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    background: var(--ui-accent);
    border: none;
    border-radius: 8px;
    color: var(--bg-primary);
    cursor: pointer;
    transition:
      background-color 0.1s,
      opacity 0.1s;
    flex-shrink: 0;
  }

  .send-btn:hover:not(:disabled) {
    background: var(--ui-accent-hover);
  }

  .send-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
