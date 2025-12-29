export interface FileStatus {
  path: string;
  status: 'modified' | 'added' | 'deleted' | 'renamed' | 'typechange' | 'untracked' | 'unknown';
}

// =============================================================================
// New simplified diff types
// =============================================================================

/** Content of a file - either text lines or binary marker */
export type FileContent = { type: 'text'; lines: string[] } | { type: 'binary' };

/** A file with its path and content */
export interface DiffFile {
  path: string;
  content: FileContent;
}

/** A contiguous range of lines (0-indexed, exclusive end) */
export interface Span {
  start: number;
  end: number;
}

/** Maps a region in the before file to a region in the after file */
export interface Alignment {
  before: Span;
  after: Span;
  /** True if this region contains changes */
  changed: boolean;
}

/** The diff for a single file between two states */
export interface FileDiff {
  /** File before the change (null if added) */
  before: DiffFile | null;
  /** File after the change (null if deleted) */
  after: DiffFile | null;
  /** Alignments mapping regions between before/after */
  alignments: Alignment[];
}

// =============================================================================
// Legacy types (to be removed)
// =============================================================================

export interface GitStatus {
  staged: FileStatus[];
  unstaged: FileStatus[];
  untracked: FileStatus[];
  branch: string | null;
  repo_path: string;
}

export interface DiffLine {
  line_type: 'context' | 'added' | 'removed';
  lineno: number;
  content: string;
}

export interface DiffHunk {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  header: string;
  lines: DiffLine[];
}

/** Half-open interval [start, end) of row indices */
export interface Span {
  start: number;
  end: number;
}

/** Source file line numbers for a changed region (1-indexed, inclusive) */
export interface SourceLines {
  /** Lines removed from the "before" file. None if pure addition */
  old_start: number | null;
  old_end: number | null;
  /** Lines added in the "after" file. None if pure deletion */
  new_start: number | null;
  new_end: number | null;
}

/** Maps corresponding regions between before/after panes */
export interface Range {
  before: Span;
  after: Span;
  /** true = region contains changes, false = identical lines */
  changed: boolean;
  /** Source file line numbers (only present for changed ranges) */
  source_lines?: SourceLines;
}

/** Content for one side of the diff */
export interface LegacyDiffSide {
  path: string | null;
  lines: DiffLine[];
}

/** @deprecated Use FileDiff instead */
export interface LegacyFileDiff {
  status: string;
  is_binary: boolean;
  hunks: DiffHunk[];
  before: LegacyDiffSide;
  after: LegacyDiffSide;
  ranges: Range[];
}

export interface CommitResult {
  oid: string;
  message: string;
}

/** A file that changed between two refs */
export interface ChangedFile {
  path: string;
  status: string;
}

/** A git reference for autocomplete */
export interface GitRef {
  name: string;
  ref_type: 'branch' | 'tag' | 'special';
}

// Review types

export interface DiffId {
  base: string; // SHA
  head: string; // SHA or "@" for working tree
}

/** A diff specification with display label */
export interface DiffSpec {
  base: string;
  head: string;
  label: string;
}

export interface Comment {
  id: string;
  file_path: string;
  range_index: number;
  text: string;
  created_at: string;
}

export interface Edit {
  id: string;
  file_path: string;
  diff: string;
  created_at: string;
}

export interface Review {
  id: DiffId;
  reviewed: string[]; // file paths
  comments: Comment[];
  edits: Edit[];
  created_at: string;
  updated_at: string;
}

export interface NewComment {
  file_path: string;
  range_index: number;
  text: string;
}

export interface NewEdit {
  file_path: string;
  diff: string;
}
