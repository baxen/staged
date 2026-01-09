<!-- SmartDiffView.svelte - Natural language diff view with code toggle -->
<script lang="ts">
  import { Loader2, Code } from 'lucide-svelte';
  import type { FileDiff } from './types';
  import { smartDiffState } from './stores/smartDiff.svelte';

  interface Props {
    diff: FileDiff | null;
  }

  let { diff }: Props = $props();

  let description = $state<{ before: string; after: string } | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let showCode = $state(false);

  let filePath = $derived(diff?.after?.path ?? diff?.before?.path ?? 'Unknown file');

  // Load description when diff changes - check cache first
  $effect(() => {
    if (diff && filePath) {
      // Check cache first
      const cached = smartDiffState.getDescription(filePath);
      if (cached) {
        description = cached;
      } else {
        loadDescription();
      }
    }
  });

  async function loadDescription() {
    if (!diff) return;

    loading = true;
    error = null;

    try {
      const result = await smartDiffState.calculateForFile(diff);
      description = result;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // Extract changed lines for code view
  let beforeLines = $derived.by(() => {
    if (!diff?.before?.content || diff.before.content.type !== 'text') return [];
    const lines = diff.before.content.lines;
    const changed = diff.alignments.filter((a) => a.changed);
    let result: { lineNum: number; text: string }[] = [];
    for (const a of changed) {
      for (let i = a.before.start; i < a.before.end; i++) {
        result.push({ lineNum: i + 1, text: lines[i] ?? '' });
      }
    }
    return result;
  });

  let afterLines = $derived.by(() => {
    if (!diff?.after?.content || diff.after.content.type !== 'text') return [];
    const lines = diff.after.content.lines;
    const changed = diff.alignments.filter((a) => a.changed);
    let result: { lineNum: number; text: string }[] = [];
    for (const a of changed) {
      for (let i = a.after.start; i < a.after.end; i++) {
        result.push({ lineNum: i + 1, text: lines[i] ?? '' });
      }
    }
    return result;
  });
</script>

<div class="smart-diff-view">
  <div class="smart-diff-header">
    <span class="file-path">{filePath}</span>
    <button
      class="code-toggle"
      class:active={showCode}
      onclick={() => (showCode = !showCode)}
      title={showCode ? 'Hide code' : 'Show code'}
    >
      <Code size={14} />
      <span>Code</span>
    </button>
  </div>

  {#if loading}
    <div class="smart-diff-loading">
      <Loader2 size={24} class="spinning" />
      <span>Analyzing changes...</span>
    </div>
  {:else if error}
    <div class="smart-diff-error">
      <span>Failed to analyze: {error}</span>
    </div>
  {:else if description}
    <div class="smart-diff-content">
      <div class="smart-diff-pane before">
        <div class="pane-label">Before</div>
        <div class="pane-description">{description.before}</div>
        {#if showCode && beforeLines.length > 0}
          <div class="pane-code">
            {#each beforeLines as line}
              <div class="code-line">
                <span class="line-num">{line.lineNum}</span>
                <span class="line-text">{line.text}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
      <div class="smart-diff-divider"></div>
      <div class="smart-diff-pane after">
        <div class="pane-label">After</div>
        <div class="pane-description">{description.after}</div>
        {#if showCode && afterLines.length > 0}
          <div class="pane-code">
            {#each afterLines as line}
              <div class="code-line">
                <span class="line-num">{line.lineNum}</span>
                <span class="line-text">{line.text}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="smart-diff-empty">
      <span>No description available</span>
    </div>
  {/if}
</div>

<style>
  .smart-diff-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    border-radius: 8px;
    overflow: hidden;
  }

  .smart-diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-chrome);
  }

  .file-path {
    font-family: 'SF Mono', 'Menlo', monospace;
    font-size: var(--size-sm);
    color: var(--text-primary);
  }

  .code-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--bg-primary);
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: var(--size-xs);
    cursor: pointer;
    transition: all 0.15s;
  }

  .code-toggle:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .code-toggle.active {
    background: var(--ui-accent);
    color: white;
  }

  .code-toggle.active :global(svg) {
    color: white;
  }

  .smart-diff-loading,
  .smart-diff-error,
  .smart-diff-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    flex: 1;
    color: var(--text-muted);
    font-size: var(--size-sm);
  }

  .smart-diff-loading :global(svg) {
    animation: spin 1s linear infinite;
    color: var(--ui-accent);
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .smart-diff-error {
    color: var(--status-deleted);
  }

  .smart-diff-content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .smart-diff-pane {
    flex: 1;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
  }

  .smart-diff-pane.before {
    background: color-mix(in srgb, var(--status-deleted) 5%, var(--bg-primary));
  }

  .smart-diff-pane.after {
    background: color-mix(in srgb, var(--status-added) 5%, var(--bg-primary));
  }

  .pane-label {
    font-size: var(--size-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .before .pane-label {
    color: var(--status-deleted);
  }

  .after .pane-label {
    color: var(--status-added);
  }

  .pane-description {
    font-size: var(--size-base);
    line-height: 1.6;
    color: var(--text-primary);
  }

  .pane-code {
    margin-top: 8px;
    padding: 8px;
    background: var(--bg-chrome);
    border-radius: 6px;
    font-family: 'SF Mono', 'Menlo', monospace;
    font-size: var(--size-xs);
    overflow-x: auto;
  }

  .code-line {
    display: flex;
    line-height: 1.5;
  }

  .line-num {
    color: var(--text-faint);
    min-width: 3ch;
    padding-right: 8px;
    text-align: right;
    user-select: none;
  }

  .line-text {
    color: var(--text-primary);
    white-space: pre;
  }

  .smart-diff-divider {
    width: 1px;
    background: var(--border-muted);
  }
</style>
