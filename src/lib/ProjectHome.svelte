<!--
  ProjectHome.svelte - The new artifact-centric homepage

  Displays projects as tabs and artifacts in a grid layout.
  This is the main surface for the artifact-centric workspace model.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    FileText,
    GitCommit,
    Plus,
    MoreHorizontal,
    Clock,
    Sparkles,
    FolderKanban,
  } from 'lucide-svelte';
  import type { Project, Artifact, ArtifactData } from './types';
  import * as projectService from './services/project';
  import ArtifactCard from './ArtifactCard.svelte';
  import NewArtifactCard from './NewArtifactCard.svelte';
  import ArtifactDetail from './ArtifactDetail.svelte';

  // State
  let projects = $state<Project[]>([]);
  let selectedProjectId = $state<string | null>(null);
  let artifacts = $state<Artifact[]>([]);
  let selectedArtifactIds = $state<Set<string>>(new Set());
  let detailArtifactId = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Derived
  let selectedProject = $derived(projects.find((p) => p.id === selectedProjectId) ?? null);
  let detailArtifact = $derived(artifacts.find((a) => a.id === detailArtifactId) ?? null);

  // Load projects on mount
  onMount(async () => {
    await loadProjects();
  });

  async function loadProjects() {
    loading = true;
    error = null;
    try {
      projects = await projectService.listProjects();

      // If no projects, create a demo one with fake artifacts
      if (projects.length === 0) {
        await createDemoData();
        projects = await projectService.listProjects();
      }

      // Select first project by default
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

  async function createDemoData() {
    // Create a demo project
    const project = await projectService.createProject('Improve Git Performance');

    // Create some demo artifacts
    await projectService.createArtifact(project.id, 'Git Integration Research', {
      type: 'markdown',
      content: `# Git Integration Analysis

## Current State
The application currently uses direct git command execution for all operations.

## Bottlenecks Identified
1. **Large repository cloning** - No shallow clone support
2. **Status checks** - Full tree walk on every status
3. **Diff generation** - Loading entire files into memory

## Recommendations
- Implement shallow clone for initial setup
- Cache git status with file system watchers
- Stream diff generation for large files

## Next Steps
Create a detailed implementation plan for the caching layer.`,
    });

    await projectService.createArtifact(project.id, 'Caching Layer Plan', {
      type: 'markdown',
      content: `# Caching Layer Implementation Plan

## Overview
Add a caching layer to reduce redundant git operations.

## Components

### 1. Status Cache
- Watch filesystem for changes
- Invalidate on file modifications
- TTL of 5 seconds for background refresh

### 2. Diff Cache
- Key by file path + before/after refs
- LRU eviction with 100MB limit
- Persist across sessions

## Implementation Order
1. Status cache (highest impact)
2. Diff cache (memory optimization)
3. Ref cache (minor improvement)`,
    });

    await projectService.createArtifact(project.id, 'Quick Notes', {
      type: 'markdown',
      content: `# Quick Notes

- Remember to check libgit2 bindings as alternative
- Look into git sparse-checkout for monorepos
- Consider WebWorker for heavy operations`,
    });

    await projectService.createArtifact(project.id, 'Architecture Deep Dive', {
      type: 'markdown',
      content: `# Architecture Deep Dive

## System Overview

This document provides a comprehensive analysis of the current system architecture, identifying key components, data flows, and potential areas for improvement.

## Core Components

### 1. Repository Manager

The Repository Manager is responsible for all git operations. It maintains a connection pool to handle concurrent operations efficiently.

**Key Responsibilities:**
- Repository discovery and initialization
- Branch management and switching
- Commit history traversal
- Diff computation

**Current Implementation:**
\`\`\`rust
pub struct RepoManager {
    repos: HashMap<PathBuf, Repository>,
    cache: LruCache<CacheKey, CachedValue>,
    watcher: FileWatcher,
}

impl RepoManager {
    pub fn get_diff(&self, spec: &DiffSpec) -> Result<Diff> {
        // Check cache first
        if let Some(cached) = self.cache.get(&spec.cache_key()) {
            return Ok(cached.clone());
        }
        
        // Compute diff
        let diff = self.compute_diff(spec)?;
        self.cache.insert(spec.cache_key(), diff.clone());
        Ok(diff)
    }
}
\`\`\`

### 2. File Watcher

The file watcher monitors the filesystem for changes and invalidates relevant caches.

**Events Handled:**
- File creation
- File modification  
- File deletion
- Directory changes

**Debouncing Strategy:**
We use a 100ms debounce window to batch rapid changes (common during saves or git operations).

### 3. UI Layer

The UI is built with Svelte 5, using runes for reactive state management.

**Component Hierarchy:**
- App.svelte (root)
  - TabBar (tab management)
  - TopBar (actions, navigation)
  - Sidebar (file list)
  - DiffViewer (main content)

## Data Flow

### Diff Request Flow

1. User selects a diff spec (e.g., "main..HEAD")
2. UI dispatches request via Tauri invoke
3. Backend resolves refs to commits
4. Diff is computed using libgit2
5. Result is cached and returned
6. UI renders the diff with syntax highlighting

### Cache Invalidation Flow

1. File watcher detects change
2. Change is debounced (100ms window)
3. Affected cache entries are identified
4. Entries are invalidated
5. UI is notified to refresh if needed

## Performance Considerations

### Memory Usage

Current memory profile for a medium-sized repository (10k files):
- Base memory: ~50MB
- Per-open-diff: ~5MB
- Syntax highlighting cache: ~20MB
- Total typical usage: ~100MB

### Latency Targets

| Operation | Target | Current |
|-----------|--------|---------|
| Initial load | <500ms | 450ms |
| File select | <50ms | 35ms |
| Diff compute | <200ms | 180ms |
| Syntax highlight | <100ms | 85ms |

## Future Improvements

### Short Term (1-2 weeks)
- Implement streaming diff for large files
- Add progress indicators for slow operations
- Optimize syntax highlighting for very long files

### Medium Term (1-2 months)
- WebWorker for CPU-intensive operations
- Incremental diff updates
- Better memory management for large repos

### Long Term (3-6 months)
- Plugin system for custom diff renderers
- Collaborative features (shared reviews)
- Integration with CI/CD systems

## Conclusion

The current architecture provides a solid foundation but has room for optimization, particularly in memory usage and handling of large repositories. The proposed improvements should address these concerns while maintaining the simplicity and reliability of the current design.`,
    });
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

  async function handleNewArtifact() {
    if (!selectedProjectId) return;

    // For now, just create a placeholder artifact
    // In the real flow, this would open an AI prompt interface
    const title = prompt('Artifact title:');
    if (!title) return;

    try {
      const artifact = await projectService.createArtifact(selectedProjectId, title, {
        type: 'markdown',
        content: `# ${title}\n\n*This artifact is being generated...*`,
      });
      artifacts = [...artifacts, artifact];
      selectedArtifactIds = new Set([artifact.id]);
      detailArtifactId = artifact.id;
    } catch (e) {
      console.error('Failed to create artifact:', e);
    }
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

  function formatRelativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  }
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
      {/each}
      <button class="tab new-tab" onclick={handleNewProject} title="New project">
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
