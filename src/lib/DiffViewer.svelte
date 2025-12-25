<script lang="ts">
  import { onMount } from 'svelte';
  import type { FileDiff, DiffRow } from './types';
  import {
    initHighlighter,
    highlightLine,
    detectLanguage,
    prepareLanguage,
    getTheme,
    type Token,
  } from './services/highlighter';
  import { createScrollSync, isLine, isCollapse } from './services/diffScroll';
  import CollapseIndicator from './components/CollapseIndicator.svelte';

  interface Props {
    diff: FileDiff | null;
  }

  let { diff }: Props = $props();

  let leftPane: HTMLDivElement | null = $state(null);
  let rightPane: HTMLDivElement | null = $state(null);
  let highlighterReady = $state(false);
  let languageReady = $state(false);
  let themeBg = $state('#1e1e1e');

  // Scroll sync controller
  const scrollSync = createScrollSync();

  // Detect language from file path
  let language = $derived(diff ? detectLanguage(diff.path) : null);

  onMount(async () => {
    await initHighlighter('github-dark');
    const theme = getTheme();
    if (theme) {
      themeBg = theme.bg;
    }
    highlighterReady = true;
  });

  // Load language when file changes
  $effect(() => {
    if (highlighterReady && diff) {
      languageReady = false;
      prepareLanguage(diff.path).then((ready) => {
        languageReady = ready;
      });
    }
  });

  function getTokens(content: string): Token[] {
    if (!highlighterReady || !languageReady) {
      return [{ content, color: '#d4d4d4' }];
    }
    return highlightLine(content, language);
  }

  function formatLineNumber(num: number | null | undefined): string {
    return num != null ? String(num) : '';
  }

  function handleLeftScroll(e: Event) {
    if (!diff) return;
    const target = e.target as HTMLDivElement;
    scrollSync.sync(target, rightPane, 'left', diff.old_content, diff.new_content);
  }

  function handleRightScroll(e: Event) {
    if (!diff) return;
    const target = e.target as HTMLDivElement;
    scrollSync.sync(target, leftPane, 'right', diff.old_content, diff.new_content);
  }
</script>

<div class="diff-viewer">
  {#if diff === null}
    <div class="empty-state">
      <p>Select a file to view changes</p>
    </div>
  {:else if diff.is_binary}
    <div class="diff-header">
      <span class="file-path">{diff.path}</span>
    </div>
    <div class="binary-notice">
      <p>Binary file - cannot display diff</p>
    </div>
  {:else}
    <div class="diff-header">
      <span class="file-path">
        {#if diff.old_path}
          <span class="old-path">{diff.old_path}</span>
          <span class="arrow">→</span>
        {/if}
        {diff.path}
      </span>
    </div>

    <div class="diff-content">
      <!-- Left pane: Original -->
      <div class="diff-pane left-pane">
        <div class="pane-header">Original</div>
        <div
          class="code-container"
          bind:this={leftPane}
          onscroll={handleLeftScroll}
          style="background-color: {themeBg}"
        >
          {#each diff.old_content as row}
            {#if isLine(row)}
              <div class="line" class:line-removed={row.line_type === 'removed'}>
                <span class="line-number" class:gutter-removed={row.line_type === 'removed'}>
                  {formatLineNumber(row.old_lineno)}
                </span>
                <span class="line-content" class:content-removed={row.line_type === 'removed'}>
                  {#each getTokens(row.content) as token}
                    <span style="color: {token.color}">{token.content}</span>
                  {/each}
                </span>
              </div>
            {:else if isCollapse(row)}
              <CollapseIndicator count={row.count} startLine={row.start_line} />
            {/if}
          {/each}
          {#if diff.old_content.length === 0}
            <div class="empty-file-notice">New file</div>
          {/if}
        </div>
      </div>

      <!-- Right pane: Modified -->
      <div class="diff-pane right-pane">
        <div class="pane-header">Modified</div>
        <div
          class="code-container"
          bind:this={rightPane}
          onscroll={handleRightScroll}
          style="background-color: {themeBg}"
        >
          {#each diff.new_content as row}
            {#if isLine(row)}
              <div class="line" class:line-added={row.line_type === 'added'}>
                <span class="line-number" class:gutter-added={row.line_type === 'added'}>
                  {formatLineNumber(row.new_lineno)}
                </span>
                <span class="line-content" class:content-added={row.line_type === 'added'}>
                  {#each getTokens(row.content) as token}
                    <span style="color: {token.color}">{token.content}</span>
                  {/each}
                </span>
              </div>
            {:else if isCollapse(row)}
              <CollapseIndicator count={row.count} startLine={row.start_line} />
            {/if}
          {/each}
          {#if diff.new_content.length === 0}
            <div class="empty-file-notice">File deleted</div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .diff-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .empty-state,
  .binary-notice {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 14px;
  }

  .diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    background-color: var(--diff-header-bg);
    border-bottom: 1px solid var(--border-primary);
  }

  .file-path {
    font-family: monospace;
    font-size: 13px;
    color: var(--status-modified);
  }

  .old-path {
    color: var(--text-muted);
    text-decoration: line-through;
  }

  .arrow {
    margin: 0 8px;
    color: var(--text-muted);
  }

  .diff-content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .diff-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .left-pane {
    border-right: 1px solid var(--border-primary);
  }

  .pane-header {
    padding: 6px 12px;
    font-size: 11px;
    text-transform: uppercase;
    color: var(--text-muted);
    background-color: var(--diff-header-bg);
    border-bottom: 1px solid var(--border-primary);
  }

  .code-container {
    flex: 1;
    overflow: auto;
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 13px;
    line-height: 1.5;
  }

  .empty-file-notice {
    padding: 20px;
    color: var(--text-muted);
    font-style: italic;
  }

  .line {
    display: flex;
    min-height: 20px;
  }

  /* Line number (gutter) styling */
  .line-number {
    width: 50px;
    padding: 0 12px;
    text-align: right;
    color: var(--diff-line-number);
    user-select: none;
    flex-shrink: 0;
  }

  .gutter-added {
    background-color: var(--diff-added-gutter);
    color: var(--diff-added-text);
  }

  .gutter-removed {
    background-color: var(--diff-removed-gutter);
    color: var(--diff-removed-text);
  }

  /* Line content styling */
  .line-content {
    flex: 1;
    padding: 0 12px;
    white-space: pre;
  }

  /* Overlay tints for diff highlighting */
  .content-added {
    background-color: var(--diff-added-overlay);
  }

  .content-removed {
    background-color: var(--diff-removed-overlay);
  }
</style>
