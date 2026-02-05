<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import {
    getPrForBranch,
    pushBranch,
    createPullRequest,
    updatePullRequest,
    type PullRequestInfo,
    type CreatePrResult,
    type Branch,
    type CommitInfo,
  } from './services/branch';
  import { openUrl } from './services/window';

  export let branch: Branch;
  export let commits: CommitInfo[] = [];

  const dispatch = createEventDispatcher<{
    close: void;
    created: { url: string; number: number };
  }>();

  // Form state
  let title = '';
  let body = '';
  let isDraft = false;

  // Loading/error state
  let isLoading = true;
  let isPushing = false;
  let isCreating = false;
  let error: string | null = null;

  // Existing PR (if any)
  let existingPr: PullRequestInfo | null = null;

  onMount(async () => {
    await checkExistingPr();
    populateDefaults();
  });

  async function checkExistingPr() {
    isLoading = true;
    error = null;
    try {
      existingPr = await getPrForBranch(branch.repoPath, branch.branchName);
      if (existingPr) {
        // Pre-fill with existing PR data
        title = existingPr.title;
        body = existingPr.body;
        isDraft = existingPr.draft;
      }
    } catch (e) {
      // No PR exists, that's fine
      existingPr = null;
    } finally {
      isLoading = false;
    }
  }

  function populateDefaults() {
    if (existingPr) return; // Already populated from existing PR

    // Default title: first commit subject or branch name
    if (commits.length > 0) {
      // Use first commit subject as title
      title = commits[0].subject;

      // Build body from remaining commits
      if (commits.length > 1) {
        const commitList = commits
          .slice(1)
          .map((c) => `- ${c.subject}`)
          .join('\n');
        body = `## Changes\n\n${commitList}`;
      }
    } else {
      // Fallback to branch name, converting kebab-case to title case
      title = branch.branchName.replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
    }
  }

  async function handleSubmit() {
    if (!title.trim()) {
      error = 'Title is required';
      return;
    }

    error = null;

    try {
      // Step 1: Push the branch
      isPushing = true;
      await pushBranch(branch.repoPath, branch.branchName, false);
      isPushing = false;

      // Step 2: Create or update PR
      isCreating = true;

      if (existingPr) {
        // Update existing PR
        await updatePullRequest(branch.repoPath, existingPr.number, title.trim(), body.trim());
        dispatch('created', { url: existingPr.url, number: existingPr.number });
      } else {
        // Create new PR
        const result = await createPullRequest(
          branch.repoPath,
          branch.branchName,
          branch.baseBranch,
          title.trim(),
          body.trim(),
          isDraft
        );
        dispatch('created', { url: result.url, number: result.number });
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      isPushing = false;
      isCreating = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      dispatch('close');
    } else if (e.key === 'Enter' && e.metaKey) {
      handleSubmit();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      dispatch('close');
    }
  }

  function viewExistingPr() {
    if (existingPr) {
      openUrl(existingPr.url);
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" on:click={handleBackdropClick}>
  <div class="modal">
    <div class="modal-header">
      <h2>{existingPr ? 'Update Pull Request' : 'Create Pull Request'}</h2>
      <button class="close-btn" on:click={() => dispatch('close')}>×</button>
    </div>

    {#if isLoading}
      <div class="loading">Checking for existing PR...</div>
    {:else}
      <form on:submit|preventDefault={handleSubmit}>
        {#if existingPr}
          <div class="existing-pr-banner">
            <span class="pr-icon">⤴</span>
            <span>
              PR #{existingPr.number} already exists for this branch.
              <button type="button" class="link-btn" on:click={viewExistingPr}>
                View on GitHub
              </button>
            </span>
          </div>
        {/if}

        <div class="form-group">
          <label for="pr-title">Title</label>
          <input
            id="pr-title"
            type="text"
            bind:value={title}
            placeholder="PR title"
            disabled={isPushing || isCreating}
          />
        </div>

        <div class="form-group">
          <label for="pr-body">Description</label>
          <textarea
            id="pr-body"
            bind:value={body}
            placeholder="Describe your changes..."
            rows="8"
            disabled={isPushing || isCreating}
          ></textarea>
        </div>

        {#if !existingPr}
          <div class="form-group checkbox-group">
            <label>
              <input type="checkbox" bind:checked={isDraft} disabled={isPushing || isCreating} />
              Create as draft
            </label>
          </div>
        {/if}

        <div class="form-info">
          <span class="branch-info">
            {branch.branchName} → {branch.baseBranch}
          </span>
          {#if commits.length > 0}
            <span class="commit-count">
              {commits.length} commit{commits.length !== 1 ? 's' : ''}
            </span>
          {/if}
        </div>

        {#if error}
          <div class="error">{error}</div>
        {/if}

        <div class="modal-actions">
          <button
            type="button"
            class="cancel-btn"
            on:click={() => dispatch('close')}
            disabled={isPushing || isCreating}
          >
            Cancel
          </button>
          <button
            type="submit"
            class="submit-btn"
            disabled={isPushing || isCreating || !title.trim()}
          >
            {#if isPushing}
              Pushing...
            {:else if isCreating}
              {existingPr ? 'Updating...' : 'Creating...'}
            {:else}
              {existingPr ? 'Push & Update PR' : 'Push & Create PR'}
            {/if}
          </button>
        </div>
      </form>
    {/if}
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-primary);
    border: 1px solid var(--border-primary);
    border-radius: 8px;
    width: 90%;
    max-width: 560px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-primary);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 24px;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0;
    line-height: 1;
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .loading {
    padding: 40px 20px;
    text-align: center;
    color: var(--text-secondary);
  }

  form {
    padding: 20px;
  }

  .existing-pr-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    margin-bottom: 16px;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .pr-icon {
    font-size: 16px;
  }

  .link-btn {
    background: none;
    border: none;
    color: var(--accent-primary);
    cursor: pointer;
    padding: 0;
    font-size: inherit;
    text-decoration: underline;
  }

  .link-btn:hover {
    color: var(--accent-secondary);
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    margin-bottom: 6px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .form-group input[type='text'],
  .form-group textarea {
    width: 100%;
    padding: 10px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 14px;
    font-family: inherit;
    box-sizing: border-box;
  }

  .form-group input[type='text']:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .form-group textarea {
    resize: vertical;
    min-height: 120px;
  }

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-primary);
  }

  .checkbox-group input[type='checkbox'] {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }

  .form-info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 6px;
    margin-bottom: 16px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .branch-info {
    font-family: var(--font-mono);
  }

  .commit-count {
    color: var(--text-tertiary);
  }

  .error {
    padding: 12px;
    background: rgba(255, 100, 100, 0.1);
    border: 1px solid rgba(255, 100, 100, 0.3);
    border-radius: 6px;
    color: #ff6464;
    font-size: 13px;
    margin-bottom: 16px;
  }

  .modal-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
  }

  .cancel-btn,
  .submit-btn {
    padding: 10px 20px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cancel-btn {
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    color: var(--text-secondary);
  }

  .cancel-btn:hover:not(:disabled) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .submit-btn {
    background: var(--accent-primary);
    border: 1px solid var(--accent-primary);
    color: white;
  }

  .submit-btn:hover:not(:disabled) {
    background: var(--accent-secondary);
    border-color: var(--accent-secondary);
  }

  .submit-btn:disabled,
  .cancel-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
