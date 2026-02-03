<!--
  SessionViewerModal.svelte - View an AI session (live or historical)

  Shows the conversation with tool calls and text. For running sessions,
  subscribes to streaming events for real-time updates.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { X, Bot, User, Loader2, Wrench, AlertCircle } from 'lucide-svelte';
  import {
    getSession,
    getSessionStatus,
    listenToSessionUpdates,
    listenToSessionStatus,
    parseAssistantContent,
    type SessionNotification,
    type SessionStatusEvent,
    type Message,
    type ContentSegment,
    type SessionStatus,
  } from './services/ai';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  interface Props {
    /** The AI session ID to display */
    sessionId: string;
    /** Title to show in the header (e.g., the prompt) */
    title?: string;
    onClose: () => void;
  }

  let { sessionId, title, onClose }: Props = $props();

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
    content: string;
    segments: DisplaySegment[];
  }

  let messages = $state<DisplayMessage[]>([]);
  let loading = $state(true);
  let isProcessing = $state(false);
  let error = $state<string | null>(null);

  // Streaming state (current turn only)
  let streamingSegments = $state<DisplaySegment[]>([]);
  let toolCallMap = $state<Map<string, DisplaySegment & { type: 'tool' }>>(new Map());

  // Refs
  let messagesContainer: HTMLDivElement;

  // Event listeners
  let unlistenUpdate: UnlistenFn | null = null;
  let unlistenStatus: UnlistenFn | null = null;

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  onMount(async () => {
    // Set up event listeners first (so we don't miss any events)
    unlistenUpdate = await listenToSessionUpdates(handleSessionUpdate);
    unlistenStatus = await listenToSessionStatus(handleSessionStatus);

    // Load session data
    await loadSession();
  });

  onDestroy(() => {
    unlistenUpdate?.();
    unlistenStatus?.();
  });

  async function loadSession() {
    loading = true;
    error = null;

    try {
      // Get session with messages
      const sessionData = await getSession(sessionId);
      if (!sessionData) {
        error = 'Session not found';
        return;
      }

      // Convert messages to display format
      messages = sessionData.messages.map(toDisplayMessage);

      // Check if session is currently processing
      const status = await getSessionStatus(sessionId);
      isProcessing = status.status === 'processing';

      scrollToBottom();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  // ==========================================================================
  // Event Handlers
  // ==========================================================================

  function handleSessionUpdate(notification: SessionNotification) {
    // Only process updates for our session
    // Note: notification.sessionId might be the ACP session ID
    // For now, if we're processing, accept updates
    if (!isProcessing) {
      return;
    }

    const update = notification.update;

    if (update.sessionUpdate === 'agent_message_chunk') {
      if ('content' in update && update.content.type === 'text') {
        const lastSegment = streamingSegments[streamingSegments.length - 1];
        if (lastSegment && lastSegment.type === 'text') {
          lastSegment.text += update.content.text;
          streamingSegments = [...streamingSegments];
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
          streamingSegments = [...streamingSegments];
        }
      }
    }
  }

  async function handleSessionStatus(event: SessionStatusEvent) {
    // Only process status for our session
    if (event.sessionId !== sessionId) {
      return;
    }

    if (event.status.status === 'idle') {
      // Turn complete - refresh from database
      await refreshFromDatabase();
      isProcessing = false;
    } else if (event.status.status === 'processing') {
      isProcessing = true;
    } else if (event.status.status === 'error') {
      error = event.status.message;
      isProcessing = false;
      streamingSegments = [];
      toolCallMap = new Map();
    }
  }

  async function refreshFromDatabase() {
    try {
      const sessionData = await getSession(sessionId);
      if (sessionData) {
        messages = sessionData.messages.map(toDisplayMessage);
      }
    } catch (e) {
      console.error('Failed to refresh from database:', e);
    }

    streamingSegments = [];
    toolCallMap = new Map();
    scrollToBottom();
  }

  function toDisplayMessage(msg: Message): DisplayMessage {
    if (msg.role === 'user') {
      return { role: 'user', content: msg.content, segments: [] };
    }
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

  function scrollToBottom() {
    tick().then(() => {
      if (messagesContainer) {
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
      }
    });
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
        <Bot size={18} />
        <span class="header-title">{title || 'Session'}</span>
        {#if isProcessing}
          <span class="status-badge processing">
            <Loader2 size={12} class="spinning" />
            Running
          </span>
        {/if}
      </div>
      <button class="close-btn" onclick={onClose}>
        <X size={18} />
      </button>
    </header>

    <div class="modal-content" bind:this={messagesContainer}>
      {#if loading}
        <div class="loading-state">
          <Loader2 size={24} class="spinning" />
          <span>Loading session...</span>
        </div>
      {:else if error}
        <div class="error-state">
          <AlertCircle size={24} />
          <span>{error}</span>
        </div>
      {:else if messages.length === 0 && !isProcessing}
        <div class="empty-state">
          <Bot size={32} />
          <span>No messages yet</span>
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

        <!-- Streaming content -->
        {#if isProcessing && streamingSegments.length > 0}
          <div class="message">
            <div class="message-icon">
              <Bot size={14} />
            </div>
            <div class="message-content">
              {#each streamingSegments as segment, i}
                {#if segment.type === 'text'}
                  <div class="message-text">
                    {segment.text}{#if i === streamingSegments.length - 1}<span class="cursor"
                        >▋</span
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
        {:else if isProcessing}
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
    max-width: 700px;
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
  }

  .header-content :global(svg) {
    flex-shrink: 0;
    color: var(--text-muted);
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

  .status-badge.processing {
    background: var(--ui-accent);
    color: var(--bg-deepest);
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
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .loading-state,
  .error-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px;
    color: var(--text-muted);
  }

  .error-state {
    color: var(--ui-danger);
  }

  .empty-state :global(svg) {
    color: var(--text-faint);
  }

  /* Messages */
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
</style>
