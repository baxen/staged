<script lang="ts">
  // Placeholder data - will be replaced with real git data
  let stagedFiles = [
    { name: 'src/main.rs', status: 'modified' },
  ];
  
  let unstagedFiles = [
    { name: 'src/lib.rs', status: 'modified' },
    { name: 'README.md', status: 'modified' },
  ];
  
  let untrackedFiles = [
    { name: 'notes.txt', status: 'untracked' },
  ];

  let selectedFile: string | null = 'src/lib.rs';

  function selectFile(name: string) {
    selectedFile = name;
  }

  function getStatusIcon(status: string): string {
    switch (status) {
      case 'modified': return 'M';
      case 'added': return 'A';
      case 'deleted': return 'D';
      case 'renamed': return 'R';
      case 'untracked': return '?';
      default: return '•';
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'modified': return '#e2c08d';
      case 'added': return '#89d185';
      case 'deleted': return '#f14c4c';
      case 'renamed': return '#4fc1ff';
      case 'untracked': return '#888';
      default: return '#d4d4d4';
    }
  }
</script>

<div class="sidebar-content">
  <div class="header">
    <h2>Changes</h2>
  </div>

  <div class="file-sections">
    {#if stagedFiles.length > 0}
      <div class="section">
        <div class="section-header">
          <span class="section-title">Staged</span>
          <span class="badge">{stagedFiles.length}</span>
        </div>
        <ul class="file-list">
          {#each stagedFiles as file}
            <li 
              class="file-item" 
              class:selected={selectedFile === file.name}
              onclick={() => selectFile(file.name)}
            >
              <span class="status-icon" style="color: {getStatusColor(file.status)}">{getStatusIcon(file.status)}</span>
              <span class="file-name">{file.name}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if unstagedFiles.length > 0}
      <div class="section">
        <div class="section-header">
          <span class="section-title">Unstaged</span>
          <span class="badge">{unstagedFiles.length}</span>
        </div>
        <ul class="file-list">
          {#each unstagedFiles as file}
            <li 
              class="file-item"
              class:selected={selectedFile === file.name}
              onclick={() => selectFile(file.name)}
            >
              <span class="status-icon" style="color: {getStatusColor(file.status)}">{getStatusIcon(file.status)}</span>
              <span class="file-name">{file.name}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if untrackedFiles.length > 0}
      <div class="section">
        <div class="section-header">
          <span class="section-title">Untracked</span>
          <span class="badge">{untrackedFiles.length}</span>
        </div>
        <ul class="file-list">
          {#each untrackedFiles as file}
            <li 
              class="file-item"
              class:selected={selectedFile === file.name}
              onclick={() => selectFile(file.name)}
            >
              <span class="status-icon" style="color: {getStatusColor(file.status)}">{getStatusIcon(file.status)}</span>
              <span class="file-name">{file.name}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
</div>

<style>
  .sidebar-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .header {
    padding: 12px 16px;
    border-bottom: 1px solid #3c3c3c;
  }

  .header h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #cccccc;
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
    background-color: #2a2d2e;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    color: #888;
  }

  .badge {
    background-color: #4d4d4d;
    color: #cccccc;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 10px;
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
    background-color: #2a2d2e;
  }

  .file-item.selected {
    background-color: #094771;
  }

  .status-icon {
    width: 16px;
    font-family: monospace;
    font-weight: bold;
    margin-right: 8px;
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
