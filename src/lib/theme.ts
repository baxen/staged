/**
 * Adaptive Theme Infrastructure for Staged
 *
 * The UI chrome colors are derived from the syntax highlighting theme.
 * This ensures a unified look where the sidebar and controls blend
 * seamlessly with the code area.
 *
 * Usage in CSS (via CSS custom properties):
 *   color: var(--text-primary);
 *   background: var(--bg-primary);
 */

export interface Theme {
  // Base colors
  bg: {
    primary: string; // Main background
    secondary: string; // Sidebar, panels
    tertiary: string; // Headers, hover states
    input: string; // Input fields
  };

  // Borders and dividers
  border: {
    primary: string; // Main borders
    subtle: string; // Subtle dividers
  };

  // Text colors
  text: {
    primary: string; // Main text
    secondary: string; // Subdued text
    muted: string; // Very subdued (hints, placeholders)
    link: string; // Links and interactive text
  };

  // Git status colors (for file list icons)
  status: {
    modified: string;
    added: string;
    deleted: string;
    renamed: string;
    untracked: string;
  };

  // Diff viewer colors
  diff: {
    // Overlay tints - applied on TOP of syntax theme background
    addedOverlay: string; // Tint for added lines (right pane)
    removedOverlay: string; // Tint for removed lines (left pane)

    // Text colors
    addedText: string; // Line number text for added
    removedText: string; // Line number text for removed
    lineNumber: string; // Default line number text

    // Other
    emptyBg: string; // Background for empty/padding lines
    headerBg: string; // Diff header background
  };

  // Interactive elements
  ui: {
    accent: string; // Primary accent (buttons, focus)
    accentHover: string; // Accent hover state
    danger: string; // Destructive actions
    dangerHover: string; // Danger hover state
    success: string; // Success states
    selection: string; // Selected items
  };

  // Syntax highlighting fallbacks (Shiki handles actual highlighting)
  syntax: {
    keyword: string;
    string: string;
    number: string;
    comment: string;
    function: string;
    variable: string;
    type: string;
    operator: string;
    punctuation: string;
  };

  // Scrollbar
  scrollbar: {
    track: string;
    thumb: string;
    thumbHover: string;
  };
}

// =============================================================================
// Color Utilities
// =============================================================================

/**
 * Parse a hex color to RGB components
 */
function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!result) return { r: 30, g: 30, b: 30 }; // fallback dark
  return {
    r: parseInt(result[1], 16),
    g: parseInt(result[2], 16),
    b: parseInt(result[3], 16),
  };
}

/**
 * Convert RGB to hex
 */
function rgbToHex(r: number, g: number, b: number): string {
  const clamp = (n: number) => Math.max(0, Math.min(255, Math.round(n)));
  return `#${[r, g, b].map((c) => clamp(c).toString(16).padStart(2, '0')).join('')}`;
}

/**
 * Lighten a color by a factor (0-1)
 */
function lighten(hex: string, factor: number): string {
  const { r, g, b } = hexToRgb(hex);
  return rgbToHex(r + (255 - r) * factor, g + (255 - g) * factor, b + (255 - b) * factor);
}

// =============================================================================
// Adaptive Theme Generator
// =============================================================================

/**
 * Create an adaptive theme based on syntax theme colors.
 * Uses the exact bg/fg/comment colors from the syntax theme for a unified look.
 */
export function createAdaptiveTheme(
  syntaxBg: string,
  syntaxFg: string,
  syntaxComment: string
): Theme {
  // Use exact colors from syntax theme
  const bg = syntaxBg;
  const fg = syntaxFg;
  const muted = syntaxComment;

  // Borders - subtle lift from background
  const borderPrimary = lighten(bg, 0.12);
  const borderSubtle = lighten(bg, 0.06);

  // Accent - use a standard blue that works across themes
  const accent = '#58a6ff';

  return {
    bg: {
      primary: bg,
      secondary: bg, // Same as primary - no layering, borders define regions
      tertiary: lighten(bg, 0.05),
      input: lighten(bg, 0.08),
    },

    border: {
      primary: borderPrimary,
      subtle: borderSubtle,
    },

    text: {
      primary: fg,
      secondary: fg, // Same as primary for cleaner look
      muted: muted, // Comment color for muted text
      link: accent,
    },

    // Status colors - consistent across themes for recognition
    status: {
      modified: '#d29922',
      added: '#3fb950',
      deleted: '#f85149',
      renamed: '#58a6ff',
      untracked: muted,
    },

    diff: {
      // Neutral smoke overlays - just subtle lightening, no color tint
      addedOverlay: 'rgba(255, 255, 255, 0.04)',
      removedOverlay: 'rgba(255, 255, 255, 0.04)',
      addedText: '#3fb950',
      removedText: '#f85149',
      lineNumber: muted,
      emptyBg: bg,
      headerBg: bg,
    },

    ui: {
      accent: '#238636',
      accentHover: '#2ea043',
      danger: '#da3633',
      dangerHover: '#f85149',
      success: '#238636',
      selection: `${accent}33`,
    },

    // Syntax colors are handled by Shiki, these are fallbacks
    syntax: {
      keyword: '#ff7b72',
      string: '#a5d6ff',
      number: '#79c0ff',
      comment: muted,
      function: '#d2a8ff',
      variable: '#ffa657',
      type: '#7ee787',
      operator: fg,
      punctuation: fg,
    },

    scrollbar: {
      track: bg,
      thumb: borderPrimary,
      thumbHover: lighten(borderPrimary, 0.1),
    },
  };
}

/**
 * Generate CSS custom properties from theme.
 * These are applied to :root for CSS-based theming.
 */
export function themeToCssVars(t: Theme): string {
  return `
    --bg-primary: ${t.bg.primary};
    --bg-secondary: ${t.bg.secondary};
    --bg-tertiary: ${t.bg.tertiary};
    --bg-input: ${t.bg.input};

    --border-primary: ${t.border.primary};
    --border-subtle: ${t.border.subtle};

    --text-primary: ${t.text.primary};
    --text-secondary: ${t.text.secondary};
    --text-muted: ${t.text.muted};
    --text-link: ${t.text.link};

    --status-modified: ${t.status.modified};
    --status-added: ${t.status.added};
    --status-deleted: ${t.status.deleted};
    --status-renamed: ${t.status.renamed};
    --status-untracked: ${t.status.untracked};

    --diff-added-overlay: ${t.diff.addedOverlay};
    --diff-removed-overlay: ${t.diff.removedOverlay};
    --diff-added-text: ${t.diff.addedText};
    --diff-removed-text: ${t.diff.removedText};
    --diff-line-number: ${t.diff.lineNumber};
    --diff-empty-bg: ${t.diff.emptyBg};
    --diff-header-bg: ${t.diff.headerBg};

    --ui-accent: ${t.ui.accent};
    --ui-accent-hover: ${t.ui.accentHover};
    --ui-danger: ${t.ui.danger};
    --ui-danger-hover: ${t.ui.dangerHover};
    --ui-success: ${t.ui.success};
    --ui-selection: ${t.ui.selection};

    --syntax-keyword: ${t.syntax.keyword};
    --syntax-string: ${t.syntax.string};
    --syntax-number: ${t.syntax.number};
    --syntax-comment: ${t.syntax.comment};
    --syntax-function: ${t.syntax.function};
    --syntax-variable: ${t.syntax.variable};
    --syntax-type: ${t.syntax.type};
    --syntax-operator: ${t.syntax.operator};
    --syntax-punctuation: ${t.syntax.punctuation};

    --scrollbar-track: ${t.scrollbar.track};
    --scrollbar-thumb: ${t.scrollbar.thumb};
    --scrollbar-thumb-hover: ${t.scrollbar.thumbHover};
  `.trim();
}
