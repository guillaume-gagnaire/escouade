import { describe, expect, it } from 'vitest';
import { encodeWav } from './wav';

describe('encodeWav', () => {
  it('writes a 16-bit stereo PCM file', () => {
    const wav = encodeWav(new Float32Array([0, 1, -1]), new Float32Array([0.5, 2, -2]), 44100);
    const v = new DataView(wav.buffer);
    const text = (o: number) => String.fromCharCode(...wav.slice(o, o + 4));
    expect([text(0), text(8), text(12), text(36)]).toEqual(['RIFF', 'WAVE', 'fmt ', 'data']);
    expect(v.getUint16(22, true)).toBe(2);
    expect(v.getUint32(24, true)).toBe(44100);
    expect(v.getUint16(34, true)).toBe(16);
    expect(v.getUint32(40, true)).toBe(3 * 4);
    expect(wav.length).toBe(44 + 12);
    // Interleaved left/right, clamped to [-1, 1].
    expect([v.getInt16(44, true), v.getInt16(46, true), v.getInt16(48, true), v.getInt16(50, true), v.getInt16(54, true)]).toEqual([
      0, 16384, 32767, 32767, -32767,
    ]);
  });
});
