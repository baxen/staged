/**
 * Spine Connectors
 *
 * Draws bezier curve connectors between corresponding changed regions
 * in the center spine. These visualize how regions in the "before"
 * pane map to the "after" pane.
 *
 * The connectors only draw the top and bottom curves - no vertical lines
 * on the edges, since those would duplicate the range borders in the text panes.
 */

import type { Alignment } from './types';

export interface ConnectorConfig {
  lineHeight: number;
  /** Vertical offset to adjust bezier alignment (positive = down, negative = up) */
  verticalOffset: number;
  /** Index of the hovered alignment (in the changed alignments list), or null */
  hoveredIndex: number | null;
}

const DEFAULT_CONFIG: ConnectorConfig = {
  lineHeight: 20,
  verticalOffset: 0,
  hoveredIndex: null,
};

/**
 * Get CSS custom property value from the document.
 */
function getCssVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

/**
 * Draw connectors between changed alignments.
 */
export function drawConnectors(
  svg: SVGSVGElement,
  alignments: Alignment[],
  beforeScroll: number,
  afterScroll: number,
  config: Partial<ConnectorConfig> = {}
): void {
  const cfg = { ...DEFAULT_CONFIG, ...config };
  const svgWidth = svg.clientWidth;
  const svgHeight = svg.clientHeight;
  const cpOffset = svgWidth * 0.5; // bezier control point offset

  // Get colors from CSS variables (matches the text pane styling)
  const fillColor = getCssVar('--diff-changed-bg', 'rgba(128, 128, 128, 0.04)');
  const hoverFillColor = getCssVar('--bg-hover', 'rgba(128, 128, 128, 0.08)');
  const strokeColor = getCssVar('--diff-range-border', 'rgba(128, 128, 128, 0.2)');

  // Clear existing and set crisp rendering
  svg.innerHTML = '';
  svg.setAttribute('shape-rendering', 'geometricPrecision');

  // Track index among changed alignments (to match hoveredIndex)
  let changedIndex = 0;

  for (const alignment of alignments) {
    if (!alignment.changed) continue;

    const isHovered = cfg.hoveredIndex === changedIndex;
    changedIndex++;

    // Calculate pixel positions relative to viewport
    // Top border: at the top edge of the first line (start * lineHeight)
    // Bottom border: at the bottom edge of the last line (end * lineHeight)
    // The CSS pseudo-elements for range-start/range-end are at top:0 and bottom:0 of their lines
    // Add 0.5px offset for crisp 1px stroke rendering on pixel boundaries
    // Bottom uses -0.5 to align with the CSS ::after pseudo-element at bottom:0
    // verticalOffset adjusts for structural differences between SVG and code container
    const beforeTop =
      alignment.before.start * cfg.lineHeight - beforeScroll + 0.5 + cfg.verticalOffset;
    const beforeBottom =
      alignment.before.end * cfg.lineHeight - beforeScroll - 0.5 + cfg.verticalOffset;
    const afterTop =
      alignment.after.start * cfg.lineHeight - afterScroll + 0.5 + cfg.verticalOffset;
    const afterBottom =
      alignment.after.end * cfg.lineHeight - afterScroll - 0.5 + cfg.verticalOffset;

    // Skip if completely out of view
    if (beforeBottom < 0 && afterBottom < 0) continue;
    if (beforeTop > svgHeight && afterTop > svgHeight) continue;

    const isInsertion = alignment.before.start === alignment.before.end;
    const isDeletion = alignment.after.start === alignment.after.end;

    // Draw filled shape (no stroke - we'll draw strokes separately for top/bottom only)
    let fillPath: string;
    if (isInsertion) {
      // Point on left, range on right
      fillPath =
        `M 0 ${beforeTop} ` +
        `C ${cpOffset} ${beforeTop}, ${svgWidth - cpOffset} ${afterTop}, ${svgWidth} ${afterTop} ` +
        `L ${svgWidth} ${afterBottom} ` +
        `C ${svgWidth - cpOffset} ${afterBottom}, ${cpOffset} ${beforeTop}, 0 ${beforeTop} Z`;
    } else if (isDeletion) {
      // Range on left, point on right
      fillPath =
        `M 0 ${beforeTop} ` +
        `C ${cpOffset} ${beforeTop}, ${svgWidth - cpOffset} ${afterTop}, ${svgWidth} ${afterTop} ` +
        `C ${svgWidth - cpOffset} ${afterTop}, ${cpOffset} ${beforeBottom}, 0 ${beforeBottom} Z`;
    } else {
      // Curved trapezoid
      fillPath =
        `M 0 ${beforeTop} ` +
        `C ${cpOffset} ${beforeTop}, ${svgWidth - cpOffset} ${afterTop}, ${svgWidth} ${afterTop} ` +
        `L ${svgWidth} ${afterBottom} ` +
        `C ${svgWidth - cpOffset} ${afterBottom}, ${cpOffset} ${beforeBottom}, 0 ${beforeBottom} Z`;
    }

    const fill = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    fill.setAttribute('d', fillPath);
    fill.setAttribute('fill', isHovered ? hoverFillColor : fillColor);
    fill.setAttribute('stroke', 'none');
    svg.appendChild(fill);

    // Draw top curve stroke only (no vertical edges)
    const topCurve =
      `M 0 ${beforeTop} ` +
      `C ${cpOffset} ${beforeTop}, ${svgWidth - cpOffset} ${afterTop}, ${svgWidth} ${afterTop}`;

    const topStroke = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    topStroke.setAttribute('d', topCurve);
    topStroke.setAttribute('fill', 'none');
    topStroke.setAttribute('stroke', strokeColor);
    topStroke.setAttribute('stroke-width', '1');
    topStroke.setAttribute('vector-effect', 'non-scaling-stroke');
    svg.appendChild(topStroke);

    // Draw bottom curve stroke only (no vertical edges)
    // For insertions/deletions where one side is a point, bottom curve connects differently
    let bottomCurve: string;
    if (isInsertion) {
      // Bottom connects afterBottom back to beforeTop (the point)
      bottomCurve =
        `M ${svgWidth} ${afterBottom} ` +
        `C ${svgWidth - cpOffset} ${afterBottom}, ${cpOffset} ${beforeTop}, 0 ${beforeTop}`;
    } else if (isDeletion) {
      // Bottom connects afterTop (the point) back to beforeBottom
      bottomCurve =
        `M ${svgWidth} ${afterTop} ` +
        `C ${svgWidth - cpOffset} ${afterTop}, ${cpOffset} ${beforeBottom}, 0 ${beforeBottom}`;
    } else {
      // Normal case: bottom connects afterBottom to beforeBottom
      bottomCurve =
        `M ${svgWidth} ${afterBottom} ` +
        `C ${svgWidth - cpOffset} ${afterBottom}, ${cpOffset} ${beforeBottom}, 0 ${beforeBottom}`;
    }

    const bottomStroke = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    bottomStroke.setAttribute('d', bottomCurve);
    bottomStroke.setAttribute('fill', 'none');
    bottomStroke.setAttribute('stroke', strokeColor);
    bottomStroke.setAttribute('stroke-width', '1');
    bottomStroke.setAttribute('vector-effect', 'non-scaling-stroke');
    svg.appendChild(bottomStroke);
  }
}
