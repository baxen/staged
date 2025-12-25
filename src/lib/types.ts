export interface FileStatus {
  path: string;
  status: 'modified' | 'added' | 'deleted' | 'renamed' | 'typechange' | 'untracked' | 'unknown';
}

export interface GitStatus {
  staged: FileStatus[];
  unstaged: FileStatus[];
  untracked: FileStatus[];
  branch: string | null;
  repo_path: string;
}

export interface DiffLine {
  line_type: 'context' | 'added' | 'removed';
  old_lineno: number | null;
  new_lineno: number | null;
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

// A row in the side-by-side view - either a line of code or a collapse indicator
export type DiffRow =
  | ({ type: 'Line' } & DiffLine)
  | {
      type: 'Collapse';
      count: number;
      start_line: number;
      other_pane_index: number;
    };

export interface FileDiff {
  path: string;
  old_path: string | null;
  status: string;
  hunks: DiffHunk[];
  is_binary: boolean;
  old_content: DiffRow[];
  new_content: DiffRow[];
}

export interface CommitResult {
  oid: string;
  message: string;
}
