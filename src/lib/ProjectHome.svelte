<!--
  ProjectHome.svelte - The artifact-centric homepage

  Displays projects as tabs and artifacts in a grid layout.
  This is the main surface for the artifact-centric workspace model.

  Keyboard shortcuts:
  - Cmd+T: New project
  - Cmd+W: Close current project (or window if last project)
  - Cmd+N: New artifact (when project is selected)
  - Ctrl+Tab: Next project tab
  - Ctrl+Shift+Tab: Previous project tab
  - Escape: Close detail modal
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { Plus, Sparkles, FolderKanban, Trash2, X } from 'lucide-svelte';
  import type { Project, Artifact } from './types';
  import * as projectService from './services/project';
  import ArtifactCard from './ArtifactCard.svelte';
  import NewArtifactCard from './NewArtifactCard.svelte';
  import ArtifactDetail from './ArtifactDetail.svelte';
  import NewArtifactModal from './NewArtifactModal.svelte';
  import NewProjectModal from './NewProjectModal.svelte';

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
  let showNewProjectModal = $state(false);

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

  function openNewProjectModal() {
    showNewProjectModal = true;
  }

  async function handleCreateProject(name: string) {
    try {
      const project = await projectService.createProject(name);
      projects = [...projects, project];
      selectedProjectId = project.id;
      showNewProjectModal = false;
    } catch (e) {
      console.error('Failed to create project:', e);
    }
  }

  async function handleDeleteProject(projectId: string) {
    const project = projects.find((p) => p.id === projectId);
    if (!project) return;

    const confirmed = await confirm(`Delete project "${project.name}" and all its artifacts?`);
    if (!confirmed) return;

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

  async function handleCloseProject(projectId: string) {
    // If this is the last project, close the window
    if (projects.length <= 1) {
      const window = getCurrentWindow();
      await window.close();
      return;
    }

    // Switch to another project first if closing the selected one
    const currentIndex = projects.findIndex((p) => p.id === projectId);
    if (selectedProjectId === projectId) {
      const newIndex = currentIndex === 0 ? 1 : currentIndex - 1;
      selectedProjectId = projects[newIndex].id;
    }

    // Remove from the open tabs list (doesn't delete the project)
    projects = projects.filter((p) => p.id !== projectId);
  }

  function handleNewArtifact() {
    if (!selectedProjectId) return;
    showNewArtifactModal = true;
  }

  function handleArtifactCreated(artifact: Artifact) {
    artifacts = [artifact, ...artifacts];
    selectedArtifactIds = new Set([artifact.id]);
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

  // Navigate to next project tab
  function nextProjectTab() {
    if (projects.length <= 1) return;
    const newIndex = selectedProjectIndex >= projects.length - 1 ? 0 : selectedProjectIndex + 1;
    selectedProjectId = projects[newIndex].id;
  }

  // Navigate to previous project tab
  function prevProjectTab() {
    if (projects.length <= 1) return;
    const newIndex = selectedProjectIndex <= 0 ? projects.length - 1 : selectedProjectIndex - 1;
    selectedProjectId = projects[newIndex].id;
  }

  // Keyboard shortcuts
  function handleKeydown(e: KeyboardEvent) {
    // Skip if in input/textarea (unless Escape)
    const target = e.target as HTMLElement;
    const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA';

    if (isInput && e.key !== 'Escape') {
      return;
    }

    const isMeta = e.metaKey || e.ctrlKey;

    // Cmd+W - Close current project tab (or window if last)
    if (e.metaKey && e.key === 'w') {
      e.preventDefault();
      if (selectedProjectId) {
        handleCloseProject(selectedProjectId);
      }
      return;
    }

    // Cmd+T - New project
    if (e.metaKey && e.key === 't') {
      e.preventDefault();
      openNewProjectModal();
      return;
    }

    // Cmd+N - New artifact (when project is selected)
    if (e.metaKey && e.key === 'n') {
      e.preventDefault();
      handleNewArtifact();
      return;
    }

    // Ctrl+Tab / Ctrl+Shift+Tab - Switch project tabs
    if (e.ctrlKey && e.key === 'Tab') {
      e.preventDefault();
      if (e.shiftKey) {
        prevProjectTab();
      } else {
        nextProjectTab();
      }
      return;
    }

    // Cmd+Shift+[ / Cmd+Shift+] - Switch project tabs (alternative)
    if (e.metaKey && e.shiftKey && (e.key === '[' || e.key === '{')) {
      e.preventDefault();
      prevProjectTab();
      return;
    }

    if (e.metaKey && e.shiftKey && (e.key === ']' || e.key === '}')) {
      e.preventDefault();
      nextProjectTab();
      return;
    }

    // Escape - Close detail modal or new artifact modal
    if (e.key === 'Escape') {
      if (showNewArtifactModal) {
        e.preventDefault();
        showNewArtifactModal = false;
      } else if (showNewProjectModal) {
        e.preventDefault();
        showNewProjectModal = false;
      } else if (detailArtifactId) {
        e.preventDefault();
        detailArtifactId = null;
      }
      return;
    }
  }

  // Event listener cleanup
  let unlistenArtifactUpdated: UnlistenFn | null = null;

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);

    // Listen for artifact updates from background generation
    listen<string>('artifact-updated', async (event) => {
      const artifactId = event.payload;
      // Refresh the artifact if it's in our current list
      const existingIndex = artifacts.findIndex((a) => a.id === artifactId);
      if (existingIndex >= 0) {
        const updated = await projectService.getArtifact(artifactId);
        if (updated) {
          artifacts = artifacts.map((a) => (a.id === artifactId ? updated : a));
          // Also update detail view if showing this artifact
          if (detailArtifactId === artifactId) {
            // Force reactivity by reassigning
            detailArtifactId = artifactId;
          }
        }
      }
    }).then((unlisten) => {
      unlistenArtifactUpdated = unlisten;
    });
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    unlistenArtifactUpdated?.();
  });
</script>

<div class="project-home">
  <!-- Project tabs -->
  <div class="project-tabs">
    <div class="tabs-list">
      {#each projects as project, index (project.id)}
        <div
          class="tab"
          class:active={project.id === selectedProjectId}
          onclick={() => handleSelectProject(project.id)}
          onkeydown={(e) => e.key === 'Enter' && handleSelectProject(project.id)}
          role="tab"
          tabindex="0"
          title={project.name}
        >
          <FolderKanban size={14} />
          <span class="tab-name">{project.name}</span>
          <button
            class="tab-close"
            onclick={(e) => {
              e.stopPropagation();
              handleCloseProject(project.id);
            }}
            title="Close tab (⌘W)"
          >
            <X size={12} />
          </button>
          {#if project.id === selectedProjectId}
            <div class="tab-indicator"></div>
          {/if}
        </div>
      {/each}
      <button class="tab new-tab" onclick={openNewProjectModal} title="New project (⌘T)">
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
        <button class="create-button" onclick={openNewProjectModal}>
          <Plus size={16} />
          New Project
        </button>
        <span class="shortcut-hint">or press ⌘T</span>
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
    contextArtifacts={artifacts.filter((a) => selectedArtifactIds.has(a.id))}
    onCreated={handleArtifactCreated}
    onClose={() => (showNewArtifactModal = false)}
    onRemoveContext={(id) => {
      const newSet = new Set(selectedArtifactIds);
      newSet.delete(id);
      selectedArtifactIds = newSet;
    }}
  />
{/if}

<!-- New project modal -->
{#if showNewProjectModal}
  <NewProjectModal onCreated={handleCreateProject} onClose={() => (showNewProjectModal = false)} />
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
  .tab:not(.new-tab) + .tab:not(.new-tab):not(.tab-close)::before {
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
    margin-left: auto;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-faint);
    cursor: pointer;
    transition: all 0.1s;
    opacity: 0;
    z-index: 1;
  }

  .tab:hover .tab-close {
    opacity: 1;
  }

  .tab.active .tab-close {
    opacity: 1;
    color: var(--text-muted);
  }

  .tab-close:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
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

  .shortcut-hint {
    font-size: var(--size-sm);
    color: var(--text-faint);
  }
</style>
