<script lang="ts">
  import { X, Plus, FolderGit2 } from 'lucide-svelte';
  import { windowState, closeTab } from './stores/tabState.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  interface Props {
    onNewTab: () => void;
    onSwitchTab: (index: number) => Promise<void>;
  }

  let { onNewTab, onSwitchTab }: Props = $props();

  async function handleSwitchTab(index: number) {
    console.log(`TabBar: Switching to tab ${index}`);
    await onSwitchTab(index);
  }

  async function handleCloseTab(tabId: string, event: MouseEvent) {
    event.stopPropagation();
    closeTab(tabId);

    // Close window if no tabs left
    if (windowState.tabs.length === 0) {
      const window = getCurrentWindow();
      await window.close();
    }
  }

  function handleNewTab() {
    onNewTab();
  }
</script>

<div class="tab-bar">
  <div class="tabs">
    {#each windowState.tabs as tab, index (tab.id)}
      <button
        class="tab"
        class:active={index === windowState.activeTabIndex}
        onclick={() => handleSwitchTab(index)}
        title={tab.repoPath}
      >
        <FolderGit2 size={14} />
        <span class="tab-name">{tab.repoName}</span>
        {#if windowState.tabs.length > 1}
          <div
            class="close-btn"
            onclick={(e) => handleCloseTab(tab.id, e)}
            title="Close tab"
            role="button"
            tabindex="0"
          >
            <X size={12} />
          </div>
        {/if}
      </button>
    {/each}
  </div>

  <button class="new-tab-btn" onclick={handleNewTab} title="Open folder in new tab">
    <Plus size={16} />
  </button>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: var(--bg-chrome);
    border-bottom: 1px solid var(--border-subtle);
  }

  .tabs {
    display: flex;
    gap: 2px;
    flex: 1;
    overflow-x: auto;
    scrollbar-width: none; /* Firefox */
  }

  .tabs::-webkit-scrollbar {
    display: none; /* Chrome, Safari */
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-primary);
    border: none;
    border-radius: 6px 6px 0 0;
    color: var(--text-muted);
    font-size: var(--size-sm);
    cursor: pointer;
    transition: all 0.1s;
    white-space: nowrap;
    min-width: 120px;
    max-width: 200px;
  }

  .tab:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--bg-chrome);
    color: var(--text-primary);
    border-bottom: 2px solid var(--ui-accent);
  }

  .tab-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .close-btn {
    display: flex;
    align-items: center;
    padding: 2px;
    background: none;
    border: none;
    border-radius: 3px;
    color: var(--text-faint);
    cursor: pointer;
    opacity: 0;
    transition: all 0.1s;
  }

  .tab:hover .close-btn {
    opacity: 1;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .new-tab-btn {
    display: flex;
    align-items: center;
    padding: 6px;
    background: var(--bg-primary);
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.1s;
  }

  .new-tab-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
