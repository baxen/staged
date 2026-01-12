/**
 * Reference Files Store
 *
 * Manages files that are pinned for viewing outside the current diff.
 * These are files from the repository that weren't changed in the diff
 * but the user wants to view/comment on during the review.
 *
 * Reference files are session-scoped - they're cleared when the diff spec changes.
 */

import type { File, FileContent } from '../types';
import { getFileAtRef } from '../services/files';

// =============================================================================
// State
// =============================================================================

export interface ReferenceFile {
  /** File path in the repository */
  path: string;
  /** File content (text lines or binary marker) */
  content: FileContent;
}

interface ReferenceFilesState {
  /** Pinned reference files */
  files: ReferenceFile[];
  /** Loading state for file fetching */
  loading: boolean;
  /** Error message if loading failed */
  error: string | null;
}

export const referenceFilesState: ReferenceFilesState = $state({
  files: [],
  loading: false,
  error: null,
});

// =============================================================================
// Getters
// =============================================================================

/**
 * Check if a path is a reference file (not a diff file).
 */
export function isReferenceFile(path: string): boolean {
  return referenceFilesState.files.some((f) => f.path === path);
}

/**
 * Get a reference file by path.
 */
export function getReferenceFile(path: string): ReferenceFile | undefined {
  return referenceFilesState.files.find((f) => f.path === path);
}

/**
 * Get all reference file paths.
 */
export function getReferenceFilePaths(): string[] {
  return referenceFilesState.files.map((f) => f.path);
}

// =============================================================================
// Actions
// =============================================================================

/**
 * Add a reference file by loading it from the repository.
 *
 * @param refName - The git ref to load the file from (e.g., HEAD, branch name, SHA)
 * @param path - The file path in the repository
 * @param repoPath - Optional repository path
 */
export async function addReferenceFile(
  refName: string,
  path: string,
  repoPath?: string
): Promise<void> {
  // Don't add duplicates
  if (isReferenceFile(path)) {
    return;
  }

  referenceFilesState.loading = true;
  referenceFilesState.error = null;

  try {
    const file = await getFileAtRef(refName, path, repoPath);
    referenceFilesState.files = [
      ...referenceFilesState.files,
      { path: file.path, content: file.content },
    ];
  } catch (e) {
    referenceFilesState.error = e instanceof Error ? e.message : String(e);
    throw e; // Re-throw so caller can handle
  } finally {
    referenceFilesState.loading = false;
  }
}

/**
 * Remove a reference file.
 */
export function removeReferenceFile(path: string): void {
  referenceFilesState.files = referenceFilesState.files.filter((f) => f.path !== path);
}

/**
 * Clear all reference files.
 * Call this when the diff spec changes.
 */
export function clearReferenceFiles(): void {
  referenceFilesState.files = [];
  referenceFilesState.error = null;
}

/**
 * Reset error state.
 */
export function clearReferenceFilesError(): void {
  referenceFilesState.error = null;
}
