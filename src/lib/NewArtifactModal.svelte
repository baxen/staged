<!--
  NewArtifactModal.svelte - Modal for creating new artifacts via AI

  Collects a prompt and optional context artifacts, then calls the AI
  to generate a markdown artifact. Only the final message becomes the artifact.
-->
<script lang="ts">
  import { X, Sparkles, FileText, Loader2, Check } from 'lucide-svelte';
  import type { Artifact } from './types';

  interface Props {
    projectId: string;
    /** Available artifacts that can be used as context */
    availableArtifacts: Artifact[];
    /** Called when artifact is successfully created */
    onCreated: (artifact: Artifact) => void;
    onClose: () => void;
  }

  let { projectId, availableArtifacts, onCreated, onClose }: Props = $props();

  // Form state
  let prompt = $state('');
  let selectedContextIds = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Textarea ref for auto-focus
  let textareaRef: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    // Auto-focus the textarea when modal opens
    textareaRef?.focus();
  });

  function toggleContext(artifactId: string) {
    const newSet = new Set(selectedContextIds);
    if (newSet.has(artifactId)) {
      newSet.delete(artifactId);
    } else {
      newSet.add(artifactId);
    }
    selectedContextIds = newSet;
  }

  async function handleSubmit() {
    if (!prompt.trim()) return;

    loading = true;
    error = null;

    try {
      // Build context from selected artifacts
      const contextArtifactIds = Array.from(selectedContextIds);

      // Import dynamically to avoid circular deps
      const { generateArtifact } = await import('./services/project');

      const artifact = await generateArtifact(projectId, prompt.trim(), contextArtifactIds);

      onCreated(artifact);
      onClose();
    } catch (e) {
      console.error('Failed to generate artifact:', e);
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    }
  }

  // Get preview of artifact content
  function getArtifactPreview(artifact: Artifact): string {
    if (artifact.data.type !== 'markdown') return '';
    const content = artifact.data.content;
    // Strip markdown headers and get first meaningful line
    const lines = content.split('\n').filter((line) => {
      const trimmed = line.trim();
      return trimmed && !trimmed.startsWith('#');
    });
    const firstLine = lines[0] || '';
    return firstLine.length > 60 ? firstLine.slice(0, 60) + '...' : firstLine;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" role="dialog" onkeydown={handleKeydown} tabindex="-1">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="presentation">
    <header class="modal-header">
      <div class="header-left">
        <div class="icon">
          <Sparkles size={18} />
        </div>
        <h2>New Artifact</h2>
      </div>
      <button class="close-button" onclick={onClose} title="Close (Esc)">
        <X size={18} />
      </button>
    </header>

    <div class="modal-body">
      <!-- Prompt input -->
      <div class="prompt-section">
        <label for="prompt-input">What would you like to create?</label>
        <textarea
          id="prompt-input"
          bind:this={textareaRef}
          bind:value={prompt}
          placeholder="Research the best practices for..., Create a plan to..., Analyze the tradeoffs between..."
          rows="4"
          disabled={loading}
        ></textarea>
        <div class="hint">Press ⌘Enter to generate</div>
      </div>

      <!-- Context selection -->
      {#if availableArtifacts.length > 0}
        <div class="context-section">
          <span class="context-label">Include as context (optional)</span>
          <div class="context-list">
            {#each availableArtifacts as artifact (artifact.id)}
              <button
                class="context-item"
                class:selected={selectedContextIds.has(artifact.id)}
                onclick={() => toggleContext(artifact.id)}
                disabled={loading}
              >
                <div class="context-checkbox">
                  {#if selectedContextIds.has(artifact.id)}
                    <Check size={12} />
                  {/if}
                </div>
                <FileText size={14} />
                <div class="context-info">
                  <span class="context-title">{artifact.title}</span>
                  <span class="context-preview">{getArtifactPreview(artifact)}</span>
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Error display -->
      {#if error}
        <div class="error-message">
          {error}
        </div>
      {/if}
    </div>

    <footer class="modal-footer">
      <button class="cancel-button" onclick={onClose} disabled={loading}> Cancel </button>
      <button class="generate-button" onclick={handleSubmit} disabled={loading || !prompt.trim()}>
        {#if loading}
          <Loader2 size={16} class="spinner" />
          Generating...
        {:else}
          <Sparkles size={16} />
          Generate
        {/if}
      </button>
    </footer>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    width: 90%;
    max-width: 560px;
    max-height: 80vh;
    background-color: var(--bg-primary);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  }

  /* Header */
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background-color: rgba(88, 166, 255, 0.15);
    color: var(--text-accent);
  }

  .modal-header h2 {
    font-size: var(--size-lg);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
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

  /* Body */
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* Prompt section */
  .prompt-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .prompt-section label {
    font-size: var(--size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .prompt-section textarea {
    width: 100%;
    padding: 12px;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: var(--size-md);
    font-family: inherit;
    resize: vertical;
    min-height: 100px;
  }

  .prompt-section textarea:focus {
    outline: none;
    border-color: var(--text-accent);
  }

  .prompt-section textarea::placeholder {
    color: var(--text-faint);
  }

  .prompt-section textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .hint {
    font-size: var(--size-xs);
    color: var(--text-faint);
    text-align: right;
  }

  /* Context section */
  .context-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .context-label {
    font-size: var(--size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .context-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 200px;
    overflow-y: auto;
  }

  .context-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
  }

  .context-item:hover {
    border-color: var(--text-accent);
  }

  .context-item.selected {
    border-color: var(--text-accent);
    background-color: rgba(88, 166, 255, 0.1);
  }

  .context-item:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .context-checkbox {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border: 1px solid var(--border-muted);
    border-radius: 4px;
    flex-shrink: 0;
    color: var(--text-accent);
  }

  .context-item.selected .context-checkbox {
    background-color: var(--text-accent);
    border-color: var(--text-accent);
    color: var(--bg-deepest);
  }

  .context-item :global(svg:not(.context-checkbox svg)) {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .context-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .context-title {
    font-size: var(--size-sm);
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .context-preview {
    font-size: var(--size-xs);
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Error */
  .error-message {
    padding: 12px;
    background-color: var(--ui-danger-bg);
    border: 1px solid var(--ui-danger);
    border-radius: 6px;
    color: var(--ui-danger);
    font-size: var(--size-sm);
  }

  /* Footer */
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .cancel-button {
    padding: 10px 20px;
    background-color: transparent;
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: var(--size-md);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cancel-button:hover:not(:disabled) {
    background-color: var(--bg-hover);
  }

  .cancel-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .generate-button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background-color: var(--ui-accent);
    border: none;
    border-radius: 8px;
    color: var(--bg-deepest);
    font-size: var(--size-md);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .generate-button:hover:not(:disabled) {
    background-color: var(--ui-accent-hover);
  }

  .generate-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
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
</style>
