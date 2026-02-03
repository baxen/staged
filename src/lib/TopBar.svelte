<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Palette, Keyboard, Settings2, Eye, EyeOff } from 'lucide-svelte';
  import ThemeSelectorModal from './ThemeSelectorModal.svelte';
  import KeyboardShortcutsModal from './KeyboardShortcutsModal.svelte';
  import SettingsModal from './SettingsModal.svelte';
  import { registerShortcut } from './services/keyboard';
  import { smartDiffState, setAnnotationsRevealed } from './stores/smartDiff.svelte';

  // Modal state
  let showThemeModal = $state(false);
  let showShortcutsModal = $state(false);
  let showSettingsModal = $state(false);

  // Smart diff state (for annotations reveal toggle)
  let annotationsRevealed = $derived(smartDiffState.annotationsRevealed);
  let hasFileAnnotations = $derived(smartDiffState.results.size > 0);

  // Start window drag from non-interactive areas
  function startDrag(e: PointerEvent) {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    const isInteractive = target.closest('button, a, input, [role="button"], .dropdown');
    if (!isInteractive) {
      e.preventDefault();
      getCurrentWindow().startDragging();
    }
  }

  // Register keyboard shortcuts
  onMount(() => {
    const unregisterTheme = registerShortcut({
      id: 'open-theme-picker',
      keys: ['p'],
      modifiers: { meta: true },
      description: 'Theme picker',
      category: 'view',
      handler: () => {
        showThemeModal = !showThemeModal;
      },
    });

    const unregisterSettings = registerShortcut({
      id: 'open-settings',
      keys: [','],
      modifiers: { meta: true },
      description: 'Open settings',
      category: 'view',
      handler: () => {
        showSettingsModal = !showSettingsModal;
      },
    });

    return () => {
      unregisterTheme();
      unregisterSettings();
    };
  });
</script>

<header class="top-bar" onpointerdown={startDrag}>
  <!-- Left section: empty (diff selector moved to sidebar) -->
  <div class="section section-left"></div>

  <!-- Center section: AI reveal toggle -->
  <div class="section section-center">
    <!-- AI Annotations reveal toggle (only show when annotations exist) -->
    {#if hasFileAnnotations}
      <button
        class="action-btn reveal-btn"
        class:active={annotationsRevealed}
        onclick={() => setAnnotationsRevealed(!annotationsRevealed)}
        title="Hold A to show explanation view"
      >
        {#if annotationsRevealed}
          <Eye size={14} />
        {:else}
          <EyeOff size={14} />
        {/if}
      </button>
    {/if}
  </div>

  <!-- Right section: Settings -->
  <div class="section section-right">
    <button
      class="icon-btn settings-btn"
      onclick={() => (showSettingsModal = true)}
      title="Settings"
    >
      <Settings2 size={14} />
    </button>

    <div class="shortcuts-picker">
      <button
        class="icon-btn shortcuts-btn"
        onclick={() => (showShortcutsModal = !showShortcutsModal)}
        class:open={showShortcutsModal}
        title="Keyboard shortcuts"
      >
        <Keyboard size={14} />
      </button>

      {#if showShortcutsModal}
        <KeyboardShortcutsModal onClose={() => (showShortcutsModal = false)} />
      {/if}
    </div>

    <div class="theme-picker">
      <button
        class="icon-btn theme-btn"
        onclick={() => (showThemeModal = !showThemeModal)}
        class:open={showThemeModal}
        title="Select theme"
      >
        <Palette size={14} />
      </button>

      {#if showThemeModal}
        <ThemeSelectorModal onClose={() => (showThemeModal = false)} />
      {/if}
    </div>
  </div>
</header>

{#if showSettingsModal}
  <SettingsModal onClose={() => (showSettingsModal = false)} />
{/if}

<style>
  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background-color: transparent;
    flex-shrink: 0;
    gap: 12px;
    -webkit-app-region: drag;
  }

  /* Make all interactive elements non-draggable */
  .top-bar button {
    -webkit-app-region: no-drag;
  }

  .section {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-left {
    flex: 1;
    justify-content: flex-start;
  }

  .section-center {
    flex: 0 0 auto;
  }

  .section-right {
    flex: 1;
    justify-content: flex-end;
  }

  /* Icon buttons */
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      color 0.1s,
      background-color 0.1s;
  }

  .icon-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  /* Shortcuts picker */
  .shortcuts-picker {
    position: relative;
  }

  .shortcuts-btn {
    padding: 5px;
    background: var(--bg-primary);
    border-radius: 6px;
  }

  .shortcuts-btn:hover,
  .shortcuts-btn.open {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Settings button */
  .settings-btn {
    padding: 5px;
    background: var(--bg-primary);
    border-radius: 6px;
  }

  .settings-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Theme picker */
  .theme-picker {
    position: relative;
  }

  .theme-btn {
    padding: 5px;
    background: var(--bg-primary);
    border-radius: 6px;
  }

  .theme-btn:hover,
  .theme-btn.open {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Action button (Commit, etc.) - icon only, label on hover */
  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    height: 24px;
    background: var(--bg-primary);
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    cursor: pointer;
    transition:
      background-color 0.1s,
      color 0.1s;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-btn :global(svg) {
    flex-shrink: 0;
  }

  /* AI Annotations reveal toggle */
  .reveal-btn.active {
    color: var(--ui-accent);
  }

  .reveal-btn.active:hover {
    color: var(--ui-accent);
  }
</style>
