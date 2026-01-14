<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface PluginInfo {
    name: string;
    version: string;
    description: string;
    author: string;
    enabled: boolean;
    commands: string[];
    hasFrontend: boolean;
  }

  let plugins = $state<PluginInfo[]>([]);
  let loading = $state(true);
  let error = $state('');

  async function loadPlugins() {
    try {
      loading = true;
      error = '';

      const pluginList = await invoke<any[]>('list_plugins');
      const commands = await invoke<string[]>('list_plugin_commands');

      plugins = pluginList.map(p => ({
        name: p.name,
        version: p.version,
        description: p.description || 'No description',
        author: p.author || 'Unknown',
        enabled: true,
        commands: commands.filter(cmd => cmd.startsWith(p.name + ':')),
        hasFrontend: !!p.frontend,
      }));
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function testPluginCommand(pluginName: string, commandName: string) {
    try {
      const result = await invoke('plugin_invoke', {
        pluginName,
        commandName: commandName.split(':')[1],
        payload: '{}'
      });
      alert(`Command executed successfully:\n${result}`);
    } catch (e) {
      alert(`Command failed:\n${e}`);
    }
  }

  function openPluginUI(pluginName: string) {
    // Use the global pluginSystem to show the modal
    const pluginSystem = (window as any).pluginSystem;
    if (pluginSystem) {
      try {
        // Modal is registered as "pluginName:modalId"
        const fullModalId = `${pluginName}:main`;
        pluginSystem.showModal(fullModalId);
      } catch (e) {
        console.error('Failed to open plugin UI:', e);
        alert(`Failed to open plugin: ${e}`);
      }
    } else {
      alert('Plugin system not initialized');
    }
  }

  onMount(() => {
    loadPlugins();
  });
</script>

<div class="plugin-settings">
  <div class="header">
    <h1>Plugin Settings</h1>
    <button class="btn-refresh" onclick={() => loadPlugins()}>
      Refresh
    </button>
  </div>

  {#if loading}
    <div class="loading">Loading plugins...</div>
  {/if}

  {#if error}
    <div class="error-banner">
      <strong>Error:</strong> {error}
    </div>
  {/if}

  {#if !loading && plugins.length === 0}
    <div class="empty-state">
      <p>No plugins installed</p>
      <p class="hint">
        Plugins should be placed in:<br>
        <code>~/Library/Application Support/staged/plugins/</code> (macOS)<br>
        <code>~/.config/staged/plugins/</code> (Linux)
      </p>
    </div>
  {/if}

  <div class="plugin-list">
    {#each plugins as plugin}
      <div class="plugin-card">
        <div class="plugin-header">
          <div class="plugin-title">
            <h2>{plugin.name}</h2>
            <span class="version">v{plugin.version}</span>
          </div>
          <div class="plugin-actions">
            {#if plugin.hasFrontend}
              <button class="btn-open" onclick={() => openPluginUI(plugin.name)}>
                Open
              </button>
            {/if}
            <span class="status-badge {plugin.enabled ? 'enabled' : 'disabled'}">
              {plugin.enabled ? 'Enabled' : 'Disabled'}
            </span>
          </div>
        </div>

        <p class="description">{plugin.description}</p>

        <div class="plugin-meta">
          <div class="meta-item">
            <span class="label">Author:</span>
            <span class="value">{plugin.author}</span>
          </div>
        </div>

        {#if plugin.commands.length > 0}
          <details class="commands-section">
            <summary>Commands ({plugin.commands.length})</summary>
            <ul class="command-list">
              {#each plugin.commands as command}
                <li>
                  <code>{command}</code>
                  <button
                    class="btn-test"
                    onclick={() => testPluginCommand(plugin.name, command)}
                  >
                    Test
                  </button>
                </li>
              {/each}
            </ul>
          </details>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .plugin-settings {
    padding: 24px;
    max-width: 900px;
    margin: 0 auto;
    color: var(--text-primary, #e0e0e0);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }

  h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 600;
  }

  .btn-refresh {
    padding: 8px 16px;
    background-color: var(--accent-color, #0078d4);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    transition: background-color 0.2s;
  }

  .btn-refresh:hover {
    background-color: var(--accent-hover, #005a9e);
  }

  .loading {
    text-align: center;
    padding: 48px;
    color: var(--text-secondary, #b0b0b0);
  }

  .error-banner {
    padding: 12px 16px;
    background-color: rgba(244, 67, 54, 0.1);
    border: 1px solid rgba(244, 67, 54, 0.3);
    border-radius: 6px;
    margin-bottom: 16px;
    color: #f44336;
  }

  .empty-state {
    text-align: center;
    padding: 48px;
    color: var(--text-secondary, #b0b0b0);
  }

  .empty-state .hint {
    margin-top: 16px;
    font-size: 13px;
    line-height: 1.8;
  }

  .empty-state code {
    background-color: rgba(255, 255, 255, 0.05);
    padding: 2px 6px;
    border-radius: 3px;
    font-family: 'Monaco', 'Menlo', monospace;
    font-size: 12px;
  }

  .plugin-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .plugin-card {
    background-color: var(--bg-secondary, #2a2a2a);
    border: 1px solid var(--border-color, #333);
    border-radius: 8px;
    padding: 20px;
  }

  .plugin-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .plugin-title {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .plugin-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .plugin-header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    display: inline-block;
  }

  .version {
    margin-left: 12px;
    padding: 2px 8px;
    background-color: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
  }

  .status-badge {
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
  }

  .status-badge.enabled {
    background-color: rgba(0, 200, 83, 0.2);
    color: #00c853;
  }

  .status-badge.disabled {
    background-color: rgba(158, 158, 158, 0.2);
    color: #9e9e9e;
  }

  .description {
    margin: 0 0 16px 0;
    color: var(--text-secondary, #b0b0b0);
    line-height: 1.5;
  }

  .plugin-meta {
    display: flex;
    gap: 24px;
    margin-bottom: 16px;
    font-size: 13px;
  }

  .meta-item .label {
    color: var(--text-muted, #888);
    margin-right: 8px;
  }

  .meta-item .value {
    color: var(--text-primary, #e0e0e0);
  }

  .commands-section {
    margin-top: 16px;
    border-top: 1px solid var(--border-color, #333);
    padding-top: 16px;
  }

  .commands-section summary {
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary, #b0b0b0);
    user-select: none;
  }

  .commands-section summary:hover {
    color: var(--text-primary, #e0e0e0);
  }

  .command-list {
    list-style: none;
    padding: 0;
    margin: 12px 0 0 0;
  }

  .command-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background-color: rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    margin-bottom: 4px;
  }

  .command-list code {
    font-family: 'Monaco', 'Menlo', monospace;
    font-size: 13px;
    color: var(--text-primary, #e0e0e0);
  }

  .btn-test {
    padding: 4px 12px;
    background-color: transparent;
    border: 1px solid var(--border-color, #333);
    border-radius: 4px;
    color: var(--text-primary, #e0e0e0);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-test:hover {
    background-color: var(--accent-color, #0078d4);
    border-color: var(--accent-color, #0078d4);
  }

  .btn-open {
    padding: 6px 16px;
    background-color: var(--accent-color, #0078d4);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: background-color 0.2s;
  }

  .btn-open:hover {
    background-color: var(--accent-hover, #005a9e);
  }
</style>
