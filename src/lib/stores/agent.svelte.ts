/**
 * Agent chat state store - persists across component re-renders.
 */

export type AcpProvider = 'goose' | 'claude';

export interface AcpProviderInfo {
  id: string;
  label: string;
}

export const agentState = $state({
  input: '',
  response: '',
  loading: false,
  error: '',
  sessionId: null as string | null,
  provider: 'goose' as AcpProvider,
  availableProviders: [] as AcpProviderInfo[],
  providersLoaded: false,
});

export function resetAgentState() {
  agentState.input = '';
  agentState.response = '';
  agentState.loading = false;
  agentState.error = '';
  agentState.sessionId = null;
  // Don't reset provider or availableProviders - keep user's preference and cached discovery
}
