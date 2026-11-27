import { describe, expect, it } from 'vitest';
import { THEME_KEY } from './appearance';
import { SEEN_KEY, skipOtherLanguage, skipPastNews } from './whats-up';

const store = (kept: Record<string, string> = {}) => {
  const m = new Map(Object.entries(kept));
  return { m, getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) };
};

describe("What's up ?", () => {
  it('stays shut at the very first start, and counts the next start from the last note', async () => {
    const el = document.createElement('whats-up');
    const s = store();
    await skipPastNews(el, s, async () => '2026-11-27');
    expect(el.getAttribute('vu')).toBe('9999-12-31');
    expect(s.m.get(SEEN_KEY)).toBe('2026-11-27');
  });

  it('opens as usual for a window that has started before (an update)', async () => {
    const el = document.createElement('whats-up');
    const s = store({ [THEME_KEY]: 'dark' });
    await skipPastNews(el, s, async () => '2026-11-27');
    expect(el.hasAttribute('vu')).toBe(false);
    expect(s.m.has(SEEN_KEY)).toBe(false);
  });

  it('stays shut when the notes cannot be read, without keeping anything', async () => {
    const el = document.createElement('whats-up');
    const s = store();
    await skipPastNews(el, s, async () => {
      throw new Error('offline');
    });
    expect(el.getAttribute('vu')).toBe('9999-12-31');
    expect(s.m.has(SEEN_KEY)).toBe(false);
  });

  it('stays shut for an interface in English, keeping nothing, and opens for one in French', () => {
    const en = document.createElement('whats-up');
    skipOtherLanguage(en, 'en');
    expect(en.getAttribute('vu')).toBe('9999-12-31');
    const fr = document.createElement('whats-up');
    skipOtherLanguage(fr, 'fr');
    expect(fr.hasAttribute('vu')).toBe(false);
  });
});
