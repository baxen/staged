<!--
  ArtifactsPanel.svelte - Displays artifacts from agent sessions

  Shows plans and implementations created by agents.
  Clicking an artifact updates the main view.
-->
<script lang="ts">
  import { FileCode, GitCommit, Calendar, User, FileEdit, MessagesSquare, Loader2, Plus } from 'lucide-svelte';
  import { artifactsState, selectedArtifactId, selectArtifact, hasUncommittedChanges, getArtifact } from './stores/artifacts.svelte';
  import type { Artifact } from './stores/artifacts.svelte';
  import { diffState, selectFile } from './stores/diffState.svelte';
  import { diffSelection } from './stores/diffSelection.svelte';
  import { setChatPanelVisible } from './stores/preferences.svelte';
  import { agentState } from './stores/agent.svelte';
  import { getReferenceFile } from './stores/referenceFiles.svelte';
  import Sidebar from './Sidebar.svelte';

  interface Props {
    onStartNewSession?: () => void;
    onAddReferenceFile?: () => void;
    onRemoveReferenceFile?: (path: string) => void;
    implementationFiles?: any[];
    implementationLoading?: boolean;
    selectedImplementationFile?: string | null;
    onImplementationFileSelect?: (path: string) => void;
  }

  let {
    onStartNewSession,
    onAddReferenceFile,
    onRemoveReferenceFile,
    implementationFiles = [],
    implementationLoading = false,
    selectedImplementationFile = null,
    onImplementationFileSelect
  }: Props = $props();

  // Determine which files to show based on selected artifact
  let currentArtifact = $derived.by(() => {
    if (!selectedArtifactId.value || selectedArtifactId.value === 'uncommitted') return null;
    return getArtifact(selectedArtifactId.value);
  });

  let showFiles = $derived(
    diffState.files.length > 0 ||
    (currentArtifact?.type === 'implementation' && implementationFiles.length > 0)
  );

  let isImplementationView = $derived(currentArtifact?.type === 'implementation');
  let filesToShow = $derived(isImplementationView ? implementationFiles : diffState.files);
  let selectedFileToShow = $derived(isImplementationView ? selectedImplementationFile : diffState.selectedFile);
  let filesLoading = $derived(isImplementationView ? implementationLoading : diffState.loading);
  let isWorkingTreeView = $derived(!isImplementationView && diffSelection.spec.head.type === 'WorkingTree');

  async function handleArtifactClick(id: string, artifact?: Artifact) {
    // Toggle selection if clicking the same artifact
    if (selectedArtifactId.value === id) {
      selectArtifact(null);
      return;
    }

    selectArtifact(id);

    // If it's a session, show the chat panel
    if (artifact?.type === 'session') {
      setChatPanelVisible(true);
    }

    // When selecting uncommitted, ensure we select the first file if available
    if (id === 'uncommitted' && diffState.files.length > 0 && !diffState.selectedFile) {
      const firstPath = diffState.files[0].after ?? diffState.files[0].before;
      if (firstPath) {
        await selectFile(firstPath);
      }
    }
  }

  function getStatusLabel(status: string): string {
    switch (status) {
      case 'planning': return 'Planning...';
      case 'refining': return 'Refining...';
      case 'implementing': return 'Implementing...';
      default: return 'Active';
    }
  }

  function formatDate(timestamp: number): string {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString();
  }

  function getAgentDisplayName(agent: string): string {
    if (agent === 'goose') return 'Goose';
    if (agent === 'claude-code') return 'Claude Code';
    return agent;
  }
</script>

<div class="artifacts-panel">
  <div class="artifacts-header">
    <h2 class="artifacts-title">Session Artifacts</h2>
    <span class="artifacts-count">{artifactsState.artifacts.length}</span>
  </div>

  <div class="artifacts-list">
    {#if artifactsState.artifacts.length === 0 && !hasUncommittedChanges(diffState.files.length, diffSelection.spec.head.type === 'WorkingTree')}
      <div class="empty-state">
        <div class="empty-icon">
          <FileCode size={32} />
        </div>
        <p class="empty-text">No artifacts yet</p>
        <p class="empty-subtext">Plans and implementations will appear here</p>
      </div>
    {:else}
      {#each artifactsState.artifacts as artifact (artifact.id)}
        <button
          class="artifact-item"
          class:selected={selectedArtifactId.value === artifact.id}
          class:session={artifact.type === 'session'}
          onclick={() => handleArtifactClick(artifact.id, artifact)}
        >
          <div class="artifact-icon">
            {#if artifact.type === 'plan'}
              <FileCode size={16} />
            {:else if artifact.type === 'session'}
              <MessagesSquare size={16} />
            {:else}
              <GitCommit size={16} />
            {/if}
          </div>
          <div class="artifact-info">
            <div class="artifact-title-row">
              <span class="artifact-title">{artifact.title}</span>
              {#if artifact.type === 'session'}
                <span class="artifact-status status-{artifact.status}">
                  {#if artifact.status !== 'active'}
                    <Loader2 size={10} class="spinning" />
                  {/if}
                  {getStatusLabel(artifact.status)}
                </span>
              {:else}
                <span class="artifact-type">{artifact.type}</span>
              {/if}
            </div>
            <div class="artifact-meta">
              <span class="artifact-agent">
                <User size={12} />
                {getAgentDisplayName(artifact.agent)}
              </span>
              <span class="artifact-date">
                <Calendar size={12} />
                {formatDate(artifact.createdAt)}
              </span>
            </div>
          </div>
        </button>
      {/each}

      <!-- Separator before uncommitted changes -->
      {#if hasUncommittedChanges(diffState.files.length, diffSelection.spec.head.type === 'WorkingTree') && artifactsState.artifacts.length > 0}
        <div class="artifacts-separator"></div>
      {/if}

      <!-- Uncommitted changes (if any) - always at bottom -->
      {#if hasUncommittedChanges(diffState.files.length, diffSelection.spec.head.type === 'WorkingTree')}
        <button
          class="artifact-item uncommitted"
          class:selected={selectedArtifactId.value === 'uncommitted'}
          onclick={() => handleArtifactClick('uncommitted')}
        >
          <div class="artifact-icon">
            <FileEdit size={16} />
          </div>
          <div class="artifact-info">
            <div class="artifact-title-row">
              <span class="artifact-title">Uncommitted changes</span>
            </div>
            <div class="artifact-meta">
              <span class="artifact-agent">
                {diffState.files.length} {diffState.files.length === 1 ? 'file' : 'files'}
              </span>
            </div>
          </div>
        </button>
      {/if}

      <!-- File list separator -->
      {#if showFiles}
        <div class="files-separator">
          <span class="files-separator-text">Files</span>
        </div>
      {/if}
    {/if}

    <!-- File list sidebar -->
    {#if showFiles}
      <div class="files-section">
        <Sidebar
          files={filesToShow}
          loading={filesLoading}
          onFileSelect={isImplementationView && onImplementationFileSelect ? onImplementationFileSelect : selectFile}
          selectedFile={selectedFileToShow}
          isWorkingTree={isWorkingTreeView}
          onAddReferenceFile={onAddReferenceFile ?? (() => {})}
          onRemoveReferenceFile={onRemoveReferenceFile ?? (() => {})}
        />
      </div>
    {/if}

    <!-- Start new session button -->
    <div class="new-session-section">
      <button class="new-session-btn" onclick={onStartNewSession}>
        <Plus size={16} />
        <span>Start New Session</span>
      </button>
    </div>
  </div>
</div>

<style>
  .artifacts-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-bg);
    color: var(--color-fg);
    font-size: var(--size-base);
    overflow: hidden;
  }

  .artifacts-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: calc(var(--size-base) * 0.8) calc(var(--size-base) * 1.2);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .artifacts-title {
    font-size: calc(var(--size-base) * 0.9);
    font-weight: 600;
    margin: 0;
    color: var(--color-fg);
  }

  .artifacts-count {
    font-size: calc(var(--size-base) * 0.85);
    color: var(--color-fg-muted);
    background: var(--color-bg-alt);
    padding: calc(var(--size-base) * 0.2) calc(var(--size-base) * 0.5);
    border-radius: calc(var(--size-base) * 0.3);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 200px;
    padding: calc(var(--size-base) * 2);
    text-align: center;
  }

  .empty-icon {
    color: var(--color-fg-muted);
    margin-bottom: calc(var(--size-base) * 1);
    opacity: 0.5;
  }

  .empty-text {
    font-size: calc(var(--size-base) * 1.1);
    font-weight: 500;
    color: var(--color-fg);
    margin: 0 0 calc(var(--size-base) * 0.5);
  }

  .empty-subtext {
    font-size: calc(var(--size-base) * 0.9);
    color: var(--color-fg-muted);
    margin: 0;
  }

  .artifacts-list {
    flex: 1;
    overflow-y: auto;
    padding: calc(var(--size-base) * 0.8);
  }

  .artifact-item {
    display: flex;
    align-items: flex-start;
    gap: calc(var(--size-base) * 0.8);
    width: 100%;
    padding: calc(var(--size-base) * 0.8);
    background: var(--color-bg-alt);
    border: 1px solid var(--color-border);
    border-radius: calc(var(--size-base) * 0.5);
    margin-bottom: calc(var(--size-base) * 0.8);
    color: inherit;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .artifact-item:hover {
    background: var(--color-bg-hover);
    border-color: var(--color-border-hover);
  }

  .artifact-item.selected {
    background: var(--color-bg);
    border-color: var(--color-accent);
    box-shadow: 0 0 0 1px var(--color-accent);
  }

  .artifact-item.uncommitted {
    border-style: dashed;
  }

  .artifact-icon {
    flex-shrink: 0;
    width: calc(var(--size-base) * 2);
    height: calc(var(--size-base) * 2);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg);
    border-radius: calc(var(--size-base) * 0.3);
    color: var(--color-accent);
  }

  .artifact-info {
    flex: 1;
    min-width: 0;
  }

  .artifact-title-row {
    display: flex;
    align-items: center;
    gap: calc(var(--size-base) * 0.6);
    margin-bottom: calc(var(--size-base) * 0.4);
  }

  .artifact-title {
    font-size: calc(var(--size-base) * 0.95);
    font-weight: 500;
    color: var(--color-fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .artifact-type {
    font-size: calc(var(--size-base) * 0.75);
    color: var(--color-fg-muted);
    background: var(--color-bg);
    padding: calc(var(--size-base) * 0.15) calc(var(--size-base) * 0.4);
    border-radius: calc(var(--size-base) * 0.25);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

  .artifact-meta {
    display: flex;
    align-items: center;
    gap: calc(var(--size-base) * 1);
    font-size: calc(var(--size-base) * 0.85);
    color: var(--color-fg-muted);
  }

  .artifact-agent,
  .artifact-date {
    display: flex;
    align-items: center;
    gap: calc(var(--size-base) * 0.3);
  }

  .artifact-item.session {
    background: var(--color-bg);
    border-left: 3px solid var(--color-accent);
  }

  .artifact-status {
    display: flex;
    align-items: center;
    gap: calc(var(--size-base) * 0.3);
    font-size: calc(var(--size-base) * 0.75);
    color: var(--color-accent);
    background: var(--color-bg-alt);
    padding: calc(var(--size-base) * 0.15) calc(var(--size-base) * 0.4);
    border-radius: calc(var(--size-base) * 0.25);
    text-transform: capitalize;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

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

  .artifacts-separator {
    height: 1px;
    background: var(--color-border);
    margin: calc(var(--size-base) * 0.8) calc(var(--size-base) * 0.8);
  }

  .files-separator {
    display: flex;
    align-items: center;
    margin: calc(var(--size-base) * 1.2) calc(var(--size-base) * 0.8) calc(var(--size-base) * 0.8);
    position: relative;
  }

  .files-separator::before {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--color-border);
    margin-right: calc(var(--size-base) * 0.8);
  }

  .files-separator::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--color-border);
    margin-left: calc(var(--size-base) * 0.8);
  }

  .files-separator-text {
    font-size: calc(var(--size-base) * 0.75);
    font-weight: 600;
    color: var(--color-fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .files-section {
    /* Remove extra margins - let the Sidebar's internal padding handle spacing */
  }

  /* Override Sidebar's default full-height scroll container behavior */
  .files-section :global(.sidebar-content) {
    height: auto !important;
    overflow: visible !important;
  }

  .files-section :global(.file-list) {
    overflow-y: visible !important;
    overflow-x: visible !important;
    flex: none !important;
    /* Remove internal padding since we want items to align with artifacts */
    padding: 0 !important;
  }

  /* Adjust section header spacing to match artifacts list */
  .files-section :global(.section-header) {
    margin: calc(var(--size-base) * 0.8) calc(var(--size-base) * 0.8) !important;
  }

  /* Adjust tree item spacing to match artifact items */
  .files-section :global(.tree-item) {
    margin: 0 calc(var(--size-base) * 0.8) !important;
  }

  .new-session-section {
    padding: calc(var(--size-base) * 0.8);
    padding-top: calc(var(--size-base) * 1.2);
  }

  .new-session-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: calc(var(--size-base) * 0.6);
    width: 100%;
    padding: calc(var(--size-base) * 0.8);
    background: var(--color-accent);
    color: white;
    border: none;
    border-radius: calc(var(--size-base) * 0.4);
    font-size: calc(var(--size-base) * 0.9);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .new-session-btn:hover {
    background: var(--color-accent-hover, var(--color-accent));
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }

  .new-session-btn:active {
    transform: translateY(0);
  }

</style>
