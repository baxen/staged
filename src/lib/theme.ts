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
    addedBg: string; // Background for added lines
    addedText: string; // Text/line numbers for added
    removedBg: string; // Background for removed lines
    removedText: string; // Text/line numbers for removed
    contextBg: string; // Background for context lines
    emptyBg: string; // Background for empty/padding lines
    lineNumber: string; // Line number text
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
    addedBg: '#2ea04326',
    addedText: '#7ee787',
    removedBg: '#f8514926',
    removedText: '#f85149',
    contextBg: '#1e1e1e',
    emptyBg: '#2d2d2d',
    lineNumber: '#6e7681',
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

    --diff-added-bg: ${t.diff.addedBg};
    --diff-added-text: ${t.diff.addedText};
    --diff-removed-bg: ${t.diff.removedBg};
    --diff-removed-text: ${t.diff.removedText};
    --diff-context-bg: ${t.diff.contextBg};
    --diff-empty-bg: ${t.diff.emptyBg};
    --diff-line-number: ${t.diff.lineNumber};
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
