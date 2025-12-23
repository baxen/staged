<script lang="ts">
  // Placeholder diff data - will be replaced with real diff parsing
  let leftLines = [
    { num: 1, content: 'fn main() {', type: 'context' },
    { num: 2, content: '    println!("Hello");', type: 'removed' },
    { num: 3, content: '}', type: 'context' },
  ];

  let rightLines = [
    { num: 1, content: 'fn main() {', type: 'context' },
    { num: 2, content: '    println!("Hello, World!");', type: 'added' },
    { num: 3, content: '}', type: 'context' },
  ];

  let fileName = 'src/lib.rs';

  function getLineClass(type: string): string {
    switch (type) {
      case 'added': return 'line-added';
      case 'removed': return 'line-removed';
      default: return 'line-context';
    }
  }
</script>

<div class="diff-viewer">
  <div class="diff-header">
    <span class="file-path">{fileName}</span>
    <div class="diff-actions">
      <button class="action-btn" title="Stage file">Stage</button>
      <button class="action-btn" title="Discard changes">Discard</button>
    </div>
  </div>

  <div class="diff-content">
    <div class="diff-pane left-pane">
      <div class="pane-header">Original</div>
      <div class="code-container">
        {#each leftLines as line}
          <div class="line {getLineClass(line.type)}">
            <span class="line-number">{line.num}</span>
            <span class="line-content">{line.content}</span>
          </div>
        {/each}
      </div>
    </div>

    <div class="diff-pane right-pane">
      <div class="pane-header">Modified</div>
      <div class="code-container">
        {#each rightLines as line}
          <div class="line {getLineClass(line.type)}">
            <span class="line-number">{line.num}</span>
            <span class="line-content">{line.content}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .diff-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    background-color: #2d2d2d;
    border-bottom: 1px solid #3c3c3c;
  }

  .file-path {
    font-family: monospace;
    font-size: 13px;
    color: #e2c08d;
  }

  .diff-actions {
    display: flex;
    gap: 8px;
  }

  .action-btn {
    padding: 4px 12px;
    font-size: 12px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }

  .action-btn:hover {
    background-color: #1177bb;
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
    border-right: 1px solid #3c3c3c;
  }

  .pane-header {
    padding: 6px 12px;
    font-size: 11px;
    text-transform: uppercase;
    color: #888;
    background-color: #2d2d2d;
    border-bottom: 1px solid #3c3c3c;
  }

  .code-container {
    flex: 1;
    overflow: auto;
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 13px;
    line-height: 1.5;
  }

  .line {
    display: flex;
    min-height: 20px;
  }

  .line-number {
    width: 50px;
    padding: 0 12px;
    text-align: right;
    color: #6e7681;
    background-color: #1e1e1e;
    user-select: none;
    flex-shrink: 0;
  }

  .line-content {
    flex: 1;
    padding: 0 12px;
    white-space: pre;
  }

  .line-context {
    background-color: #1e1e1e;
  }

  .line-context .line-content {
    background-color: #1e1e1e;
  }

  .line-added {
    background-color: #2ea04326;
  }

  .line-added .line-number {
    background-color: #2ea04326;
    color: #7ee787;
  }

  .line-added .line-content {
    background-color: #2ea04326;
  }

  .line-removed {
    background-color: #f8514926;
  }

  .line-removed .line-number {
    background-color: #f8514926;
    color: #f85149;
  }

  .line-removed .line-content {
    background-color: #f8514926;
  }
</style>
