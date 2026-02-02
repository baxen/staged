<!--
  ChatPanel.svelte - AI chat interface panel

  A slide-out panel for chatting with an AI agent via ACP.
  Uses the new chat session architecture with SQLite persistence.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { X, Send, Bot, User, Loader2, Wrench, AlertCircle } from 'lucide-svelte';
  import {
    checkAiAvailable,
    createChatSession,
    getChatSession,
    sendChatPrompt,
    listenToSessionUpdates,
    listenToSessionStatus,
    parseAssistantContent,
    type SessionNotification,
    type SessionStatusEvent,
    type ChatMessage,
    type ContentSegment,
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

  /** A segment for display - text or tool call */
  type DisplaySegment =
    | { type: 'text'; text: string }
    | { type: 'tool'; id: string; title: string; status: string };

  /** Display message - user has plain text, assistant has segments */
  interface DisplayMessage {
    role: 'user' | 'assistant';
    /** For user: plain text. For assistant: unused (use segments) */
    content: string;
    /** For assistant: ordered segments. For user: empty */
    segments: DisplaySegment[];
  }

  let messages = $state<DisplayMessage[]>([]);
  let inputValue = $state('');
  let isLoading = $state(false);
  let sessionId = $state<string | null>(null);
  let error = $state<string | null>(null);
  let aiAvailable = $state<boolean | null>(null);
  let aiAgentName = $state<string>('');

  // Streaming state (current turn only)
  // Segments arrive in order: text, tool, more text, etc.
  let streamingSegments = $state<DisplaySegment[]>([]);
  // Track tool calls by ID for updates
  let toolCallMap = $state<Map<string, DisplaySegment & { type: 'tool' }>>(new Map());

  // Refs
  let messagesContainer: HTMLDivElement;
  let inputElement: HTMLTextAreaElement;

  // Event listeners
  let unlistenUpdate: UnlistenFn | null = null;
  let unlistenStatus: UnlistenFn | null = null;

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
    unlistenStatus = await listenToSessionStatus(handleSessionStatus);

    // Focus input
    inputElement?.focus();
  });

  onDestroy(() => {
    unlistenUpdate?.();
    unlistenStatus?.();
  });

  // ==========================================================================
  // Event Handlers
  // ==========================================================================

  function handleSessionUpdate(notification: SessionNotification) {
    // Note: notification.sessionId is the ACP session ID, not our chat session ID.
    // For now, we process all updates since we only have one active chat.
    // TODO: When supporting multiple chats, we'll need to map ACP session IDs to our IDs.
    if (!isLoading) {
      // Ignore updates when we're not expecting them
      return;
    }

    const update = notification.update;

    if (update.sessionUpdate === 'agent_message_chunk') {
      if ('content' in update && update.content.type === 'text') {
        // Append to the last text segment, or create a new one
        const lastSegment = streamingSegments[streamingSegments.length - 1];
        if (lastSegment && lastSegment.type === 'text') {
          lastSegment.text += update.content.text;
          streamingSegments = [...streamingSegments]; // trigger reactivity
        } else {
          streamingSegments = [...streamingSegments, { type: 'text', text: update.content.text }];
        }
        scrollToBottom();
      }
    } else if (update.sessionUpdate === 'tool_call') {
      if ('toolCallId' in update) {
        const toolSegment: DisplaySegment & { type: 'tool' } = {
          type: 'tool',
          id: update.toolCallId,
          title: update.title,
          status: update.status,
        };
        toolCallMap.set(update.toolCallId, toolSegment);
        streamingSegments = [...streamingSegments, toolSegment];
        scrollToBottom();
      }
    } else if (update.sessionUpdate === 'tool_call_update') {
      if ('toolCallId' in update) {
        const existing = toolCallMap.get(update.toolCallId);
        if (existing && update.fields) {
          if (update.fields.title) existing.title = update.fields.title;
          if (update.fields.status) existing.status = update.fields.status;
          streamingSegments = [...streamingSegments]; // trigger reactivity
        }
      }
    }
  }

  async function handleSessionStatus(event: SessionStatusEvent) {
    // Only process status for our session
    if (!sessionId || event.sessionId !== sessionId) {
      return;
    }

    if (event.status.status === 'idle') {
      // Turn complete - refresh from database to get canonical state
      await refreshFromDatabase();
      isLoading = false;
    } else if (event.status.status === 'error') {
      error = event.status.message;
      isLoading = false;
      // Clear streaming state
      streamingSegments = [];
      toolCallMap = new Map();
    }
  }

  async function refreshFromDatabase() {
    if (!sessionId) return;

    try {
      const session = await getChatSession(sessionId);
      if (session) {
        // Convert persisted messages to display format
        messages = session.messages.map(toDisplayMessage);
      }
    } catch (e) {
      console.error('Failed to refresh from database:', e);
    }

    // Clear streaming state
    streamingSegments = [];
    toolCallMap = new Map();
    scrollToBottom();
  }

  function toDisplayMessage(msg: ChatMessage): DisplayMessage {
    if (msg.role === 'user') {
      return { role: 'user', content: msg.content, segments: [] };
    }
    // Assistant: parse content as segments
    const contentSegments = parseAssistantContent(msg.content);
    const segments: DisplaySegment[] = contentSegments.map((seg: ContentSegment) => {
      if (seg.type === 'text') {
        return { type: 'text' as const, text: seg.text };
      } else {
        return {
          type: 'tool' as const,
          id: seg.id,
          title: seg.title,
          status: seg.status,
        };
      }
    });
    return { role: 'assistant', content: '', segments };
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
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        return;
      }
    }

    // Add user message to display immediately
    messages = [...messages, { role: 'user', content, segments: [] }];

    // Reset streaming state
    streamingSegments = [];
    toolCallMap = new Map();
    isLoading = true;

    await tick();
    scrollToBottom();

    try {
      // Send prompt - response streams via events
      await sendChatPrompt(sessionId, content);
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
            {#if message.role === 'user'}
              <div class="message-text">{message.content}</div>
            {:else}
              <!-- Assistant: render segments in order -->
              {#each message.segments as segment}
                {#if segment.type === 'text'}
                  <div class="message-text">{segment.text}</div>
                {:else}
                  <div class="tool-call" class:completed={segment.status === 'completed'}>
                    <Wrench size={12} />
                    <span class="tool-title">{segment.title}</span>
                  </div>
                {/if}
              {/each}
            {/if}
          </div>
        </div>
      {/each}

      <!-- Streaming message (current turn) - segments in arrival order -->
      {#if isLoading && streamingSegments.length > 0}
        <div class="message">
          <div class="message-icon">
            <Bot size={14} />
          </div>
          <div class="message-content">
            {#each streamingSegments as segment, i}
              {#if segment.type === 'text'}
                <div class="message-text">
                  {segment.text}{#if i === streamingSegments.length - 1}<span class="cursor">▋</span
                    >{/if}
                </div>
              {:else}
                <div
                  class="tool-call"
                  class:running={segment.status === 'running'}
                  class:completed={segment.status === 'completed'}
                >
                  {#if segment.status === 'running'}
                    <Loader2 size={12} class="spinning" />
                  {:else}
                    <Wrench size={12} />
                  {/if}
                  <span class="tool-title">{segment.title}</span>
                </div>
              {/if}
            {/each}
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
