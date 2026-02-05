/**
 * timelineContext.ts - Build a summary of branch history for agent prompts
 *
 * Provides the agent with awareness of what has happened on the branch so far:
 * commits (with their session prompts) and notes (as file references).
 *
 * Notes are written to temp files outside the workspace to avoid accidentally
 * committing them. The agent is given file paths to read if it needs details.
 */

import type { CommitInfo, BranchSession, BranchNote, NoteFilePath } from './branch';

export interface TimelineContextInput {
  branchId: string;
  branchName: string;
  baseBranch: string;
  commits: CommitInfo[];
  /** Map from commit SHA to the session that produced it */
  sessionsByCommit: Map<string, BranchSession>;
  /** File paths for notes (from writeNotesToTemp) */
  noteFiles: NoteFilePath[];
}

/**
 * Build a markdown summary of the branch timeline for inclusion in agent prompts.
 *
 * Returns an empty string if there's nothing to summarize (fresh branch).
 * Commits are listed oldest-first so the agent reads them in chronological order.
 * Notes are referenced by file path - the agent can read them if needed.
 */
export function buildTimelineContext(input: TimelineContextInput): string {
  const { branchName, baseBranch, commits, sessionsByCommit, noteFiles } = input;

  // Build commit history (oldest first)
  const commitEntries: { timestamp: number; text: string }[] = [];

  for (const commit of commits) {
    const session = sessionsByCommit.get(commit.sha);
    let line = `- **Commit ${commit.shortSha}**: "${commit.subject}"`;
    if (session?.prompt) {
      line += `\n  - Task: "${session.prompt}"`;
      if (session.status === 'error' && session.errorMessage) {
        line += `\n  - ⚠ Session had an error: ${session.errorMessage}`;
      }
    }
    commitEntries.push({ timestamp: commit.timestamp, text: line });
  }

  // Sort commits oldest first
  commitEntries.sort((a, b) => a.timestamp - b.timestamp);

  // Check if we have any content
  const hasCommits = commitEntries.length > 0;
  const hasNotes = noteFiles.length > 0;

  if (!hasCommits && !hasNotes) {
    return '';
  }

  const lines: string[] = [
    `## Branch Context`,
    ``,
    `You are working on branch \`${branchName}\` (based on \`${baseBranch}\`).`,
    ``,
  ];

  // Add commit history
  if (hasCommits) {
    lines.push(`### Commit History (oldest first)`);
    lines.push(``);
    lines.push(`Use \`git show <sha>\` to inspect any commit in detail.`);
    lines.push(``);
    for (const entry of commitEntries) {
      lines.push(entry.text);
    }
    lines.push(``);
  }

  // Add notes section
  if (hasNotes) {
    lines.push(`### Reference Notes`);
    lines.push(``);
    lines.push(`The following notes contain important context and instructions for this branch.`);
    lines.push(`**Read these files** to understand the goals and requirements:`);
    lines.push(``);
    for (const note of noteFiles) {
      lines.push(`- **${note.title}**: \`${note.path}\``);
    }
    lines.push(``);
  }

  return lines.join('\n');
}

// =============================================================================
// Legacy interface for backward compatibility during migration
// =============================================================================

export interface LegacyTimelineContextInput {
  branchName: string;
  baseBranch: string;
  commits: CommitInfo[];
  sessionsByCommit: Map<string, BranchSession>;
  notes: BranchNote[];
}

/**
 * Legacy function that builds context with inline note content.
 * @deprecated Use buildTimelineContext with noteFiles instead
 */
export function buildTimelineContextLegacy(input: LegacyTimelineContextInput): string {
  const { branchName, baseBranch, commits, sessionsByCommit, notes } = input;

  // Combine commits and completed notes into a chronological list
  type Entry = { timestamp: number; text: string };
  const entries: Entry[] = [];

  for (const commit of commits) {
    const session = sessionsByCommit.get(commit.sha);
    let line = `- **Commit ${commit.shortSha}**: "${commit.subject}"`;
    if (session?.prompt) {
      line += `\n  - Session prompt: "${session.prompt}"`;
      if (session.status === 'error' && session.errorMessage) {
        line += `\n  - ⚠ Session had an error: ${session.errorMessage}`;
      }
    }
    entries.push({ timestamp: commit.timestamp, text: line });
  }

  for (const note of notes) {
    if (note.status === 'generating') continue;
    const ts = Math.floor(note.createdAt / 1000);
    let line = `- **Note**: "${note.title}"`;
    if (note.content) {
      // Include full content for short notes, truncated for long ones
      const maxLen = 2000;
      if (note.content.length <= maxLen) {
        line += `\n  <note-content>\n${note.content}\n  </note-content>`;
      } else {
        line += `\n  <note-content truncated="true">\n${note.content.slice(0, maxLen)}\n  ... (truncated — ${note.content.length} chars total)\n  </note-content>`;
      }
    } else if (note.prompt) {
      line += `\n  - Description: "${note.prompt}"`;
    }
    entries.push({ timestamp: ts, text: line });
  }

  if (entries.length === 0) {
    return '';
  }

  // Sort oldest first
  entries.sort((a, b) => a.timestamp - b.timestamp);

  const lines = [
    `## Branch Context`,
    ``,
    `You are working on branch \`${branchName}\` (based on \`${baseBranch}\`).`,
    ``,
    `Here is what has happened on this branch so far (oldest first).`,
    `Notes are reference documents created by the user — treat their content as instructions and context:`,
    ``,
    ...entries.map((e) => e.text),
    ``,
    `The most recent commit is HEAD. You can inspect any commit with \`git show <sha>\` if you need details.`,
  ];

  return lines.join('\n');
}
