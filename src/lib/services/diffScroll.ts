/**
 * Diff Scroll Synchronization
 *
 * Handles anchor-based scroll sync between two diff panes.
 * Context lines act as anchors - they exist in both panes and should align.
 *
 * ## How it works:
 * 1. Build a map of anchor positions (context lines that exist in both panes)
 * 2. When scrolling, find the nearest anchor above current scroll position
 * 3. Scroll the other pane to align that anchor
 * 4. Between anchors, the other pane stays put
 *
 * ## Known issues / TODOs:
 * - Scroll can be jumpy when transitioning between anchors
 * - Need to handle edge cases at start/end of file
 * - Consider smoothing or easing for better UX
 */

import type { DiffRow } from '../types';

const LINE_HEIGHT = 20; // Must match CSS .line min-height

/** Check if a DiffRow is a Line */
export function isLine(row: DiffRow): row is DiffRow & { type: 'Line' } {
  return row.type === 'Line';
}

/** Check if a DiffRow is a Collapse indicator */
export function isCollapse(row: DiffRow): row is DiffRow & { type: 'Collapse' } {
  return row.type === 'Collapse';
}

/** Bidirectional anchor maps between old and new content */
export interface AnchorMaps {
  oldToNew: Map<number, number>;
  newToOld: Map<number, number>;
}

/**
 * Build anchor maps from diff content.
 * Anchors are context lines that appear in both panes.
 */
export function buildAnchorMaps(oldContent: DiffRow[], newContent: DiffRow[]): AnchorMaps {
  const oldToNew = new Map<number, number>();
  const newToOld = new Map<number, number>();

  let oldIdx = 0;
  let newIdx = 0;

  while (oldIdx < oldContent.length && newIdx < newContent.length) {
    const oldRow = oldContent[oldIdx];
    const newRow = newContent[newIdx];

    // Both are context lines - they're anchors
    if (
      isLine(oldRow) &&
      isLine(newRow) &&
      oldRow.line_type === 'context' &&
      newRow.line_type === 'context'
    ) {
      oldToNew.set(oldIdx, newIdx);
      newToOld.set(newIdx, oldIdx);
      oldIdx++;
      newIdx++;
    } else if (isLine(oldRow) && oldRow.line_type === 'removed') {
      oldIdx++;
    } else if (isLine(newRow) && newRow.line_type === 'added') {
      newIdx++;
    } else if (isCollapse(oldRow)) {
      oldIdx++;
    } else if (isCollapse(newRow)) {
      newIdx++;
    } else {
      // Fallback
      oldIdx++;
      newIdx++;
    }
  }

  return { oldToNew, newToOld };
}

/**
 * Find the nearest anchor at or before the given row index.
 */
export function findNearestAnchor(
  rowIndex: number,
  anchorMap: Map<number, number>
): { sourceIdx: number; targetIdx: number } | null {
  for (let i = rowIndex; i >= 0; i--) {
    if (anchorMap.has(i)) {
      return { sourceIdx: i, targetIdx: anchorMap.get(i)! };
    }
  }
  return null;
}

/**
 * Calculate the target scroll position based on anchor alignment.
 */
export function calculateTargetScroll(
  sourceScrollTop: number,
  anchorMap: Map<number, number>,
  targetContentLength: number
): number {
  const sourceRowIndex = Math.floor(sourceScrollTop / LINE_HEIGHT);
  const sourceRowOffset = sourceScrollTop % LINE_HEIGHT;

  const anchor = findNearestAnchor(sourceRowIndex, anchorMap);

  if (!anchor) {
    // No anchor found, stay at current position or top
    return 0;
  }

  const rowsPastAnchor = sourceRowIndex - anchor.sourceIdx;
  const targetRowIndex = anchor.targetIdx + rowsPastAnchor;

  // Clamp to valid range
  const clampedTargetRow = Math.max(0, Math.min(targetRowIndex, targetContentLength - 1));

  return clampedTargetRow * LINE_HEIGHT + sourceRowOffset;
}

/**
 * Scroll sync controller - manages state and prevents feedback loops.
 */
export function createScrollSync() {
  let isSyncing = false;
  let lastScrollSource: 'left' | 'right' | null = null;

  return {
    /**
     * Sync scroll from source pane to target pane.
     * Returns true if sync was performed, false if skipped (feedback prevention).
     */
    sync(
      source: HTMLDivElement,
      target: HTMLDivElement | null,
      side: 'left' | 'right',
      oldContent: DiffRow[],
      newContent: DiffRow[]
    ): boolean {
      if (isSyncing || !target) return false;

      // Prevent feedback loop
      if (lastScrollSource !== null && lastScrollSource !== side) {
        return false;
      }

      isSyncing = true;
      lastScrollSource = side;

      const { oldToNew, newToOld } = buildAnchorMaps(oldContent, newContent);
      const anchorMap = side === 'left' ? oldToNew : newToOld;
      const targetContent = side === 'left' ? newContent : oldContent;

      const targetScrollTop = calculateTargetScroll(
        source.scrollTop,
        anchorMap,
        targetContent.length
      );

      // Only update if difference is significant
      if (Math.abs(target.scrollTop - targetScrollTop) > 1) {
        target.scrollTop = targetScrollTop;
      }

      // Sync horizontal scroll directly
      target.scrollLeft = source.scrollLeft;

      requestAnimationFrame(() => {
        isSyncing = false;
        setTimeout(() => {
          lastScrollSource = null;
        }, 50);
      });

      return true;
    },
  };
}
