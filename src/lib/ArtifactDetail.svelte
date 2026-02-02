<!--
  ArtifactDetail.svelte - Detail view for a selected artifact

  Shows the full content of a markdown artifact or commit info.
  Shows generating state with placeholder when artifact is being created.
  Supports editing and refinement actions.
  Can toggle to show the underlying AI session transcript.
-->
<script lang="ts">
  import {
    X,
    FileText,
    GitCommit,
    Clock,
    Loader2,
    AlertCircle,
    MessageSquare,
    FileOutput,
  } from 'lucide-svelte';
  import type { Artifact, Session } from './types';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import { getSessions } from './services/project';

  interface Props {
    artifact: Artifact;
    onClose?: () => void;
  }

  let { artifact, onClose }: Props = $props();

  // View mode: 'artifact' or 'session'
  let viewMode: 'artifact' | 'session' = $state('artifact');

  // Session data
  let sessions: Session[] = $state([]);
  let loadingSessions = $state(false);
  let sessionError: string | null = $state(null);

  let isMarkdown = $derived(artifact.data.type === 'markdown');
  let isCommit = $derived(artifact.data.type === 'commit');
  let isGenerating = $derived(artifact.status === 'generating');
  let isError = $derived(artifact.status === 'error');

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

  // Parse session transcript into messages
  interface SessionMessage {
    role: 'user' | 'assistant';
    content: string;
  }

  let sessionMessages = $derived.by((): SessionMessage[] => {
    if (sessions.length === 0) return [];
    // Use the most recent session
    const session = sessions[0];
    try {
      return JSON.parse(session.transcript) as SessionMessage[];
    } catch {
      // Fallback: treat as raw text
      return [{ role: 'assistant', content: session.transcript }];
    }
  });

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  }

  async function loadSessions() {
    if (sessions.length > 0 || loadingSessions) return;

    loadingSessions = true;
    sessionError = null;

    try {
      sessions = await getSessions(artifact.id);
    } catch (e) {
      sessionError = e instanceof Error ? e.message : 'Failed to load session';
    } finally {
      loadingSessions = false;
    }
  }

  function switchToSession() {
    viewMode = 'session';
    loadSessions();
  }

  function switchToArtifact() {
    viewMode = 'artifact';
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
      {#if isMarkdown && artifact.status === 'complete'}
        <div class="view-toggle">
          <button
            class="toggle-button"
            class:active={viewMode === 'artifact'}
            onclick={switchToArtifact}
            title="View artifact"
          >
            <FileOutput size={14} />
            <span>Artifact</span>
          </button>
          <button
            class="toggle-button"
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
    {#if isGenerating}
      <div class="generating-content">
        <div class="generating-indicator">
          <Loader2 size={24} class="spinner" />
          <span>Generating with AI...</span>
        </div>
        <p class="generating-hint">
          The artifact content will appear here when generation completes.
        </p>
      </div>
    {:else if isError}
      <div class="error-content">
        <div class="error-indicator">
          <AlertCircle size={24} />
          <span>Generation Failed</span>
        </div>
        <p class="error-message">{artifact.errorMessage || 'An unknown error occurred'}</p>
      </div>
    {:else if viewMode === 'session'}
      <!-- Session view -->
      <div class="session-content">
        {#if loadingSessions}
          <div class="loading-sessions">
            <Loader2 size={20} class="spinner" />
            <span>Loading session...</span>
          </div>
        {:else if sessionError}
          <div class="session-error">
            <AlertCircle size={16} />
            <span>{sessionError}</span>
          </div>
        {:else if sessions.length === 0}
          <div class="no-session">
            <MessageSquare size={20} />
            <span>No session recorded for this artifact</span>
          </div>
        {:else}
          <div class="session-messages">
            {#each sessionMessages as message}
              <div
                class="message"
                class:user={message.role === 'user'}
                class:assistant={message.role === 'assistant'}
              >
                <div class="message-header">
                  <span class="message-role">{message.role === 'user' ? 'You' : 'AI'}</span>
                </div>
                <div class="message-content">
                  {#if message.role === 'assistant'}
                    {@html DOMPurify.sanitize(marked(message.content) as string)}
                  {:else}
                    <p>{message.content}</p>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
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
    align-items: center;
    background-color: var(--bg-elevated);
    border-radius: 6px;
    padding: 2px;
  }

  .toggle-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: var(--size-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toggle-button:hover {
    color: var(--text-primary);
  }

  .toggle-button.active {
    background-color: var(--bg-primary);
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

  /* Generating state */
  .generating-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    color: var(--text-muted);
  }

  .generating-indicator {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--text-accent);
    font-size: var(--size-lg);
  }

  .generating-hint {
    font-size: var(--size-sm);
    color: var(--text-faint);
    margin: 0;
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

  /* Session content */
  .session-content {
    height: 100%;
  }

  .loading-sessions,
  .session-error,
  .no-session {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    color: var(--text-muted);
    font-size: var(--size-sm);
  }

  .session-error {
    color: var(--ui-danger);
  }

  .session-messages {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .message {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .message-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .message-role {
    font-size: var(--size-sm);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .message.user .message-role {
    color: var(--text-accent);
  }

  .message.assistant .message-role {
    color: var(--status-added);
  }

  .message-content {
    padding-left: 0;
    font-size: var(--size-md);
    line-height: 1.6;
    color: var(--text-primary);
  }

  .message.user .message-content {
    background-color: var(--bg-elevated);
    padding: 12px 16px;
    border-radius: 8px;
    border-left: 3px solid var(--text-accent);
  }

  .message.user .message-content p {
    margin: 0;
    white-space: pre-wrap;
  }

  .message.assistant .message-content {
    border-left: 3px solid var(--status-added);
    padding-left: 16px;
  }

  /* Markdown content in session messages */
  .message.assistant .message-content :global(h1) {
    font-size: var(--size-xl);
    font-weight: 600;
    margin: 0 0 16px 0;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .message.assistant .message-content :global(h2) {
    font-size: var(--size-lg);
    font-weight: 600;
    margin: 24px 0 12px 0;
  }

  .message.assistant .message-content :global(h3) {
    font-size: var(--size-md);
    font-weight: 600;
    margin: 20px 0 8px 0;
  }

  .message.assistant .message-content :global(p) {
    margin: 0 0 12px 0;
  }

  .message.assistant .message-content :global(ul),
  .message.assistant .message-content :global(ol) {
    margin: 0 0 12px 0;
    padding-left: 24px;
  }

  .message.assistant .message-content :global(li) {
    margin: 4px 0;
  }

  .message.assistant .message-content :global(code) {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: var(--size-sm);
    background-color: var(--bg-elevated);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .message.assistant .message-content :global(pre) {
    background-color: var(--bg-deepest);
    border-radius: 8px;
    padding: 16px;
    overflow-x: auto;
    margin: 12px 0;
  }

  .message.assistant .message-content :global(pre code) {
    background: none;
    padding: 0;
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
