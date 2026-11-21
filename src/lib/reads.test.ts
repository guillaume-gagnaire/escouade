import { describe, expect, it } from 'vitest';
import { newestReads } from './reads';

describe('newestReads', () => {
  it('shows an answer while a newer read is on its way, never an older one after a newer (ESC-522)', () => {
    const reads = newestReads();
    const [a, b, c] = [reads.ask(), reads.ask(), reads.ask()];
    // The first answer comes while two newer reads are asked: shown, not dropped.
    expect(reads.take(a)).toBe(true);
    // The third, then the second, older, which comes too late.
    expect(reads.take(c)).toBe(true);
    expect(reads.take(b)).toBe(false);
  });

  it('shows each answer once', () => {
    const reads = newestReads();
    const a = reads.ask();
    expect(reads.take(a)).toBe(true);
    expect(reads.take(a)).toBe(false);
  });
});
