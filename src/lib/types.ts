// =============================================================================
// Git Ref types
// =============================================================================

/** A reference to a point in git history (or working tree) */
export type GitRef =
  | { type: 'WorkingTree' }
  | { type: 'Rev'; value: string }
  | { type: 'MergeBase' };

/** Get display string for a GitRef */
export function gitRefDisplay(ref: GitRef): string {
  if (ref.type === 'WorkingTree') return '@';
  if (ref.type === 'MergeBase') return 'merge-base';
  return ref.value;
}

/** Get a ref name suitable for git commands (e.g., for loading reference files) */
export function gitRefName(ref: GitRef): string {
  if (ref.type === 'WorkingTree') return 'HEAD';
  if (ref.type === 'MergeBase') return 'HEAD'; // For file loading, use HEAD
  return ref.value;
}

/** Inferred type of a ref string for display purposes */
export type RefType = 'branch' | 'tag' | 'remote' | 'special';

/**
 * Infer the type of a ref from its string representation.
 * Best-effort heuristic for display icons.
 */
export function inferRefType(ref: string): RefType {
  // Special refs
  if (ref === '@' || ref === 'HEAD' || ref.startsWith('HEAD~') || ref.startsWith('HEAD^')) {
    return 'special';
  }
  // Tags (from refs/tags/ or common tag patterns like v1.0.0)
  if (ref.startsWith('refs/tags/') || /^v?\d+\.\d+/.test(ref)) {
    return 'tag';
  }
  // Remotes (origin/*, upstream/*, etc.)
  if (ref.includes('/') && !ref.startsWith('refs/')) {
    return 'remote';
  }
  // Default to branch
  return 'branch';
}

/** What we're diffing - always base..head */
export interface DiffSpec {
  base: GitRef;
  head: GitRef;
}

/** Convenience constructors for DiffSpec */
export const DiffSpec = {
  /** Uncommitted changes: HEAD..@ */
  uncommitted(): DiffSpec {
    return {
      base: { type: 'Rev', value: 'HEAD' },
      head: { type: 'WorkingTree' },
    };
  },

  /** Last commit: HEAD~1..HEAD */
  lastCommit(): DiffSpec {
    return {
      base: { type: 'Rev', value: 'HEAD~1' },
      head: { type: 'Rev', value: 'HEAD' },
    };
  },

  /** Branch changes: merge-base(defaultBranch, HEAD)..WorkingTree */
  branchChanges(): DiffSpec {
    return {
      base: { type: 'MergeBase' },
      head: { type: 'WorkingTree' },
    };
  },

  /** Custom range */
  custom(base: GitRef, head: GitRef): DiffSpec {
    return { base, head };
  },

  /** From two rev strings */
  fromRevs(base: string, head: string): DiffSpec {
    return {
      base: { type: 'Rev', value: base },
      head: { type: 'Rev', value: head },
    };
  },

  /** Display as "base..head" */
  display(spec: DiffSpec): string {
    const formatRef = (ref: GitRef): string => {
      if (ref.type === 'WorkingTree') return '@';
      if (ref.type === 'MergeBase') return 'merge-base';
      return ref.value;
    };
    return `${formatRef(spec.base)}..${formatRef(spec.head)}`;
  },
};

// =============================================================================
// File types
// =============================================================================

/** A contiguous range of lines (0-indexed, exclusive end) */
export interface Span {
  start: number;
  end: number;
}

/** Content of a file - either text lines or binary marker */
export type FileContent = { type: 'Text'; lines: string[] } | { type: 'Binary' };

/** A file with its path and content */
export interface File {
  path: string;
  content: FileContent;
}

/** Summary of a file in the diff (for sidebar) */
export interface FileDiffSummary {
  before: string | null;
  after: string | null;
}

/** Maps a region in the before file to a region in the after file */
export interface Alignment {
  before: Span;
  after: Span;
  /** True if this region contains changes */
  changed: boolean;
}

/** Full diff content for rendering a single file */
export interface FileDiff {
  /** File before the change (null if added) */
  before: File | null;
  /** File after the change (null if deleted) */
  after: File | null;
  /** Alignments mapping regions between before/after */
  alignments: Alignment[];
}

// =============================================================================
// GitHub types
// =============================================================================

/** A pull request from GitHub (for display in picker) */
export interface PullRequest {
  number: number;
  title: string;
  author: string;
  /** Target branch (e.g., "main") */
  base_ref: string;
  /** Source branch (e.g., "feature-x") */
  head_ref: string;
  draft: boolean;
  updated_at: string;
}

/** GitHub authentication status */
export interface GitHubAuthStatus {
  authenticated: boolean;
  /** Help text if not authenticated */
  setup_hint: string | null;
}

/** Result of syncing a review to GitHub */
export interface GitHubSyncResult {
  /** URL to the pending review on GitHub */
  review_url: string;
  /** Number of comments synced */
  comment_count: number;
}

// =============================================================================
// Review types
// =============================================================================

/** Identifies a diff by its two endpoints */
export interface DiffId {
  before: string;
  after: string;
}

/** A comment attached to a specific location in a file */
export interface Comment {
  id: string;
  path: string;
  /** The line range this comment applies to (0-indexed, exclusive end) */
  span: Span;
  content: string;
  /** Who authored this comment */
  author: 'user' | 'ai';
  /** Category (only for AI comments) */
  category?: AnnotationCategory;
  /** When the comment was created (ISO timestamp) */
  created_at?: string;
}

/** An edit made during review, stored as a unified diff */
export interface Edit {
  id: string;
  path: string;
  diff: string;
}

/** A review attached to a specific diff */
export interface Review {
  id: DiffId;
  reviewed: string[];
  comments: Comment[];
  edits: Edit[];
  reference_files: string[];
}

/** Input for creating a new comment */
export interface NewComment {
  path: string;
  span: Span;
  content: string;
}

/** Input for recording a new edit */
export interface NewEdit {
  path: string;
  diff: string;
}

// =============================================================================
// Smart Diff (AI) types
// =============================================================================

/** A span of lines for AI annotations (0-indexed, exclusive end) */
export interface LineSpan {
  start: number;
  end: number;
}

/** Category of AI annotation */
export type AnnotationCategory = 'explanation' | 'warning' | 'suggestion' | 'context';

/** A single AI annotation on a diff */
export interface SmartDiffAnnotation {
  id: string;
  /** Description of the old state (for before_span annotations) */
  before_description?: string;
  /** File path this annotation belongs to (for changeset-level analysis) */
  file_path?: string;
  /** Span in 'before' content (undefined if only applies to 'after') */
  before_span?: LineSpan;
  /** Span in 'after' content (undefined if only applies to 'before') */
  after_span?: LineSpan;
  /** The AI commentary */
  content: string;
  /** Category for styling */
  category: AnnotationCategory;
}

/** Result of AI analysis on a single file's diff */
export interface SmartDiffResult {
  /** TL;DR summary of the file's changes */
  overview: string;
  /** Span-based annotations for this file */
  annotations: SmartDiffAnnotation[];
}

/** Summary portion of changeset analysis (used for storage) */
export interface ChangesetSummary {
  /** High-level summary of what this changeset accomplishes */
  summary: string;
  /** Key changes organized by theme/area */
  key_changes: string[];
  /** Potential concerns or things to watch out for */
  concerns: string[];
}

/** Complete analysis of an entire changeset */
export interface ChangesetAnalysis {
  /** High-level summary of what this changeset accomplishes */
  summary: string;
  /** Key changes organized by theme/area */
  key_changes: string[];
  /** Potential concerns or things to watch out for */
  concerns: string[];
  /** Annotations keyed by file path */
  file_annotations: Record<string, SmartDiffAnnotation[]>;
}

// =============================================================================
// Project types (artifact-centric model)
// =============================================================================

/** A goal-oriented collection of artifacts */
export interface Project {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

/** Type-specific data for an artifact */
export type ArtifactData =
  | { type: 'markdown'; content: string }
  | { type: 'commit'; repo: string; branch: string; commitSha: string };

/** Generation status of an artifact */
export type ArtifactStatus = 'generating' | 'complete' | 'error';

/** The persistent output of AI work */
export interface Artifact {
  id: string;
  projectId: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  parentArtifactId?: string;
  /** Type-specific data (markdown content, commit info, etc.) */
  data: ArtifactData;
  /** Generation status */
  status: ArtifactStatus;
  /** Error message if status is 'error' */
  errorMessage?: string;
}

/** An ephemeral AI conversation that produced an artifact */
export interface Session {
  id: string;
  artifactId: string;
  createdAt: string;
  /** The conversation transcript */
  transcript: string;
}

// =============================================================================
// ACP Session Streaming Types (matches agent-client-protocol SDK)
// =============================================================================

/** Notification sent by agent during session - emitted as "session-update" event */
export interface SessionNotification {
  sessionId: string;
  update: SessionUpdate;
}

/** Discriminated union of session update types */
export type SessionUpdate =
  | SessionUpdateAgentMessageChunk
  | SessionUpdateUserMessageChunk
  | SessionUpdateAgentThoughtChunk
  | SessionUpdateToolCall
  | SessionUpdateToolCallUpdate
  | SessionUpdatePlan
  | SessionUpdateOther;

export interface SessionUpdateAgentMessageChunk extends ContentChunk {
  sessionUpdate: 'agent_message_chunk';
}

export interface SessionUpdateUserMessageChunk extends ContentChunk {
  sessionUpdate: 'user_message_chunk';
}

export interface SessionUpdateAgentThoughtChunk extends ContentChunk {
  sessionUpdate: 'agent_thought_chunk';
}

export interface SessionUpdateToolCall extends ToolCall {
  sessionUpdate: 'tool_call';
}

export interface SessionUpdateToolCallUpdate extends ToolCallUpdatePayload {
  sessionUpdate: 'tool_call_update';
}

export interface SessionUpdatePlan {
  sessionUpdate: 'plan';
  entries: PlanEntry[];
}

export interface SessionUpdateOther {
  sessionUpdate: string;
  [key: string]: unknown;
}

export interface ContentChunk {
  content: ContentBlock;
}

export type ContentBlock =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mimeType: string }
  | { type: string; [key: string]: unknown }; // Catch-all

export interface ToolCall {
  toolCallId: string;
  title: string;
  status: ToolCallStatus;
  kind: ToolKind;
  locations: ToolCallLocation[];
  content: ToolCallContent[];
}

export interface ToolCallUpdatePayload {
  toolCallId: string;
  fields: ToolCallUpdateFields;
}

export interface ToolCallUpdateFields {
  title?: string;
  status?: ToolCallStatus;
  content?: ToolCallContent[];
  locations?: ToolCallLocation[];
}

export type ToolCallStatus = 'pending' | 'in_progress' | 'completed' | 'failed';
export type ToolKind =
  | 'read'
  | 'edit'
  | 'delete'
  | 'move'
  | 'search'
  | 'execute'
  | 'think'
  | 'fetch'
  | 'switch_mode'
  | 'other';

export interface ToolCallLocation {
  path: string;
  line?: number;
}

export type ToolCallContent =
  | { type: 'content'; content: ContentBlock }
  | { type: 'diff'; path: string; oldText?: string; newText: string }
  | { type: 'terminal'; terminalId: string };

export interface PlanEntry {
  title: string;
  status?: 'pending' | 'in_progress' | 'completed' | 'failed';
}

// Session complete event (custom, emitted as "session-complete")
export interface SessionCompleteEvent {
  sessionId: string;
  transcript: FinalizedMessage[];
}

// Session error event (custom, emitted as "session-error")
export interface SessionErrorEvent {
  sessionId: string;
  error: string;
}

// Finalized message for storage/display (stored in DB)
export type FinalizedMessage =
  | { role: 'user'; content: string }
  | { role: 'assistant'; content: string; toolCalls?: ToolCallSummary[] };

export interface ToolCallSummary {
  id: string;
  title: string;
  status: string;
  locations: string[];
  resultPreview?: string;
}

// =============================================================================
// Live Session State (frontend-only, not persisted)
// =============================================================================

export interface LiveToolCall {
  id: string;
  title: string;
  status: ToolCallStatus;
  kind: ToolKind;
  locations: string[];
  preview?: string;
}

export interface LiveSession {
  sessionId: string;
  isStreaming: boolean;
  currentText: string;
  toolCalls: Map<string, LiveToolCall>;
  error?: string;
  finalTranscript: FinalizedMessage[] | null;
}
