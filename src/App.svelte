<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { ChevronDown } from 'lucide-svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import DiffViewer from './lib/DiffViewer.svelte';
  import DiffSelectorModal from './lib/DiffSelectorModal.svelte';
  import { getDiff, resolveRef, getRepoInfo } from './lib/services/git';
  import {
    subscribeToFileChanges,
    startWatching,
    stopWatching,
    type Unsubscribe,
  } from './lib/services/statusEvents';
  import type { FileDiff, DiffSpec } from './lib/types';
  import { getFilePath } from './lib/diffUtils';
  import { themeToCssVars, createAdaptiveTheme } from './lib/theme';
  import {
    SYNTAX_THEMES,
    setSyntaxTheme,
    getTheme,
    type SyntaxThemeName,
  } from './lib/services/highlighter';

  // UI scaling
  const SIZE_STEP = 1;
  const SIZE_MIN = 10;
  const SIZE_MAX = 24;
  const SIZE_DEFAULT = 13;
  const SIZE_STORAGE_KEY = 'staged-size-base';

  let sizeBase = $state(SIZE_DEFAULT);

  function loadSavedSize() {
    const saved = localStorage.getItem(SIZE_STORAGE_KEY);
    if (saved) {
      const parsed = parseInt(saved, 10);
      if (!isNaN(parsed) && parsed >= SIZE_MIN && parsed <= SIZE_MAX) {
        sizeBase = parsed;
      }
    }
    applySize();
  }

  function applySize() {
    document.documentElement.style.setProperty('--size-base', `${sizeBase}px`);
  }

  function increaseSize() {
    if (sizeBase < SIZE_MAX) {
      sizeBase += SIZE_STEP;
      applySize();
      localStorage.setItem(SIZE_STORAGE_KEY, String(sizeBase));
    }
  }

  function decreaseSize() {
    if (sizeBase > SIZE_MIN) {
      sizeBase -= SIZE_STEP;
      applySize();
      localStorage.setItem(SIZE_STORAGE_KEY, String(sizeBase));
    }
  }

  function resetSize() {
    sizeBase = SIZE_DEFAULT;
    applySize();
    localStorage.setItem(SIZE_STORAGE_KEY, String(sizeBase));
  }

  function handleKeydown(event: KeyboardEvent) {
    // Cmd/Ctrl + Shift + = (plus) to increase size
    // Cmd/Ctrl + Shift + - (minus) to decrease size
    // Cmd/Ctrl + Shift + 0 to reset size
    if ((event.metaKey || event.ctrlKey) && event.shiftKey) {
      if (event.key === '=' || event.key === '+') {
        event.preventDefault();
        increaseSize();
      } else if (event.key === '-' || event.key === '_') {
        event.preventDefault();
        decreaseSize();
      } else if (event.key === '0') {
        event.preventDefault();
        resetSize();
      }
    }
    // T to cycle syntax themes
    if (event.key === 't' && !event.metaKey && !event.ctrlKey && !event.shiftKey) {
      // Don't trigger if typing in an input
      const target = event.target as HTMLElement;
      if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
        event.preventDefault();
        cycleSyntaxTheme();
      }
    }
  }

  // ==========================================================================
  // Theme (Adaptive - derived from syntax theme)
  // ==========================================================================

  function applyCssVars(cssVars: string) {
    cssVars.split('\n').forEach((line) => {
      const match = line.match(/^\s*(--[\w-]+):\s*(.+);?\s*$/);
      if (match) {
        document.documentElement.style.setProperty(match[1], match[2].replace(';', ''));
      }
    });
  }

  /**
   * Apply adaptive theme based on current syntax theme colors.
   */
  function applyAdaptiveTheme() {
    const themeInfo = getTheme();
    if (themeInfo) {
      const adaptiveTheme = createAdaptiveTheme(themeInfo.bg, themeInfo.fg, themeInfo.comment);
      const cssVars = themeToCssVars(adaptiveTheme);
      applyCssVars(cssVars);
    }
  }

  // ==========================================================================
  // Syntax Theme
  // ==========================================================================

  const SYNTAX_THEME_STORAGE_KEY = 'staged-syntax-theme';

  let currentSyntaxTheme = $state<SyntaxThemeName>('github-dark');
  // Trigger re-render when syntax theme changes
  let syntaxThemeVersion = $state(0);

  async function loadSavedSyntaxTheme() {
    const saved = localStorage.getItem(SYNTAX_THEME_STORAGE_KEY);
    if (saved && SYNTAX_THEMES.includes(saved as SyntaxThemeName)) {
      currentSyntaxTheme = saved as SyntaxThemeName;
    }
    // Always initialize highlighter with the current theme (saved or default)
    await setSyntaxTheme(currentSyntaxTheme);
    // Apply adaptive theme from syntax theme colors
    applyAdaptiveTheme();
  }

  async function selectSyntaxTheme(name: SyntaxThemeName) {
    currentSyntaxTheme = name;
    await setSyntaxTheme(name);
    localStorage.setItem(SYNTAX_THEME_STORAGE_KEY, name);
    // Bump version to trigger re-render of diff viewer
    syntaxThemeVersion++;
    // Update chrome to match new syntax theme
    applyAdaptiveTheme();
  }

  function cycleSyntaxTheme() {
    const currentIndex = SYNTAX_THEMES.indexOf(currentSyntaxTheme);
    const nextIndex = (currentIndex + 1) % SYNTAX_THEMES.length;
    selectSyntaxTheme(SYNTAX_THEMES[nextIndex]);
  }

  // ==========================================================================
  // Diff Selector
  // ==========================================================================

  // Available diff presets
  const DIFF_PRESETS: DiffSpec[] = [
    { base: 'HEAD', head: '@', label: 'Working Changes' },
    { base: 'main', head: '@', label: 'Against main' },
    { base: 'HEAD~1', head: 'HEAD', label: 'Last Commit' },
  ];

  // Current diff spec - default to working changes
  let currentDiffSpec = $state<DiffSpec>(DIFF_PRESETS[0]);
  let diffSelectorOpen = $state(false);
  let customDiffModalOpen = $state(false);

  // Resolved SHAs for tooltip display
  let resolvedBaseSha = $state<string | null>(null);
  let resolvedHeadSha = $state<string | null>(null);

  // Derived values for easy access
  let diffBase = $derived(currentDiffSpec.base);
  let diffHead = $derived(currentDiffSpec.head);

  // Is this a preset or custom diff?
  let isPreset = $derived(
    DIFF_PRESETS.some(
      (p) => p.base === diffBase && p.head === diffHead && p.label === currentDiffSpec.label
    )
  );

  // Display label: short name for presets, base..head for custom
  let displayLabel = $derived(isPreset ? currentDiffSpec.label : `${diffBase}..${diffHead}`);

  // Tooltip with full details
  let tooltipText = $derived(() => {
    const basePart = resolvedBaseSha ? `${diffBase} (${resolvedBaseSha})` : diffBase;
    const headPart = resolvedHeadSha ? `${diffHead} (${resolvedHeadSha})` : diffHead;
    return `${basePart} → ${headPart}`;
  });

  async function updateResolvedShas() {
    try {
      resolvedBaseSha = await resolveRef(diffBase);
      resolvedHeadSha = await resolveRef(diffHead);
    } catch {
      resolvedBaseSha = null;
      resolvedHeadSha = null;
    }
  }

  function selectDiffSpec(spec: DiffSpec) {
    currentDiffSpec = spec;
    diffSelectorOpen = false;
    // Clear current selection and reload
    selectedFile = null;
    currentDiff = null;
    // Update resolved SHAs for tooltip
    updateResolvedShas();
    // Reload all diffs
    loadAllDiffs();
  }

  function handleCustomDiffSelect(base: string, head: string, label: string) {
    selectDiffSpec({ base, head, label });
  }

  function openCustomDiffModal() {
    diffSelectorOpen = false;
    customDiffModalOpen = true;
  }

  function toggleDiffSelector() {
    diffSelectorOpen = !diffSelectorOpen;
  }

  function closeDiffSelector() {
    diffSelectorOpen = false;
  }

  // ==========================================================================
  // Diff State
  // ==========================================================================

  // All diffs for the current base..head
  let allDiffs: FileDiff[] = $state([]);
  let diffsLoading = $state(true);
  let diffsError: string | null = $state(null);

  // Currently selected file and its diff
  let selectedFile: string | null = $state(null);
  let currentDiff = $derived.by(() => {
    if (!selectedFile) return null;
    return allDiffs.find((d) => getFilePath(d) === selectedFile) ?? null;
  });

  let sidebarRef: Sidebar | null = $state(null);

  // Watcher cleanup function
  let unsubscribe: Unsubscribe | null = null;

  // Current repo path (for watcher)
  let currentRepoPath: string | null = $state(null);

  /**
   * Load all diffs for the current base..head.
   */
  async function loadAllDiffs() {
    diffsLoading = true;
    diffsError = null;

    try {
      allDiffs = await getDiff(diffBase, diffHead);

      // Auto-select first file if none selected
      if (!selectedFile && allDiffs.length > 0) {
        selectedFile = getFilePath(allDiffs[0]);
      }

      // Update sidebar with the new file list
      sidebarRef?.setDiffs(allDiffs);
    } catch (e) {
      diffsError = e instanceof Error ? e.message : String(e);
      allDiffs = [];
    } finally {
      diffsLoading = false;
    }
  }

  /**
   * Handle file change notifications from the watcher.
   * Only relevant when diffHead is "@" (working tree).
   */
  async function handleFilesChanged() {
    // Only reload diffs if we're viewing the working tree
    if (diffHead !== '@') {
      return;
    }

    // Reload all diffs
    await loadAllDiffs();

    // Check if currently selected file still exists
    if (selectedFile) {
      const stillExists = allDiffs.some((d) => getFilePath(d) === selectedFile);
      if (!stillExists) {
        // File no longer has changes - select first available or clear
        selectedFile = allDiffs.length > 0 ? getFilePath(allDiffs[0]) : null;
      }
    }
  }

  onMount(async () => {
    // Load saved preferences
    loadSavedSize();
    // Load syntax theme (this also applies the adaptive chrome theme)
    await loadSavedSyntaxTheme();

    // Listen for keyboard shortcuts
    window.addEventListener('keydown', handleKeydown);

    // Resolve initial SHAs for tooltip
    updateResolvedShas();

    // Load initial diffs
    await loadAllDiffs();

    // Get repo path for watcher
    try {
      const info = await getRepoInfo();
      if (info?.repo_path) {
        currentRepoPath = info.repo_path;
        await startWatching(info.repo_path);
        console.log('Started watching:', info.repo_path);
      }
    } catch (e) {
      console.error('Failed to start watcher:', e);
    }

    // Subscribe to file change events from the backend
    unsubscribe = await subscribeToFileChanges(handleFilesChanged);
  });

  onDestroy(() => {
    // Clean up keyboard listener
    window.removeEventListener('keydown', handleKeydown);

    // Clean up watcher and event listeners
    unsubscribe?.();
    stopWatching().catch(() => {
      // Ignore errors on cleanup
    });
  });

  function handleFileSelect(path: string) {
    selectedFile = path;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<main onclick={closeDiffSelector}>
  <!-- Diff selector header -->
  <header class="diff-header">
    <div class="diff-selector-container">
      <button
        class="diff-selector"
        class:open={diffSelectorOpen}
        onclick={(e) => {
          e.stopPropagation();
          toggleDiffSelector();
        }}
        title={tooltipText()}
      >
        <span class="diff-label">{displayLabel}</span>
        <ChevronDown size={14} />
      </button>

      {#if diffSelectorOpen}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="diff-dropdown" onclick={(e) => e.stopPropagation()}>
          {#each DIFF_PRESETS as preset}
            <button
              class="diff-option"
              class:selected={preset.base === diffBase && preset.head === diffHead}
              onclick={() => selectDiffSpec(preset)}
            >
              <span class="option-label">{preset.label}</span>
              <span class="option-spec">{preset.base}..{preset.head}</span>
            </button>
          {/each}
          <div class="dropdown-divider"></div>
          <button class="diff-option" onclick={openCustomDiffModal}>
            <span class="option-label">Custom...</span>
          </button>
        </div>
      {/if}
    </div>

    <!-- Theme picker -->
    <div class="theme-picker">
      <span class="picker-label">Theme:</span>
      <select
        class="theme-select"
        onchange={(e) =>
          selectSyntaxTheme((e.target as HTMLSelectElement).value as SyntaxThemeName)}
      >
        {#each SYNTAX_THEMES as name}
          <option value={name} selected={name === currentSyntaxTheme}>{name}</option>
        {/each}
      </select>
    </div>
  </header>

  <div class="app-container">
    <section class="main-content">
      {#if diffsLoading}
        <div class="loading-state">
          <p>Loading...</p>
        </div>
      {:else if diffsError}
        <div class="error-state">
          <p>Error loading diff:</p>
          <p class="error-message">{diffsError}</p>
        </div>
      {:else}
        <DiffViewer diff={currentDiff} {diffBase} {diffHead} {sizeBase} {syntaxThemeVersion} />
      {/if}
    </section>
    <aside class="sidebar">
      <Sidebar
        bind:this={sidebarRef}
        onFileSelect={handleFileSelect}
        {selectedFile}
        {diffBase}
        {diffHead}
      />
    </aside>
  </div>
</main>

<DiffSelectorModal
  open={customDiffModalOpen}
  onClose={() => (customDiffModalOpen = false)}
  onSelect={handleCustomDiffSelect}
  currentBase={diffBase}
  currentHead={diffHead}
/>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background-color: var(--bg-chrome);
    color: var(--text-primary);
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background-color: var(--bg-chrome);
  }

  /* Header - part of unified chrome, no border */
  .diff-header {
    display: flex;
    align-items: center;
    padding: 6px 12px;
    background-color: transparent;
    flex-shrink: 0;
  }

  .diff-selector-container {
    position: relative;
  }

  .diff-selector {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--text-primary);
    font-size: var(--size-sm);
    cursor: pointer;
    transition:
      background-color 0.15s,
      border-color 0.15s;
  }

  .diff-selector:hover {
    background-color: var(--bg-hover);
  }

  .diff-selector.open {
    background-color: var(--bg-hover);
    border-color: var(--border-muted);
  }

  .diff-label {
    font-weight: 500;
  }

  .diff-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    box-shadow: var(--shadow-elevated);
    min-width: 200px;
    z-index: 100;
    overflow: hidden;
  }

  .diff-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: var(--size-sm);
    text-align: left;
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .diff-option:hover {
    background-color: var(--bg-hover);
  }

  .diff-option.selected {
    background-color: var(--ui-selection);
  }

  .option-label {
    font-weight: 500;
  }

  .option-spec {
    font-family: monospace;
    color: var(--text-muted);
    font-size: var(--size-xs);
  }

  .dropdown-divider {
    height: 1px;
    background: var(--border-subtle);
    margin: 4px 0;
  }

  .app-container {
    display: flex;
    flex: 1;
    overflow: hidden;
    padding: 0 8px 8px 8px;
    gap: 8px;
  }

  .sidebar {
    width: 260px;
    min-width: 180px;
    background-color: transparent;
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

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: var(--size-lg);
  }

  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--status-deleted);
    font-size: var(--size-lg);
  }

  .error-message {
    font-family: monospace;
    font-size: var(--size-sm);
    color: var(--text-muted);
    margin-top: 8px;
  }

  /* Theme picker - minimal */
  .theme-picker {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .picker-label {
    font-size: var(--size-xs);
    color: var(--text-faint);
  }

  .theme-select {
    padding: 2px 4px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    cursor: pointer;
  }

  .theme-select:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .theme-select:focus {
    outline: none;
    border-color: var(--border-muted);
  }
</style>
