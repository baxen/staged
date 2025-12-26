/**
 * Color Theme Infrastructure for Staged
 *
 * All colors in the app should reference this theme.
 * This makes it easy to tune the look of the app by adjusting values here.
 *
 * Usage in Svelte components:
 *   import { theme } from './theme';
 *   <div style="color: {theme.text.primary}">
 *
 * Usage in CSS (via CSS custom properties set in app.css):
 *   color: var(--text-primary);
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
    // Only removed lines (left pane) and added lines (right pane) get overlays
    addedOverlay: string; // Tint for added lines (right pane)
    removedOverlay: string; // Tint for removed lines (left pane)

    // Gutter (line number column) backgrounds
    addedGutter: string; // Gutter bg for added lines
    removedGutter: string; // Gutter bg for removed lines

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

  // Syntax highlighting (for future use)
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

/**
 * Default dark theme - inspired by VS Code Dark+
 */
export const darkTheme: Theme = {
  bg: {
    primary: '#1e1e1e',
    secondary: '#252526',
    tertiary: '#2d2d2d',
    input: '#3c3c3c',
  },

  border: {
    primary: '#3c3c3c',
    subtle: '#2d2d2d',
  },

  text: {
    primary: '#d4d4d4',
    secondary: '#cccccc',
    muted: '#888888',
    link: '#4fc1ff',
  },

  status: {
    modified: '#e2c08d',
    added: '#89d185',
    deleted: '#f14c4c',
    renamed: '#4fc1ff',
    untracked: '#888888',
  },

  diff: {
    // Overlay tints - transparent colors layered on syntax theme background
    // Both use gray - subtle tint to indicate changed regions
    addedOverlay: 'rgba(110, 118, 129, 0.10)',
    removedOverlay: 'rgba(110, 118, 129, 0.10)',

    // Gutter backgrounds - green for added, red for removed
    addedGutter: 'rgba(46, 160, 67, 0.25)',
    removedGutter: 'rgba(248, 81, 73, 0.20)',

    // Text colors
    addedText: '#7ee787',
    removedText: '#f85149',
    lineNumber: '#6e7681',

    // Other
    emptyBg: '#2d2d2d',
    headerBg: '#2d2d2d',
  },

  ui: {
    accent: '#0e639c',
    accentHover: '#1177bb',
    danger: '#5a1d1d',
    dangerHover: '#742a2a',
    success: '#2ea043',
    selection: '#094771',
  },

  syntax: {
    keyword: '#569cd6',
    string: '#ce9178',
    number: '#b5cea8',
    comment: '#6a9955',
    function: '#dcdcaa',
    variable: '#9cdcfe',
    type: '#4ec9b0',
    operator: '#d4d4d4',
    punctuation: '#d4d4d4',
  },

  scrollbar: {
    track: '#1e1e1e',
    thumb: '#424242',
    thumbHover: '#4f4f4f',
  },
};

/**
 * The active theme - change this to switch themes
 */
export const theme: Theme = darkTheme;

/**
 * Generate CSS custom properties from theme
 * This can be injected into :root for CSS-based theming
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
    --diff-added-gutter: ${t.diff.addedGutter};
    --diff-removed-gutter: ${t.diff.removedGutter};
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
