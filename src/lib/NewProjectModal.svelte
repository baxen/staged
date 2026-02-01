<!--
  NewProjectModal.svelte - Modal for creating a new project

  Simple modal to collect a project name.
-->
<script lang="ts">
  import { X, FolderKanban } from 'lucide-svelte';

  interface Props {
    onCreated: (name: string) => void;
    onClose: () => void;
  }

  let { onCreated, onClose }: Props = $props();

  let name = $state('');
  let inputRef: HTMLInputElement | undefined = $state();

  $effect(() => {
    // Auto-focus the input when modal opens
    inputRef?.focus();
  });

  function handleSubmit() {
    const trimmed = name.trim();
    if (!trimmed) return;
    onCreated(trimmed);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'Enter') {
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
          <FolderKanban size={18} />
        </div>
        <h2>New Project</h2>
      </div>
      <button class="close-button" onclick={onClose} title="Close (Esc)">
        <X size={18} />
      </button>
    </header>

    <div class="modal-body">
      <div class="input-section">
        <label for="project-name">Project name</label>
        <input
          id="project-name"
          type="text"
          bind:this={inputRef}
          bind:value={name}
          placeholder="My Project"
        />
      </div>
    </div>

    <footer class="modal-footer">
      <button class="cancel-button" onclick={onClose}> Cancel </button>
      <button class="create-button" onclick={handleSubmit} disabled={!name.trim()}>
        Create Project
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
    max-width: 420px;
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
    padding: 20px;
  }

  .input-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .input-section label {
    font-size: var(--size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .input-section input {
    width: 100%;
    padding: 12px;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: var(--size-md);
    font-family: inherit;
  }

  .input-section input:focus {
    outline: none;
    border-color: var(--text-accent);
  }

  .input-section input::placeholder {
    color: var(--text-faint);
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

  .cancel-button:hover {
    background-color: var(--bg-hover);
  }

  .create-button {
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

  .create-button:hover:not(:disabled) {
    background-color: var(--ui-accent-hover);
  }

  .create-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
