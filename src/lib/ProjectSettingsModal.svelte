<!--
  ProjectSettingsModal.svelte - Manage project settings and actions

  Tabs:
  1. General - Project path and subpath
  2. Actions - Configurable actions for the project
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    X,
    Settings,
    Play,
    Wand,
    CheckCircle,
    Zap,
    Plus,
    Trash2,
    Loader2,
    Save,
    Pencil,
    FlaskConical,
    BrushCleaning,
  } from 'lucide-svelte';
  import type { GitProject, ProjectAction, ActionType, SuggestedAction } from './services/branch';
  import * as branchService from './services/branch';

  interface Props {
    project: GitProject;
    onClose: () => void;
    onUpdated?: (project: GitProject) => void;
  }

  let { project, onClose, onUpdated }: Props = $props();

  // Active tab
  type Tab = 'general' | 'actions';
  let activeTab = $state<Tab>('general');

  // General tab state
  let subpath = $state(project.subpath || '');
  let saving = $state(false);

  // Actions tab state
  let actions = $state<ProjectAction[]>([]);
  let loadingActions = $state(false);
  let detecting = $state(false);
  let editingAction = $state<ProjectAction | null>(null);
  let editForm = $state({
    name: '',
    command: '',
    actionType: 'run' as ActionType,
    autoCommit: false,
  });

  // Load actions when switching to actions tab
  $effect(() => {
    if (activeTab === 'actions' && actions.length === 0 && !loadingActions) {
      loadActions();
    }
  });

  async function loadActions() {
    loadingActions = true;
    try {
      actions = await branchService.listProjectActions(project.id);
    } catch (e) {
      console.error('Failed to load actions:', e);
    } finally {
      loadingActions = false;
    }
  }

  async function detectActions() {
    detecting = true;
    try {
      const suggested = await branchService.detectProjectActions(project.id);

      // Add suggested actions that don't already exist
      const existingCommands = new Set(actions.map((a) => a.command));
      let nextSortOrder = Math.max(...actions.map((a) => a.sortOrder), 0) + 1;

      for (const suggestion of suggested) {
        if (!existingCommands.has(suggestion.command)) {
          const newAction = await branchService.createProjectAction(
            project.id,
            suggestion.name,
            suggestion.command,
            suggestion.actionType,
            nextSortOrder++,
            suggestion.autoCommit
          );
          actions = [...actions, newAction];
        }
      }
    } catch (e) {
      console.error('Failed to detect actions:', e);
    } finally {
      detecting = false;
    }
  }

  function startAddAction() {
    editForm = {
      name: '',
      command: '',
      actionType: 'run',
      autoCommit: false,
    };
    editingAction = {} as ProjectAction; // Empty object signals "adding"
  }

  function startEditAction(action: ProjectAction) {
    editForm = {
      name: action.name,
      command: action.command,
      actionType: action.actionType,
      autoCommit: action.autoCommit,
    };
    editingAction = action;
  }

  function cancelEdit() {
    editingAction = null;
  }

  async function saveAction() {
    if (!editForm.name || !editForm.command) return;

    try {
      if (!editingAction?.id) {
        // Adding new action
        const nextSortOrder = Math.max(...actions.map((a) => a.sortOrder), 0) + 1;
        const newAction = await branchService.createProjectAction(
          project.id,
          editForm.name,
          editForm.command,
          editForm.actionType,
          nextSortOrder,
          editForm.autoCommit
        );
        actions = [...actions, newAction];
      } else {
        // Updating existing action
        await branchService.updateProjectAction(
          editingAction.id,
          editForm.name,
          editForm.command,
          editForm.actionType,
          editingAction.sortOrder,
          editForm.autoCommit
        );
        actions = actions.map((a) =>
          a.id === editingAction.id
            ? {
                ...a,
                name: editForm.name,
                command: editForm.command,
                actionType: editForm.actionType,
                autoCommit: editForm.autoCommit,
              }
            : a
        );
      }
      editingAction = null;
    } catch (e) {
      console.error('Failed to save action:', e);
    }
  }

  async function deleteAction(actionId: string) {
    try {
      await branchService.deleteProjectAction(actionId);
      actions = actions.filter((a) => a.id !== actionId);
    } catch (e) {
      console.error('Failed to delete action:', e);
    }
  }

  async function saveGeneral() {
    saving = true;
    try {
      await branchService.updateGitProject(project.id, subpath || null);
      if (onUpdated) {
        onUpdated({ ...project, subpath: subpath || null });
      }
    } catch (e) {
      console.error('Failed to save project:', e);
    } finally {
      saving = false;
    }
  }

  function getActionIcon(actionType: ActionType) {
    switch (actionType) {
      case 'prerun':
        return Zap;
      case 'format':
        return Wand;
      case 'check':
        return CheckCircle;
      case 'test':
        return FlaskConical;
      case 'cleanUp':
        return BrushCleaning;
      case 'run':
        return Play;
    }
  }

  function getActionTypeColor(actionType: ActionType): string {
    switch (actionType) {
      case 'prerun':
        return 'var(--color-warning)';
      case 'format':
        return 'var(--color-info)';
      case 'check':
        return 'var(--color-success)';
      case 'test':
        return 'var(--status-added)';
      case 'cleanUp':
        return 'var(--text-muted)';
      case 'run':
        return 'var(--color-primary)';
    }
  }

  // Group actions by type
  let groupedActions = $derived.by(() => {
    const groups: Record<ActionType, ProjectAction[]> = {
      prerun: [],
      format: [],
      check: [],
      test: [],
      cleanUp: [],
      run: [],
    };
    for (const action of actions) {
      groups[action.actionType].push(action);
    }
    return groups;
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && !editingAction) {
      onClose();
      event.preventDefault();
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget && !editingAction) {
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
  onkeydown={(e) => e.key === 'Escape' && !editingAction && onClose()}
>
  <div class="modal">
    <header class="modal-header">
      <h2>
        <Settings size={16} />
        Project Settings
      </h2>
      <button class="close-btn" onclick={onClose}>
        <X size={16} />
      </button>
    </header>

    <div class="modal-body">
      <!-- Tabs -->
      <div class="tabs">
        <button
          class="tab"
          class:active={activeTab === 'general'}
          onclick={() => (activeTab = 'general')}
        >
          <Settings size={14} />
          General
        </button>
        <button
          class="tab"
          class:active={activeTab === 'actions'}
          onclick={() => (activeTab = 'actions')}
        >
          <Play size={14} />
          Actions
        </button>
      </div>

      <!-- General Tab -->
      {#if activeTab === 'general'}
        <div class="tab-content">
          <div class="setting-row">
            <div class="setting-info">
              <div class="setting-label">Repository Path</div>
              <div class="setting-value">{project.repoPath}</div>
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <div class="setting-label">Subpath</div>
              <div class="setting-description">
                Optional path within the repository (for monorepos)
              </div>
            </div>
            <input
              type="text"
              class="subpath-input"
              bind:value={subpath}
              placeholder="e.g., packages/frontend"
            />
          </div>

          <div class="button-row">
            <button class="primary-btn" onclick={saveGeneral} disabled={saving}>
              {#if saving}
                <Loader2 size={14} class="spinner" />
              {:else}
                <Save size={14} />
              {/if}
              Save
            </button>
          </div>
        </div>
      {/if}

      <!-- Actions Tab -->
      {#if activeTab === 'actions'}
        <div class="tab-content actions-tab">
          {#if editingAction}
            <!-- Edit Form -->
            <div class="edit-form">
              <h3>{editingAction.id ? 'Edit Action' : 'New Action'}</h3>

              <div class="form-group">
                <label for="action-name">Name</label>
                <input
                  id="action-name"
                  type="text"
                  bind:value={editForm.name}
                  placeholder="e.g., Lint"
                />
              </div>

              <div class="form-group">
                <label for="action-command">Command</label>
                <input
                  id="action-command"
                  type="text"
                  bind:value={editForm.command}
                  placeholder="e.g., npm run lint"
                />
              </div>

              <div class="form-group">
                <label for="action-type">Type</label>
                <select id="action-type" bind:value={editForm.actionType}>
                  <option value="run">Run - Manual execution</option>
                  <option value="format">Format - Auto-fix issues</option>
                  <option value="check">Check - Validation only</option>
                  <option value="test">Test - Run tests</option>
                  <option value="cleanUp">Clean Up - Remove build artifacts</option>
                  <option value="prerun">Prerun - Auto-run on branch creation</option>
                </select>
              </div>

              <div class="form-group checkbox-group">
                <label>
                  <input type="checkbox" bind:checked={editForm.autoCommit} />
                  Auto-commit changes after successful execution
                </label>
              </div>

              <div class="button-row">
                <button class="secondary-btn" onclick={cancelEdit}> Cancel </button>
                <button
                  class="primary-btn"
                  onclick={saveAction}
                  disabled={!editForm.name || !editForm.command}
                >
                  <Save size={14} />
                  Save
                </button>
              </div>
            </div>
          {:else}
            <!-- Actions List -->
            <div class="actions-header">
              <button class="secondary-btn" onclick={detectActions} disabled={detecting}>
                {#if detecting}
                  <Loader2 size={14} class="spinner" />
                {:else}
                  <Zap size={14} />
                {/if}
                Detect Actions
              </button>
              <button class="primary-btn" onclick={startAddAction}>
                <Plus size={14} />
                Add Action
              </button>
            </div>

            {#if loadingActions}
              <div class="loading-state">
                <Loader2 size={24} class="spinner" />
                <span>Loading actions...</span>
              </div>
            {:else if actions.length === 0}
              <div class="empty-state">
                <Play size={32} />
                <p>No actions configured</p>
                <p class="empty-hint">
                  Click "Detect Actions" to find common scripts, or add one manually
                </p>
              </div>
            {:else}
              <div class="actions-list">
                {#each Object.entries(groupedActions) as [type, typeActions]}
                  {#if typeActions.length > 0}
                    <div class="action-group">
                      <div
                        class="group-header"
                        style="color: {getActionTypeColor(type as ActionType)}"
                      >
                        <svelte:component this={getActionIcon(type as ActionType)} size={14} />
                        {type.charAt(0).toUpperCase() + type.slice(1)}
                      </div>
                      {#each typeActions as action (action.id)}
                        <div class="action-item">
                          <div class="action-info">
                            <div class="action-name">{action.name}</div>
                            <code class="action-command">{action.command}</code>
                            {#if action.autoCommit}
                              <div class="action-badge">Commits to git</div>
                            {/if}
                          </div>
                          <div class="action-controls">
                            <button
                              class="icon-btn"
                              onclick={() => startEditAction(action)}
                              title="Edit"
                            >
                              <Pencil size={14} />
                            </button>
                            <button
                              class="icon-btn danger"
                              onclick={() => deleteAction(action.id)}
                              title="Delete"
                            >
                              <Trash2 size={14} />
                            </button>
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                {/each}
              </div>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal {
    background: var(--bg-primary);
    border-radius: 8px;
    width: min(700px, 90vw);
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    border: 1px solid var(--border-color);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .modal-header h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    border-radius: 4px;
    transition: all 0.15s;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 0;
  }

  .tabs {
    display: flex;
    gap: 4px;
    padding: 12px 16px 0;
    border-bottom: 1px solid var(--border-color);
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    border-radius: 6px 6px 0 0;
    transition: all 0.15s;
    position: relative;
  }

  .tab:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--text-primary);
    background: var(--bg-secondary);
  }

  .tab.active::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--color-primary);
  }

  .tab-content {
    padding: 20px;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 16px 0;
  }

  .setting-row + .setting-row {
    border-top: 1px solid var(--border-color);
  }

  .setting-info {
    flex: 1;
  }

  .setting-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .setting-value {
    font-size: 13px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }

  .setting-description {
    font-size: 12px;
    color: var(--text-tertiary);
    margin-top: 4px;
  }

  .subpath-input {
    flex: 1;
    max-width: 300px;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
  }

  .subpath-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .button-row {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 16px;
  }

  .primary-btn,
  .secondary-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s;
    border: 1px solid transparent;
  }

  .primary-btn {
    background: var(--color-primary);
    color: white;
    border: none;
  }

  .primary-btn:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .secondary-btn {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-color: var(--border-color);
  }

  .secondary-btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .secondary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Actions Tab */
  .actions-tab {
    min-height: 400px;
  }

  .actions-header {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-bottom: 16px;
  }

  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: var(--text-secondary);
    text-align: center;
    gap: 12px;
  }

  .empty-state {
    gap: 12px;
  }

  .empty-state p {
    margin: 0;
  }

  .empty-hint {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .actions-list {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .action-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
  }

  .action-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    transition: all 0.15s;
  }

  .action-item:hover {
    background: var(--bg-hover);
  }

  .action-info {
    flex: 1;
    min-width: 0;
  }

  .action-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .action-command {
    display: block;
    font-size: 12px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    background: var(--bg-hover);
    padding: 4px 8px;
    border-radius: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 4px;
  }

  .action-badge {
    display: inline-block;
    font-size: 10px;
    padding: 2px 6px;
    background: var(--color-info);
    color: white;
    border-radius: 3px;
    margin-top: 4px;
  }

  .action-controls {
    display: flex;
    gap: 4px;
  }

  .icon-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 6px;
    display: flex;
    align-items: center;
    border-radius: 4px;
    transition: all 0.15s;
  }

  .icon-btn:hover {
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .icon-btn.danger:hover {
    background: var(--color-error);
    color: white;
  }

  /* Edit Form */
  .edit-form {
    max-width: 500px;
    margin: 0 auto;
  }

  .edit-form h3 {
    margin: 0 0 20px;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 6px;
  }

  .form-group input[type='text'],
  .form-group select {
    width: 100%;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
  }

  .form-group input[type='text']:focus,
  .form-group select:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .checkbox-group input[type='checkbox'] {
    cursor: pointer;
  }

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
