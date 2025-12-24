<script lang="ts">
  import { onMount } from 'svelte';
  import { getGitStatus } from './services/git';
  import type { FileStatus, GitStatus } from './types';

  export type FileCategory = 'staged' | 'unstaged' | 'untracked';

  interface Props {
    onFileSelect?: (path: string, category: FileCategory) => void;
    selectedFile?: string | null;
  }

  let { onFileSelect, selectedFile = null }: Props = $props();

  let gitStatus: GitStatus | null = $state(null);
  let error: string | null = $state(null);
  let loading = $state(true);

  onMount(() => {
    loadStatus();
  });

  export async function loadStatus() {
    loading = true;
    error = null;
    try {
      gitStatus = await getGitStatus();
      // Auto-select first file if none selected
      if (!selectedFile && gitStatus && onFileSelect) {
        if (gitStatus.unstaged.length > 0) {
          onFileSelect(gitStatus.unstaged[0].path, 'unstaged');
        } else if (gitStatus.staged.length > 0) {
          onFileSelect(gitStatus.staged[0].path, 'staged');
        } else if (gitStatus.untracked.length > 0) {
          onFileSelect(gitStatus.untracked[0].path, 'untracked');
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function selectFile(path: string, category: FileCategory) {
    onFileSelect?.(path, category);
  }

  function getStatusIcon(status: string): string {
    switch (status) {
      case 'modified':
        return 'M';
      case 'added':
        return 'A';
      case 'deleted':
        return 'D';
      case 'renamed':
        return 'R';
      case 'untracked':
        return '?';
      default:
        return '•';
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'modified':
        return 'var(--status-modified)';
      case 'added':
        return 'var(--status-added)';
      case 'deleted':
        return 'var(--status-deleted)';
      case 'renamed':
        return 'var(--status-renamed)';
      case 'untracked':
        return 'var(--status-untracked)';
      default:
        return 'var(--text-primary)';
    }
  }

  function getFileName(path: string): string {
    return path.split('/').pop() || path;
  }

  function getFileDir(path: string): string {
    const parts = path.split('/');
    if (parts.length > 1) {
      return parts.slice(0, -1).join('/') + '/';
    }
    return '';
  }
</script>

<div class="sidebar-content">
  <div class="header">
    <h2>Changes</h2>
    <button class="refresh-btn" onclick={loadStatus} title="Refresh">↻</button>
  </div>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if error}
    <div class="error">
      <p>Error: {error}</p>
      <button onclick={loadStatus}>Retry</button>
    </div>
  {:else if gitStatus}
    <div class="file-sections">
      {#if gitStatus.staged.length > 0}
        <div class="section">
          <div class="section-header">
            <span class="section-title">Staged</span>
            <span class="badge staged-badge">{gitStatus.staged.length}</span>
          </div>
          <ul class="file-list">
            {#each gitStatus.staged as file}
              <li
                class="file-item"
                class:selected={selectedFile === file.path}
                onclick={() => selectFile(file.path, 'staged')}
              >
                <span class="status-icon" style="color: {getStatusColor(file.status)}"
                  >{getStatusIcon(file.status)}</span
                >
                <span class="file-path">
                  <span class="file-dir">{getFileDir(file.path)}</span>
                  <span class="file-name">{getFileName(file.path)}</span>
                </span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      {#if gitStatus.unstaged.length > 0}
        <div class="section">
          <div class="section-header">
            <span class="section-title">Unstaged</span>
            <span class="badge">{gitStatus.unstaged.length}</span>
          </div>
          <ul class="file-list">
            {#each gitStatus.unstaged as file}
              <li
                class="file-item"
                class:selected={selectedFile === file.path}
                onclick={() => selectFile(file.path, 'unstaged')}
              >
                <span class="status-icon" style="color: {getStatusColor(file.status)}"
                  >{getStatusIcon(file.status)}</span
                >
                <span class="file-path">
                  <span class="file-dir">{getFileDir(file.path)}</span>
                  <span class="file-name">{getFileName(file.path)}</span>
                </span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      {#if gitStatus.untracked.length > 0}
        <div class="section">
          <div class="section-header">
            <span class="section-title">Untracked</span>
            <span class="badge">{gitStatus.untracked.length}</span>
          </div>
          <ul class="file-list">
            {#each gitStatus.untracked as file}
              <li
                class="file-item"
                class:selected={selectedFile === file.path}
                onclick={() => selectFile(file.path, 'untracked')}
              >
                <span class="status-icon" style="color: {getStatusColor(file.status)}"
                  >{getStatusIcon(file.status)}</span
                >
                <span class="file-path">
                  <span class="file-dir">{getFileDir(file.path)}</span>
                  <span class="file-name">{getFileName(file.path)}</span>
                </span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      {#if gitStatus.staged.length === 0 && gitStatus.unstaged.length === 0 && gitStatus.untracked.length === 0}
        <div class="empty-state">
          <p>No changes</p>
          <p class="empty-hint">Working tree is clean</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .sidebar-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-primary);
  }

  .header h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .refresh-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .refresh-btn:hover {
    background-color: var(--bg-input);
    color: var(--text-secondary);
  }

  .loading,
  .error,
  .empty-state {
    padding: 20px 16px;
    text-align: center;
    color: var(--text-muted);
  }

  .error {
    color: var(--status-deleted);
  }

  .error button {
    margin-top: 8px;
    padding: 4px 12px;
    background-color: var(--bg-input);
    border: none;
    border-radius: 4px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .empty-state p {
    margin: 0;
  }

  .empty-hint {
    font-size: 12px;
    margin-top: 4px !important;
  }

  .file-sections {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .section {
    margin-bottom: 8px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 16px;
    cursor: pointer;
  }

  .section-header:hover {
    background-color: var(--bg-tertiary);
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .badge {
    background-color: var(--bg-input);
    color: var(--text-secondary);
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 10px;
  }

  .staged-badge {
    background-color: var(--ui-success);
    color: white;
  }

  .file-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .file-item {
    display: flex;
    align-items: center;
    padding: 4px 16px 4px 24px;
    cursor: pointer;
    font-size: 13px;
  }

  .file-item:hover {
    background-color: var(--bg-tertiary);
  }

  .file-item.selected {
    background-color: var(--ui-selection);
  }

  .status-icon {
    width: 16px;
    font-family: monospace;
    font-weight: bold;
    margin-right: 8px;
    flex-shrink: 0;
  }

  .file-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-dir {
    color: var(--text-muted);
  }

  .file-name {
    color: var(--text-primary);
  }
</style>
