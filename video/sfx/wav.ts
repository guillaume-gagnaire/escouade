const toInt16 = (x: number) => Math.round(Math.max(-1, Math.min(1, x)) * 32767);

/** A 16-bit PCM stereo WAV file from samples in [-1, 1]. */
export function encodeWav(left: Float32Array, right: Float32Array, sampleRate: number): Uint8Array {
  const data = left.length * 4;
  const buf = new ArrayBuffer(44 + data);
  const v = new DataView(buf);
  const text = (o: number, s: string) => [...s].forEach((c, i) => v.setUint8(o + i, c.charCodeAt(0)));
  text(0, 'RIFF');
  v.setUint32(4, 36 + data, true);
  text(8, 'WAVE');
  text(12, 'fmt ');
  v.setUint32(16, 16, true);
  v.setUint16(20, 1, true);
  v.setUint16(22, 2, true);
  v.setUint32(24, sampleRate, true);
  v.setUint32(28, sampleRate * 4, true);
  v.setUint16(32, 4, true);
  v.setUint16(34, 16, true);
  text(36, 'data');
  v.setUint32(40, data, true);
  for (let i = 0; i < left.length; i++) {
    v.setInt16(44 + i * 4, toInt16(left[i]), true);
    v.setInt16(46 + i * 4, toInt16(right[i]), true);
  }
  return new Uint8Array(buf);
}
