/**
 * Font Preferences Store
 *
 * Manages font selection for UI and code (monospace) fonts.
 * Discovers system fonts via Tauri backend and persists selection to localStorage.
 *
 * Rebuildable: This module owns all font state. Components import reactive state directly.
 */

import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Constants
// =============================================================================

const STORAGE_KEY = 'staged-fonts';
const DEFAULT_MONO = 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace';
const SYSTEM_FONT_STACK =
  '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif';

// Keywords to identify monospace fonts
const MONO_KEYWORDS = [
  'mono',
  'code',
  'console',
  'courier',
  'menlo',
  'consolas',
  'terminal',
  'source code',
  'fira code',
  'jetbrains',
  'hack',
  'iosevka',
  'cascadia',
  'inconsolata',
  'droid sans mono',
  'ubuntu mono',
  'dejavu sans mono',
  'liberation mono',
  'anonymous pro',
  'input',
];

// =============================================================================
// Types
// =============================================================================

export interface FontSelection {
  ui: string; // Empty string = system default
  mono: string; // Empty string = system default
}

export interface CategorizedFonts {
  ui: string[];
  mono: string[];
}

// =============================================================================
// Reactive State
// =============================================================================

/**
 * All available system fonts (loaded from backend).
 */
export const systemFonts = $state<{ fonts: string[] }>({ fonts: [] });

/**
 * Current font selection.
 */
export const fontSelection = $state<FontSelection>({
  ui: '',
  mono: '',
});

/**
 * Fonts categorized into UI and monospace.
 * Derived from systemFonts.
 */
export function getCategorizedFonts(): CategorizedFonts {
  const mono: string[] = [];
  const ui: string[] = [];

  for (const font of systemFonts.fonts) {
    const lower = font.toLowerCase();
    if (MONO_KEYWORDS.some((kw) => lower.includes(kw))) {
      mono.push(font);
    } else {
      ui.push(font);
    }
  }

  return { ui, mono };
}

// =============================================================================
// CSS Application (internal)
// =============================================================================

function applyFonts(): void {
  const root = document.documentElement;

  if (fontSelection.ui) {
    root.style.setProperty('--font-family', `"${fontSelection.ui}", ${SYSTEM_FONT_STACK}`);
  } else {
    root.style.setProperty('--font-family', SYSTEM_FONT_STACK);
  }

  if (fontSelection.mono) {
    root.style.setProperty('--font-mono', `"${fontSelection.mono}", ${DEFAULT_MONO}`);
  } else {
    root.style.setProperty('--font-mono', DEFAULT_MONO);
  }
}

function saveToStorage(): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(fontSelection));
}

function loadFromStorage(): void {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      fontSelection.ui = parsed.ui || '';
      fontSelection.mono = parsed.mono || '';
    }
  } catch {
    // Ignore parse errors, use defaults
  }
}

// =============================================================================
// Actions
// =============================================================================

/**
 * Load system fonts from the backend.
 */
export async function loadSystemFonts(): Promise<void> {
  try {
    const fonts = await invoke<string[]>('list_system_fonts');
    systemFonts.fonts = fonts;
  } catch (err) {
    console.error('Failed to load system fonts:', err);
    systemFonts.fonts = [];
  }
}

/**
 * Set the UI font family.
 */
export function setUiFont(family: string): void {
  fontSelection.ui = family.trim();
  applyFonts();
  saveToStorage();
}

/**
 * Set the monospace (code) font family.
 */
export function setMonoFont(family: string): void {
  fontSelection.mono = family.trim();
  applyFonts();
  saveToStorage();
}

/**
 * Initialize fonts on app load.
 * Loads saved preferences and applies them, then loads system fonts.
 */
export async function initFonts(): Promise<void> {
  loadFromStorage();
  applyFonts();
  await loadSystemFonts();
}
