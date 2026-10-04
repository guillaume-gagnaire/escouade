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
    expect(spoken({ id: 'x', text: 'Bonjour' })).toBe('Bonjour');
  });

  it('writes for the voice the words it would say wrong, as they sound', () => {
    const say = (text: string) => spoken({ id: 'x', text });
    // « Git » with a hard g, as the French say it; not « jit ».
    expect(say('le dépôt git, Git Bash')).toBe('le dépôt guite, Guite Bash');
    expect(say('sur GitHub')).toBe('sur Guite-Heub');
    // A chat room, « tchatte »; not the cat.
    expect(say('un vrai chat : du markdown')).toBe('un vrai tchatte : du markdown');
    expect(say('tes fichiers .env, ouverte avec gh, sur macOS')).toBe('tes fichiers point E N V, ouverte avec G H, sur Mac');
    // Only whole words: « digital », « chateau », « ghost » stay as they are.
    expect(say('digital chateau ghost')).toBe('digital chateau ghost');
    // A line can still say something else entirely.
    expect(spoken({ id: 'x', text: 'git', say: 'autre' })).toBe('autre');
  });

  it('says every line through that table: no « git » or « chat » left as written', () => {
    for (const l of SCRIPT.flatMap((s) => s.lines)) expect(spoken(l), l.id).not.toMatch(/\b(git|chat|gh)\b|\.env|macOS/i);
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
