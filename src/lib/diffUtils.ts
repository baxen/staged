/**
 * Diff Utilities
 *
 * Pure helper functions for diff display.
 */

import type { LegacyFileDiff, Range } from './types';

/**
 * Get display path, handling renames.
 */
export function getDisplayPath(diff: LegacyFileDiff): string {
  const { before, after } = diff;

  if (before.path && after.path && before.path !== after.path) {
    return `${before.path} → ${after.path}`;
  }
  return after.path || before.path || '';
}

/**
 * Check if a line is at the start or end of a changed range.
 * Used to draw horizontal separator lines in CSS.
 *
 * For empty spans (e.g., the "before" side of a pure insert), we draw a single
 * line to avoid the double-thick appearance from adjacent top/bottom borders.
 * - If there's a preceding line, draw on its bottom edge
 * - If at file start (no preceding line), draw on the following line's top edge
 */
export function getLineBoundary(
  ranges: Range[],
  side: 'before' | 'after',
  lineIndex: number
): { isStart: boolean; isEnd: boolean } {
  for (const range of ranges) {
    if (!range.changed) continue;

    const span = side === 'before' ? range.before : range.after;

    // Empty span: draw a single line at the insertion point.
    // Use range-start on the line AT span.start (its top edge aligns with
    // where the connector attaches at span.start * lineHeight).
    if (span.start === span.end) {
      if (lineIndex === span.start) {
        return { isStart: true, isEnd: false };
      }
      continue;
    }

    if (lineIndex === span.start) {
      return { isStart: true, isEnd: lineIndex === span.end - 1 };
    }
    if (lineIndex === span.end - 1) {
      return { isStart: false, isEnd: true };
    }
  }
  return { isStart: false, isEnd: false };
}

/**
 * Detect language from diff paths (prefers after path).
 */
export function getLanguageFromDiff<T>(
  diff: LegacyFileDiff,
  detectLanguage: (path: string) => T | null
): T | null {
  if (diff.after.path) return detectLanguage(diff.after.path);
  if (diff.before.path) return detectLanguage(diff.before.path);
  return null;
}
