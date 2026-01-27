<!--
  EmptyState.svelte - Empty state with planning interface
  
  Shows when there are no changes to review. Provides a planning interface
  where users can describe work they want to do, triggering an AI agent
  to create a plan that's displayed inline.
  
  Uses the plan store for state management - the plan is a first-class entity
  separate from the agent conversation used to create it.
-->
<script lang="ts">
  import { Sparkles, Loader2, RotateCcw, Play, Send } from 'lucide-svelte';
  import GitTreeAnimation from './GitTreeAnimation.svelte';
  import AgentSelector, { type AgentId } from './AgentSelector.svelte';
  import { agentState, sendMessage as sendAgentMessage, clearSession } from './stores/agent.svelte';
  import {
    planState,
    startDrafting,
    updatePlanContent,
    markPlanReady,
    startRefining,
    markRefiningComplete,
    startImplementing,
    markImplementationComplete,
    setImplementAgent,
    setPlanError,
    clearPlan,
  } from './stores/plan.svelte';
  import { repoState } from './stores/repoState.svelte';
  import { getActiveTab } from './stores/tabState.svelte';
  import { renderMarkdown } from './services/markdown';

  // Local UI state
  let planDescription = $state('');
  let commentInput = $state('');
  let selectedAgent = $state<AgentId>('goose');

  // Derived state from plan store - read directly from planState for reactivity
  let plan = $derived(planState.plan);
  let implementAgent = $derived(planState.plan?.implementAgent ?? 'goose');

  // Plan content: prefer plan store, but during streaming also check agent messages
  let planContent = $derived.by(() => {
    const storedContent = planState.plan?.content ?? '';
    const status = planState.plan?.status;

    // During drafting/refining, also check agent messages for latest content
    if (status === 'drafting' || status === 'refining') {
      const assistantMessages = agentState.messages.filter((m) => m.role === 'assistant');
      if (assistantMessages.length > 0) {
        const latestFromAgent = assistantMessages[assistantMessages.length - 1].content;
        // Use whichever is longer (agent might have more recent content)
        if (latestFromAgent.length > storedContent.length) {
          return latestFromAgent;
        }
      }
    }
    return storedContent;
  });

  let planStatus = $derived(planState.plan?.status ?? null);

  // UI state derivations
  let isLoading = $derived(
    planState.plan?.status === 'drafting' ||
      planState.plan?.status === 'refining' ||
      planState.plan?.status === 'implementing'
  );
  let showPlan = $derived(planState.plan !== null && planState.plan?.status === 'ready');
  let isImplementing = $derived(planState.plan?.status === 'implementing');

  // Loading message based on plan status
  let loadingMessage = $derived.by(() => {
    switch (planStatus) {
      case 'drafting':
        return { title: 'Planning your work...', subtitle: 'Analyzing the codebase' };
      case 'refining':
        return { title: 'Updating the plan...', subtitle: 'Incorporating your feedback' };
      case 'implementing':
        return { title: 'Implementing the plan...', subtitle: 'Making code changes' };
      default:
        return { title: 'Working...', subtitle: '' };
    }
  });

  // Track if we've seen streaming start - prevents false transitions on mount
  let hasSeenStreaming = $state(false);

  // Debug: log state changes
  $effect(() => {
    console.log(
      '[EmptyState] Plan status:',
      planState.plan?.status,
      'isStreaming:',
      agentState.isStreaming,
      'hasSeenStreaming:',
      hasSeenStreaming
    );
  });

  // Watch agent streaming to update plan content
  // We track messages length and isStreaming to ensure reactivity
  $effect(() => {
    const isStreaming = agentState.isStreaming;
    const messagesLength = agentState.messages.length;
    const status = planState.plan?.status;

    if (isStreaming) {
      hasSeenStreaming = true;
    }

    // When agent is streaming and we're in a planning state, update plan content
    if (isStreaming && (status === 'drafting' || status === 'refining')) {
      const assistantMessages = agentState.messages.filter((m) => m.role === 'assistant');
      if (assistantMessages.length > 0) {
        const latestContent = assistantMessages[assistantMessages.length - 1].content;
        console.log('[EmptyState] Updating plan content, length:', latestContent.length);
        updatePlanContent(latestContent);
      }
    }
  });

  // Watch for streaming completion to transition plan status
  // Only transition if we've actually seen streaming happen (not on initial mount)
  $effect(() => {
    const isStreaming = agentState.isStreaming;
    const currentPlan = planState.plan;
    const seenStreaming = hasSeenStreaming;

    if (!isStreaming && currentPlan && seenStreaming) {
      console.log('[EmptyState] Streaming complete, transitioning from:', currentPlan.status);
      if (currentPlan.status === 'drafting') {
        markPlanReady();
        hasSeenStreaming = false;
      } else if (currentPlan.status === 'refining') {
        markRefiningComplete();
        hasSeenStreaming = false;
      } else if (currentPlan.status === 'implementing') {
        markImplementationComplete();
        hasSeenStreaming = false;
      }
    }
  });

  // Watch for agent errors
  $effect(() => {
    const error = agentState.error;
    const currentPlan = planState.plan;

    if (error && currentPlan) {
      setPlanError(error);
    }
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!planDescription.trim() || isLoading) return;
    if (!repoState.currentPath) {
      console.error('No repository path available');
      return;
    }

    const activeTab = getActiveTab();
    const tabAgentState = activeTab?.agentState;

    // Start drafting in plan store
    startDrafting(planDescription.trim(), selectedAgent);

    // Create a planning-focused prompt
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
    await sendAgentMessage(planningPrompt, repoState.currentPath, selectedAgent, tabAgentState);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit(e);
    }
  }

  function handleCommentKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleAddComment();
    }
  }

  function handleStartOver() {
    clearPlan();
    clearSession();
    planDescription = '';
    commentInput = '';
  }

  async function handleAddComment() {
    if (!commentInput.trim() || isLoading || !plan) return;
    if (!repoState.currentPath) return;

    const activeTab = getActiveTab();
    const tabAgentState = activeTab?.agentState;

    // Transition to refining state
    startRefining();

    const refinementPrompt = `Based on the plan here, please incorporate this feedback and return an updated plan:

${commentInput.trim()}

Return the complete updated plan as markdown, keeping the same structure (Overview, Files to Modify, Files to Create, Implementation Steps, Considerations). Don't make any code changes yet.`;

    commentInput = '';
    await sendAgentMessage(
      refinementPrompt,
      repoState.currentPath,
      plan.planningAgent,
      tabAgentState
    );
  }

  function handleImplementAgentChange(agent: AgentId) {
    setImplementAgent(agent);
  }

  async function handleImplement() {
    if (isLoading || !plan) return;
    if (!repoState.currentPath) return;

    const activeTab = getActiveTab();
    const tabAgentState = activeTab?.agentState;

    // Capture the plan content before any state changes
    const planContentToImplement = plan.content;
    const agentToUse = plan.implementAgent;

    // Transition to implementing state BEFORE clearing session
    startImplementing();

    // Clear the agent session to start fresh for implementation
    clearSession();
    if (tabAgentState) {
      tabAgentState.sessionId = null;
      tabAgentState.agentId = null;
      tabAgentState.status = 'disconnected';
      tabAgentState.messages = [];
      tabAgentState.currentMessageId = null;
      tabAgentState.currentToolCall = null;
      tabAgentState.error = null;
    }

    // Build the implement prompt with the captured plan content
    const implementPrompt = `I have a plan that I'd like you to implement. Here's the plan:

${planContentToImplement}

Please implement this plan. Make the necessary code changes following the implementation steps outlined above. Don't try to start any server or apps, just make the code changes as per the plan. And lint/format the code as needed.`;

    await sendAgentMessage(implementPrompt, repoState.currentPath, agentToUse, tabAgentState);
  }
</script>

<div class="empty-state-container">
  {#if isImplementing}
    <!-- Implementing loader - shows plan with overlay -->
    <div class="plan-view implementing">
      <div class="plan-header">
        <h2 class="plan-title">
          <Sparkles size={20} />
          Your Plan
        </h2>
      </div>
      <div class="plan-content faded">
        {@html renderMarkdown(planContent)}
      </div>
      <div class="implementing-overlay">
        <Loader2 size={24} class="spinning" />
        <p class="loading-text">{loadingMessage.title}</p>
        <p class="loading-subtext">{loadingMessage.subtitle}</p>
      </div>
    </div>
  {:else if showPlan}
    <!-- Plan display view -->
    <div class="plan-view">
      <div class="plan-header">
        <h2 class="plan-title">
          <Sparkles size={20} />
          Your Plan
        </h2>
      </div>
      <div class="plan-content">
        {@html renderMarkdown(planContent)}
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
            disabled={isLoading}
          />
          <button
            class="send-btn"
            onclick={handleAddComment}
            disabled={isLoading || !commentInput.trim()}
            title="Send feedback"
          >
            <Send size={14} />
          </button>
        </div>
        <div class="implement-wrapper">
          <AgentSelector
            value={implementAgent}
            onchange={handleImplementAgentChange}
            disabled={isLoading}
          />
          <button
            class="action-btn primary"
            onclick={handleImplement}
            disabled={isLoading}
            title="Implement this plan"
          >
            <Play size={14} />
            <span>Implement</span>
          </button>
        </div>
      </div>
    </div>
  {:else if isLoading}
    <!-- Loading state (drafting or refining) -->
    <GitTreeAnimation />
    <div class="planning-section">
      <div class="loading-state">
        <Loader2 size={24} class="spinning" />
        <p class="loading-text">{loadingMessage.title}</p>
        <p class="loading-subtext">{loadingMessage.subtitle}</p>
        {#if planContent}
          <div class="streaming-preview">
            {@html renderMarkdown(planContent)}
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <!-- Initial planning input view -->
    <GitTreeAnimation />

    <div class="planning-section">
      <p class="title">Ready to start something new?</p>
      <p class="subtitle">Describe what you want to build and let AI help you plan</p>

      <form class="planning-form" onsubmit={handleSubmit}>
        <textarea
          class="planning-input"
          placeholder="Describe the feature or change you want to make..."
          bind:value={planDescription}
          onkeydown={handleKeydown}
          rows="3"
        ></textarea>

        <div class="form-actions">
          <AgentSelector value={selectedAgent} onchange={(agent) => (selectedAgent = agent)} />

          <button type="submit" class="planning-submit" disabled={!planDescription.trim()}>
            <Sparkles size={16} />
            <span>Plan with AI</span>
          </button>
        </div>
      </form>

      <p class="hint">Press <kbd>⌘</kbd><kbd>Enter</kbd> to submit</p>
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
    width: 100%;
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

  .streaming-preview {
    margin-top: 16px;
    padding: 16px;
    background: var(--bg-primary);
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
    text-align: left;
    max-height: 300px;
    overflow-y: auto;
    width: 100%;
    font-size: var(--size-sm);
    color: var(--text-primary);
    line-height: 1.6;
  }

  /* Plan display view */
  .plan-view {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    max-width: 800px;
    overflow: hidden;
    position: relative;
  }

  .plan-view.implementing .plan-header {
    opacity: 0.5;
  }

  .plan-content.faded {
    opacity: 0.3;
    pointer-events: none;
  }

  .implementing-overlay {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 32px;
    background: var(--bg-primary);
    border-radius: 12px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.15);
    z-index: 10;
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
  .plan-content :global(h2),
  .streaming-preview :global(h2) {
    font-size: var(--size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin: 24px 0 12px 0;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .plan-content :global(h2:first-child),
  .streaming-preview :global(h2:first-child) {
    margin-top: 0;
  }

  .plan-content :global(h3),
  .streaming-preview :global(h3) {
    font-size: var(--size-sm);
    font-weight: 600;
    color: var(--text-primary);
    margin: 16px 0 8px 0;
  }

  .plan-content :global(p),
  .streaming-preview :global(p) {
    margin: 0 0 12px 0;
  }

  .plan-content :global(ul),
  .plan-content :global(ol),
  .streaming-preview :global(ul),
  .streaming-preview :global(ol) {
    margin: 0 0 12px 0;
    padding-left: 24px;
  }

  .plan-content :global(li),
  .streaming-preview :global(li) {
    margin: 4px 0;
  }

  .plan-content :global(code),
  .streaming-preview :global(code) {
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 0.9em;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .plan-content :global(pre),
  .streaming-preview :global(pre) {
    margin: 12px 0;
    padding: 12px;
    border-radius: 6px;
    background: var(--bg-hover);
    overflow-x: auto;
  }

  .plan-content :global(pre code),
  .streaming-preview :global(pre code) {
    padding: 0;
    background: none;
  }

  .plan-content :global(strong),
  .streaming-preview :global(strong) {
    font-weight: 600;
    color: var(--text-primary);
  }

  .plan-content :global(a),
  .streaming-preview :global(a) {
    color: var(--text-accent);
    text-decoration: none;
  }

  .plan-content :global(a:hover),
  .streaming-preview :global(a:hover) {
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

  /* Form actions row */
  .form-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  /* Implement button with agent selector */
  .implement-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
</style>
