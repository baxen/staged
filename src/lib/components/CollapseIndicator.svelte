<script lang="ts">
  /**
   * CollapseIndicator - IntelliJ-style chevron showing collapsed lines
   *
   * Displays a visual indicator that N lines exist in the other pane
   * but are not shown in this pane. Clicking could expand to show context.
   */

  interface Props {
    /** Number of lines collapsed */
    count: number;
    /** Optional: starting line number in the other pane */
    startLine?: number;
    /** Optional: click handler for expansion */
    onClick?: () => void;
  }

  let { count, startLine, onClick }: Props = $props();
</script>

<div
  class="collapse-indicator"
  onclick={onClick}
  onkeydown={(e) => e.key === 'Enter' && onClick?.()}
  role={onClick ? 'button' : undefined}
  tabindex={onClick ? 0 : undefined}
>
  <span class="collapse-chevron">»</span>
  <span class="collapse-count">{count}</span>
</div>

<style>
  .collapse-indicator {
    display: flex;
    align-items: center;
    height: 20px;
    padding: 0 8px;
    background: linear-gradient(
      to bottom,
      var(--bg-secondary) 0%,
      var(--bg-tertiary) 50%,
      var(--bg-secondary) 100%
    );
    border-top: 1px solid var(--border-primary);
    border-bottom: 1px solid var(--border-primary);
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    user-select: none;
  }

  .collapse-indicator:hover {
    background: var(--bg-tertiary);
  }

  .collapse-chevron {
    font-weight: bold;
    margin-right: 4px;
    color: var(--text-secondary);
  }

  .collapse-count {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
  }
</style>
