<script lang="ts">
  import { X, AlertCircle } from 'lucide-svelte';

  interface Props {
    message: string;
    onClose: () => void;
  }

  let { message, onClose }: Props = $props();

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="modal-backdrop"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  onclick={handleBackdropClick}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
>
  <div class="modal error-modal">
    <header class="modal-header">
      <AlertCircle size={16} />
      <h2>Error</h2>
      <button class="icon-btn" onclick={onClose}>
        <X size={16} />
      </button>
    </header>
    <div class="modal-body">
      <p>{message}</p>
    </div>
    <div class="modal-footer">
      <button class="primary-btn" onclick={onClose}>OK</button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--shadow-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-chrome);
    border-radius: 12px;
    box-shadow: var(--shadow-elevated);
    width: 420px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .modal-header :global(svg:first-child) {
    color: var(--ui-danger);
  }

  .modal-header h2 {
    margin: 0;
    flex: 1;
    font-size: var(--size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      color 0.1s,
      background-color 0.1s;
  }

  .icon-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .modal-body {
    padding: 20px;
  }

  .modal-body p {
    margin: 0;
    font-size: var(--size-sm);
    line-height: 1.5;
    color: var(--text-primary);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .primary-btn {
    padding: 8px 16px;
    background: var(--ui-accent);
    border: none;
    border-radius: 6px;
    color: var(--bg-chrome);
    font-size: var(--size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.1s;
  }

  .primary-btn:hover {
    opacity: 0.9;
  }
</style>
