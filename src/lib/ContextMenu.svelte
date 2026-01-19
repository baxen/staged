<!--
  ContextMenu.svelte - Context menu for code navigation

  Shows "Go to Definition" and other actions when right-clicking on code.
-->
<script lang="ts">
  import { Navigation, Copy, X } from 'lucide-svelte';

  interface MenuItem {
    id: string;
    label: string;
    shortcut?: string;
    icon?: typeof Navigation;
    disabled?: boolean;
    action: () => void;
  }

  interface Props {
    /** X position of the menu */
    x: number;
    /** Y position of the menu */
    y: number;
    /** Symbol name being targeted (shown in menu) */
    symbolName?: string;
    /** Whether to show the "Go to Definition" option */
    canGoToDefinition?: boolean;
    /** Called when "Go to Definition" is clicked */
    onGoToDefinition?: () => void;
    /** Called when "Copy Symbol Name" is clicked */
    onCopySymbol?: () => void;
    /** Called when the menu should close */
    onClose: () => void;
  }

  let {
    x,
    y,
    symbolName,
    canGoToDefinition = true,
    onGoToDefinition,
    onCopySymbol,
    onClose,
  }: Props = $props();

  // Build menu items based on available actions
  const menuItems: MenuItem[] = $derived.by(() => {
    const items: MenuItem[] = [];

    if (canGoToDefinition && symbolName && onGoToDefinition) {
      items.push({
        id: 'go-to-definition',
        label: 'Go to Definition',
        shortcut: '\u2318Click',
        icon: Navigation,
        action: onGoToDefinition,
      });
    }

    if (symbolName && onCopySymbol) {
      items.push({
        id: 'copy-symbol',
        label: 'Copy Symbol Name',
        icon: Copy,
        action: onCopySymbol,
      });
    }

    return items;
  });

  // Adjust position to keep menu on screen
  let menuElement: HTMLDivElement | null = $state(null);
  let adjustedX = $state(0);
  let adjustedY = $state(0);

  // Update adjusted positions when props or menu size change
  $effect(() => {
    // Start with prop values
    let newX = x;
    let newY = y;

    if (menuElement) {
      const rect = menuElement.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      // Adjust X if menu would overflow right edge
      if (x + rect.width > viewportWidth - 10) {
        newX = viewportWidth - rect.width - 10;
      }

      // Adjust Y if menu would overflow bottom edge
      if (y + rect.height > viewportHeight - 10) {
        newY = y - rect.height;
      }
    }

    adjustedX = newX;
    adjustedY = newY;
  });

  function handleItemClick(item: MenuItem) {
    if (!item.disabled) {
      item.action();
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop to catch clicks outside the menu -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="context-menu-backdrop" onclick={handleBackdropClick}>
  <div
    bind:this={menuElement}
    class="context-menu"
    style="left: {adjustedX}px; top: {adjustedY}px;"
    role="menu"
  >
    {#if symbolName}
      <div class="menu-header">
        <span class="symbol-name">{symbolName}</span>
      </div>
    {/if}

    {#if menuItems.length > 0}
      {#each menuItems as item (item.id)}
        <button
          class="menu-item"
          class:disabled={item.disabled}
          onclick={() => handleItemClick(item)}
          role="menuitem"
          disabled={item.disabled}
        >
          {#if item.icon}
            {@const Icon = item.icon}
            <Icon size={14} />
          {/if}
          <span class="menu-item-label">{item.label}</span>
          {#if item.shortcut}
            <span class="menu-item-shortcut">{item.shortcut}</span>
          {/if}
        </button>
      {/each}
    {:else}
      <div class="menu-empty">No actions available</div>
    {/if}
  </div>
</div>

<style>
  .context-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
  }

  .context-menu {
    position: fixed;
    min-width: 180px;
    max-width: 280px;
    background-color: var(--bg-chrome);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    padding: 4px;
    z-index: 1001;
  }

  .menu-header {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-primary);
    margin-bottom: 4px;
  }

  .symbol-name {
    font-family: var(--font-mono);
    font-size: var(--size-xs);
    color: var(--text-muted);
    word-break: break-all;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--text-primary);
    font-size: var(--size-sm);
    text-align: left;
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .menu-item:hover:not(.disabled) {
    background-color: var(--bg-hover);
  }

  .menu-item.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .menu-item-label {
    flex: 1;
  }

  .menu-item-shortcut {
    font-size: var(--size-xs);
    color: var(--text-faint);
  }

  .menu-empty {
    padding: 8px 10px;
    font-size: var(--size-sm);
    color: var(--text-faint);
    text-align: center;
  }
</style>
