<!--
  NewArtifactModal.svelte - Modal for creating new artifacts via AI

  Collects a prompt and uses pre-selected context artifacts from the project page.
  Context is shown as compact chips that can be removed.
-->
<script lang="ts">
  import { X, Sparkles, FileText, Loader2 } from 'lucide-svelte';
  import type { Artifact } from './types';

  interface Props {
    projectId: string;
    /** Artifacts already selected on the project page to use as context */
    contextArtifacts: Artifact[];
    /** Called when artifact is successfully created */
    onCreated: (artifact: Artifact) => void;
    onClose: () => void;
    /** Called to remove an artifact from context (updates selection on project page) */
    onRemoveContext: (artifactId: string) => void;
  }

  let { projectId, contextArtifacts, onCreated, onClose, onRemoveContext }: Props = $props();

  // Form state
  let prompt = $state('');
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Textarea ref for auto-focus
  let textareaRef: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    // Auto-focus the textarea when modal opens
    textareaRef?.focus();
  });

  async function handleSubmit() {
    if (!prompt.trim()) return;

    loading = true;
    error = null;

    try {
      // Build context from the pre-selected artifacts
      const contextArtifactIds = contextArtifacts.map((a) => a.id);

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
      <!-- Context chips (if any selected) -->
      {#if contextArtifacts.length > 0}
        <div class="context-section">
          <span class="context-label">Context</span>
          <div class="context-chips">
            {#each contextArtifacts as artifact (artifact.id)}
              <div class="context-chip">
                <FileText size={12} />
                <span class="chip-title">{artifact.title}</span>
                <button
                  class="chip-remove"
                  onclick={() => onRemoveContext(artifact.id)}
                  disabled={loading}
                  title="Remove from context"
                >
                  <X size={12} />
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}

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
        <div class="hint">
          {#if contextArtifacts.length === 0}
            <span class="hint-tip">Tip: Select artifacts on the project page to use as context</span
            >
          {/if}
          <span>⌘Enter to generate</span>
        </div>
      </div>

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
    display: flex;
    justify-content: space-between;
    font-size: var(--size-xs);
    color: var(--text-faint);
  }

  .hint-tip {
    color: var(--text-muted);
  }

  /* Context section */
  .context-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .context-label {
    font-size: var(--size-xs);
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .context-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .context-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background-color: rgba(88, 166, 255, 0.1);
    border: 1px solid rgba(88, 166, 255, 0.3);
    border-radius: 4px;
    font-size: var(--size-xs);
    color: var(--text-accent);
  }

  .context-chip :global(svg) {
    flex-shrink: 0;
  }

  .chip-title {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    margin: -2px -4px -2px 0;
    background: transparent;
    border: none;
    border-radius: 3px;
    color: var(--text-accent);
    cursor: pointer;
    opacity: 0.6;
    transition: all 0.1s;
  }

  .chip-remove:hover {
    opacity: 1;
    background-color: rgba(88, 166, 255, 0.2);
  }

  .chip-remove:disabled {
    cursor: not-allowed;
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
