<!--
  ChatPanel.svelte - AI chat interface panel

  A slide-out panel for chatting with an AI agent via ACP.
  Supports streaming responses and displays tool calls.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { X, Send, Bot, User, Loader2, Wrench, AlertCircle } from 'lucide-svelte';
  import {
    checkAiAvailable,
    sendAgentPromptStreaming,
    listenToSessionUpdates,
    listenToSessionComplete,
    listenToSessionError,
    type SessionNotification,
    type SessionCompleteEvent,
    type SessionErrorEvent,
    type ToolCallSummary,
  } from './services/ai';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  interface Props {
    repoPath?: string;
    onClose: () => void;
  }

  let { repoPath, onClose }: Props = $props();

  // ==========================================================================
  // State
  // ==========================================================================

  /** Message in the chat history */
  interface ChatMessage {
    role: 'user' | 'assistant';
    content: string;
    toolCalls?: ToolCallSummary[];
    isStreaming?: boolean;
  }

  /** Active tool call being displayed */
  interface ActiveToolCall {
    id: string;
    title: string;
    status: string;
  }

  let messages = $state<ChatMessage[]>([]);
  let inputValue = $state('');
  let isLoading = $state(false);
  let sessionId = $state<string | null>(null);
  let error = $state<string | null>(null);
  let aiAvailable = $state<boolean | null>(null);
  let aiAgentName = $state<string>('');

  // Streaming state
  let streamingContent = $state('');
  let activeToolCalls = $state<Map<string, ActiveToolCall>>(new Map());

  // Refs
  let messagesContainer: HTMLDivElement;
  let inputElement: HTMLTextAreaElement;

  // Event listeners
  let unlistenUpdate: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  onMount(async () => {
    // Check AI availability
    try {
      aiAgentName = await checkAiAvailable();
      aiAvailable = true;
    } catch (e) {
      aiAvailable = false;
      error = e instanceof Error ? e.message : String(e);
    }

    // Set up event listeners
    unlistenUpdate = await listenToSessionUpdates(handleSessionUpdate);
    unlistenComplete = await listenToSessionComplete(handleSessionComplete);
    unlistenError = await listenToSessionError(handleSessionError);

    // Focus input
    inputElement?.focus();
  });

  onDestroy(() => {
    unlistenUpdate?.();
    unlistenComplete?.();
    unlistenError?.();
  });

  // ==========================================================================
  // Event Handlers
  // ==========================================================================

  function handleSessionUpdate(notification: SessionNotification) {
    // DEBUG: Log raw notification to see exact shape
    console.log('[ChatPanel] session-update raw:', JSON.stringify(notification, null, 2));

    const update = notification.update;

    // ACP uses "sessionUpdate" as the discriminator with snake_case values
    if (update.sessionUpdate === 'agent_message_chunk') {
      // Handle text chunks - ContentBlock uses "type" discriminator
      if ('content' in update && update.content.type === 'text') {
        streamingContent += update.content.text;
        scrollToBottom();
      }
    } else if (update.sessionUpdate === 'tool_call') {
      // New tool call started
      if ('toolCallId' in update) {
        activeToolCalls.set(update.toolCallId, {
          id: update.toolCallId,
          title: update.title,
          status: update.status,
        });
        activeToolCalls = new Map(activeToolCalls);
      }
    } else if (update.sessionUpdate === 'tool_call_update') {
      // Tool call updated
      if ('toolCallId' in update) {
        const existing = activeToolCalls.get(update.toolCallId);
        if (existing) {
          if (update.fields.title) existing.title = update.fields.title;
          if (update.fields.status) existing.status = update.fields.status;
          activeToolCalls = new Map(activeToolCalls);
        }
      }
    }
  }

  function handleSessionComplete(event: SessionCompleteEvent) {
    // DEBUG: Log complete event
    console.log('[ChatPanel] session-complete:', JSON.stringify(event, null, 2));

    // Finalize the streaming message
    if (streamingContent || activeToolCalls.size > 0) {
      const toolCalls: ToolCallSummary[] = Array.from(activeToolCalls.values()).map((tc) => ({
        id: tc.id,
        title: tc.title,
        status: tc.status,
      }));

      messages.push({
        role: 'assistant',
        content: streamingContent,
        toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
      });
      messages = [...messages];
    }

    // Update session ID for continuity
    sessionId = event.sessionId;

    // Reset streaming state
    streamingContent = '';
    activeToolCalls = new Map();
    isLoading = false;

    scrollToBottom();
  }

  function handleSessionError(event: SessionErrorEvent) {
    // DEBUG: Log error event
    console.log('[ChatPanel] session-error:', JSON.stringify(event, null, 2));

    error = event.error;
    isLoading = false;
    streamingContent = '';
    activeToolCalls = new Map();
  }

  // ==========================================================================
  // Actions
  // ==========================================================================

  async function sendMessage() {
    const content = inputValue.trim();
    if (!content || isLoading || !aiAvailable) return;

    // Add user message
    messages.push({ role: 'user', content });
    messages = [...messages];
    inputValue = '';
    error = null;

    // Reset streaming state
    streamingContent = '';
    activeToolCalls = new Map();
    isLoading = true;

    await tick();
    scrollToBottom();

    try {
      // Send with streaming - response comes via events
      const response = await sendAgentPromptStreaming(content, {
        repoPath,
        sessionId: sessionId ?? undefined,
      });

      // Save session ID for future messages
      sessionId = response.sessionId;
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

  // Auto-resize textarea
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
      {#each messages as message}
        <div class="message" class:user={message.role === 'user'}>
          <div class="message-icon">
            {#if message.role === 'user'}
              <User size={14} />
            {:else}
              <Bot size={14} />
            {/if}
          </div>
          <div class="message-content">
            {#if message.toolCalls && message.toolCalls.length > 0}
              <div class="tool-calls">
                {#each message.toolCalls as toolCall}
                  <div class="tool-call" class:completed={toolCall.status === 'completed'}>
                    <Wrench size={12} />
                    <span class="tool-title">{toolCall.title}</span>
                  </div>
                {/each}
              </div>
            {/if}
            <div class="message-text">{message.content}</div>
          </div>
        </div>
      {/each}

      <!-- Streaming message -->
      {#if isLoading && (streamingContent || activeToolCalls.size > 0)}
        <div class="message">
          <div class="message-icon">
            <Bot size={14} />
          </div>
          <div class="message-content">
            {#if activeToolCalls.size > 0}
              <div class="tool-calls">
                {#each Array.from(activeToolCalls.values()) as toolCall}
                  <div class="tool-call" class:running={toolCall.status === 'running'}>
                    {#if toolCall.status === 'running'}
                      <Loader2 size={12} class="spinning" />
                    {:else}
                      <Wrench size={12} />
                    {/if}
                    <span class="tool-title">{toolCall.title}</span>
                  </div>
                {/each}
              </div>
            {/if}
            {#if streamingContent}
              <div class="message-text">{streamingContent}<span class="cursor">▋</span></div>
            {/if}
          </div>
        </div>
      {:else if isLoading}
        <div class="message">
          <div class="message-icon">
            <Bot size={14} />
          </div>
          <div class="message-content">
            <div class="message-text thinking">
              <Loader2 size={14} class="spinning" />
              <span>Thinking...</span>
            </div>
          </div>
        </div>
      {/if}
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

  .message {
    display: flex;
    gap: 10px;
  }

  .message.user {
    flex-direction: row-reverse;
  }

  .message-icon {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary);
    border-radius: 50%;
    color: var(--text-muted);
  }

  .message.user .message-icon {
    background: var(--ui-accent);
    color: var(--bg-primary);
  }

  .message-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .message.user .message-content {
    align-items: flex-end;
  }

  .message-text {
    font-size: var(--size-sm);
    color: var(--text-primary);
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .message.user .message-text {
    background: var(--ui-accent);
    color: var(--bg-primary);
    padding: 8px 12px;
    border-radius: 12px 12px 4px 12px;
    max-width: 85%;
  }

  .message:not(.user) .message-text {
    background: var(--bg-primary);
    padding: 8px 12px;
    border-radius: 12px 12px 12px 4px;
    max-width: 85%;
  }

  .message-text.thinking {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
  }

  .cursor {
    animation: blink 1s step-end infinite;
    color: var(--text-muted);
  }

  @keyframes blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0;
    }
  }

  .tool-calls {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tool-call {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--size-xs);
    color: var(--text-muted);
    padding: 4px 8px;
    background: var(--bg-primary);
    border-radius: 4px;
    border: 1px solid var(--border-subtle);
  }

  .tool-call.running {
    border-color: var(--text-accent);
  }

  .tool-call.completed {
    border-color: var(--ui-accent);
  }

  .tool-call :global(svg) {
    flex-shrink: 0;
  }

  .tool-title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
