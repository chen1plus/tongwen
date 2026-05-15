import { Adapter } from './lib.ts';

export function stripTranscriptTags(s: string): string {
  return s.replace(/<\/?TRANSCRIPT>/g, '').trim();
}

export const voiceinkAdapter: Adapter = {
  suffix: 'voiceink',
  preprocess: stripTranscriptTags,
};
