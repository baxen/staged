<script lang="ts">
  import Sidebar from './lib/Sidebar.svelte'
  import DiffViewer from './lib/DiffViewer.svelte'
  import CommitPanel from './lib/CommitPanel.svelte'
</script>

<main>
  <div class="app-container">
    <aside class="sidebar">
      <Sidebar />
    </aside>
    <section class="main-content">
      <DiffViewer />
    </section>
  </div>
  <footer class="commit-panel">
    <CommitPanel />
  </footer>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background-color: #1e1e1e;
    color: #d4d4d4;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .app-container {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    width: 280px;
    min-width: 200px;
    background-color: #252526;
    border-right: 1px solid #3c3c3c;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .commit-panel {
    height: 120px;
    min-height: 80px;
    background-color: #252526;
    border-top: 1px solid #3c3c3c;
  }
</style>
