<!--
  NewArtifactCard.svelte - Card for creating new artifacts

  This is a placeholder for the AI prompt interface that will
  eventually appear here. For now it's a simple button.
-->
<script lang="ts">
  import { Plus, Sparkles } from 'lucide-svelte';

  interface Props {
    onNewArtifact?: () => void;
  }

  let { onNewArtifact }: Props = $props();
</script>

<button class="new-artifact-card" onclick={onNewArtifact}>
  <div class="icon">
    <Sparkles size={20} strokeWidth={1.5} />
  </div>
  <div class="content">
    <span class="label">New Artifact</span>
    <span class="hint">Ask AI to research, plan, or code</span>
  </div>
</button>

<style>
  .new-artifact-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 24px 16px;
    min-height: 120px;
    background-color: transparent;
    border: 1px dashed var(--border-subtle);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .new-artifact-card:hover {
    border-color: var(--text-accent);
    background-color: rgba(88, 166, 255, 0.05);
  }

  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 10px;
    background-color: var(--bg-elevated);
    color: var(--text-accent);
    transition: all 0.15s ease;
  }

  .new-artifact-card:hover .icon {
    background-color: var(--text-accent);
    color: var(--bg-deepest);
  }

  .content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .label {
    font-size: var(--size-md);
    font-weight: 500;
    color: var(--text-primary);
  }

  .hint {
    font-size: var(--size-sm);
    color: var(--text-faint);
  }
</style>
