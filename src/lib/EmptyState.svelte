<!--
  EmptyState.svelte - Empty state with planning interface
  
  Shows when there are no changes to review. Provides a planning interface
  where users can describe work they want to do, triggering an AI agent
  to create a plan that's displayed inline.
-->
<script lang="ts">
  import { Sparkles, Loader2, RotateCcw, Play, Send } from 'lucide-svelte';
  import GitTreeAnimation from './GitTreeAnimation.svelte';
  import {
    agentState,
    sendMessage as sendAgentMessage,
    clearMessages,
  } from './stores/agent.svelte';
  import { repoState } from './stores/repoState.svelte';
  import { getActiveTab } from './stores/tabState.svelte';
  import { renderMarkdown } from './services/markdown';

  type ActionType = 'planning' | 'refining' | 'implementing';

  let planDescription = $state('');
  let commentInput = $state('');
  let currentAction = $state<ActionType | null>(null);
  let isSubmitting = $derived(agentState.isStreaming);

  // Reset action when streaming completes
  $effect(() => {
    if (!agentState.isStreaming && currentAction) {
      currentAction = null;
    }
  });

  // Infer action type from conversation context when we don't have explicit currentAction
  // This handles the case when switching tabs while an action is in progress
  let inferredAction = $derived.by((): ActionType => {
    // If we have an explicit action (just triggered), use it
    if (currentAction) return currentAction;

    // Otherwise, infer from the last user message
    const userMessages = agentState.messages.filter((m) => m.role === 'user');
    if (userMessages.length === 0) return 'planning';

    const lastUserMessage = userMessages[userMessages.length - 1].content;

    if (lastUserMessage.includes('please implement the plan')) {
      return 'implementing';
    } else if (lastUserMessage.includes('incorporate this feedback')) {
      return 'refining';
    }
    return 'planning';
  });

  // Loading message based on current or inferred action
  let loadingMessage = $derived.by(() => {
    const action = currentAction ?? inferredAction;
    switch (action) {
      case 'planning':
        return { title: 'Planning your work...', subtitle: 'Analyzing the codebase' };
      case 'refining':
        return { title: 'Updating the plan...', subtitle: 'Incorporating your feedback' };
      case 'implementing':
        return { title: 'Implementing the plan...', subtitle: 'Making code changes' };
      default:
        return { title: 'Working...', subtitle: '' };
    }
  });

  // Get the latest assistant message as the plan
  let planContent = $derived.by(() => {
    const assistantMessages = agentState.messages.filter((m) => m.role === 'assistant');
    if (assistantMessages.length === 0) return null;
    return assistantMessages[assistantMessages.length - 1].content;
  });

  // Show plan view when we have a plan and not currently streaming
  let showPlan = $derived(planContent && !isSubmitting);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!planDescription.trim() || isSubmitting) return;
    if (!repoState.currentPath) {
      console.error('No repository path available');
      return;
    }

    const activeTab = getActiveTab();
    const tabAgentState = activeTab?.agentState;

    // Create a planning-focused prompt that returns the plan as markdown
    const planningPrompt = `I want to plan some work in this repository. Here's what I want to accomplish:

${planDescription.trim()}

Please analyze the codebase and create a detailed plan. Return the plan as markdown with the following structure:

## Overview
Brief summary of the changes needed

## Files to Modify
List of existing files that will need changes

## Files to Create
List of new files that need to be created (if any)

## Implementation Steps
Step-by-step approach to implement this

## Considerations
Any potential risks, edge cases, or things to watch out for

Focus on planning only - don't make any code changes. Just return the plan as markdown.`;

    planDescription = '';
    currentAction = 'planning';
    await sendAgentMessage(planningPrompt, repoState.currentPath, 'goose', tabAgentState);
  }

  function handleKeydown(e: KeyboardEvent) {
    // Submit on Cmd/Ctrl+Enter
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit(e);
    }
  }

  function handleCommentKeydown(e: KeyboardEvent) {
    // Submit comment on Enter (without shift)
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleAddComment();
    }
  }

  function handleStartOver() {
    clearMessages();
    planDescription = '';
    commentInput = '';
  }

  async function handleAddComment() {
    if (!commentInput.trim() || isSubmitting) return;
    if (!repoState.currentPath) return;

    const activeTab = getActiveTab();
    const tabAgentState = activeTab?.agentState;

    // Send the comment as a refinement request
    const refinementPrompt = `Based on the plan above, please incorporate this feedback and return an updated plan:

${commentInput.trim()}

Return the complete updated plan as markdown, keeping the same structure (Overview, Files to Modify, Files to Create, Implementation Steps, Considerations). Don't make any code changes yet.`;

    commentInput = '';
    currentAction = 'refining';
    await sendAgentMessage(refinementPrompt, repoState.currentPath, 'goose', tabAgentState);
  }

  async function handleImplement() {
    if (isSubmitting) return;
    if (!repoState.currentPath) return;

    const activeTab = getActiveTab();
    const tabAgentState = activeTab?.agentState;

    // Send the implement command
    const implementPrompt = `Great, please implement the plan above. Make the necessary code changes following the implementation steps outlined in the plan.`;

    currentAction = 'implementing';
    await sendAgentMessage(implementPrompt, repoState.currentPath, 'goose', tabAgentState);
  }
</script>

<div class="empty-state-container">
  {#if showPlan}
    <!-- Plan display view -->
    <div class="plan-view">
      <div class="plan-header">
        <h2 class="plan-title">
          <Sparkles size={20} />
          Your Plan
        </h2>
      </div>
      <div class="plan-content">
        {@html renderMarkdown(planContent ?? '')}
      </div>
      <div class="plan-actions">
        <button class="action-btn secondary" onclick={handleStartOver} title="Start over">
          <RotateCcw size={14} />
          <span>Start Over</span>
        </button>
        <div class="comment-input-wrapper">
          <input
            type="text"
            class="comment-input"
            placeholder="Request changes to the plan..."
            bind:value={commentInput}
            onkeydown={handleCommentKeydown}
            disabled={isSubmitting}
          />
          <button
            class="send-btn"
            onclick={handleAddComment}
            disabled={isSubmitting || !commentInput.trim()}
            title="Send feedback"
          >
            <Send size={14} />
          </button>
        </div>
        <button
          class="action-btn primary"
          onclick={handleImplement}
          disabled={isSubmitting}
          title="Implement this plan"
        >
          <Play size={14} />
          <span>Implement</span>
        </button>
      </div>
    </div>
  {:else}
    <!-- Planning input view -->
    <GitTreeAnimation />

    <div class="planning-section">
      {#if isSubmitting}
        <div class="loading-state">
          <Loader2 size={24} class="spinning" />
          <p class="loading-text">{loadingMessage.title}</p>
          <p class="loading-subtext">{loadingMessage.subtitle}</p>
        </div>
      {:else}
        <p class="title">Ready to start something new?</p>
        <p class="subtitle">Describe what you want to build and let AI help you plan</p>

        <form class="planning-form" onsubmit={handleSubmit}>
          <textarea
            class="planning-input"
            placeholder="Describe the feature or change you want to make..."
            bind:value={planDescription}
            onkeydown={handleKeydown}
            disabled={isSubmitting}
            rows="3"
          ></textarea>

          <button
            type="submit"
            class="planning-submit"
            disabled={isSubmitting || !planDescription.trim()}
          >
            <Sparkles size={16} />
            <span>Plan with AI</span>
          </button>
        </form>

        <p class="hint">Press <kbd>⌘</kbd><kbd>Enter</kbd> to submit</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .empty-state-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    width: 100%;
    gap: 32px;
    padding: 24px;
    overflow: hidden;
  }

  /* Planning input view */
  .planning-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    max-width: 480px;
    width: 100%;
  }

  .title {
    font-size: var(--size-xl);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0 0 8px 0;
  }

  .subtitle {
    font-size: var(--size-md);
    color: var(--text-muted);
    margin: 0 0 24px 0;
  }

  .planning-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
  }

  .planning-input {
    width: 100%;
    padding: 12px 14px;
    border: 1px solid var(--border-muted);
    border-radius: 8px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: inherit;
    line-height: 1.5;
    resize: vertical;
    min-height: 80px;
    transition: border-color 0.15s ease;
  }

  .planning-input:focus {
    outline: none;
    border-color: var(--ui-accent);
  }

  .planning-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .planning-input::placeholder {
    color: var(--text-faint);
  }

  .planning-submit {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 20px;
    border: none;
    border-radius: 8px;
    background: var(--ui-accent);
    color: white;
    font-size: var(--size-sm);
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color 0.15s ease,
      opacity 0.15s ease;
  }

  .planning-submit:hover:not(:disabled) {
    background: var(--ui-accent-hover);
  }

  .planning-submit:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .hint {
    margin: 12px 0 0 0;
    font-size: var(--size-xs);
    color: var(--text-faint);
  }

  .hint kbd {
    display: inline-block;
    padding: 2px 5px;
    font-size: 10px;
    font-family: inherit;
    background: var(--bg-hover);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    margin: 0 1px;
  }

  /* Loading state */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .loading-text {
    font-size: var(--size-lg);
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
  }

  .loading-subtext {
    font-size: var(--size-sm);
    color: var(--text-muted);
    margin: 0;
  }

  /* Plan display view */
  .plan-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    max-width: 800px;
    overflow: hidden;
  }

  .plan-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0 16px 0;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .plan-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--size-lg);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .plan-title :global(svg) {
    color: var(--ui-accent);
  }

  .plan-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px 0;
    color: var(--text-primary);
    font-size: var(--size-sm);
    line-height: 1.6;
  }

  /* Plan actions bar */
  .plan-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 0 0 0;
    border-top: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: var(--size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .action-btn.secondary {
    background: var(--bg-primary);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
  }

  .action-btn.secondary:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--border-emphasis);
  }

  .action-btn.primary {
    background: var(--ui-accent);
    border: none;
    color: white;
  }

  .action-btn.primary:hover:not(:disabled) {
    background: var(--ui-accent-hover);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .comment-input-wrapper {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border-muted);
    border-radius: 6px;
    padding: 4px 4px 4px 12px;
    transition: border-color 0.15s ease;
  }

  .comment-input-wrapper:focus-within {
    border-color: var(--ui-accent);
  }

  .comment-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--size-sm);
    font-family: inherit;
    outline: none;
  }

  .comment-input::placeholder {
    color: var(--text-faint);
  }

  .comment-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 6px 8px;
    border: none;
    border-radius: 4px;
    background: var(--bg-hover);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .send-btn:hover:not(:disabled) {
    background: var(--ui-accent);
    color: white;
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Markdown styles for plan content */
  .plan-content :global(h2) {
    font-size: var(--size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin: 24px 0 12px 0;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .plan-content :global(h2:first-child) {
    margin-top: 0;
  }

  .plan-content :global(h3) {
    font-size: var(--size-sm);
    font-weight: 600;
    color: var(--text-primary);
    margin: 16px 0 8px 0;
  }

  .plan-content :global(p) {
    margin: 0 0 12px 0;
  }

  .plan-content :global(ul),
  .plan-content :global(ol) {
    margin: 0 0 12px 0;
    padding-left: 24px;
  }

  .plan-content :global(li) {
    margin: 4px 0;
  }

  .plan-content :global(code) {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 0.9em;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .plan-content :global(pre) {
    margin: 12px 0;
    padding: 12px;
    border-radius: 6px;
    background: var(--bg-hover);
    overflow-x: auto;
  }

  .plan-content :global(pre code) {
    padding: 0;
    background: none;
  }

  .plan-content :global(strong) {
    font-weight: 600;
    color: var(--text-primary);
  }

  .plan-content :global(a) {
    color: var(--text-accent);
    text-decoration: none;
  }

  .plan-content :global(a:hover) {
    text-decoration: underline;
  }

  /* Spinning animation for loader */
  :global(.spinning) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
