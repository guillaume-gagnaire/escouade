import { describe, expect, it } from 'vitest';
import { codeComplete, isOpen, pickPositions, typedCode, wordCount, wordsTyped } from './team-words';

describe('the helpers of the team screens', () => {
  it('picks three distinct places among the 24 words, in order', () => {
    for (let i = 0; i < 50; i++) {
      const p = pickPositions();
      expect(p).toHaveLength(3);
      expect(new Set(p).size).toBe(3);
      expect([...p].sort((a, b) => a - b)).toEqual(p);
      expect(p.every((x) => x >= 0 && x < 24)).toBe(true);
    }
    // The randomness given decides: the same draw gives the same places.
    let seed = 0.42;
    const rand = () => (seed = (seed * 9301 + 0.49297) % 1);
    const a = pickPositions(24, 3, rand);
    seed = 0.42;
    expect(pickPositions(24, 3, rand)).toEqual(a);
  });

  it('reads a short code as typed: case, O and I/L, spaces and dashes', () => {
    expect(typedCode('k7q4m2xz')).toBe('K7Q4-M2XZ');
    expect(typedCode(' k7q o-il zz ')).toBe('K7Q0-11ZZ');
    expect(typedCode('k7q')).toBe('K7Q');
    expect(typedCode('K7Q4-M2XZ-EXTRA')).toBe('K7Q4-M2XZ');
    expect(codeComplete('K7Q4-M2XZ')).toBe(true);
    expect(codeComplete('K7Q4-M2X')).toBe(false);
    // U is not of the alphabet.
    expect(codeComplete('K7Q4-M2XU')).toBe(false);
  });

  it('counts the words of a recovery key as typed', () => {
    expect(wordsTyped('  Abandon\n ability\tABLE ')).toBe('abandon ability able');
    expect(wordCount('')).toBe(0);
    expect(wordCount(Array(24).fill('zoo').join('  '))).toBe(24);
  });

  it('says whether an invitation is still open', () => {
    expect(isOpen(100, 99_000)).toBe(true);
    expect(isOpen(100, 100_000)).toBe(false);
  });
});
