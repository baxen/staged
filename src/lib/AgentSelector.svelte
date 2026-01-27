<!--
  AgentSelector.svelte - Dropdown selector for AI agents
  
  Reusable component for selecting between Goose and Claude agents.
  Used in both the planning empty state and the chat panel.
-->
<script lang="ts">
  import { Bot, ChevronDown } from 'lucide-svelte';

  export type AgentId = 'goose' | 'claude-code';

  interface Props {
    value: AgentId;
    onchange: (agent: AgentId) => void;
    disabled?: boolean;
  }

  let { value, onchange, disabled = false }: Props = $props();

  const agentLabels: Record<AgentId, string> = {
    goose: 'Goose',
    'claude-code': 'Claude',
  };

  const agentDescriptions: Record<AgentId, string> = {
    goose: "Block's open-source AI developer agent",
    'claude-code': "Anthropic's AI coding assistant",
  };

  let dropdownOpen = $state(false);
  let dropdownEl: HTMLDivElement | undefined = $state();

  // Close dropdown when clicking outside
  $effect(() => {
    if (!dropdownOpen) return;

    function handleClickOutside(e: MouseEvent) {
      if (dropdownEl && !dropdownEl.contains(e.target as Node)) {
        dropdownOpen = false;
      }
    }

    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  });

  function selectAgent(agent: AgentId) {
    onchange(agent);
    dropdownOpen = false;
  }
</script>

<div class="dropdown" bind:this={dropdownEl}>
  <button
    type="button"
    class="dropdown-trigger"
    class:open={dropdownOpen}
    onclick={() => (dropdownOpen = !dropdownOpen)}
    title={agentDescriptions[value]}
    {disabled}
  >
    <Bot size={14} />
    <span class="dropdown-label">{agentLabels[value]}</span>
    <ChevronDown size={12} />
  </button>
  {#if dropdownOpen}
    <div class="dropdown-menu">
      {#each Object.entries(agentLabels) as [agent, label]}
        <button
          type="button"
          class="dropdown-option"
          class:selected={value === agent}
          onclick={() => selectAgent(agent as AgentId)}
        >
          <span class="dropdown-option-label">{label}</span>
          <span class="dropdown-option-desc">{agentDescriptions[agent as AgentId]}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dropdown {
    position: relative;
  }

  .dropdown-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: var(--bg-primary);
    border: none;
    border-radius: 6px;
    color: var(--text-primary);
    font-size: var(--size-xs);
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .dropdown-trigger:hover:not(:disabled),
  .dropdown-trigger.open {
    background: var(--bg-hover);
  }

  .dropdown-trigger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dropdown-trigger :global(svg) {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .dropdown-trigger :global(svg:last-child) {
    transition: transform 0.15s;
  }

  .dropdown-trigger.open :global(svg:last-child) {
    transform: rotate(180deg);
  }

  .dropdown-label {
    font-weight: 500;
  }

  .dropdown-menu {
    position: absolute;
    bottom: 100%;
    left: 0;
    margin-bottom: 4px;
    min-width: 220px;
    background: var(--bg-chrome);
    border: 1px solid var(--border-muted);
    border-radius: 8px;
    box-shadow: var(--shadow-elevated);
    overflow: hidden;
    z-index: 100;
  }

  .dropdown-option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: none;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    transition: background-color 0.1s;
  }

  .dropdown-option:hover {
    background-color: var(--bg-hover);
  }

  .dropdown-option.selected {
    background-color: var(--bg-primary);
  }

  .dropdown-option-label {
    font-size: var(--size-xs);
    font-weight: 500;
    color: var(--text-primary);
  }

  .dropdown-option-desc {
    font-size: calc(var(--size-xs) - 1px);
    color: var(--text-faint);
  }
</style>
