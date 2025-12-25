/**
 * Syntax Highlighting Service
 *
 * Self-contained module that wraps Shiki for syntax highlighting.
 * All Shiki-specific logic lives here - the rest of the app just sees
 * simple Token[] arrays.
 *
 * Languages are lazy-loaded on demand for fast startup.
 *
 * Rebuildable: To swap highlighting libraries, rewrite this file
 * with the same exports. No other files need to change.
 */

import { createHighlighter, type Highlighter, type ThemedToken, type BundledLanguage } from 'shiki';

// Simple token type that doesn't leak Shiki internals
export interface Token {
  content: string;
  color: string;
}

// Theme info exposed to the app
export interface HighlighterTheme {
  name: string;
  bg: string;
  fg: string;
}

// Singleton highlighter instance
let highlighter: Highlighter | null = null;
let currentTheme: HighlighterTheme | null = null;
let initPromise: Promise<void> | null = null;

// Track which languages we've attempted to load (to avoid repeated failures)
const loadedLanguages = new Set<string>();
const failedLanguages = new Set<string>();

// Core languages loaded at startup (most common, fast init)
const CORE_LANGUAGES: BundledLanguage[] = [
  'typescript',
  'javascript',
  'json',
  'markdown',
  'html',
  'css',
];

// All supported languages (lazy loaded on demand)
const SUPPORTED_LANGUAGES: BundledLanguage[] = [
  // Core (loaded at startup)
  'typescript',
  'javascript',
  'json',
  'markdown',
  'html',
  'css',

  // Systems
  'rust',
  'go',
  'c',
  'cpp',
  'zig',
  'nim',

  // JVM/.NET
  'java',
  'kotlin',
  'scala',
  'groovy',
  'csharp',
  'fsharp',

  // Mobile
  'dart',
  'swift',
  'objective-c',

  // Scripting
  'python',
  'ruby',
  'php',
  'perl',
  'lua',

  // Functional
  'haskell',
  'elixir',
  'erlang',
  'clojure',
  'ocaml',

  // Data science
  'r',
  'julia',

  // Web frameworks
  'svelte',
  'vue',
  'astro',
  'scss',
  'sass',
  'less',

  // Shell
  'bash',
  'shellscript',
  'powershell',

  // Data formats
  'yaml',
  'toml',
  'xml',

  // Build systems
  'make',
  'cmake',
  'nix',

  // DevOps/config
  'dockerfile',
  'nginx',
  'graphql',
  'terraform',
  'prisma',

  // Blockchain
  'solidity',

  // Other
  'sql',
  'diff',
  'wasm',
  'latex',
];

// Map file extensions to Shiki language IDs
const EXTENSION_MAP: Record<string, BundledLanguage> = {
  // TypeScript/JavaScript
  ts: 'typescript',
  tsx: 'typescript',
  mts: 'typescript',
  cts: 'typescript',
  js: 'javascript',
  jsx: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',

  // Python
  py: 'python',
  pyi: 'python',
  pyw: 'python',

  // Rust
  rs: 'rust',

  // Go
  go: 'go',

  // Zig
  zig: 'zig',

  // Data formats
  json: 'json',
  jsonc: 'json',
  json5: 'json',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
  xml: 'xml',
  svg: 'xml',
  plist: 'xml',

  // Web
  html: 'html',
  htm: 'html',
  xhtml: 'html',
  css: 'css',
  scss: 'scss',
  sass: 'sass',
  less: 'less',
  svelte: 'svelte',
  vue: 'vue',
  astro: 'astro',

  // Shell
  sh: 'bash',
  bash: 'bash',
  zsh: 'bash',
  fish: 'bash',
  ksh: 'bash',
  ps1: 'powershell',
  psm1: 'powershell',

  // Docs
  md: 'markdown',
  markdown: 'markdown',
  mdx: 'markdown',

  // Database
  sql: 'sql',
  mysql: 'sql',
  pgsql: 'sql',

  // Diff
  diff: 'diff',
  patch: 'diff',

  // C family
  c: 'c',
  h: 'c',
  cpp: 'cpp',
  cc: 'cpp',
  cxx: 'cpp',
  hpp: 'cpp',
  hxx: 'cpp',
  hh: 'cpp',

  // JVM
  java: 'java',
  kt: 'kotlin',
  kts: 'kotlin',
  scala: 'scala',
  sc: 'scala',
  groovy: 'groovy',
  gradle: 'groovy',
  clj: 'clojure',
  cljs: 'clojure',
  cljc: 'clojure',

  // .NET
  cs: 'csharp',
  fs: 'fsharp',
  fsx: 'fsharp',
  fsi: 'fsharp',

  // Apple/Mobile
  swift: 'swift',
  m: 'objective-c',
  mm: 'objective-c',
  dart: 'dart',

  // Ruby
  rb: 'ruby',
  rake: 'ruby',
  gemspec: 'ruby',

  // PHP
  php: 'php',

  // Perl
  pl: 'perl',
  pm: 'perl',

  // Lua
  lua: 'lua',

  // Functional
  hs: 'haskell',
  lhs: 'haskell',
  ex: 'elixir',
  exs: 'elixir',
  erl: 'erlang',
  hrl: 'erlang',
  ml: 'ocaml',
  mli: 'ocaml',

  // Data science
  r: 'r',
  R: 'r',
  jl: 'julia',

  // Systems (additional)
  nim: 'nim',

  // Build systems
  makefile: 'make',
  mk: 'make',
  cmake: 'cmake',
  nix: 'nix',

  // DevOps
  dockerfile: 'dockerfile',
  tf: 'terraform',
  hcl: 'terraform',
  prisma: 'prisma',
  graphql: 'graphql',
  gql: 'graphql',
  nginx: 'nginx',

  // Blockchain
  sol: 'solidity',

  // Other
  wasm: 'wasm',
  wat: 'wasm',
  tex: 'latex',
  ltx: 'latex',
};

/**
 * Initialize the highlighter with a theme.
 * Only loads core languages at startup for fast init.
 * Other languages are lazy-loaded on demand.
 *
 * This is idempotent - multiple calls return the same instance.
 */
export async function initHighlighter(themeName: string = 'github-dark'): Promise<void> {
  // Return existing instance if already initialized
  if (highlighter) {
    return;
  }

  // If initialization is in progress, wait for it
  if (initPromise) {
    return initPromise;
  }

  // Start initialization
  initPromise = (async () => {
    highlighter = await createHighlighter({
      themes: [themeName],
      langs: CORE_LANGUAGES,
    });

    // Mark core languages as loaded
    CORE_LANGUAGES.forEach((lang) => loadedLanguages.add(lang));

    // Extract theme colors
    const theme = highlighter.getTheme(themeName);
    currentTheme = {
      name: themeName,
      bg: theme.bg || '#1e1e1e',
      fg: theme.fg || '#d4d4d4',
    };
  })();

  return initPromise;
}

/**
 * Get the current theme info (background, foreground colors).
 * Returns null if highlighter not initialized.
 */
export function getTheme(): HighlighterTheme | null {
  return currentTheme;
}

/**
 * Detect language from file path/extension.
 * Returns null for unknown extensions.
 */
export function detectLanguage(filePath: string): BundledLanguage | null {
  // Handle special filenames
  const filename = filePath.split('/').pop()?.toLowerCase() || '';
  if (filename === 'dockerfile') return 'dockerfile';
  if (filename === 'makefile' || filename === 'gnumakefile') return 'make';
  if (filename === 'cmakelists.txt') return 'cmake';

  const ext = filePath.split('.').pop()?.toLowerCase() || '';
  return EXTENSION_MAP[ext] || null;
}

/**
 * Check if a language is in our supported set.
 */
function isSupportedLanguage(lang: string): lang is BundledLanguage {
  return SUPPORTED_LANGUAGES.includes(lang as BundledLanguage);
}

/**
 * Ensure a language is loaded, loading it lazily if needed.
 * Returns true if language is ready to use, false if unavailable.
 */
async function ensureLanguageLoaded(lang: BundledLanguage): Promise<boolean> {
  if (!highlighter) return false;

  // Already loaded
  if (loadedLanguages.has(lang)) return true;

  // Already failed to load
  if (failedLanguages.has(lang)) return false;

  // Not in our supported set
  if (!isSupportedLanguage(lang)) {
    failedLanguages.add(lang);
    return false;
  }

  // Try to load it
  try {
    await highlighter.loadLanguage(lang);
    loadedLanguages.add(lang);
    return true;
  } catch {
    failedLanguages.add(lang);
    return false;
  }
}

/**
 * Highlight a single line of code.
 * Returns tokens with content and color.
 *
 * If highlighter isn't ready or language unsupported, returns
 * a single token with the full content and default foreground color.
 */
export function highlightLine(code: string, lang: BundledLanguage | null): Token[] {
  const fallback = [{ content: code, color: currentTheme?.fg || '#d4d4d4' }];

  if (!highlighter || !currentTheme || !lang) {
    return fallback;
  }

  // If language isn't loaded yet, return fallback (will be loaded async)
  if (!loadedLanguages.has(lang)) {
    return fallback;
  }

  try {
    const result = highlighter.codeToTokens(code, {
      lang,
      theme: currentTheme.name,
    });

    const tokens = result.tokens[0] || [];
    return tokens.map((token: ThemedToken) => ({
      content: token.content,
      color: token.color || currentTheme!.fg,
    }));
  } catch {
    return fallback;
  }
}

/**
 * Prepare a language for highlighting (async).
 * Call this when a file is selected to ensure its language is loaded.
 * Returns true if language is ready.
 */
export async function prepareLanguage(filePath: string): Promise<boolean> {
  const lang = detectLanguage(filePath);
  if (!lang) return false;
  return ensureLanguageLoaded(lang);
}

/**
 * Highlight multiple lines at once (more efficient for full files).
 * Returns an array of token arrays, one per line.
 */
export function highlightLines(code: string, lang: BundledLanguage | null): Token[][] {
  const fallbackLine = (line: string) => [{ content: line, color: currentTheme?.fg || '#d4d4d4' }];

  if (!highlighter || !currentTheme || !lang || !loadedLanguages.has(lang)) {
    return code.split('\n').map(fallbackLine);
  }

  try {
    const result = highlighter.codeToTokens(code, {
      lang,
      theme: currentTheme.name,
    });

    return result.tokens.map((lineTokens: ThemedToken[]) =>
      lineTokens.map((token: ThemedToken) => ({
        content: token.content,
        color: token.color || currentTheme!.fg,
      }))
    );
  } catch {
    return code.split('\n').map(fallbackLine);
  }
}
