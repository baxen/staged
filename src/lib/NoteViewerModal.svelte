<!--
  NoteViewerModal.svelte - View a branch note (live or historical)

  Shows the markdown content of a note. For generating notes,
  subscribes to streaming events for real-time updates via the shared store.
  
  Session viewing is accessed via a button (consistent with commit sessions).
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { X, FileText, Loader2, AlertCircle, MessageSquare } from 'lucide-svelte';
  import type { BranchNote } from './services/branch';
  import {
    connectToSession,
    disconnectFromSession,
    type StreamingSessionState,
    type ConnectOptions,
  } from './stores/streamingSession.svelte';
  import StreamingMessages from './StreamingMessages.svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';

  interface Props {
    note: BranchNote;
    /** Whether this is a live generating note */
    isLive?: boolean;
    onClose: () => void;
    /** Callback to open the session viewer modal */
    onViewSession?: (sessionId: string, title: string) => void;
  }

  let { note, isLive = false, onClose, onViewSession }: Props = $props();

  // Streaming store connection
  let streamState = $state<StreamingSessionState | null>(null);
  let connectOptions: ConnectOptions | undefined;

  // Derived state
  let isGenerating = $derived(note.status === 'generating');
  let isError = $derived(note.status === 'error');
  let hasSession = $derived(!!note.aiSessionId);

  // Render markdown content
  let renderedContent = $derived.by(() => {
    if (!note.content) return '';
    const rawHtml = marked(note.content) as string;
    return DOMPurify.sanitize(rawHtml);
  });

  // Refs
  let contentContainer: HTMLDivElement;

  onMount(async () => {
    if (isLive && isGenerating && note.aiSessionId) {
      connectOptions = {
        onIdle: () => onClose(),
        onError: () => {
          // Error state is handled via note.status
        },
      };
      streamState = connectToSession(note.aiSessionId, connectOptions);
    }
  });

  onDestroy(() => {
    if (note.aiSessionId) {
      disconnectFromSession(note.aiSessionId, connectOptions);
    }
  });

  function handleViewSession() {
    if (note.aiSessionId && onViewSession) {
      onViewSession(note.aiSessionId, note.title);
    }
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
        <FileText size={18} class="header-icon" />
        <span class="header-title">{note.title}</span>
        {#if isGenerating}
          <span class="status-badge generating">
            <Loader2 size={12} class="spinning" />
            Generating
          </span>
        {/if}
      </div>
      <div class="header-right">
        {#if hasSession && !isGenerating && onViewSession}
          <button class="session-btn" onclick={handleViewSession} title="View session">
            <MessageSquare size={14} />
            Session
          </button>
        {/if}
        <button class="close-btn" onclick={onClose}>
          <X size={18} />
        </button>
      </div>
    </header>

    <div class="modal-content" bind:this={contentContainer}>
      {#if isGenerating}
        <!-- Live streaming view during generation -->
        <div class="generating-view">
          <StreamingMessages
            messages={[]}
            streamingSegments={streamState?.streamingSegments ?? []}
            isActive={true}
            waitingText="Generating note..."
          />
        </div>
      {:else if isError}
        <div class="error-content">
          <div class="error-indicator">
            <AlertCircle size={24} />
            <span>Generation Failed</span>
          </div>
          <p class="error-message">{note.errorMessage || 'An unknown error occurred'}</p>
        </div>
      {:else}
        <!-- Content view (rendered markdown) -->
        <div class="markdown-content">
          {#if note.content}
            {@html renderedContent}
          {:else}
            <p class="empty-content">No content yet</p>
          {/if}
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
    flex: 1;
  }

  :global(.header-icon) {
    flex-shrink: 0;
    color: var(--text-accent);
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

  .status-badge.generating {
    background: var(--text-accent);
    color: var(--bg-deepest);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .session-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: transparent;
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .session-btn:hover {
    border-color: var(--ui-accent);
    color: var(--ui-accent);
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
    padding: 20px;
  }

  /* Generating view */
  .generating-view {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Error state */
  .error-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    padding: 40px;
  }

  .error-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--ui-danger);
    font-size: var(--size-lg);
  }

  .error-message {
    font-size: var(--size-sm);
    color: var(--ui-danger);
    margin: 0;
    text-align: center;
    max-width: 400px;
  }

  /* Markdown content */
  .markdown-content {
    font-size: var(--size-md);
    line-height: 1.6;
    color: var(--text-primary);
  }

  .empty-content {
    color: var(--text-faint);
    font-style: italic;
    text-align: center;
    padding: 40px;
  }

  .markdown-content :global(h1) {
    font-size: var(--size-xl);
    font-weight: 600;
    margin: 0 0 16px 0;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .markdown-content :global(h2) {
    font-size: var(--size-lg);
    font-weight: 600;
    margin: 24px 0 12px 0;
  }

  .markdown-content :global(h3) {
    font-size: var(--size-md);
    font-weight: 600;
    margin: 20px 0 8px 0;
  }

  .markdown-content :global(p) {
    margin: 0 0 12px 0;
  }

  .markdown-content :global(ul),
  .markdown-content :global(ol) {
    margin: 0 0 12px 0;
    padding-left: 24px;
  }

  .markdown-content :global(li) {
    margin: 4px 0;
  }

  .markdown-content :global(code) {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: var(--size-sm);
    background-color: var(--bg-elevated);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .markdown-content :global(pre) {
    background-color: var(--bg-deepest);
    border-radius: 8px;
    padding: 16px;
    overflow-x: auto;
    margin: 12px 0;
  }

  .markdown-content :global(pre code) {
    background: none;
    padding: 0;
  }

  .markdown-content :global(strong) {
    font-weight: 600;
  }

  .markdown-content :global(a) {
    color: var(--text-accent);
    text-decoration: none;
  }

  .markdown-content :global(a:hover) {
    text-decoration: underline;
  }

  /* Spinner animation */
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
