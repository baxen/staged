// AI description service - calls goose to describe code changes

import { invoke } from '@tauri-apps/api/core';

/**
 * Describe a code change using goose AI.
 *
 * @param filePath - Path to the file being changed
 * @param beforeLines - Lines before the change
 * @param afterLines - Lines after the change
 * @returns A description of what changed
 */
export async function describeHunk(
  filePath: string,
  beforeLines: string[],
  afterLines: string[]
): Promise<string> {
  return invoke<string>('describe_hunk', {
    filePath,
    beforeLines,
    afterLines,
  });
}
