// What the screens of the team server compute without the backend (TS-3): the three words of the recovery key to type
// back, a short code as it is typed, what is left of an invitation.

/** `k` distinct places among `count` (from 0), in order: the words to type back. */
export function pickPositions(count = 24, k = 3, random: () => number = Math.random): number[] {
  const all = Array.from({ length: count }, (_, i) => i);
  for (let i = all.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [all[i], all[j]] = [all[j], all[i]];
  }
  return all.slice(0, Math.min(k, count)).sort((a, b) => a - b);
}

/** Crockford's base 32: what a short code is made of (no I, L, O, U). */
const CODE = /^[0-9A-HJKMNP-TV-Z]{10}$/;

/**
 * A short code as typed, shown in two halves of five (`K7Q4M-2XZ8P`): upper case, `O` read as `0`, `I` and `L` as `1`,
 * spaces and dashes aside, 10 characters at most.
 */
export function typedCode(text: string): string {
  const chars = text
    .toUpperCase()
    .replace(/[\s-]/g, '')
    .replace(/O/g, '0')
    .replace(/[IL]/g, '1')
    .replace(/[^0-9A-Z]/g, '')
    .slice(0, 10);
  return chars.length > 5 ? `${chars.slice(0, 5)}-${chars.slice(5)}` : chars;
}

/** Whether a code as typed is a whole one. */
export function codeComplete(text: string): boolean {
  return CODE.test(typedCode(text).replace('-', ''));
}

/** The six words of the team's fingerprint as typed back from the rescue sheet: the same as the 24 words. */
export const SIX = 6;

/** The 24 words as typed back for a recovery: spaces of any kind, any case, one space between each. */
export function wordsTyped(text: string): string {
  return text.trim().toLowerCase().split(/\s+/).filter(Boolean).join(' ');
}

export function wordCount(text: string): number {
  const typed = wordsTyped(text);
  return typed ? typed.split(' ').length : 0;
}

/** An invitation still open at `nowMs` (its end in seconds). */
export function isOpen(expiresAt: number, nowMs = Date.now()): boolean {
  return expiresAt * 1000 > nowMs;
}
