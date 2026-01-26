/**
 * Markdown renderer for chat messages using marked.
 * Supports full GitHub Flavored Markdown: tables, task lists, strikethrough, etc.
 * Code blocks are syntax highlighted using Shiki (same as diff viewer).
 */

import { marked, type Tokens } from 'marked';
import {
  highlightLines,
  detectLanguage,
  prepareLanguage,
  getTheme,
  type Token,
} from './highlighter';
import type { BundledLanguage } from 'shiki';

// Configure marked for chat context
marked.setOptions({
  gfm: true, // GitHub Flavored Markdown (tables, strikethrough, etc.)
  breaks: true, // Convert \n to <br> (more natural for chat)
});

// Map markdown code fence language identifiers to Shiki BundledLanguage
// These are common aliases people use in markdown that don't match file extensions
const LANG_ALIASES: Record<string, BundledLanguage> = {
  // Full names
  python: 'python',
  javascript: 'javascript',
  typescript: 'typescript',
  rust: 'rust',
  golang: 'go',
  ruby: 'ruby',
  shell: 'bash',
  bash: 'bash',
  zsh: 'bash',
  fish: 'bash',
  powershell: 'powershell',
  csharp: 'csharp',
  fsharp: 'fsharp',
  cpp: 'cpp',
  objectivec: 'objective-c',
  'objective-c': 'objective-c',
  kotlin: 'kotlin',
  scala: 'scala',
  swift: 'swift',
  dart: 'dart',
  elixir: 'elixir',
  erlang: 'erlang',
  haskell: 'haskell',
  clojure: 'clojure',
  ocaml: 'ocaml',
  lua: 'lua',
  perl: 'perl',
  php: 'php',
  java: 'java',
  sql: 'sql',
  graphql: 'graphql',
  dockerfile: 'dockerfile',
  docker: 'dockerfile',
  yaml: 'yaml',
  toml: 'toml',
  json: 'json',
  jsonc: 'json',
  xml: 'xml',
  html: 'html',
  css: 'css',
  scss: 'scss',
  sass: 'sass',
  less: 'less',
  markdown: 'markdown',
  diff: 'diff',
  makefile: 'make',
  make: 'make',
  cmake: 'cmake',
  nginx: 'nginx',
  terraform: 'terraform',
  hcl: 'terraform',
  prisma: 'prisma',
  solidity: 'solidity',
  latex: 'latex',
  tex: 'latex',
  nix: 'nix',
  zig: 'zig',
  nim: 'nim',
  julia: 'julia',
  r: 'r',

  // React/JSX/TSX - these use typescript/javascript highlighting
  react: 'typescript',
  jsx: 'javascript',
  tsx: 'typescript',

  // Svelte/Vue/Astro
  svelte: 'svelte',
  vue: 'vue',
  astro: 'astro',

  // Common short aliases
  py: 'python',
  js: 'javascript',
  ts: 'typescript',
  rs: 'rust',
  go: 'go',
  rb: 'ruby',
  sh: 'bash',
  ps1: 'powershell',
  cs: 'csharp',
  fs: 'fsharp',
  kt: 'kotlin',
  ex: 'elixir',
  erl: 'erlang',
  hs: 'haskell',
  clj: 'clojure',
  ml: 'ocaml',
  pl: 'perl',
  yml: 'yaml',
  md: 'markdown',
  tf: 'terraform',
  sol: 'solidity',
  jl: 'julia',
};

/**
 * Escape HTML entities to prevent XSS.
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Convert tokens to HTML spans.
 */
function tokensToHtml(tokens: Token[]): string {
  return tokens
    .map((t) => `<span style="color:${t.color}">${escapeHtml(t.content)}</span>`)
    .join('');
}

/**
 * Resolve a markdown language identifier to a Shiki BundledLanguage.
 * Tries the alias map first, then falls back to file extension detection.
 */
function resolveLanguage(lang: string): BundledLanguage | null {
  const normalized = lang.toLowerCase();

  // Check alias map first
  if (normalized in LANG_ALIASES) {
    return LANG_ALIASES[normalized];
  }

  // Fall back to file extension detection
  return detectLanguage(`file.${normalized}`);
}

/**
 * Highlight code using Shiki and return HTML.
 * Falls back to plain escaped code if highlighting fails.
 */
function highlightCode(code: string, lang: string | undefined): string {
  const theme = getTheme();
  const bgColor = theme?.bg || '#1e1e1e';

  // Resolve the language
  const detectedLang = lang ? resolveLanguage(lang) : null;

  // Highlight the code
  const lines = highlightLines(code, detectedLang);
  const highlightedLines = lines.map((lineTokens) => tokensToHtml(lineTokens));

  return `<pre style="background:${bgColor};padding:8px;border-radius:4px;overflow-x:auto;margin:0.5em 0"><code>${highlightedLines.join('\n')}</code></pre>`;
}

// Custom renderer for code blocks
const renderer = new marked.Renderer();

renderer.code = function ({ text, lang }: Tokens.Code): string {
  // Resolve and prepare the language (async load if needed)
  const resolvedLang = lang ? resolveLanguage(lang) : null;
  if (resolvedLang) {
    prepareLanguage(`file.${resolvedLang}`);
  }
  return highlightCode(text, lang);
};

/**
 * Render markdown text to HTML.
 */
export function renderMarkdown(text: string): string {
  return marked.parse(text, { async: false, renderer }) as string;
}
