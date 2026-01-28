<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { AlertCircle } from 'lucide-svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import DiffViewer from './lib/DiffViewer.svelte';
  import EmptyState from './lib/EmptyState.svelte';
  import TopBar from './lib/TopBar.svelte';
  import FileSearchModal from './lib/FileSearchModal.svelte';
  import TabBar from './lib/TabBar.svelte';
  import ChatPanel from './lib/ChatPanel.svelte';
  import ArtifactsPanel from './lib/ArtifactsPanel.svelte';
  import ArtifactDetailView from './lib/ArtifactDetailView.svelte';
  import { listRefs, getMergeBase } from './lib/services/git';
  import { getWindowLabel } from './lib/services/window';
  import {
    windowState,
    addTab,
    closeTab,
    switchTab,
    setWindowLabel,
    loadTabsFromStorage,
    getActiveTab,
    markRepoNeedsRefresh,
    clearNeedsRefresh,
  } from './lib/stores/tabState.svelte';
  import { createDiffState } from './lib/stores/diffState.svelte';
  import { createCommentsState } from './lib/stores/comments.svelte';
  import { createDiffSelection } from './lib/stores/diffSelection.svelte';
  import { DiffSpec, inferRefType } from './lib/types';
  import type { DiffSpec as DiffSpecType } from './lib/types';
  import { initWatcher, watchRepo, type Unsubscribe } from './lib/services/statusEvents';
  import { referenceFileAsDiff } from './lib/diffUtils';
  import {
    addReferenceFile,
    removeReferenceFile,
    loadReferenceFiles,
    clearReferenceFiles,
    getReferenceFile,
    getReferenceFilePaths,
  } from './lib/stores/referenceFiles.svelte';
  import {
    preferences,
    loadSavedSize,
    loadSavedSyntaxTheme,
    loadSavedSidebarPosition,
    loadSavedSidebarWidth,
    loadSavedChatPanelVisible,
    getCustomKeyboardBindings,
    registerPreferenceShortcuts,
  } from './lib/stores/preferences.svelte';
  import { loadCustomBindings } from './lib/services/keyboard';
  import { registerShortcut } from './lib/services/keyboard';
  import {
    diffSelection,
    selectPreset,
    selectCustomDiff,
    resetDiffSelection,
    setDefaultBranch,
    getDefaultBranch,
    type DiffPreset,
  } from './lib/stores/diffSelection.svelte';
  import {
    diffState,
    getCurrentDiff,
    loadFiles,
    refreshFiles,
    selectFile,
    resetState,
  } from './lib/stores/diffState.svelte';
  import {
    commentsState,
    loadComments,
    setCurrentPath,
    clearComments,
    setReferenceFilesLoader,
  } from './lib/stores/comments.svelte';
  import {
    repoState,
    initRepoState,
    setCurrentRepo,
    openRepoPicker,
  } from './lib/stores/repoState.svelte';
  import {
    clearResults as clearSmartDiffResults,
    loadAnalysisFromDb,
  } from './lib/stores/smartDiff.svelte';
  import {
    agentState,
    createAgentState,
    initAgentEventListener,
    cleanupAgentEventListener,
    registerSession,
    unregisterSession,
    clearSession,
  } from './lib/stores/agent.svelte';
  import {
    planState,
    createPlanState,
    syncFromTab as syncPlanFromTab,
    syncToTab as syncPlanToTab,
    markPlanReady,
    markRefiningComplete,
    markImplementationComplete,
    updatePlanContent,
    setPlanError,
    clearPlan,
  } from './lib/stores/plan.svelte';
  import { addPlanArtifact } from './lib/stores/artifacts.svelte';
  import {
    artifactsState,
    createArtifactsState,
    syncFromTab as syncArtifactsFromTab,
    syncToTab as syncArtifactsToTab,
    selectedArtifactId,
    upsertSessionArtifact,
    removeSessionArtifact,
    selectArtifact,
  } from './lib/stores/artifacts.svelte';
  import { setChatPanelVisible } from './lib/stores/preferences.svelte';

  // UI State
  let unsubscribeWatcher: Unsubscribe | null = null;
  let showFileSearch = $state(false);
  let unsubscribeMenuOpenFolder: Unsubscribe | null = null;
  let unsubscribeMenuCloseTab: Unsubscribe | null = null;
  let unsubscribeMenuCloseWindow: Unsubscribe | null = null;

  // Implementation artifact state (for RHS file list)
  let implementationState = $state({
    files: [] as any[],
    loading: false,
    selectedFile: null as string | null,
    onFileSelect: (_path: string) => {}
  });


  // Load files, comments, and AI analysis for current spec
  async function loadAll() {
    const repoPath = repoState.currentPath ?? undefined;
    await loadFiles(diffSelection.spec, repoPath);
    await loadComments(diffSelection.spec, repoPath);
    // Load any saved AI analysis from database
    await loadAnalysisFromDb(repoPath ?? null, diffSelection.spec);
  }

  // Update comments store when selected file changes
  $effect(() => {
    const diff = getCurrentDiff();
    const path = diff?.after?.path ?? diff?.before?.path ?? null;
    setCurrentPath(path);
  });

  async function handleFilesChanged(changedRepoPath: string) {
    const activeTab = getActiveTab();

    // If this is NOT the active tab's repo, mark those tabs as needing refresh
    if (!activeTab || activeTab.repoPath !== changedRepoPath) {
      markRepoNeedsRefresh(changedRepoPath);
      return;
    }

    // Only refresh if viewing working tree
    if (diffSelection.spec.head.type !== 'WorkingTree') {
      // Mark as needing refresh for when user switches back to working tree
      activeTab.needsRefresh = true;
      console.debug('[App] Files changed but not viewing working tree, marked for refresh');
      return;
    }

    await refreshFiles(diffSelection.spec, repoState.currentPath ?? undefined);
    // Reload comments - they may have changed after a commit
    await loadComments(diffSelection.spec);

    // Save updated state back to tab
    syncGlobalToTab();
  }

  // Preset selection
  async function handlePresetSelect(preset: DiffPreset) {
    resetState();
    clearReferenceFiles();
    clearSmartDiffResults();

    // Branch Changes uses merge-base for cleaner diffs
    if (preset.label === 'Branch Changes') {
      try {
        const mergeBaseSha = await getMergeBase(
          getDefaultBranch(),
          'HEAD',
          repoState.currentPath ?? undefined
        );
        const spec: DiffSpecType = {
          base: { type: 'Rev', value: mergeBaseSha },
          head: { type: 'WorkingTree' },
        };
        selectCustomDiff(spec, preset.label);
      } catch (e) {
        console.error('Failed to compute merge-base:', e);
        diffState.error = `Failed to compute merge base: ${e}`;
        diffState.loading = false;
        return;
      }
    } else {
      selectPreset(preset);
    }

    await loadAll();

    // Clear needsRefresh since we just loaded fresh data
    const tab = getActiveTab();
    if (tab) clearNeedsRefresh(tab);

    // Save updated state back to tab
    syncGlobalToTab();
  }

  // Custom diff selection (from DiffSelectorModal or PRSelectorModal)
  async function handleCustomDiff(spec: DiffSpecType, label?: string, prNumber?: number) {
    resetState();
    clearReferenceFiles();
    clearSmartDiffResults();
    selectCustomDiff(spec, label, prNumber);
    await loadAll();

    // Clear needsRefresh since we just loaded fresh data
    const tab = getActiveTab();
    if (tab) clearNeedsRefresh(tab);

    // Save updated state back to tab
    syncGlobalToTab();
  }

  // Repo change - reload everything
  async function handleRepoChange() {
    resetState();
    clearComments();
    clearReferenceFiles();
    clearSmartDiffResults();

    if (repoState.currentPath) {
      watchRepo(repoState.currentPath);

      // Load refs and detect default branch for new repo
      try {
        const refs = await listRefs(repoState.currentPath);
        const defaultBranch = detectDefaultBranch(refs);
        setDefaultBranch(defaultBranch);
        // Mark repo as valid since we got refs
        setCurrentRepo(repoState.currentPath);
      } catch (e) {
        // Repo doesn't exist or isn't a git repo - show friendly error
        const errorMsg = e instanceof Error ? e.message : String(e);
        if (errorMsg.includes('No such file or directory')) {
          diffState.error = `Repository not found: ${repoState.currentPath}`;
        } else if (errorMsg.includes('not a git repository')) {
          diffState.error = `Not a git repository: ${repoState.currentPath}`;
        } else {
          diffState.error = errorMsg;
        }
        diffState.loading = false;
        console.error('Failed to load refs:', e);
        return;
      }

      // Reset diff selection to "Uncommitted" and load
      resetDiffSelection();
      await loadAll();

      // Save updated state back to tab
      syncGlobalToTab();
    }
  }

  // Menu Event Handlers
  async function handleMenuOpenFolder() {
    // Add a new tab for the selected repo
    await handleNewTab();
  }

  function handleMenuCloseTab() {
    // Close the active tab
    const activeTab = getActiveTab();
    if (!activeTab) return;

    // Unregister the session if the tab has one
    if (activeTab.agentState.sessionId) {
      unregisterSession(activeTab.agentState.sessionId);
    }

    closeTab(activeTab.id);

    // Close window if no tabs left
    if (windowState.tabs.length === 0) {
      const window = getCurrentWindow();
      window.close();
      return;
    }

    // Sync the new active tab's state to global
    syncTabToGlobal();

    // Watch the new active tab's repo (fire-and-forget)
    const newTab = getActiveTab();
    if (newTab) {
      watchRepo(newTab.repoPath);
    }
  }

  async function handleMenuCloseWindow() {
    // Close the current window
    const window = getCurrentWindow();
    await window.close();
  }

  /**
   * Detect the default branch (main, master, etc.) from available refs.
   */
  function detectDefaultBranch(refs: string[]): string {
    // Filter to likely branch names (not remotes, not tags)
    const branchNames = refs.filter((r) => inferRefType(r) === 'branch');

    // Check common default branch names in order of preference
    const candidates = ['main', 'master', 'develop', 'trunk'];
    for (const name of candidates) {
      if (branchNames.includes(name)) {
        return name;
      }
    }

    // Fallback to first branch, or 'main' if no branches
    return branchNames[0] ?? 'main';
  }

  /**
   * Extract repository name from path.
   */
  function extractRepoName(path: string): string {
    const parts = path.split('/');
    return parts[parts.length - 1] || path;
  }

  /**
   * Sync active tab's state to global singletons.
   * This allows existing components to work without changes.
   */
  function syncTabToGlobal() {
    const tab = getActiveTab();
    if (!tab) return;

    console.log(`Syncing tab "${tab.repoName}" to global state`);

    // Copy active tab's state to global singletons (property by property for reactivity)
    diffState.currentSpec = tab.diffState.currentSpec;
    diffState.currentRepoPath = tab.diffState.currentRepoPath;
    diffState.files = tab.diffState.files;
    diffState.diffCache = tab.diffState.diffCache;
    diffState.selectedFile = tab.diffState.selectedFile;
    diffState.scrollTargetLine = tab.diffState.scrollTargetLine;
    diffState.loading = tab.diffState.loading;
    diffState.loadingFile = tab.diffState.loadingFile;
    diffState.error = tab.diffState.error;

    commentsState.comments = tab.commentsState.comments;
    commentsState.reviewedPaths = tab.commentsState.reviewedPaths;
    commentsState.currentPath = tab.commentsState.currentPath;
    commentsState.currentSpec = tab.commentsState.currentSpec;
    commentsState.currentRepoPath = tab.commentsState.currentRepoPath;
    commentsState.loading = tab.commentsState.loading;

    diffSelection.spec = tab.diffSelection.spec;
    diffSelection.label = tab.diffSelection.label;
    diffSelection.prNumber = tab.diffSelection.prNumber;

    // Sync agent state (copy arrays to avoid shared references)
    agentState.messages = [...tab.agentState.messages];
    agentState.status = tab.agentState.status;
    agentState.isStreaming = tab.agentState.isStreaming;
    agentState.currentToolCall = tab.agentState.currentToolCall;
    agentState.error = tab.agentState.error;
    agentState.sessionId = tab.agentState.sessionId;
    agentState.agentId = tab.agentState.agentId;
    agentState.currentMessageId = tab.agentState.currentMessageId;

    // Register the session if the tab has one (enables event routing for this tab)
    if (tab.agentState.sessionId) {
      registerSession(tab.agentState.sessionId, tab.agentState);
    }

    // Sync plan state
    syncPlanFromTab(tab.planState);

    // Sync artifacts state
    syncArtifactsFromTab(tab.artifactsState);

    // Update repo state
    setCurrentRepo(tab.repoPath);
  }

  /**
   * Sync global singletons back to active tab.
   * Called after state changes to preserve tab state.
   */
  function syncGlobalToTab() {
    const tab = getActiveTab();
    if (!tab) return;

    console.log(`Saving global state to tab "${tab.repoName}"`);

    // Copy global state back to active tab
    tab.diffState.currentSpec = diffState.currentSpec;
    tab.diffState.currentRepoPath = diffState.currentRepoPath;
    tab.diffState.files = diffState.files;
    tab.diffState.diffCache = diffState.diffCache;
    tab.diffState.selectedFile = diffState.selectedFile;
    tab.diffState.scrollTargetLine = diffState.scrollTargetLine;
    tab.diffState.loading = diffState.loading;
    tab.diffState.loadingFile = diffState.loadingFile;
    tab.diffState.error = diffState.error;

    tab.commentsState.comments = commentsState.comments;
    tab.commentsState.reviewedPaths = commentsState.reviewedPaths;
    tab.commentsState.currentPath = commentsState.currentPath;
    tab.commentsState.currentSpec = commentsState.currentSpec;
    tab.commentsState.currentRepoPath = commentsState.currentRepoPath;
    tab.commentsState.loading = commentsState.loading;

    tab.diffSelection.spec = diffSelection.spec;
    tab.diffSelection.label = diffSelection.label;
    tab.diffSelection.prNumber = diffSelection.prNumber;

    // Sync agent state back to tab (copy arrays to avoid shared references)
    tab.agentState.messages = [...agentState.messages];
    tab.agentState.status = agentState.status;
    tab.agentState.isStreaming = agentState.isStreaming;
    tab.agentState.currentToolCall = agentState.currentToolCall;
    tab.agentState.error = agentState.error;
    tab.agentState.sessionId = agentState.sessionId;
    tab.agentState.agentId = agentState.agentId;
    tab.agentState.currentMessageId = agentState.currentMessageId;

    // Sync plan state back to tab
    syncPlanToTab(tab.planState);

    // Sync artifacts state back to tab
    syncArtifactsToTab(tab.artifactsState);
  }

  /**
   * Initialize a newly created tab with data.
   */
  async function initializeNewTab(tab: any) {
    try {
      // Load refs and detect default branch
      const refs = await listRefs(tab.repoPath);
      const defaultBranch = detectDefaultBranch(refs);
      setDefaultBranch(defaultBranch);

      // Reset to uncommitted preset
      resetDiffSelection();

      // Load files and comments
      await loadFiles(diffSelection.spec, tab.repoPath);
      await loadComments(diffSelection.spec, tab.repoPath);

      // Save state back to tab
      syncGlobalToTab();
    } catch (e) {
      console.error('Failed to initialize tab:', e);
      diffState.error = e instanceof Error ? e.message : String(e);
      diffState.loading = false;
    }
  }

  /**
   * Handle tab switching.
   * Watcher is already running for the repo - no restart needed.
   */
  async function handleTabSwitch(index: number) {
    console.log(`Switching to tab ${index}`);

    // Save current tab state before switching
    syncGlobalToTab();

    // Switch to new tab (synchronous - no watcher restart)
    switchTab(index);
    console.log(`Active tab after switch:`, getActiveTab()?.repoName);

    // Clear smart diff results (they're per-diff, not persisted per-tab)
    clearSmartDiffResults();

    // Load new tab state
    syncTabToGlobal();

    // Initialize tab if it hasn't been loaded yet (e.g., restored from storage)
    const tab = getActiveTab();
    if (tab && tab.diffState.currentSpec === null) {
      initializeNewTab(tab);
    } else if (tab?.needsRefresh && diffSelection.spec.head.type === 'WorkingTree') {
      // Tab was marked dirty while inactive - refresh now
      console.debug(`[App] Tab "${tab.repoName}" needs refresh, loading files`);
      clearNeedsRefresh(tab);
      await refreshFiles(diffSelection.spec, repoState.currentPath ?? undefined);
      await loadComments(diffSelection.spec);
      syncGlobalToTab();
    }
  }

  /**
   * Handle new tab creation.
   */
  async function handleNewTab() {
    const repoPath = await openRepoPicker();
    if (!repoPath) return;

    // Save current tab state before creating new one
    syncGlobalToTab();

    const repoName = extractRepoName(repoPath);
    addTab(
      repoPath,
      repoName,
      createDiffState,
      createCommentsState,
      createDiffSelection,
      createAgentState,
      createPlanState,
      createArtifactsState
    );

    // Start watching the new repo (idempotent - won't restart if already watching)
    watchRepo(repoPath);

    // Sync to the new tab
    syncTabToGlobal();

    // Initialize the new tab
    const newTab = getActiveTab();
    if (newTab) {
      await initializeNewTab(newTab);
    }
  }

  // Get current diff - check reference files first
  // Check if current selection is a reference file
  let isCurrentFileReference = $derived(
    diffState.selectedFile !== null && getReferenceFile(diffState.selectedFile) !== undefined
  );

  let currentDiff = $derived.by(() => {
    const selectedPath = diffState.selectedFile;
    if (!selectedPath) return getCurrentDiff();

    // Check if it's a reference file
    const refFile = getReferenceFile(selectedPath);
    if (refFile) {
      return referenceFileAsDiff(refFile.path, refFile.content);
    }

    // Otherwise, get the regular diff
    return getCurrentDiff();
  });

  // Handle file selection from file search modal
  async function handleReferenceFileSelect(path: string) {
    try {
      // Determine which ref to use for loading the file
      // Use the "head" ref of the current diff
      const headRef = diffSelection.spec.head;
      const refName = headRef.type === 'WorkingTree' ? 'HEAD' : headRef.value;
      await addReferenceFile(refName, path, diffSelection.spec, repoState.currentPath ?? undefined);
      showFileSearch = false;
      // Select the newly added file
      selectFile(path);
    } catch (e) {
      console.error('Failed to add reference file:', e);
      // Keep modal open so user sees the error
    }
  }

  // Handle removing a reference file
  function handleRemoveReferenceFile(path: string) {
    removeReferenceFile(path, diffSelection.spec, repoState.currentPath ?? undefined);
  }

  // Show empty state when we have a repo, finished loading, no error, but no files
  // Exception: Don't show empty state if we just finished implementing (show diff viewer instead)
  let showEmptyState = $derived(
    repoState.currentPath &&
      !diffState.loading &&
      !diffState.error &&
      diffState.files.length === 0 &&
      planState.plan?.status !== 'complete'
  );

  let isWorkingTree = $derived(diffSelection.spec.head.type === 'WorkingTree');

  // Track session artifacts for active sessions (always active)
  $effect(() => {
    const sessionId = agentState.sessionId;
    const currentPlan = planState.plan;
    const messages = agentState.messages;
    const agentId = agentState.agentId;

    if (sessionId) {
      let artifactId: string;

      // If there's a plan, use plan-based info
      if (currentPlan && (currentPlan.status === 'drafting' || currentPlan.status === 'refining' || currentPlan.status === 'implementing')) {
        const title = currentPlan.description.slice(0, 60) + (currentPlan.description.length > 60 ? '...' : '');
        const artifactStatus = currentPlan.status === 'drafting' ? 'planning' : currentPlan.status;

        artifactId = upsertSessionArtifact(
          sessionId,
          title,
          artifactStatus,
          currentPlan.description,
          currentPlan.status === 'implementing' ? currentPlan.implementAgent : currentPlan.planningAgent
        );
      } else {
        // No plan yet - create a generic session artifact based on first message
        const userMessages = messages.filter((m) => m.role === 'user');
        const firstUserMessage = userMessages.length > 0 ? userMessages[0].content : 'New session';
        const title = firstUserMessage.slice(0, 60) + (firstUserMessage.length > 60 ? '...' : '');

        artifactId = upsertSessionArtifact(
          sessionId,
          title,
          'active',
          firstUserMessage,
          agentId ?? 'goose'
        );
      }

      // Auto-select the session artifact and show chat panel
      selectArtifact(artifactId);
      setChatPanelVisible(true);
    }
  });

  // Track if we've seen streaming start - prevents false transitions on mount
  let hasSeenStreaming = $state(false);

  // Watch agent streaming to update plan content (must be always active)
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
        updatePlanContent(latestContent);
      }
    }
  });

  // Watch for streaming completion to transition plan status (must be always active)
  $effect(() => {
    const isStreaming = agentState.isStreaming;
    const currentPlan = planState.plan;
    const seenStreaming = hasSeenStreaming;
    const sessionId = agentState.sessionId;

    if (!isStreaming && currentPlan && seenStreaming) {
      console.log('[App] Streaming complete, transitioning from:', currentPlan.status);
      if (currentPlan.status === 'drafting') {
        markPlanReady();
        // Create plan artifact
        const title = currentPlan.description.slice(0, 60) + (currentPlan.description.length > 60 ? '...' : '');
        addPlanArtifact(
          title,
          currentPlan.content,
          currentPlan.description,
          currentPlan.planningAgent
        );
        console.log('[App] Created plan artifact');

        // Remove session artifact after completion
        if (sessionId) {
          removeSessionArtifact(sessionId);
        }

        hasSeenStreaming = false;
      } else if (currentPlan.status === 'refining') {
        markRefiningComplete();

        // Remove session artifact after completion
        if (sessionId) {
          removeSessionArtifact(sessionId);
        }

        hasSeenStreaming = false;
      } else if (currentPlan.status === 'implementing') {
        markImplementationComplete();

        // Remove session artifact after completion
        if (sessionId) {
          removeSessionArtifact(sessionId);
        }

        hasSeenStreaming = false;
      }
    }
  });

  // Watch for agent errors (must be always active)
  $effect(() => {
    const error = agentState.error;
    if (error && planState.plan) {
      console.error('[App] Agent error during plan operation:', error);
      setPlanError(error);
    }
  });

  /**
   * Handle starting a new session from the artifacts panel
   */
  function handleStartNewSession() {
    // Remove any existing session artifact
    if (agentState.sessionId) {
      removeSessionArtifact(agentState.sessionId);
    }

    // Clear the current session and plan
    clearSession();
    clearPlan();

    // Clear any artifact selection
    selectArtifact(null);

    // Show the chat panel
    setChatPanelVisible(true);
  }

  // Lifecycle
  let unregisterPreferenceShortcuts: (() => void) | null = null;
  let unregisterFileSearchShortcut: (() => void) | null = null;

  onMount(() => {
    loadSavedSize();
    loadSavedSidebarPosition();
    loadSavedSidebarWidth();
    loadSavedChatPanelVisible();
    unregisterPreferenceShortcuts = registerPreferenceShortcuts();

    // Apply custom keyboard bindings after a short delay to let shortcuts register
    setTimeout(() => {
      loadCustomBindings(getCustomKeyboardBindings());
    }, 100);

    // Register Cmd+O to open file search
    unregisterFileSearchShortcut = registerShortcut({
      id: 'open-file-search',
      keys: ['o'],
      modifiers: { meta: true },
      description: 'Open file search',
      category: 'files',
      handler: () => {
        if (repoState.currentPath && !diffState.error) {
          showFileSearch = true;
        }
      },
    });

    // Register the reference files loader so comments store can trigger it
    setReferenceFilesLoader(loadReferenceFiles);

    // Initialize agent event listener
    initAgentEventListener();

    (async () => {
      await loadSavedSyntaxTheme();

      // Get window label and initialize tab state
      const label = await getWindowLabel();
      setWindowLabel(label);

      // Load tabs from storage (if any)
      loadTabsFromStorage(
        createDiffState,
        createCommentsState,
        createDiffSelection,
        createAgentState,
        createPlanState,
        createArtifactsState
      );

      // Initialize watcher listener once (handles all repos)
      unsubscribeWatcher = await initWatcher(handleFilesChanged);

      // Start watchers for all restored tabs (idempotent - dedupes same repos)
      for (const tab of windowState.tabs) {
        watchRepo(tab.repoPath);
      }

      // Register menu event listeners
      unsubscribeMenuOpenFolder = await listen('menu:open-folder', handleMenuOpenFolder);
      unsubscribeMenuCloseTab = await listen('menu:close-tab', handleMenuCloseTab);
      unsubscribeMenuCloseWindow = await listen('menu:close-window', handleMenuCloseWindow);

      // Initialize repo state (resolves canonical path, adds to recent repos)
      const repoPath = await initRepoState();

      if (repoPath) {
        // Create initial tab if no tabs loaded from storage
        if (windowState.tabs.length === 0) {
          const repoName = extractRepoName(repoPath);
          addTab(
            repoPath,
            repoName,
            createDiffState,
            createCommentsState,
            createDiffSelection,
            createAgentState,
            createPlanState,
            createArtifactsState
          );
        }

        // Sync the active tab to global state
        syncTabToGlobal();

        // Watch the active tab's repo
        const tab = getActiveTab();
        if (tab) {
          await watchRepo(tab.repoPath);

          // Initialize the active tab
          await initializeNewTab(tab);
        }
      }
    })();
  });

  onDestroy(() => {
    unregisterPreferenceShortcuts?.();
    unregisterFileSearchShortcut?.();
    unsubscribeWatcher?.();
    unsubscribeMenuOpenFolder?.();
    unsubscribeMenuCloseTab?.();
    unsubscribeMenuCloseWindow?.();
    // Cleanup agent event listener
    cleanupAgentEventListener();
  });
</script>

<main>
  {#if windowState.tabs.length > 0}
    <TabBar onNewTab={handleNewTab} onSwitchTab={handleTabSwitch} />
  {/if}

  <TopBar
    onPresetSelect={handlePresetSelect}
    onCustomDiff={handleCustomDiff}
    onCommit={() => {
      const tab = getActiveTab();
      if (tab) handleFilesChanged(tab.repoPath);
    }}
  />

  <div class="layout-container">
    <!-- Left side: main content + chat panel -->
    <div class="left-side">
      <div class="app-container">
        {#if showEmptyState}
          <!-- Full-width empty state -->
          <section class="main-content full-width">
            <EmptyState />
          </section>
        {:else}
          <!-- Main content area -->
          <section class="main-content">
            {#if selectedArtifactId.value}
              <!-- Artifact detail view -->
              <ArtifactDetailView
                sizeBase={preferences.sizeBase}
                syntaxThemeVersion={preferences.syntaxThemeVersion}
                onImplementationStateChange={(state) => { implementationState = state; }}
              />
            {:else if diffState.loading}
              <div class="loading-state">
                <p>Loading...</p>
              </div>
            {:else if diffState.error}
              <div class="error-state">
                <AlertCircle size={18} />
                <p class="error-message">{diffState.error}</p>
              </div>
            {:else}
              <!-- Regular diff view - full width -->
              <DiffViewer
                diff={currentDiff}
                sizeBase={preferences.sizeBase}
                syntaxThemeVersion={preferences.syntaxThemeVersion}
                loading={diffState.loadingFile !== null}
                isReferenceFile={isCurrentFileReference}
              />
            {/if}
          </section>
        {/if}
      </div>

      <!-- Bottom chat panel (only on left side) -->
      {#if repoState.currentPath && preferences.chatPanelVisible}
        <ChatPanel hasActiveSession={agentState.sessionId !== null} />
      {/if}
    </div>

    <!-- Right side: Artifacts panel (full height) -->
    {#if repoState.currentPath}
      <aside class="artifacts-sidebar">
        <ArtifactsPanel
          onStartNewSession={handleStartNewSession}
          onAddReferenceFile={() => (showFileSearch = true)}
          onRemoveReferenceFile={handleRemoveReferenceFile}
          implementationFiles={implementationState.files}
          implementationLoading={implementationState.loading}
          selectedImplementationFile={implementationState.selectedFile}
          onImplementationFileSelect={implementationState.onFileSelect}
        />
      </aside>
    {/if}
  </div>
</main>

{#if showFileSearch}
  {@const headRef = diffSelection.spec.head}
  <FileSearchModal
    refName={headRef.type === 'WorkingTree' ? 'HEAD' : headRef.value}
    repoPath={repoState.currentPath ?? undefined}
    existingPaths={[
      ...diffState.files
        .map((f) => f.after?.toString() ?? f.before?.toString() ?? '')
        .filter(Boolean),
      ...getReferenceFilePaths(),
    ]}
    onSelect={handleReferenceFileSelect}
    onClose={() => (showFileSearch = false)}
  />
{/if}

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background-color: var(--bg-chrome);
    color: var(--text-primary);
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background-color: var(--bg-chrome);
  }

  .layout-container {
    display: flex;
    flex: 1;
    overflow: hidden;
    padding: 0 8px 8px 8px;
    gap: 8px;
  }

  .left-side {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    gap: 8px;
  }

  .app-container {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .artifacts-sidebar {
    width: 320px;
    min-width: 280px;
    max-width: 400px;
    background-color: transparent;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: var(--size-lg);
  }

  .error-state {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    color: var(--text-muted);
  }

  .error-message {
    font-size: var(--size-md);
    margin: 0;
  }
</style>
