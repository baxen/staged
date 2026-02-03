<!--
  ArtifactDetail.svelte - Detail view for a selected artifact

  Shows the full content of a markdown artifact or commit info.
  Shows generating state with live streaming progress when artifact is being created.
  Can toggle to show the underlying AI session that generated the artifact.
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    X,
    FileText,
    GitCommit,
    Clock,
    Loader2,
    AlertCircle,
    MessageSquare,
    FileOutput,
    Bot,
    User,
    Wrench,
  } from 'lucide-svelte';
  import type { Artifact } from './types';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import {
    getSession,
    parseAssistantContent,
    listenToSessionUpdates,
    type SessionFull,
    type ContentSegment,
    type SessionNotification,
  } from './services/ai';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  interface Props {
    artifact: Artifact;
    onClose?: () => void;
  }

  let { artifact, onClose }: Props = $props();

  // View mode: 'artifact' shows the rendered content, 'session' shows the conversation
  let viewMode = $state<'artifact' | 'session'>('artifact');

  // Session data (loaded when switching to session view)
  let session = $state<SessionFull | null>(null);
  let sessionLoading = $state(false);
  let sessionError = $state<string | null>(null);

  // Streaming state for live generation
  type DisplaySegment =
    | { type: 'text'; text: string }
    | { type: 'tool'; id: string; title: string; status: string };

  let streamingSegments = $state<DisplaySegment[]>([]);
  let toolCallMap = $state<Map<string, DisplaySegment & { type: 'tool' }>>(new Map());
  let unlistenUpdates: UnlistenFn | null = null;

  let isMarkdown = $derived(artifact.data.type === 'markdown');
  let isCommit = $derived(artifact.data.type === 'commit');
  let isGenerating = $derived(artifact.status === 'generating');
  let isError = $derived(artifact.status === 'error');
  let hasSession = $derived(!!artifact.sessionId);

  // Render markdown content
  let renderedContent = $derived.by(() => {
    if (artifact.data.type !== 'markdown') return '';
    const rawHtml = marked(artifact.data.content) as string;
    return DOMPurify.sanitize(rawHtml);
  });

  // Get commit info
  let commitInfo = $derived.by(() => {
    if (artifact.data.type !== 'commit') return null;
    return {
      repo: artifact.data.repo,
      branch: artifact.data.branch,
      sha: artifact.data.commitSha,
    };
  });

  function formatDate(timestamp: number): string {
    const date = new Date(timestamp);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  }

  // Set up streaming listener when generating
  onMount(async () => {
    if (isGenerating) {
      unlistenUpdates = await listenToSessionUpdates(handleSessionUpdate);
    }
  });

  onDestroy(() => {
    unlistenUpdates?.();
  });

  // Handle streaming updates during generation
  function handleSessionUpdate(notification: SessionNotification) {
    // Process all updates when generating (we only have one active generation at a time)
    if (!isGenerating) return;

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

  async function loadSession() {
    if (!artifact.sessionId || session) return;

    sessionLoading = true;
    sessionError = null;

    try {
      session = await getSession(artifact.sessionId);
      if (!session) {
        sessionError = 'Session not found';
      }
    } catch (e) {
      sessionError = e instanceof Error ? e.message : String(e);
    } finally {
      sessionLoading = false;
    }
  }

  function switchToSession() {
    viewMode = 'session';
    loadSession();
  }

  function switchToArtifact() {
    viewMode = 'artifact';
  }

  /** Parse assistant message content into display segments */
  function parseSegments(content: string): DisplaySegment[] {
    const segments = parseAssistantContent(content);
    return segments.map((seg: ContentSegment) => {
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
  }
</script>

<div class="artifact-detail">
  <header class="detail-header">
    <div class="header-left">
      <div class="type-icon" class:markdown={isMarkdown} class:commit={isCommit}>
        {#if isMarkdown}
          <FileText size={16} />
        {:else}
          <GitCommit size={16} />
        {/if}
      </div>
      <h2 class="title">{artifact.title}</h2>
    </div>
    <div class="header-right">
      {#if hasSession && !isGenerating}
        <div class="view-toggle">
          <button
            class="toggle-btn"
            class:active={viewMode === 'artifact'}
            onclick={switchToArtifact}
            title="View artifact"
          >
            <FileOutput size={14} />
            <span>Artifact</span>
          </button>
          <button
            class="toggle-btn"
            class:active={viewMode === 'session'}
            onclick={switchToSession}
            title="View session"
          >
            <MessageSquare size={14} />
            <span>Session</span>
          </button>
        </div>
      {/if}
      <button class="close-button" onclick={onClose} title="Close">
        <X size={18} />
      </button>
    </div>
  </header>

  <div class="detail-meta">
    <span class="meta-item">
      <Clock size={12} />
      Created {formatDate(artifact.createdAt)}
    </span>
    {#if artifact.updatedAt !== artifact.createdAt}
      <span class="meta-item">
        Updated {formatDate(artifact.updatedAt)}
      </span>
    {/if}
  </div>

  <div class="detail-content">
    {#if viewMode === 'session' && !isGenerating}
      <!-- Session view (completed artifact) -->
      <div class="session-view">
        {#if sessionLoading}
          <div class="session-loading">
            <Loader2 size={24} class="spinner" />
            <span>Loading session...</span>
          </div>
        {:else if sessionError}
          <div class="session-error">
            <AlertCircle size={24} />
            <span>{sessionError}</span>
          </div>
        {:else if session}
          <div class="messages">
            {#each session.messages as message}
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
                    <div class="message-text user-text">{message.content}</div>
                  {:else}
                    <!-- Assistant: render segments in order -->
                    {#each parseSegments(message.content) as segment}
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
          </div>
        {/if}
      </div>
    {:else if isGenerating}
      <!-- Live streaming view during generation -->
      <div class="generating-view">
        <div class="generating-header">
          <Loader2 size={16} class="spinner" />
          <span>Generating with AI...</span>
        </div>

        {#if streamingSegments.length > 0}
          <div class="streaming-content">
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
                        <Loader2 size={12} class="spinner" />
                      {:else}
                        <Wrench size={12} />
                      {/if}
                      <span class="tool-title">{segment.title}</span>
                    </div>
                  {/if}
                {/each}
              </div>
            </div>
          </div>
        {:else}
          <div class="waiting-content">
            <p class="generating-hint">Waiting for AI response...</p>
          </div>
        {/if}
      </div>
    {:else if isError}
      <div class="error-content">
        <div class="error-indicator">
          <AlertCircle size={24} />
          <span>Generation Failed</span>
        </div>
        <p class="error-message">{artifact.errorMessage || 'An unknown error occurred'}</p>
      </div>
    {:else if isMarkdown}
      <div class="markdown-content">
        {@html renderedContent}
      </div>
    {:else if commitInfo}
      <div class="commit-content">
        <div class="commit-field">
          <span class="field-label">Repository</span>
          <span class="field-value">{commitInfo.repo}</span>
        </div>
        <div class="commit-field">
          <span class="field-label">Branch</span>
          <span class="field-value branch">{commitInfo.branch}</span>
        </div>
        <div class="commit-field">
          <span class="field-label">Commit</span>
          <span class="field-value mono">{commitInfo.sha}</span>
        </div>
        <button class="view-diff-button"> View Diff </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .artifact-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-primary);
    border-radius: 8px;
    overflow: hidden;
  }

  /* Header */
  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    background-color: var(--bg-primary);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .type-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    flex-shrink: 0;
  }

  .type-icon.markdown {
    background-color: rgba(88, 166, 255, 0.15);
    color: var(--text-accent);
  }

  .type-icon.commit {
    background-color: rgba(63, 185, 80, 0.15);
    color: var(--status-added);
  }

  .title {
    font-size: var(--size-lg);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* View toggle */
  .view-toggle {
    display: flex;
    background: var(--bg-elevated);
    border-radius: 6px;
    padding: 2px;
    gap: 2px;
  }

  .toggle-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toggle-btn:hover {
    color: var(--text-primary);
  }

  .toggle-btn.active {
    background: var(--bg-primary);
    color: var(--text-primary);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .close-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .close-button:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Meta */
  .detail-meta {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 20px 12px 20px;
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--size-sm);
    color: var(--text-faint);
  }

  /* Content */
  .detail-content {
    flex: 1;
    overflow: auto;
    padding: 20px;
  }

  /* Generating view (live streaming) */
  .generating-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .generating-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-bottom: 16px;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-accent);
    font-size: var(--size-sm);
    font-weight: 500;
  }

  .streaming-content {
    flex: 1;
    overflow: auto;
  }

  .waiting-content {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
  }

  .generating-hint {
    font-size: var(--size-sm);
    color: var(--text-faint);
    margin: 0;
  }

  /* Session view */
  .session-view {
    height: 100%;
  }

  .session-loading,
  .session-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
    color: var(--text-muted);
  }

  .session-error {
    color: var(--ui-danger);
  }

  .messages {
    display: flex;
    flex-direction: column;
    gap: 16px;
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
    background: var(--bg-elevated);
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
    background: var(--bg-elevated);
    padding: 8px 12px;
    border-radius: 12px 12px 12px 4px;
    max-width: 85%;
  }

  .message-text.user-text {
    background: var(--ui-accent);
    color: var(--bg-primary);
    border-radius: 12px 12px 4px 12px;
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
    background: var(--bg-elevated);
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

  /* Spinner animation */
  :global(.spinner) {
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

  /* Error state */
  .error-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
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

  /* Commit content */
  .commit-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .commit-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: var(--size-sm);
    color: var(--text-faint);
  }

  .field-value {
    font-size: var(--size-md);
    color: var(--text-primary);
  }

  .field-value.branch {
    color: var(--status-renamed);
  }

  .field-value.mono {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: var(--size-sm);
  }

  .view-diff-button {
    align-self: flex-start;
    padding: 10px 20px;
    background-color: var(--ui-accent);
    border: none;
    border-radius: 8px;
    color: var(--bg-deepest);
    font-size: var(--size-md);
    font-weight: 500;
    cursor: pointer;
    margin-top: 8px;
    transition: background-color 0.15s ease;
  }

  .view-diff-button:hover {
    background-color: var(--ui-accent-hover);
  }
</style>
