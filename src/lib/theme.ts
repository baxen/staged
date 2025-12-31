/**
 * Adaptive Theme Infrastructure for Staged
 *
 * The UI chrome colors are derived from the syntax highlighting theme.
 * This ensures a unified look where the sidebar and controls blend
 * seamlessly with the code area.
 *
 * Key insight: we detect light vs dark themes and adjust our derivation
 * strategy accordingly. Light themes need darkening, dark themes need lightening.
 *
 * Usage in CSS (via CSS custom properties):
 *   color: var(--text-primary);
 *   background: var(--bg-primary);
 */

export interface Theme {
  // Is this a light or dark theme?
  isDark: boolean;

  // Base colors
  bg: {
    primary: string; // Main background (same as syntax theme)
    elevated: string; // Floating elements (header, tooltips)
    sunken: string; // Recessed areas (sidebar)
    hover: string; // Hover states
  };

  // Borders and dividers
  border: {
    subtle: string; // Very subtle dividers
    muted: string; // Standard borders
    emphasis: string; // Emphasized borders (focus, range markers)
  };

  // Text colors
  text: {
    primary: string; // Main text
    muted: string; // Subdued text (from comment color)
    faint: string; // Very subdued (placeholders)
    accent: string; // Links and interactive text
  };

  // Git status colors (adapted for light/dark)
  status: {
    modified: string;
    added: string;
    deleted: string;
    renamed: string;
    untracked: string;
  };

  // Diff viewer colors
  diff: {
    addedBg: string; // Background tint for added lines
    removedBg: string; // Background tint for removed lines
    changedBg: string; // Neutral background for changed lines (not add/remove specific)
    rangeBorder: string; // Border color for change range markers
  };

  // Interactive elements
  ui: {
    accent: string; // Primary accent
    accentHover: string; // Accent hover
    danger: string; // Destructive actions
    selection: string; // Selected items background
  };

  // Scrollbar
  scrollbar: {
    thumb: string;
    thumbHover: string;
  };
}

// =============================================================================
// Color Utilities
// =============================================================================

interface RGB {
  r: number;
  g: number;
  b: number;
}

function hexToRgb(hex: string): RGB {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!result) return { r: 128, g: 128, b: 128 };
  return {
    r: parseInt(result[1], 16),
    g: parseInt(result[2], 16),
    b: parseInt(result[3], 16),
  };
}

function rgbToHex({ r, g, b }: RGB): string {
  const clamp = (n: number) => Math.max(0, Math.min(255, Math.round(n)));
  return `#${[r, g, b].map((c) => clamp(c).toString(16).padStart(2, '0')).join('')}`;
}

/**
 * Calculate relative luminance (0-1) for a color.
 * Used to determine if a theme is light or dark.
 */
function luminance(hex: string): number {
  const { r, g, b } = hexToRgb(hex);
  const [rs, gs, bs] = [r, g, b].map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

/**
 * Mix two colors by a factor (0 = color1, 1 = color2)
 */
function mix(hex1: string, hex2: string, factor: number): string {
  const c1 = hexToRgb(hex1);
  const c2 = hexToRgb(hex2);
  return rgbToHex({
    r: c1.r + (c2.r - c1.r) * factor,
    g: c1.g + (c2.g - c1.g) * factor,
    b: c1.b + (c2.b - c1.b) * factor,
  });
}

/**
 * Adjust a color toward white (positive) or black (negative)
 */
function adjust(hex: string, amount: number): string {
  const target = amount > 0 ? '#ffffff' : '#000000';
  return mix(hex, target, Math.abs(amount));
}

/**
 * Create a semi-transparent overlay color
 */
function overlay(hex: string, alpha: number): string {
  const { r, g, b } = hexToRgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

// =============================================================================
// Adaptive Theme Generator
// =============================================================================

/**
 * Create an adaptive theme based on syntax theme colors.
 *
 * The key insight is detecting light vs dark and adjusting accordingly:
 * - Dark themes: lighten to create elevation/emphasis
 * - Light themes: darken to create elevation/emphasis
 */
export function createAdaptiveTheme(
  syntaxBg: string,
  syntaxFg: string,
  syntaxComment: string
): Theme {
  const isDark = luminance(syntaxBg) < 0.5;

  // Direction multiplier: +1 for dark (lighten), -1 for light (darken)
  const dir = isDark ? 1 : -1;

  // Base adjustments - smaller values for subtlety
  const elevate = (amount: number) => adjust(syntaxBg, dir * amount);
  const sink = (amount: number) => adjust(syntaxBg, -dir * amount);

  // Accent colors that work on both light and dark
  const accentBlue = isDark ? '#58a6ff' : '#0969da';
  const accentGreen = isDark ? '#3fb950' : '#1a7f37';
  const accentRed = isDark ? '#f85149' : '#cf222e';
  const accentOrange = isDark ? '#d29922' : '#9a6700';

  // Border that's visible but not harsh
  const borderBase = mix(syntaxBg, syntaxFg, isDark ? 0.15 : 0.12);

  return {
    isDark,

    bg: {
      primary: syntaxBg,
      elevated: elevate(0.04), // Subtle lift for floating elements
      sunken: sink(0.02), // Subtle recession for sidebar
      hover: elevate(0.06), // Hover state
    },

    border: {
      subtle: mix(syntaxBg, syntaxFg, 0.08),
      muted: borderBase,
      emphasis: mix(syntaxBg, syntaxFg, isDark ? 0.25 : 0.2),
    },

    text: {
      primary: syntaxFg,
      muted: syntaxComment,
      faint: mix(syntaxBg, syntaxComment, 0.5),
      accent: accentBlue,
    },

    status: {
      modified: accentOrange,
      added: accentGreen,
      deleted: accentRed,
      renamed: accentBlue,
      untracked: syntaxComment,
    },

    diff: {
      // Very subtle tints - the syntax highlighting should dominate
      addedBg: overlay(accentGreen, isDark ? 0.08 : 0.1),
      removedBg: overlay(accentRed, isDark ? 0.08 : 0.1),
      // Neutral highlight for changed lines - subtle foreground tint
      changedBg: overlay(syntaxFg, isDark ? 0.04 : 0.06),
      // Range borders need to be visible but not distracting
      rangeBorder: mix(syntaxBg, syntaxFg, isDark ? 0.2 : 0.15),
    },

    ui: {
      accent: accentGreen,
      accentHover: isDark ? '#2ea043' : '#2da44e',
      danger: accentRed,
      selection: overlay(accentBlue, 0.2),
    },

    scrollbar: {
      thumb: borderBase,
      thumbHover: mix(syntaxBg, syntaxFg, 0.25),
    },
  };
}

/**
 * Generate CSS custom properties from theme.
 */
export function themeToCssVars(t: Theme): string {
  return `
    --theme-is-dark: ${t.isDark ? '1' : '0'};

    --bg-primary: ${t.bg.primary};
    --bg-elevated: ${t.bg.elevated};
    --bg-sunken: ${t.bg.sunken};
    --bg-hover: ${t.bg.hover};

    --border-subtle: ${t.border.subtle};
    --border-muted: ${t.border.muted};
    --border-emphasis: ${t.border.emphasis};

    --text-primary: ${t.text.primary};
    --text-muted: ${t.text.muted};
    --text-faint: ${t.text.faint};
    --text-accent: ${t.text.accent};

    --status-modified: ${t.status.modified};
    --status-added: ${t.status.added};
    --status-deleted: ${t.status.deleted};
    --status-renamed: ${t.status.renamed};
    --status-untracked: ${t.status.untracked};

    --diff-added-bg: ${t.diff.addedBg};
    --diff-removed-bg: ${t.diff.removedBg};
    --diff-changed-bg: ${t.diff.changedBg};
    --diff-range-border: ${t.diff.rangeBorder};

    --ui-accent: ${t.ui.accent};
    --ui-accent-hover: ${t.ui.accentHover};
    --ui-danger: ${t.ui.danger};
    --ui-selection: ${t.ui.selection};

    --scrollbar-thumb: ${t.scrollbar.thumb};
    --scrollbar-thumb-hover: ${t.scrollbar.thumbHover};
  `.trim();
}
