<script lang="ts">
  import { GitBranch, Tag, Diamond, X, AlertCircle } from 'lucide-svelte';
  import { getRefs, resolveRef } from './services/git';
  import type { GitRef } from './types';
  import { WORKDIR } from './stores/diffSelection.svelte';

  interface Props {
    initialBase: string;
    initialHead: string;
    onSubmit: (base: string, head: string) => void;
    onClose: () => void;
  }

  let { initialBase, initialHead, onSubmit, onClose }: Props = $props();

  // Use a function to capture initial values properly
  let baseInput = $state('');
  let headInput = $state('');

  // Initialize from props on first render
  $effect(() => {
    // Only set once when component mounts
    if (baseInput === '' && headInput === '') {
      baseInput = initialBase;
      headInput = initialHead;
    }
  });

  let error = $state<string | null>(null);
  let validating = $state(false);

  // Autocomplete state
  let allRefs = $state<GitRef[]>([]);
  let activeInput = $state<'base' | 'head' | null>(null);
  let selectedIndex = $state(0);

  // Load refs on mount
  $effect(() => {
    getRefs().then((refs) => {
      allRefs = refs;
    });
  });

  // Clear error when inputs change
  $effect(() => {
    const _ = [baseInput, headInput];
    error = null;
  });

  // Filtered refs for autocomplete
  let filteredRefs = $derived.by(() => {
    if (!activeInput) return [];
    const query = (activeInput === 'base' ? baseInput : headInput).toLowerCase();
    return allRefs.filter((r) => {
      // WORKDIR can only be used as head
      if (activeInput === 'base' && r.name === WORKDIR) return false;
      return r.name.toLowerCase().includes(query);
    });
  });

  function selectSuggestion(ref: GitRef) {
    if (activeInput === 'base') {
      baseInput = ref.name;
    } else if (activeInput === 'head') {
      headInput = ref.name;
    }
    selectedIndex = 0;
    activeInput = null;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if (activeInput) {
        activeInput = null;
      } else {
        onClose();
      }
      event.preventDefault();
    } else if (event.key === 'Enter') {
      if (activeInput && filteredRefs.length > 0 && selectedIndex < filteredRefs.length) {
        selectSuggestion(filteredRefs[selectedIndex]);
        event.preventDefault();
      } else if (!activeInput) {
        handleSubmit();
        event.preventDefault();
      }
    } else if (event.key === 'ArrowDown' && activeInput) {
      event.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, filteredRefs.length - 1);
    } else if (event.key === 'ArrowUp' && activeInput) {
      event.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (event.key === 'Tab' && activeInput && filteredRefs.length > 0) {
      event.preventDefault();
      selectSuggestion(filteredRefs[selectedIndex]);
    }
  }

  async function handleSubmit() {
    error = null;
    validating = true;

    try {
      // Validate: WORKDIR can only be used as head
      if (baseInput === WORKDIR) {
        error = 'Working directory can only be used as the "after" state';
        validating = false;
        return;
      }

      // Validate refs exist
      const baseSha = await resolveRef(baseInput);
      const headSha = await resolveRef(headInput);

      if (!baseSha) {
        error = `Cannot resolve: ${baseInput}`;
        validating = false;
        return;
      }
      if (!headSha) {
        error = `Cannot resolve: ${headInput}`;
        validating = false;
        return;
      }

      onSubmit(baseInput, headInput);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      validating = false;
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
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
  <div class="modal">
    <header class="modal-header">
      <h2>Custom Diff Range</h2>
      <button class="close-btn" onclick={onClose}>
        <X size={16} />
      </button>
    </header>

    <div class="modal-body">
      <p class="description">
        Compare changes between two git references. The diff shows what changed going from "before"
        to "after".
      </p>

      <div class="input-group">
        <label for="base-input">Before (base)</label>
        <div class="ref-input-container">
          <input
            id="base-input"
            type="text"
            class="ref-input"
            bind:value={baseInput}
            placeholder="e.g., main, HEAD~3, v1.0.0"
            onfocus={() => {
              activeInput = 'base';
              selectedIndex = 0;
            }}
            onblur={() => setTimeout(() => (activeInput = null), 150)}
            autocomplete="off"
            spellcheck="false"
          />
          {#if activeInput === 'base' && filteredRefs.length > 0}
            <div class="suggestions">
              {#each filteredRefs.slice(0, 8) as ref, i}
                <button
                  class="suggestion"
                  class:selected={i === selectedIndex}
                  onmousedown={() => selectSuggestion(ref)}
                >
                  {#if ref.ref_type === 'branch'}
                    <GitBranch size={12} />
                  {:else if ref.ref_type === 'tag'}
                    <Tag size={12} />
                  {:else}
                    <Diamond size={12} />
                  {/if}
                  <span>{ref.name}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <span class="hint">The starting point for comparison</span>
      </div>

      <div class="input-group">
        <label for="head-input">After (head)</label>
        <div class="ref-input-container">
          <input
            id="head-input"
            type="text"
            class="ref-input"
            bind:value={headInput}
            placeholder="e.g., HEAD, feature-branch, WORKDIR"
            onfocus={() => {
              activeInput = 'head';
              selectedIndex = 0;
            }}
            onblur={() => setTimeout(() => (activeInput = null), 150)}
            autocomplete="off"
            spellcheck="false"
          />
          {#if activeInput === 'head' && filteredRefs.length > 0}
            <div class="suggestions">
              {#each filteredRefs.slice(0, 8) as ref, i}
                <button
                  class="suggestion"
                  class:selected={i === selectedIndex}
                  onmousedown={() => selectSuggestion(ref)}
                >
                  {#if ref.ref_type === 'branch'}
                    <GitBranch size={12} />
                  {:else if ref.ref_type === 'tag'}
                    <Tag size={12} />
                  {:else}
                    <Diamond size={12} />
                  {/if}
                  <span>{ref.name}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <span class="hint">The ending point (use WORKDIR for uncommitted changes)</span>
      </div>

      {#if error}
        <div class="error">
          <AlertCircle size={14} />
          <span>{error}</span>
        </div>
      {/if}
    </div>

    <footer class="modal-footer">
      <button class="btn btn-secondary" onclick={onClose}>Cancel</button>
      <button class="btn btn-primary" onclick={handleSubmit} disabled={validating}>
        {validating ? 'Validating...' : 'View Diff'}
      </button>
    </footer>
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
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-header h2 {
    margin: 0;
    font-size: var(--size-base);
    font-weight: 600;
    color: var(--text-primary);
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
    background-color: var(--bg-hover);
  }

  .modal-body {
    padding: 20px;
  }

  .description {
    margin: 0 0 20px;
    font-size: var(--size-sm);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .input-group {
    margin-bottom: 16px;
  }

  .input-group label {
    display: block;
    margin-bottom: 6px;
    font-size: var(--size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .ref-input-container {
    position: relative;
  }

  .ref-input {
    width: 100%;
    padding: 10px 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    box-sizing: border-box;
    transition:
      border-color 0.1s,
      background-color 0.1s;
  }

  .ref-input::placeholder {
    color: var(--text-faint);
  }

  .ref-input:focus {
    outline: none;
    border-color: var(--border-emphasis);
    background-color: var(--bg-hover);
  }

  .hint {
    display: block;
    margin-top: 4px;
    font-size: var(--size-xs);
    color: var(--text-faint);
  }

  .suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    box-shadow: var(--shadow-elevated);
    overflow: hidden;
    z-index: 10;
  }

  .suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .suggestion:hover,
  .suggestion.selected {
    background-color: var(--bg-hover);
  }

  .suggestion :global(svg) {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .error {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background-color: var(--ui-danger-bg);
    border-radius: 6px;
    color: var(--ui-danger);
    font-size: var(--size-sm);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .btn {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: var(--size-sm);
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color 0.1s,
      opacity 0.1s;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--border-subtle);
  }

  .btn-primary {
    background: var(--ui-accent);
    color: var(--bg-primary);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--ui-accent-hover);
  }
</style>
