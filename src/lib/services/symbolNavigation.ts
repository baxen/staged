/**
 * Symbol Navigation Service
 *
 * Provides "Go to Definition" functionality by leveraging the backend
 * Tree-sitter parsing to find symbol definitions.
 */

import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Types
// =============================================================================

/** The kind of symbol (function, class, variable, etc.) */
export type SymbolKind =
  | 'function'
  | 'method'
  | 'class'
  | 'interface'
  | 'type'
  | 'enum'
  | 'variable'
  | 'constant'
  | 'module'
  | 'import'
  | 'property'
  | 'unknown';

/** Information about a symbol at a position */
export interface SymbolInfo {
  /** The symbol name */
  name: string;
  /** The kind of symbol */
  kind: SymbolKind;
  /** Line number (0-indexed) */
  line: number;
  /** Column number (0-indexed) */
  column: number;
  /** End column number (0-indexed) */
  endColumn: number;
  /** Preview of the context line */
  context: string | null;
  /** Language of the file */
  language: string;
}

/** Result of a definition search */
export interface DefinitionResult {
  /** Path to the file containing the definition */
  filePath: string;
  /** Line number (0-indexed) */
  line: number;
  /** Column number (0-indexed) */
  column: number;
  /** End column number (0-indexed) */
  endColumn: number;
  /** The symbol name */
  name: string;
  /** The kind of symbol */
  kind: SymbolKind;
  /** Preview of the definition line */
  preview: string;
}

// =============================================================================
// API
// =============================================================================

/**
 * Get symbol information at a specific position in a file.
 *
 * @param refName - Git ref (commit SHA, branch name, or "WORKDIR")
 * @param filePath - Path to the file relative to repo root
 * @param line - Line number (0-indexed)
 * @param column - Column number (0-indexed)
 * @param repoPath - Optional path to the repository
 * @returns Symbol info if found, null otherwise
 */
export async function getSymbolAtPosition(
  refName: string,
  filePath: string,
  line: number,
  column: number,
  repoPath?: string
): Promise<SymbolInfo | null> {
  return invoke<SymbolInfo | null>('get_symbol_at_position', {
    repoPath: repoPath ?? null,
    refName,
    filePath,
    line,
    column,
  });
}

/**
 * Find the definition of a symbol.
 *
 * @param refName - Git ref (commit SHA, branch name, or "WORKDIR")
 * @param filePath - Path to the file where the symbol was found
 * @param symbolName - Name of the symbol to find
 * @param symbolLine - Line where the symbol was found (0-indexed)
 * @param symbolColumn - Column where the symbol was found (0-indexed)
 * @param repoPath - Optional path to the repository
 * @returns Definition result if found, null otherwise
 */
export async function findDefinition(
  refName: string,
  filePath: string,
  symbolName: string,
  symbolLine: number,
  symbolColumn: number,
  repoPath?: string
): Promise<DefinitionResult | null> {
  return invoke<DefinitionResult | null>('find_definition', {
    repoPath: repoPath ?? null,
    refName,
    filePath,
    symbolName,
    symbolLine,
    symbolColumn,
  });
}

/**
 * Check if a file type supports symbol navigation.
 *
 * @param filePath - Path to the file
 * @returns True if the file type supports symbol navigation
 */
export async function supportsSymbolNavigation(filePath: string): Promise<boolean> {
  return invoke<boolean>('supports_symbol_navigation', { filePath });
}

// =============================================================================
// Helpers
// =============================================================================

/**
 * Calculate the position (line, column) from a click event on a code line.
 *
 * @param event - The mouse event
 * @param lineElement - The DOM element containing the line
 * @param lineNumber - The line number (0-indexed)
 * @returns The calculated position
 */
export function calculatePositionFromClick(
  event: MouseEvent,
  lineElement: HTMLElement,
  lineNumber: number
): { line: number; column: number } {
  // Get the character position from the click
  const rect = lineElement.getBoundingClientRect();
  const x = event.clientX - rect.left;

  // Use the font metrics to estimate column
  // Get computed style to find the font
  const style = window.getComputedStyle(lineElement);
  const fontSize = parseFloat(style.fontSize);

  // Monospace font: estimate character width
  // Create a temporary element to measure character width
  const measureSpan = document.createElement('span');
  measureSpan.style.font = style.font;
  measureSpan.style.visibility = 'hidden';
  measureSpan.style.position = 'absolute';
  measureSpan.textContent = 'M'; // Use 'M' as reference character
  document.body.appendChild(measureSpan);
  const charWidth = measureSpan.getBoundingClientRect().width;
  document.body.removeChild(measureSpan);

  // Calculate column from x position
  const column = Math.floor(x / charWidth);

  return { line: lineNumber, column: Math.max(0, column) };
}

/**
 * Find the token element at a specific position within a line element.
 *
 * @param lineElement - The DOM element containing the line
 * @param event - The mouse event
 * @returns The token element if found, null otherwise
 */
export function findTokenAtPosition(
  lineElement: HTMLElement,
  event: MouseEvent
): HTMLElement | null {
  // Find all span elements (tokens) in the line
  const tokens = lineElement.querySelectorAll('span[data-token]');

  for (const token of tokens) {
    const rect = token.getBoundingClientRect();
    if (
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom
    ) {
      return token as HTMLElement;
    }
  }

  return null;
}

/**
 * Get the text content and position of a token element.
 *
 * @param tokenElement - The token DOM element
 * @param lineNumber - The line number (0-indexed)
 * @returns Token info if the element is valid
 */
export function getTokenInfo(
  tokenElement: HTMLElement,
  lineNumber: number
): { text: string; line: number; column: number; endColumn: number } | null {
  const text = tokenElement.textContent;
  if (!text) return null;

  // Get column from data attribute or calculate from position
  const column = parseInt(tokenElement.dataset.column ?? '0', 10);
  const endColumn = column + text.length;

  return { text, line: lineNumber, column, endColumn };
}
