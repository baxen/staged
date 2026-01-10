// AI description service

import { invoke } from '@tauri-apps/api/core';

export interface HunkDescription {
  before: string;
  after: string;
}

/**
 * Describe a code change
 *
 * @param filePath - Path to the file being changed
 * @param beforeLines - Lines before the change
 * @param afterLines - Lines after the change
 * @returns Before/after descriptions of the change
 */
export async function describeHunk(
  filePath: string,
  beforeLines: string[],
  afterLines: string[]
): Promise<HunkDescription> {
  return invoke<HunkDescription>('describe_hunk', {
    filePath,
    beforeLines,
    afterLines,
  });
}
