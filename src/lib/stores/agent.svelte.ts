/**
 * Agent store - manages chat state for AI agent integration via ACP.
 *
 * Uses Tauri events to receive streaming responses from the agent.
 * Supports per-tab sessions via the factory pattern.
 */
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as agentService from '../services/agent';

// =============================================================================
// Types
// =============================================================================

/** Role of a message sender */
export type MessageRole = 'user' | 'assistant';

/** A chat message */
export interface AgentMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: number;
}

/** Tool call status for display */
export interface ToolCallInfo {
  id: string;
  name: string;
  status: 'running' | 'complete' | 'error';
  timestamp: number;
}

/** Agent connection status */
export type AgentStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

/** Content block from backend */
type ContentBlock = { type: 'text'; text: string };

/** Tool result from backend */
interface ToolResult {
  status: string;
  value?: unknown;
  error?: string;
}

/** Agent events from backend */
type AgentEvent =
  | { type: 'content_chunk'; session_id: string; message_id: string; content: ContentBlock }
  | { type: 'tool_call_start'; session_id: string; tool_call_id: string; tool_name: string }
  | { type: 'tool_call_complete'; session_id: string; tool_call_id: string; result: ToolResult }
  | { type: 'complete'; session_id: string }
  | { type: 'error'; session_id: string; error: string };

// =============================================================================
// State Interface
// =============================================================================

/**
 * Agent state type for factory pattern.
 */
export interface AgentState {
  messages: AgentMessage[];
  status: AgentStatus;
  isStreaming: boolean;
  currentToolCall: ToolCallInfo | null;
  error: string | null;
  sessionId: string | null;
  /** Internal: current message being streamed */
  currentMessageId: string | null;
}

// =============================================================================
// Factory Function
// =============================================================================

/**
 * Create a new isolated agent state instance.
 * Used by the tab system to create per-tab state.
 */
export function createAgentState(): AgentState {
  return {
    messages: [],
    status: 'disconnected',
    isStreaming: false,
    currentToolCall: null,
    error: null,
    sessionId: null,
    currentMessageId: null,
  };
}

// =============================================================================
// Reactive State (Singleton)
// =============================================================================

/**
 * Module-level singleton state.
 * Gets synced to/from the active tab's agentState.
 */
export const agentState = $state(createAgentState());

// =============================================================================
// Session Registry
// =============================================================================

/**
 * Map of session IDs to their tab's agent state.
 * Used by the event listener to route events to the correct state.
 */
const sessionRegistry = new Map<string, AgentState>();

/**
 * Register a session with its state for event routing.
 */
function registerSession(sessionId: string, state: AgentState): void {
  sessionRegistry.set(sessionId, state);
}

/**
 * Unregister a session.
 */
export function unregisterSession(sessionId: string): void {
  sessionRegistry.delete(sessionId);
}

// =============================================================================
// Event Handling
// =============================================================================

let unlistenFn: UnlistenFn | null = null;
let listenerInitialized = false;

/**
 * Initialize the global event listener for agent events.
 * Routes events to the appropriate session's state.
 */
export async function initAgentEventListener(): Promise<void> {
  if (listenerInitialized) return;
  listenerInitialized = true;

  unlistenFn = await listen<AgentEvent>('agent-event', (event) => {
    const agentEvent = event.payload;

    // Get session ID from event
    const sessionId = 'session_id' in agentEvent ? agentEvent.session_id : null;
    if (!sessionId) return;

    // Find the state for this session
    const state = sessionRegistry.get(sessionId);
    if (!state) {
      console.warn(`Received event for unknown session: ${sessionId}`);
      return;
    }

    // Process the event
    switch (agentEvent.type) {
      case 'content_chunk': {
        if (agentEvent.content.type === 'text') {
          const existingIndex = state.messages.findIndex(
            (m) => m.id === agentEvent.message_id || m.id === state.currentMessageId
          );

          if (existingIndex >= 0) {
            // Append to existing message
            const existing = state.messages[existingIndex];
            state.messages = [
              ...state.messages.slice(0, existingIndex),
              { ...existing, content: existing.content + agentEvent.content.text },
              ...state.messages.slice(existingIndex + 1),
            ];
          } else {
            // Create new message
            state.currentMessageId = agentEvent.message_id;
            state.messages = [
              ...state.messages,
              {
                id: agentEvent.message_id,
                role: 'assistant',
                content: agentEvent.content.text,
                timestamp: Date.now(),
              },
            ];
          }
        }
        break;
      }

      case 'tool_call_start': {
        state.currentToolCall = {
          id: agentEvent.tool_call_id,
          name: agentEvent.tool_name,
          status: 'running',
          timestamp: Date.now(),
        };
        break;
      }

      case 'tool_call_complete': {
        if (state.currentToolCall && state.currentToolCall.id === agentEvent.tool_call_id) {
          state.currentToolCall = {
            ...state.currentToolCall,
            status: agentEvent.result.error ? 'error' : 'complete',
          };
          // Clear after a short delay
          const toolCallId = agentEvent.tool_call_id;
          setTimeout(() => {
            if (state.currentToolCall?.id === toolCallId) {
              state.currentToolCall = null;
            }
          }, 500);
        }
        break;
      }

      case 'complete': {
        state.isStreaming = false;
        state.currentMessageId = null;
        break;
      }

      case 'error': {
        state.isStreaming = false;
        state.currentMessageId = null;
        state.error = agentEvent.error;
        state.status = 'error';
        break;
      }
    }
  });
}

/**
 * Cleanup event listener (call on app unmount).
 */
export function cleanupAgentEventListener(): void {
  if (unlistenFn) {
    unlistenFn();
    unlistenFn = null;
    listenerInitialized = false;
  }
}

// =============================================================================
// Actions
// =============================================================================

/**
 * Send a message to the agent.
 * Creates a session if one doesn't exist.
 */
export async function sendMessage(content: string, workingDir: string): Promise<void> {
  if (!content.trim()) return;

  // Initialize event listener if needed
  await initAgentEventListener();

  // Create session if needed
  if (!agentState.sessionId) {
    agentState.status = 'connecting';
    try {
      const session = await agentService.createSession(workingDir, 'Staged Session');
      agentState.sessionId = session.id;
      agentState.status = 'connected';
      // Register this session for event routing
      registerSession(session.id, agentState);
    } catch (e) {
      agentState.error = e instanceof Error ? e.message : String(e);
      agentState.status = 'error';
      return;
    }
  }

  // Add user message
  const userMessage: AgentMessage = {
    id: crypto.randomUUID(),
    role: 'user',
    content: content.trim(),
    timestamp: Date.now(),
  };
  agentState.messages = [...agentState.messages, userMessage];

  // Start streaming
  agentState.isStreaming = true;
  agentState.error = null;

  try {
    await agentService.sendMessage(agentState.sessionId, content.trim());
  } catch (e) {
    agentState.isStreaming = false;
    agentState.error = e instanceof Error ? e.message : String(e);
    agentState.status = 'error';
  }
}

/**
 * Clear all messages and reset state.
 */
export function clearMessages(): void {
  agentState.messages = [];
  agentState.error = null;
  agentState.currentToolCall = null;
  agentState.isStreaming = false;
  agentState.currentMessageId = null;
  // Keep session alive - don't reset sessionId
}

/**
 * Clear any error state.
 */
export function clearError(): void {
  agentState.error = null;
  if (agentState.status === 'error') {
    agentState.status = agentState.sessionId ? 'connected' : 'disconnected';
  }
}
