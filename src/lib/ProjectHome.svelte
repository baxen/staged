<!--
  ProjectHome.svelte - The artifact-centric homepage

  Displays projects as tabs and artifacts in a grid layout.
  This is the main surface for the artifact-centric workspace model.
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Plus, Sparkles, FolderKanban, Trash2 } from 'lucide-svelte';
  import type { Project, Artifact } from './types';
  import * as projectService from './services/project';
  import ArtifactCard from './ArtifactCard.svelte';
  import NewArtifactCard from './NewArtifactCard.svelte';
  import ArtifactDetail from './ArtifactDetail.svelte';
  import NewArtifactModal from './NewArtifactModal.svelte';

  // Props for external control
  interface Props {
    /** Called when user wants to close this project tab */
    onCloseProject?: () => void;
  }

  let { onCloseProject }: Props = $props();

  // State
  let projects = $state<Project[]>([]);
  let selectedProjectId = $state<string | null>(null);
  let artifacts = $state<Artifact[]>([]);
  let selectedArtifactIds = $state<Set<string>>(new Set());
  let detailArtifactId = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Modal state
  let showNewArtifactModal = $state(false);

  // Derived
  let selectedProject = $derived(projects.find((p) => p.id === selectedProjectId) ?? null);
  let detailArtifact = $derived(artifacts.find((a) => a.id === detailArtifactId) ?? null);
  let selectedProjectIndex = $derived(projects.findIndex((p) => p.id === selectedProjectId));

  // Load projects on mount
  onMount(async () => {
    await loadProjects();
  });

  async function loadProjects() {
    loading = true;
    error = null;
    try {
      projects = await projectService.listProjects();

      // Select first project by default if we have any
      if (projects.length > 0 && !selectedProjectId) {
        selectedProjectId = projects[0].id;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  // Load artifacts when project changes
  $effect(() => {
    if (selectedProjectId) {
      loadArtifacts(selectedProjectId);
    } else {
      artifacts = [];
    }
  });

  async function loadArtifacts(projectId: string) {
    try {
      artifacts = await projectService.listArtifacts(projectId);
    } catch (e) {
      console.error('Failed to load artifacts:', e);
      artifacts = [];
    }
  }

  function handleSelectProject(projectId: string) {
    selectedProjectId = projectId;
    selectedArtifactIds = new Set();
    detailArtifactId = null;
  }

  function handleSelectArtifact(artifactId: string) {
    // Toggle selection in the set
    const newSet = new Set(selectedArtifactIds);
    if (newSet.has(artifactId)) {
      newSet.delete(artifactId);
    } else {
      newSet.add(artifactId);
    }
    selectedArtifactIds = newSet;
  }

  function handleOpenArtifactDetail(artifactId: string) {
    detailArtifactId = detailArtifactId === artifactId ? null : artifactId;
  }

  async function handleNewProject() {
    const name = prompt('Project name:');
    if (!name) return;

    try {
      const project = await projectService.createProject(name);
      projects = [...projects, project];
      selectedProjectId = project.id;
    } catch (e) {
      console.error('Failed to create project:', e);
    }
  }

  async function handleDeleteProject(projectId: string) {
    const project = projects.find((p) => p.id === projectId);
    if (!project) return;

    if (!confirm(`Delete project "${project.name}" and all its artifacts?`)) return;

    try {
      await projectService.deleteProject(projectId);
      projects = projects.filter((p) => p.id !== projectId);

      // Select another project if we deleted the current one
      if (selectedProjectId === projectId) {
        selectedProjectId = projects.length > 0 ? projects[0].id : null;
      }
    } catch (e) {
      console.error('Failed to delete project:', e);
    }
  }

  function handleNewArtifact() {
    if (!selectedProjectId) return;
    showNewArtifactModal = true;
  }

  function handleArtifactCreated(artifact: Artifact) {
    artifacts = [artifact, ...artifacts];
    selectedArtifactIds = new Set([artifact.id]);
    detailArtifactId = artifact.id;
  }

  async function handleDeleteArtifact(artifactId: string) {
    try {
      await projectService.deleteArtifact(artifactId);
      artifacts = artifacts.filter((a) => a.id !== artifactId);
      // Remove from selection set
      if (selectedArtifactIds.has(artifactId)) {
        const newSet = new Set(selectedArtifactIds);
        newSet.delete(artifactId);
        selectedArtifactIds = newSet;
      }
      // Close detail if showing this artifact
      if (detailArtifactId === artifactId) {
        detailArtifactId = null;
      }
    } catch (e) {
      console.error('Failed to delete artifact:', e);
    }
  }

  // Keyboard shortcuts
  function handleKeydown(e: KeyboardEvent) {
    // Skip if in input/textarea
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') {
      return;
    }

    const isMeta = e.metaKey || e.ctrlKey;

    // Cmd+W - Close current project tab
    if (isMeta && e.key === 'w') {
      e.preventDefault();
      if (onCloseProject) {
        onCloseProject();
      }
      return;
    }

    // Cmd+T - New project
    if (isMeta && e.key === 't') {
      e.preventDefault();
      handleNewProject();
      return;
    }

    // Cmd+N - New artifact (when project is selected)
    if (isMeta && e.key === 'n') {
      e.preventDefault();
      handleNewArtifact();
      return;
    }

    // Ctrl+Tab / Ctrl+Shift+Tab - Switch project tabs
    if (e.ctrlKey && e.key === 'Tab') {
      e.preventDefault();
      if (projects.length <= 1) return;

      if (e.shiftKey) {
        // Previous tab
        const newIndex = selectedProjectIndex <= 0 ? projects.length - 1 : selectedProjectIndex - 1;
        selectedProjectId = projects[newIndex].id;
      } else {
        // Next tab
        const newIndex = selectedProjectIndex >= projects.length - 1 ? 0 : selectedProjectIndex + 1;
        selectedProjectId = projects[newIndex].id;
      }
      return;
    }

    // Escape - Close detail modal
    if (e.key === 'Escape') {
      if (detailArtifactId) {
        e.preventDefault();
        detailArtifactId = null;
      }
      return;
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="project-home">
  <!-- Project tabs -->
  <div class="project-tabs">
    <div class="tabs-list">
      {#each projects as project, index (project.id)}
        <button
          class="tab"
          class:active={project.id === selectedProjectId}
          onclick={() => handleSelectProject(project.id)}
          title={project.name}
        >
          <FolderKanban size={14} />
          <span class="tab-name">{project.name}</span>
          {#if project.id === selectedProjectId}
            <div class="tab-indicator"></div>
          {/if}
        </button>
        {#if projects.length > 1 && project.id === selectedProjectId}
          <div
            class="tab-close"
            onclick={(e) => {
              e.stopPropagation();
              handleDeleteProject(project.id);
            }}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                handleDeleteProject(project.id);
              }
            }}
            role="button"
            tabindex="0"
            title="Delete project"
          >
            <Trash2 size={12} />
          </div>
        {/if}
      {/each}
      <button class="tab new-tab" onclick={handleNewProject} title="New project (⌘T)">
        <Plus size={14} />
      </button>
    </div>
  </div>

  <!-- Main content area -->
  <div class="content">
    {#if loading}
      <div class="loading-state">
        <p>Loading...</p>
      </div>
    {:else if error}
      <div class="error-state">
        <p>{error}</p>
      </div>
    {:else if selectedProject}
      <div class="project-content">
        <!-- Artifacts grid -->
        <div class="artifacts-section">
          <div class="section-header">
            <h2>Artifacts</h2>
            <span class="count">{artifacts.length}</span>
          </div>

          <div class="artifacts-grid">
            <!-- New artifact card -->
            <NewArtifactCard onNewArtifact={handleNewArtifact} />

            <!-- Existing artifacts -->
            {#each artifacts as artifact (artifact.id)}
              <ArtifactCard
                {artifact}
                selected={selectedArtifactIds.has(artifact.id)}
                onSelect={() => handleSelectArtifact(artifact.id)}
                onOpenDetail={() => handleOpenArtifactDetail(artifact.id)}
                onDelete={() => handleDeleteArtifact(artifact.id)}
              />
            {/each}
          </div>
        </div>
      </div>
    {:else}
      <div class="empty-state">
        <Sparkles size={48} strokeWidth={1} />
        <h2>Welcome to Staged</h2>
        <p>Create your first project to get started</p>
        <button class="create-button" onclick={handleNewProject}>
          <Plus size={16} />
          New Project
        </button>
      </div>
    {/if}
  </div>
</div>

<!-- Detail modal -->
{#if detailArtifact}
  <div
    class="modal-backdrop"
    onclick={() => (detailArtifactId = null)}
    onkeydown={(e) => e.key === 'Escape' && (detailArtifactId = null)}
    role="button"
    tabindex="-1"
  >
    <div
      class="modal-content"
      role="dialog"
      tabindex="-1"
      onkeydown={() => {}}
      onclick={(e) => e.stopPropagation()}
    >
      <ArtifactDetail artifact={detailArtifact} onClose={() => (detailArtifactId = null)} />
    </div>
  </div>
{/if}

<!-- New artifact modal -->
{#if showNewArtifactModal && selectedProjectId}
  <NewArtifactModal
    projectId={selectedProjectId}
    availableArtifacts={artifacts}
    onCreated={handleArtifactCreated}
    onClose={() => (showNewArtifactModal = false)}
  />
{/if}

<style>
  .project-home {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-chrome);
  }

  /* Project tabs - matches TabBar styling */
  .project-tabs {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px 0 8px;
    background: var(--bg-deepest);
    flex-shrink: 0;
  }

  .tabs-list {
    position: relative;
    display: flex;
    gap: 2px;
    overflow-x: auto;
    overflow-y: visible;
    scrollbar-width: none;
    /* Padding to accommodate curved corners */
    padding: 0 12px;
    margin: 0 -12px;
  }

  .tabs-list::-webkit-scrollbar {
    display: none;
  }

  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: 6px 6px 0 0;
    color: var(--text-muted);
    font-size: var(--size-sm);
    cursor: pointer;
    transition: color 0.1s;
    white-space: nowrap;
    min-width: 120px;
    max-width: 200px;
    z-index: 1;
  }

  /* Tab indicator with curved corners */
  .tab-indicator {
    position: absolute;
    inset: 0;
    background: var(--bg-chrome);
    border-radius: 6px 6px 0 0;
    z-index: -1;
  }

  .tab-indicator::before,
  .tab-indicator::after {
    content: '';
    position: absolute;
    bottom: 0;
    width: 12px;
    height: 12px;
    background: var(--bg-deepest);
  }

  .tab-indicator::before {
    left: -12px;
    border-bottom-right-radius: 8px;
    box-shadow: 6px 0 0 0 var(--bg-chrome);
  }

  .tab-indicator::after {
    right: -12px;
    border-bottom-left-radius: 8px;
    box-shadow: -6px 0 0 0 var(--bg-chrome);
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--text-primary);
  }

  /* Vertical separators between tabs (not before first, not after last regular tab) */
  .tab:not(.new-tab) + .tab:not(.new-tab)::before {
    content: '';
    position: absolute;
    left: -1px;
    top: 50%;
    transform: translateY(-50%);
    width: 1px;
    height: 16px;
    background: var(--border-subtle);
  }

  /* Hide separator when either adjacent tab is active */
  .tab.active + .tab:not(.new-tab)::before,
  .tab:not(.new-tab):has(+ .tab.active)::before {
    display: none;
  }

  .tab-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: left;
  }

  .tab-close {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    margin-left: -8px;
    margin-right: 4px;
    margin-bottom: 3px;
    background: var(--bg-hover);
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.1s;
  }

  .tab-close:hover {
    background: var(--ui-danger-bg);
    color: var(--ui-danger);
  }

  .tab.new-tab {
    padding: 6px;
    margin-left: 4px;
    margin-bottom: 3px;
    border-radius: 6px;
    color: var(--text-faint);
    min-width: unset;
    max-width: unset;
  }

  .tab.new-tab:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Content area */
  .content {
    flex: 1;
    overflow: auto;
    padding: 24px;
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-content {
    display: flex;
    flex-direction: column;
    width: 90%;
    max-width: 900px;
    max-height: 80vh;
    background-color: var(--bg-primary);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  }

  .loading-state,
  .error-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  .error-state {
    color: var(--ui-danger);
  }

  /* Project content */
  .project-content {
    max-width: 1200px;
    margin: 0 auto;
  }

  .artifacts-section {
    margin-bottom: 32px;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
  }

  .section-header h2 {
    font-size: var(--size-lg);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
  }

  .count {
    font-size: var(--size-sm);
    color: var(--text-faint);
  }

  /* Artifacts grid */
  .artifacts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }

  /* Empty state */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    color: var(--text-muted);
  }

  .empty-state h2 {
    font-size: var(--size-xl);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
  }

  .empty-state p {
    margin: 0;
    color: var(--text-muted);
  }

  .create-button {
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
    transition: background-color 0.15s ease;
  }

  .create-button:hover {
    background-color: var(--ui-accent-hover);
  }
</style>
