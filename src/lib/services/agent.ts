/**
 * Tauri command bindings for agent integration via ACP.
 */
import { invoke } from '@tauri-apps/api/core';

// Session info returned from backend
export interface SessionInfo {
  id: string;
  name: string;
  agentId: string;
  workingDir: string;
  createdAt: number;
  messageCount: number;
  acpSessionId?: string;
}

/**
 * Create a new agent session.
 * @param workingDir - The working directory for the agent
 * @param name - A name for the session
 * @param agentId - The agent to use (e.g., 'goose', 'claude-code'). Defaults to 'goose'.
 */
export async function createSession(
  workingDir: string,
  name: string,
  agentId?: string
): Promise<SessionInfo> {
  return invoke<SessionInfo>('agent_create_session', { workingDir, name, agentId });
}

/**
 * Send a message to an agent session.
 * Responses are streamed via the "agent-event" Tauri event.
 * @param sessionId - The session to send the message to
 * @param message - The message text to send
 */
export async function sendMessage(sessionId: string, message: string): Promise<void> {
  return invoke<void>('agent_send_message', { sessionId, message });
}
