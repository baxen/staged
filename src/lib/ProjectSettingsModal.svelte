<!--
  ProjectSettingsModal.svelte - Modal for editing project settings

  Allows editing:
  - Project name (display name)
  - Subpath (for monorepos - sets the cwd for AI sessions)
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import { X, Folder } from 'lucide-svelte';
  import type { GitProject } from './services/branch';
  import * as branchService from './services/branch';
  import { listDirectory, type DirEntry } from './services/files';

  interface Props {
    project: GitProject;
    onSave: (project: GitProject) => void;
    onClose: () => void;
  }

  let { project, onSave, onClose }: Props = $props();

  // Form state - initialized from project prop using untrack since this is intentional
  let name = $state(untrack(() => project.name));
  let subpath = $state(untrack(() => project.subpath || ''));
  let saving = $state(false);
  let error = $state<string | null>(null);

  // Subpath suggestions
  let subpathInputFocused = $state(false);
  let subpathSuggestions = $state<DirEntry[]>([]);
  let subpathSelectedIndex = $state(0);
  let subpathSearchTimeout: ReturnType<typeof setTimeout> | null = null;

  // Validation
  let isValid = $derived(name.trim().length > 0);

  // Split the current subpath value into a directory to list and a partial segment to filter by
  function getSubpathContext(): { dir: string; query: string } {
    const trimmed = subpath.replace(/^\/+/, '');
    if (!trimmed || trimmed.endsWith('/')) {
      const parentRel = trimmed.replace(/\/+$/, '');
      return {
        dir: parentRel ? project.repoPath + '/' + parentRel : project.repoPath,
        query: '',
      };
    }
    const lastSlash = trimmed.lastIndexOf('/');
    if (lastSlash === -1) {
      return { dir: project.repoPath, query: trimmed };
    }
    return {
      dir: project.repoPath + '/' + trimmed.slice(0, lastSlash),
      query: trimmed.slice(lastSlash + 1),
    };
  }

  // Reload the directory listing whenever subpath or focus changes
  $effect(() => {
    if (!subpathInputFocused) {
      subpathSuggestions = [];
      return;
    }

    if (subpathSearchTimeout) clearTimeout(subpathSearchTimeout);

    const { dir } = getSubpathContext();
    subpathSearchTimeout = setTimeout(async () => {
      try {
        const allEntries = await listDirectory(dir);
        subpathSuggestions = allEntries.filter((e) => e.isDir);
        subpathSelectedIndex = 0;
      } catch {
        subpathSuggestions = [];
      }
    }, 100);
  });

  // Filter the loaded listing by the partial trailing segment
  let filteredSubpathSuggestions = $derived.by(() => {
    const { query: q } = getSubpathContext();
    if (!q) return subpathSuggestions;
    const lower = q.toLowerCase();
    return subpathSuggestions.filter((e) => e.name.toLowerCase().includes(lower));
  });

  function selectSubpathSuggestion(entry: DirEntry) {
    subpath = entry.path.replace(project.repoPath + '/', '') + '/';
    subpathSelectedIndex = 0;
  }

  function handleSubpathKeydown(e: KeyboardEvent) {
    if (filteredSubpathSuggestions.length === 0) return;
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      subpathSelectedIndex = Math.min(subpathSelectedIndex + 1, filteredSubpathSuggestions.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      subpathSelectedIndex = Math.max(subpathSelectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      selectSubpathSuggestion(filteredSubpathSuggestions[subpathSelectedIndex]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      subpathInputFocused = false;
    }
  }

  async function handleSave() {
    if (!isValid || saving) return;

    saving = true;
    error = null;

    try {
      // Normalize subpath: remove leading/trailing slashes, empty string becomes null
      const normalizedSubpath = subpath.trim().replace(/^\/+|\/+$/g, '') || null;

      await branchService.updateGitProject(project.id, name.trim(), normalizedSubpath);

      // Return updated project
      onSave({
        ...project,
        name: name.trim(),
        subpath: normalizedSubpath,
        updatedAt: Date.now(),
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      saving = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSave();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={onClose} onkeydown={handleKeydown}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h2>Project Settings</h2>
      <button class="close-button" onclick={onClose} title="Close">
        <X size={18} />
      </button>
    </div>

    <div class="modal-body">
      <div class="form-group">
        <label for="project-name">Name</label>
        <input
          id="project-name"
          type="text"
          bind:value={name}
          placeholder="Project name"
          disabled={saving}
        />
        <span class="help-text">Display name for this project</span>
      </div>

      <div class="form-group">
        <label for="project-subpath">Subpath</label>
        <input
          id="project-subpath"
          type="text"
          bind:value={subpath}
          placeholder="e.g., packages/frontend"
          disabled={saving}
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          spellcheck="false"
          onfocus={() => (subpathInputFocused = true)}
          onblur={() => (subpathInputFocused = false)}
          onkeydown={handleSubpathKeydown}
        />
        {#if subpathInputFocused && filteredSubpathSuggestions.length > 0}
          <div class="subpath-suggestions">
            {#each filteredSubpathSuggestions as entry, index (entry.path)}
              <button
                class="subpath-suggestion"
                class:selected={index === subpathSelectedIndex}
                onmousedown={(e) => {
                  e.preventDefault();
                  selectSubpathSuggestion(entry);
                }}
              >
                <Folder size={14} class="suggestion-icon" />
                <span class="suggestion-name">{entry.name}</span>
              </button>
            {/each}
          </div>
        {/if}
        <span class="help-text">
          For monorepos: subdirectory to use as working directory for AI sessions
        </span>
      </div>

      <div class="repo-info">
        <span class="repo-label">Repository:</span>
        <span class="repo-path">{project.repoPath}</span>
      </div>

      {#if error}
        <div class="error-message">{error}</div>
      {/if}
    </div>

    <div class="modal-footer">
      <button class="cancel-button" onclick={onClose} disabled={saving}> Cancel </button>
      <button class="save-button" onclick={handleSave} disabled={!isValid || saving}>
        {saving ? 'Saving...' : 'Save'}
      </button>
    </div>
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
    background-color: var(--bg-primary);
    border-radius: 12px;
    width: 100%;
    max-width: 480px;
    max-height: 90vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
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

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow-y: auto;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: var(--size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .form-group input {
    padding: 10px 12px;
    background-color: var(--bg-deepest);
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--size-md);
    transition: border-color 0.15s ease;
  }

  .form-group input:focus {
    outline: none;
    border-color: var(--ui-accent);
  }

  .form-group input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .form-group input::placeholder {
    color: var(--text-faint);
  }

  /* Subpath suggestions dropdown */
  .subpath-suggestions {
    max-height: 160px;
    overflow-y: auto;
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    padding: 4px;
    background-color: var(--bg-primary);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .subpath-suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    background: transparent;
    border: none;
    border-radius: 4px;
    text-align: left;
    cursor: pointer;
    font-size: var(--size-sm);
    color: var(--text-primary);
    transition: background-color 0.1s;
  }

  .subpath-suggestion:hover,
  .subpath-suggestion.selected {
    background-color: var(--bg-hover);
  }

  :global(.suggestion-icon) {
    color: var(--text-faint);
    flex-shrink: 0;
  }

  .suggestion-name {
    font-family: 'SF Mono', 'Menlo', monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .help-text {
    font-size: var(--size-xs);
    color: var(--text-muted);
  }

  .repo-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    background-color: var(--bg-deepest);
    border-radius: 6px;
  }

  .repo-label {
    font-size: var(--size-xs);
    color: var(--text-muted);
  }

  .repo-path {
    font-size: var(--size-sm);
    color: var(--text-primary);
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    word-break: break-all;
  }

  .error-message {
    padding: 10px 12px;
    background-color: var(--ui-danger-bg);
    border-radius: 6px;
    color: var(--ui-danger);
    font-size: var(--size-sm);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
  }

  .cancel-button,
  .save-button {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: var(--size-md);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cancel-button {
    background: transparent;
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
  }

  .cancel-button:hover:not(:disabled) {
    border-color: var(--text-muted);
    color: var(--text-primary);
  }

  .save-button {
    background-color: var(--ui-accent);
    border: none;
    color: var(--bg-deepest);
  }

  .save-button:hover:not(:disabled) {
    background-color: var(--ui-accent-hover);
  }

  .save-button:disabled,
  .cancel-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
