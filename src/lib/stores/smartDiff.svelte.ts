// Smart diff store - manages AI descriptions state with caching

import { describeHunk, type HunkDescription } from '../services/ai';
import type { FileDiff } from '../types';

interface CachedDescription {
  description: HunkDescription;
  contentHash: string;
}

function hashContent(beforeLines: string[], afterLines: string[]): string {
  return beforeLines.join('\n') + '|||' + afterLines.join('\n');
}

let enabled = $state(false);
let loading = $state(false);
let loadingPath = $state<string | null>(null);
let cache = $state<Map<string, CachedDescription>>(new Map());

export const smartDiffState = {
  get enabled() {
    return enabled;
  },
  get loading() {
    return loading;
  },
  get loadingPath() {
    return loadingPath;
  },

  toggle() {
    enabled = !enabled;
  },

  getDescription(path: string): HunkDescription | undefined {
    return cache.get(path)?.description;
  },

  hasDescription(path: string): boolean {
    return cache.has(path);
  },

  async calculateForFile(file: FileDiff): Promise<HunkDescription | null> {
    const filePath = file.after?.path ?? file.before?.path;
    if (!filePath) return null;

    const beforeLines = file.before?.content.type === 'Text' ? file.before.content.lines : [];
    const afterLines = file.after?.content.type === 'Text' ? file.after.content.lines : [];

    const changedAlignments = file.alignments.filter((a) => a.changed);
    let beforeContent: string[] = [];
    let afterContent: string[] = [];

    for (const alignment of changedAlignments) {
      beforeContent.push(...beforeLines.slice(alignment.before.start, alignment.before.end));
      afterContent.push(...afterLines.slice(alignment.after.start, alignment.after.end));
    }

    const contentHash = hashContent(beforeContent, afterContent);
    const cached = cache.get(filePath);
    if (cached && cached.contentHash === contentHash) {
      return cached.description;
    }

    loading = true;
    loadingPath = filePath;

    try {
      console.log('=== SMART DIFF: Calculating for', filePath, '===');
      const result = await describeHunk(filePath, beforeContent, afterContent);
      console.log('Before:', result.before);
      console.log('After:', result.after);

      cache = new Map(cache).set(filePath, { description: result, contentHash });

      return result;
    } catch (error) {
      console.error('Smart diff failed for', filePath, error);
      return null;
    } finally {
      loading = false;
      loadingPath = null;
    }
  },

  clearCache() {
    cache = new Map();
  },
};
