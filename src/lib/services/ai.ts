import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// =============================================================================
// Types - Sessions
// =============================================================================

/** A session (persisted in SQLite) */
export interface Session {
  id: string;
  workingDir: string;
  agentId: string;
  title: string | null;
  createdAt: number;
  updatedAt: number;
}

/** Message role */
export type MessageRole = 'user' | 'assistant';

/** A message in a session */
export interface Message {
  id: number;
  sessionId: string;
  role: MessageRole;
  /** For user: plain text. For assistant: JSON array of ContentSegment */
  content: string;
  createdAt: number;
}

/** A segment of assistant content (text or tool call), stored in order */
export type ContentSegment =
  | { type: 'text'; text: string }
  | { type: 'toolCall'; id: string; title: string; status: string; locations?: string[] };

/** Full session with all messages */
export interface SessionFull {
  session: Session;
  messages: Message[];
}

/** Parse assistant message content into segments */
export function parseAssistantContent(content: string): ContentSegment[] {
  try {
    return JSON.parse(content) as ContentSegment[];
  } catch {
    // Fallback for plain text (shouldn't happen with new format)
    return [{ type: 'text', text: content }];
  }
}

/** Session status (live state) */
export type SessionStatus =
  | { status: 'idle' }
  | { status: 'processing' }
  | { status: 'error'; message: string };

/** Session status event payload */
export interface SessionStatusEvent {
  sessionId: string;
  status: SessionStatus;
}

// =============================================================================
// Types - ACP SDK (streaming events)
// =============================================================================

/** Content block types from ACP */
export type ContentBlock =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mimeType: string }
  | { type: 'resource'; uri: string; mimeType?: string; text?: string };

/** Session update types from ACP */
export type SessionUpdate =
  | { sessionUpdate: 'agent_message_chunk'; content: ContentBlock }
  | {
      sessionUpdate: 'tool_call';
      toolCallId: string;
      title: string;
      status: string;
      locations?: Array<{ path: string }>;
    }
  | {
      sessionUpdate: 'tool_call_update';
      toolCallId: string;
      fields: { status?: string; title?: string; content?: unknown[] };
    }
  | { sessionUpdate: 'user_message_chunk'; content: ContentBlock }
  | { sessionUpdate: 'agent_thought_chunk'; content: ContentBlock }
  | { sessionUpdate: string }; // Catch-all

/** Session notification from ACP */
export interface SessionNotification {
  sessionId: string;
  update: SessionUpdate;
}

// =============================================================================
// Types - Legacy (for backward compatibility)
// =============================================================================

/** Available ACP provider info */
export interface AcpProviderInfo {
  id: string;
  label: string;
}

/** Response from legacy send_agent_prompt */
export interface AgentPromptResponse {
  response: string;
  sessionId: string;
}

/** Tool call summary (legacy) */
export interface ToolCallSummary {
  id: string;
  title: string;
  status: string;
  locations?: string[];
  resultPreview?: string;
}

/** Finalized message (legacy) */
export type FinalizedMessage =
  | { role: 'user'; content: string }
  | { role: 'assistant'; content: string; toolCalls?: ToolCallSummary[] };

/** Session complete event (legacy) */
export interface SessionCompleteEvent {
  sessionId: string;
  transcript: FinalizedMessage[];
}

/** Session error event (legacy) */
export interface SessionErrorEvent {
  sessionId: string;
  error: string;
}

// =============================================================================
// Session Commands
// =============================================================================

/**
 * Create a new session.
 * Returns the session ID.
 */
export async function createSession(workingDir: string, agentId?: string): Promise<string> {
  return invoke<string>('create_session', {
    workingDir,
    agentId: agentId ?? null,
  });
}

/**
 * List all sessions.
 */
export async function listSessions(): Promise<Session[]> {
  return invoke<Session[]>('list_sessions');
}

/**
 * List sessions for a specific working directory.
 */
export async function listSessionsForDir(workingDir: string): Promise<Session[]> {
  return invoke<Session[]>('list_sessions_for_dir', { workingDir });
}

/**
 * Get full session with all messages.
 */
export async function getSession(sessionId: string): Promise<SessionFull | null> {
  return invoke<SessionFull | null>('get_session', { sessionId });
}

/**
 * Get session status (idle, processing, error).
 */
export async function getSessionStatus(sessionId: string): Promise<SessionStatus> {
  return invoke<SessionStatus>('get_session_status', { sessionId });
}

/**
 * Send a prompt to a session.
 * Streams response via events, persists on completion.
 */
export async function sendPrompt(sessionId: string, prompt: string): Promise<void> {
  return invoke<void>('send_prompt', { sessionId, prompt });
}

/**
 * Delete a session.
 */
export async function deleteSession(sessionId: string): Promise<void> {
  return invoke<void>('delete_session', { sessionId });
}

/**
 * Update session title.
 */
export async function updateSessionTitle(sessionId: string, title: string): Promise<void> {
  return invoke<void>('update_session_title', { sessionId, title });
}

// =============================================================================
// Legacy Commands (kept for backward compatibility)
// =============================================================================

/**
 * Discover available ACP providers on the system.
 */
export async function discoverAcpProviders(): Promise<AcpProviderInfo[]> {
  return invoke<AcpProviderInfo[]>('discover_acp_providers');
}

/**
 * Check if an AI agent is available.
 */
export async function checkAiAvailable(): Promise<string> {
  return invoke<string>('check_ai_available');
}

/**
 * Send a prompt to the AI agent (non-streaming, legacy).
 */
export async function sendAgentPrompt(
  prompt: string,
  options?: {
    repoPath?: string;
    sessionId?: string;
    provider?: string;
  }
): Promise<AgentPromptResponse> {
  return invoke<AgentPromptResponse>('send_agent_prompt', {
    repoPath: options?.repoPath ?? null,
    prompt,
    sessionId: options?.sessionId ?? null,
    provider: options?.provider ?? null,
  });
}

/**
 * Send a prompt with streaming (legacy).
 */
export async function sendAgentPromptStreaming(
  prompt: string,
  options?: {
    repoPath?: string;
    sessionId?: string;
    provider?: string;
  }
): Promise<AgentPromptResponse> {
  return invoke<AgentPromptResponse>('send_agent_prompt_streaming', {
    repoPath: options?.repoPath ?? null,
    prompt,
    sessionId: options?.sessionId ?? null,
    provider: options?.provider ?? null,
  });
}

// =============================================================================
// Event Listeners
// =============================================================================

/**
 * Listen for session update events (streaming chunks, tool calls).
 */
export async function listenToSessionUpdates(
  callback: (notification: SessionNotification) => void
): Promise<UnlistenFn> {
  return listen<SessionNotification>('session-update', (event) => {
    callback(event.payload);
  });
}

/**
 * Listen for session status changes.
 */
export async function listenToSessionStatus(
  callback: (event: SessionStatusEvent) => void
): Promise<UnlistenFn> {
  return listen<SessionStatusEvent>('session-status', (event) => {
    callback(event.payload);
  });
}

/**
 * Listen for session complete events (legacy).
 */
export async function listenToSessionComplete(
  callback: (event: SessionCompleteEvent) => void
): Promise<UnlistenFn> {
  return listen<SessionCompleteEvent>('session-complete', (event) => {
    callback(event.payload);
  });
}

/**
 * Listen for session error events (legacy).
 */
export async function listenToSessionError(
  callback: (event: SessionErrorEvent) => void
): Promise<UnlistenFn> {
  return listen<SessionErrorEvent>('session-error', (event) => {
    callback(event.payload);
  });
}

// =============================================================================
// Backward Compatibility Aliases
// =============================================================================

// These aliases allow existing code to work without changes
export type ChatSession = Session;
export type ChatMessage = Message;
export type ChatSessionFull = SessionFull;
export const createChatSession = createSession;
export const listChatSessions = listSessions;
export const listChatSessionsForDir = listSessionsForDir;
export const getChatSession = getSession;
export const getChatSessionStatus = getSessionStatus;
export const sendChatPrompt = sendPrompt;
export const deleteChatSession = deleteSession;
export const updateChatSessionTitle = updateSessionTitle;
