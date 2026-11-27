import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { THEME_KEY } from './appearance';
import { MAX_NOTES, NOTES, noteKey, notesFolder, SEEN_KEY, seenAtStart, showWhatsUp } from './whats-up';

const store = (kept: Record<string, string> = {}) => {
  const m = new Map(Object.entries(kept));
  return { m, getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) };
};

const template = () => {
  const tpl = document.createElement('template');
  tpl.innerHTML = '<whats-up src="/whats-up/" app="Escouade" lang="fr"></whats-up>';
  document.body.append(tpl);
  return tpl;
};

describe("What's up ?: what counts as seen at the start", () => {
  it('stays shut at the very first start, and keeps the newest note at once (no fetch to wait for)', () => {
    const s = store();
    expect(seenAtStart(s, ['2026-11-20', '2026-11-27'])).toBe('2026-11-27');
    expect(s.m.get(SEEN_KEY)).toBe('2026-11-27');
  });

  it('stays shut at a first start without any note, keeping nothing', () => {
    const s = store();
    expect(seenAtStart(s, [])).toBe('9999-12-31');
    expect(s.m.has(SEEN_KEY)).toBe(false);
  });

  it('lets the window read its own storage after a single new note (an update)', () => {
    const s = store({ [THEME_KEY]: 'dark', [SEEN_KEY]: '2026-11-20' });
    expect(seenAtStart(s, ['2026-11-20', '2026-11-27'])).toBeNull();
    expect(s.m.get(SEEN_KEY)).toBe('2026-11-20');
  });

  it('shows the newest note alone after skipped versions: the one before it counts as seen', () => {
    const s = store({ [THEME_KEY]: 'dark', [SEEN_KEY]: '2026-11-13' });
    expect(seenAtStart(s, ['2026-11-20', '2026-11-27', '2026-11-27-2'])).toBe('2026-11-27');
  });

  it('sorts as the window does: 2026-11-27-10 comes after 2026-11-27-2', () => {
    const s = store({ [THEME_KEY]: 'dark', [SEEN_KEY]: '2026-11-20' });
    expect(seenAtStart(s, ['2026-11-27-10', '2026-11-27', '2026-11-27-2'])).toBe('2026-11-27-2');
    expect(noteKey('2026-11-27') < noteKey('2026-11-27-2')).toBe(true);
  });

  it('opens the newest for a window that started before any note was seen, as the window does', () => {
    const s = store({ [THEME_KEY]: 'light' });
    expect(seenAtStart(s, ['2026-11-20', '2026-11-27'])).toBeNull();
  });

  it('says nothing when the storage cannot be read', () => {
    const broken = {
      getItem: () => {
        throw new Error('denied');
      },
      setItem: () => {},
    };
    expect(seenAtStart(broken, ['2026-11-27'])).toBeNull();
  });

  it('stays shut at a first start whose storage refuses to keep', () => {
    const full = {
      getItem: () => null,
      setItem: () => {
        throw new Error('quota');
      },
    };
    expect(seenAtStart(full, ['2026-11-27'])).toBe('2026-11-27');
  });
});

describe("What's up ?: the window in the interface's language", () => {
  it('puts the French notes and words in the page for an interface in French', () => {
    const el = showWhatsUp(template(), null, 'fr')!;
    expect(el.isConnected).toBe(true);
    expect(el.getAttribute('src')).toBe('/whats-up/');
    expect(el.getAttribute('lang')).toBe('fr');
    expect(el.hasAttribute('vu')).toBe(false);
    expect(document.querySelector('template')).toBeNull();
    el.remove();
  });

  it('puts the English notes and words in the page for an interface in English, with what counts as seen', () => {
    const el = showWhatsUp(template(), '2026-11-27', 'en')!;
    expect(el.getAttribute('src')).toBe('/whats-up/en/');
    expect(el.getAttribute('lang')).toBe('en');
    expect(el.getAttribute('vu')).toBe('2026-11-27');
    el.remove();
  });

  it('does nothing without its template', () => {
    expect(showWhatsUp(null, null, 'fr')).toBeNull();
  });
});

describe("What's up ?: the notes in public/whats-up", () => {
  const root = join(__dirname, '../../public/whats-up');
  const files = (dir: string) => readdirSync(dir).filter((f) => f !== 'en' && f !== 'whats-up.js' && f !== 'index.json');
  // `<id>.json`, `<id>.css`, `<id>-<name>.webp` (a capture's name starts with a letter: `2026-11-27-2-page`).
  const ofNote = (f: string) => /^(\d{4}-\d\d-\d\d(?:-\d+)?)(?:\.json|\.css|-[a-z][\w-]*\.(?:webp|png|jpe?g|avif))$/.exec(f)?.[1];

  it(`keeps ${MAX_NOTES} notes at most, oldest first: their captures go into the binary`, () => {
    expect(NOTES.length).toBeGreaterThan(0);
    expect(NOTES.length).toBeLessThanOrEqual(MAX_NOTES);
    expect([...NOTES].sort((a, b) => (noteKey(a) < noteKey(b) ? -1 : 1))).toEqual(NOTES);
  });

  it('has every note in French and in English, with the same ids', () => {
    const en = JSON.parse(readFileSync(join(root, 'en/index.json'), 'utf8'));
    expect(en.ids).toEqual(NOTES);
    for (const lang of ['fr', 'en'] as const) {
      const dir = join(root, notesFolder(lang).slice('/whats-up/'.length));
      for (const id of NOTES) {
        const note = JSON.parse(readFileSync(join(dir, `${id}.json`), 'utf8'));
        expect(note.id).toBe(id);
        expect(note.ecrans.length).toBeGreaterThan(0);
        expect(note.ecrans.length).toBeLessThanOrEqual(5);
        expect(existsSync(join(dir, `${id}.css`))).toBe(true);
        // Each capture a scene names is there, in light and in dark.
        for (const e of note.ecrans) {
          for (const [, img] of (e.scene as string).matchAll(/(?:src|data-sombre)="([^"]+)"/g)) {
            expect(existsSync(join(dir, img)), `${lang}/${img}`).toBe(true);
          }
        }
      }
    }
  });

  it('holds nothing but the notes listed (an older one is taken out with its captures)', () => {
    for (const dir of [root, join(root, 'en')]) {
      for (const f of files(dir)) expect(NOTES, f).toContain(ofNote(f));
    }
  });
});
