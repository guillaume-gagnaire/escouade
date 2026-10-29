import { describe, expect, it } from 'vitest';
import { cleanDeep, cleanText } from './cleantext';

// ESC-290: what a tool that read a binary file gives the conversation.
describe('cleanText', () => {
  it('lets ordinary text through untouched, tabs, line ends and pairs of UTF-16 included', () => {
    const text = 'ligne 1\tcolonne\r\nligne 2 — émoji 🎧 et 日本語';
    expect(cleanText(text)).toBe(text);
  });

  it('shows the control characters by their picture', () => {
    expect(cleanText('a\x00b\x07c\x7f')).toBe('a␀b␇c␡');
  });

  it('leaves a terminal’s colours and titles out, and shows a lone escape', () => {
    expect(cleanText('\x1b[31mrouge\x1b[0m \x1b]0;titre\x07fin \x1bx')).toBe('rouge fin ␛x');
  });

  it('puts � in place of a lone half of a UTF-16 pair', () => {
    expect(cleanText('a\ud800b')).toBe('a�b');
    expect(cleanText('a\udc00')).toBe('a�');
    expect(cleanText('🎧\ud83c')).toBe('🎧�');
  });

  it('cleans the bytes of a WAV file read as text into something with no control left', () => {
    const bytes = new Uint8Array(4096);
    for (let i = 0; i < bytes.length; i++) bytes[i] = (i * 37 + 11) % 256;
    bytes.set(new TextEncoder().encode('RIFF\x00\x00\x00\x00WAVEfmt '), 0);
    const read = new TextDecoder('latin1').decode(bytes);
    const clean = cleanText(read);
    expect(clean).not.toMatch(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f\ud800-\udfff]/);
    expect(clean.startsWith('RIFF␀␀␀␀WAVEfmt ')).toBe(true);
  });

  it('is quick on a line of 5 MB', () => {
    const line = 'x'.repeat(5_000_000) + '\x00';
    const started = performance.now();
    expect(cleanText(line).endsWith('␀')).toBe(true);
    expect(performance.now() - started).toBeLessThan(2000);
  });
});

describe('cleanDeep', () => {
  it('cleans every text of an item, however deep', () => {
    const item = { kind: 'tool', input: { command: 'cat a.wav' }, result: { text: 'x\x00y', parts: ['\x1b[1mz'] }, n: 3, ok: true };
    expect(cleanDeep(item)).toEqual({
      kind: 'tool',
      input: { command: 'cat a.wav' },
      result: { text: 'x␀y', parts: ['z'] },
      n: 3,
      ok: true,
    });
  });
});
