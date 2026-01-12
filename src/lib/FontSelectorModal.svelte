<script lang="ts">
  import { Search } from 'lucide-svelte';
  import {
    fontSelection,
    getCategorizedFonts,
    setUiFont,
    setMonoFont,
  } from './stores/fonts.svelte';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let searchQuery = $state('');
  let selectedIndex = $state(-1);
  let activeTab = $state<'ui' | 'mono'>('mono'); // Default to mono since this is a code viewer
  let searchInputRef = $state<HTMLInputElement | null>(null);
  let dropdownRef = $state<HTMLDivElement | null>(null);

  // Focus search input on mount
  $effect(() => {
    searchInputRef?.focus();
  });

  // Filter fonts based on search and active tab
  // Note: getCategorizedFonts() reads from systemFonts.$state, so we call it
  // inside the derived to ensure reactivity when fonts load
  let filteredFonts = $derived.by(() => {
    const categorized = getCategorizedFonts();
    const fonts = activeTab === 'ui' ? categorized.ui : categorized.mono;
    if (!searchQuery.trim()) return fonts;
    const query = searchQuery.toLowerCase();
    return fonts.filter((f) => f.toLowerCase().includes(query));
  });

  // Current selection for active tab
  let currentSelection = $derived(activeTab === 'ui' ? fontSelection.ui : fontSelection.mono);

  // Reset selection when filter or tab changes
  $effect(() => {
    const _ = filteredFonts;
    selectedIndex = -1;
  });

  function handleFontSelect(fontName: string) {
    if (activeTab === 'ui') {
      setUiFont(fontName);
    } else {
      setMonoFont(fontName);
    }
    onClose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onClose();
      event.preventDefault();
    } else if (event.key === 'Enter') {
      if (filteredFonts.length > 0 && selectedIndex >= 0 && selectedIndex < filteredFonts.length) {
        handleFontSelect(filteredFonts[selectedIndex]);
        event.preventDefault();
      }
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, filteredFonts.length - 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (selectedIndex > 0) {
        selectedIndex = selectedIndex - 1;
      }
    } else if (event.key === 'Tab') {
      event.preventDefault();
      activeTab = activeTab === 'ui' ? 'mono' : 'ui';
      searchQuery = '';
    }
  }

  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (dropdownRef && !dropdownRef.contains(target) && !target.closest('.font-btn')) {
      onClose();
    }
  }

  /**
   * Get the font stack for preview styling
   */
  function getFontStack(fontName: string): string {
    if (activeTab === 'mono') {
      return `"${fontName}", ui-monospace, monospace`;
    }
    return `"${fontName}", system-ui, sans-serif`;
  }
</script>

<svelte:window onkeydown={handleKeydown} onclick={handleClickOutside} />

<div class="font-dropdown" bind:this={dropdownRef}>
  <!-- Tab buttons -->
  <div class="tab-bar">
    <button
      class="tab-btn"
      class:active={activeTab === 'mono'}
      onclick={() => {
        activeTab = 'mono';
        searchQuery = '';
      }}
    >
      Code
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === 'ui'}
      onclick={() => {
        activeTab = 'ui';
        searchQuery = '';
      }}
    >
      UI
    </button>
  </div>

  <div class="search-container">
    <Search size={14} class="search-icon" />
    <input
      bind:this={searchInputRef}
      type="text"
      class="search-input"
      placeholder="Search fonts..."
      bind:value={searchQuery}
      autocomplete="off"
      spellcheck="false"
    />
  </div>

  <div class="font-list">
    <!-- System default option -->
    <button
      class="font-item"
      class:active={currentSelection === ''}
      class:selected={selectedIndex === -1 && currentSelection === ''}
      onclick={() => handleFontSelect('')}
    >
      <span class="font-name system-default">System Default</span>
    </button>

    {#each filteredFonts as font, i (font)}
      <button
        class="font-item"
        class:active={font === currentSelection}
        class:selected={i === selectedIndex}
        onclick={() => handleFontSelect(font)}
      >
        <span class="font-name" style="font-family: {getFontStack(font)};">{font}</span>
      </button>
    {:else}
      <div class="no-results">No fonts match "{searchQuery}"</div>
    {/each}
  </div>
</div>

<style>
  .font-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: var(--bg-chrome);
    border: 1px solid var(--border-muted);
    border-radius: 8px;
    box-shadow: var(--shadow-elevated);
    z-index: 100;
    width: 260px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--border-subtle);
  }

  .tab-btn {
    flex: 1;
    padding: 8px 12px;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: var(--size-xs);
    font-weight: 500;
    cursor: pointer;
    transition:
      color 0.1s,
      background-color 0.1s;
  }

  .tab-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .tab-btn.active {
    color: var(--text-primary);
    background-color: var(--bg-primary);
  }

  .search-container {
    position: relative;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .search-container :global(.search-icon) {
    position: absolute;
    left: 20px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-faint);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 6px 8px 6px 30px;
    background: var(--bg-primary);
    border: 1px solid var(--border-muted);
    border-radius: 5px;
    color: var(--text-primary);
    font-size: var(--size-xs);
    box-sizing: border-box;
    transition:
      border-color 0.1s,
      background-color 0.1s;
  }

  .search-input::placeholder {
    color: var(--text-faint);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--border-emphasis);
    background-color: var(--bg-hover);
  }

  .font-list {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    max-height: 320px;
    padding: 4px 0;
  }

  .font-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--size-xs);
    text-align: left;
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .font-item:hover,
  .font-item.selected {
    background-color: var(--bg-hover);
  }

  .font-item.active {
    background-color: var(--bg-primary);
  }

  .font-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .font-name.system-default {
    color: var(--text-muted);
    font-style: italic;
  }

  .no-results {
    padding: 16px 12px;
    text-align: center;
    color: var(--text-muted);
    font-size: var(--size-xs);
  }
</style>
