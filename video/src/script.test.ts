import { describe, expect, it } from 'vitest';
import { lineKey, SCRIPT, spoken } from './script';
import VOICE from './voice.json';

describe('script', () => {
  it('opens on the logo and closes on it', () => {
    expect(SCRIPT[0].id).toBe('intro');
    expect(SCRIPT.at(-1)!.id).toBe('outro');
  });

  it('has unique scene ids and line keys', () => {
    const scenes = SCRIPT.map((s) => s.id);
    expect(new Set(scenes).size).toBe(scenes.length);
    const keys = SCRIPT.flatMap((s) => s.lines.map((l) => lineKey(s.id, l.id)));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('gives every scene a title and something to say', () => {
    for (const s of SCRIPT) {
      expect(s.title.trim(), s.id).not.toBe('');
      expect(s.lines.length, s.id).toBeGreaterThan(0);
    }
  });

  it('says « claude point A I » where it shows claude.ai', () => {
    const line = SCRIPT.flatMap((s) => s.lines).find((l) => l.text.includes('claude.ai'))!;
    expect(spoken(line)).toContain('claude point A I');
    expect(spoken({ id: 'x', text: 'Bonjour', en: 'Hello' })).toBe('Bonjour');
  });

  it('writes for the voice the words it would say wrong, as they sound', () => {
    const say = (text: string) => spoken({ id: 'x', text, en: text });
    // « Git » with a hard g, as the French say it; not « jit ».
    expect(say('le dépôt git, Git Bash')).toBe('le dépôt guite, Guite Bash');
    expect(say('sur GitHub')).toBe('sur Guite-Heub');
    // A chat room, « tchatte »; not the cat.
    expect(say('un vrai chat : du markdown')).toBe('un vrai tchatte : du markdown');
    expect(say('tes fichiers .env, ouverte avec gh, sur macOS')).toBe('tes fichiers point E N V, ouverte avec G H, sur Mac');
    // Only whole words: « digital », « chateau », « ghost » stay as they are.
    expect(say('digital chateau ghost')).toBe('digital chateau ghost');
    // A line can still say something else entirely.
    expect(spoken({ id: 'x', text: 'git', en: 'git', say: 'autre' })).toBe('autre');
  });

  it('says every line through that table: no « git » or « chat » left as written', () => {
    for (const l of SCRIPT.flatMap((s) => s.lines)) expect(spoken(l), l.id).not.toMatch(/\b(git|chat|gh)\b|\.env|macOS/i);
  });

  it('says the French text, never the English one: the English is for the eyes (subtitles, titles)', () => {
    const line = { id: 'x', text: 'Bonjour', en: 'Hello' };
    expect(spoken(line)).toBe('Bonjour');
  });

  it('has its voice recorded, line by line, with the text said now', () => {
    const voice = VOICE as Record<string, { text: string; duration: number }>;
    const keys = SCRIPT.flatMap((s) => s.lines.map((l) => [lineKey(s.id, l.id), spoken(l)] as const));
    for (const [key, text] of keys) {
      expect(voice[key], `${key} : lance npm run voice`).toBeDefined();
      expect(voice[key].text, `${key} a changé : lance npm run voice`).toBe(text);
      expect(voice[key].duration).toBeGreaterThan(0.5);
    }
    // No clip left over from a line that is gone.
    expect(Object.keys(voice).sort()).toEqual(keys.map(([k]) => k).sort());
  });
});

describe('the English script', () => {
  const lines = SCRIPT.flatMap((s) => s.lines.map((l) => ({ key: lineKey(s.id, l.id), ...l })));

  it('translates every title and every line, next to the French one', () => {
    for (const s of SCRIPT) expect(s.titleEn?.trim(), `${s.id} : titleEn`).toBeTruthy();
    for (const l of lines) expect(l.en?.trim(), `${l.key} : en`).toBeTruthy();
  });

  it('is English: not a copy of the French, and no French accent or guillemet left', () => {
    const french = /[àâäçéèêëîïôöùûüœ«»]/i;
    for (const s of SCRIPT) {
      expect(s.titleEn, s.id).not.toBe(s.title);
      expect(s.titleEn, s.id).not.toMatch(french);
    }
    for (const l of lines) {
      expect(l.en, l.key).not.toBe(l.text);
      expect(l.en, l.key).not.toMatch(french);
    }
  });

  it('is written like the app’s English: typographic apostrophes and quotes, one space after a stop', () => {
    const texts = [...SCRIPT.map((s) => s.titleEn), ...lines.map((l) => l.en)];
    for (const t of texts) {
      expect(t, t).not.toMatch(/['"]/);
      expect(t, t).not.toMatch(/ {2}|\s$|^\s|\.\.\./);
    }
  });

  it('is as short as a product launch needs: about the length of the French, never much longer', () => {
    for (const l of lines) expect(l.en.length, l.key).toBeLessThanOrEqual(Math.ceil(l.text.length * 1.15));
  });

  it('keeps the product’s names and the glossary’s words', () => {
    const all = lines.map((l) => l.en).join(' ');
    // Fixed by the app’s glossary: the Kanban is never a « board » here, an agent is an agent.
    expect(all).toMatch(/Kanban/);
    expect(all).not.toMatch(/\b(board|boards)\b/i);
    expect(all).not.toMatch(/\bautopilote\b|\bauto-pilot\b|\bauto pilot\b/i);
    expect(all).toMatch(/\bautopilot\b/i);
    expect(all).toMatch(/\bworktree\b/);
    // The « À tester » column is « To review » (the glossary), whatever the French says.
    expect(all).toMatch(/To do, in progress, to review, done/);
    expect(all).not.toMatch(/\bto test,|\bTo test\b/);
  });
});
