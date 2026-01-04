<!--
  ScrollbarMarkers.svelte - Visual indicators on the scrollbar track
  
  Shows horizontal markers indicating where changes and comments are located
  in the file, so users can see at a glance if there are more below/above
  their current scroll position.
  
  Markers are positioned proportionally based on line numbers relative to
  total file length.
-->
<script lang="ts">
  import type { Alignment, Comment } from './types';

  interface Props {
    /** All alignments for the current diff */
    alignments: Alignment[];
    /** Comments for the current file */
    comments: Comment[];
    /** Total number of lines in the file (for proportional positioning) */
    totalLines: number;
    /** Which side we're showing markers for */
    side: 'before' | 'after';
  }

  let { alignments, comments, totalLines, side }: Props = $props();

  // Changed alignments only (context regions don't need markers)
  let changedAlignments = $derived(alignments.filter((a) => a.changed));

  // Compute marker positions as percentages
  let alignmentMarkers = $derived.by(() => {
    if (totalLines === 0) return [];

    return changedAlignments.map((alignment) => {
      const span = side === 'before' ? alignment.before : alignment.after;
      // Position at the start of the range
      const startPercent = (span.start / totalLines) * 100;
      // Height based on range size (minimum 2px worth, roughly 0.5%)
      const rangeSize = span.end - span.start;
      const heightPercent = Math.max(0.5, (rangeSize / totalLines) * 100);

      return {
        top: startPercent,
        height: heightPercent,
        type: 'alignment' as const,
      };
    });
  });

  // Comment markers (only for 'after' side since comments are on the after pane)
  let commentMarkers = $derived.by(() => {
    if (side !== 'after' || totalLines === 0) return [];

    // Filter out global comments (0,0 span)
    return comments
      .filter((c) => c.span.start !== 0 || c.span.end !== 0)
      .map((comment) => {
        const startPercent = (comment.span.start / totalLines) * 100;
        const rangeSize = Math.max(1, comment.span.end - comment.span.start);
        const heightPercent = Math.max(0.5, (rangeSize / totalLines) * 100);

        return {
          top: startPercent,
          height: heightPercent,
          type: 'comment' as const,
        };
      });
  });

  // Combine all markers
  let allMarkers = $derived([...alignmentMarkers, ...commentMarkers]);
</script>

<div class="scrollbar-markers" class:left-side={side === 'before'}>
  {#each allMarkers as marker}
    <div
      class="marker"
      class:alignment={marker.type === 'alignment'}
      class:comment={marker.type === 'comment'}
      style="top: {marker.top}%; height: {marker.height}%;"
    ></div>
  {/each}
</div>

<style>
  .scrollbar-markers {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 8px; /* Match scrollbar width */
    pointer-events: none;
    z-index: 1;
  }

  .scrollbar-markers.left-side {
    right: auto;
    left: 0;
  }

  .marker {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    width: 6px;
    min-height: 2px;
    border-radius: 1px;
  }

  .marker.alignment {
    background-color: var(--diff-range-border);
  }

  .marker.comment {
    background-color: var(--diff-comment-highlight);
  }
</style>
