import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// =============================================================================
// Types
// =============================================================================

/** Available ACP provider info */
export interface AcpProviderInfo {
  id: string;
  label: string;
}

/** Response from sending a prompt */
export interface AgentPromptResponse {
  response: string;
  sessionId: string;
}

/** Tool call summary (from finalized transcript) */
export interface ToolCallSummary {
  id: string;
  title: string;
  status: string;
  locations?: string[];
  resultPreview?: string;
}

/** Finalized message for storage/display */
export type FinalizedMessage =
  | { role: 'user'; content: string }
  | { role: 'assistant'; content: string; toolCalls?: ToolCallSummary[] };

/** Session complete event payload */
export interface SessionCompleteEvent {
  sessionId: string;
  transcript: FinalizedMessage[];
}

/** Session error event payload */
export interface SessionErrorEvent {
  sessionId: string;
  error: string;
}

// =============================================================================
// ACP SDK Types (from session-update events)
//
// The ACP SDK uses serde with specific discriminators:
// - SessionUpdate: tag = "sessionUpdate", values are snake_case
// - ContentBlock: tag = "type", values are snake_case
// =============================================================================

/** Content block types from ACP (discriminator: "type") */
export type ContentBlock =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mimeType: string }
  | { type: 'resource'; uri: string; mimeType?: string; text?: string };

/** Tool call status */
export type ToolCallStatus = 'running' | 'completed' | 'failed';

/** Tool call from ACP */
export interface ToolCall {
  toolCallId: string;
  title: string;
  status: ToolCallStatus;
  locations?: Array<{ path: string }>;
}

/** Tool call update from ACP */
export interface ToolCallUpdate {
  toolCallId: string;
  fields: {
    status?: ToolCallStatus;
    title?: string;
    content?: unknown[];
  };
}

/** Session update types from ACP (discriminator: "sessionUpdate", snake_case) */
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
  | { sessionUpdate: string }; // Catch-all for other update types

/** Session notification from ACP (camelCase field names) */
export interface SessionNotification {
  sessionId: string;
  update: SessionUpdate;
}

// =============================================================================
// Commands
// =============================================================================

/**
 * Discover available ACP providers on the system.
 */
export async function discoverAcpProviders(): Promise<AcpProviderInfo[]> {
  return invoke<AcpProviderInfo[]>('discover_acp_providers');
}

/**
 * Check if an AI agent is available.
 * Returns the agent name if available, throws if not.
 */
export async function checkAiAvailable(): Promise<string> {
  return invoke<string>('check_ai_available');
}

/**
 * Send a prompt to the AI agent (non-streaming).
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
 * Send a prompt to the AI agent with streaming.
 * Subscribe to events using listenToSessionUpdates.
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
 * Listen for session complete events.
 */
export async function listenToSessionComplete(
  callback: (event: SessionCompleteEvent) => void
): Promise<UnlistenFn> {
  return listen<SessionCompleteEvent>('session-complete', (event) => {
    callback(event.payload);
  });
}

/**
 * Listen for session error events.
 */
export async function listenToSessionError(
  callback: (event: SessionErrorEvent) => void
): Promise<UnlistenFn> {
  return listen<SessionErrorEvent>('session-error', (event) => {
    callback(event.payload);
  });
}
