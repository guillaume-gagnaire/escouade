// The video's sound effects, by code: the app's chime, a mouse click and a whoosh between scenes.

export const SR = 44100;

/** The app's chime (src-tauri/src/notify.rs): A5 then E6 130 ms later, each fading over 0.65 s. */
export function chime(): Float32Array {
  const out = new Float32Array(Math.round(0.8 * SR));
  [880, 1318.5].forEach((freq, i) => {
    const offset = Math.round(i * 0.13 * SR);
    for (let k = 0; k < Math.round(0.65 * SR) && offset + k < out.length; k++) {
      const t = k / SR;
      const env = t < 0.02 ? (t / 0.02) * 0.28 : 0.28 * (0.0004 / 0.28) ** ((t - 0.02) / 0.58);
      out[offset + k] += env * Math.sin(2 * Math.PI * freq * t);
    }
  });
  return out;
}

/** A soft mouse click: a short, damped high tick. */
export function click(): Float32Array {
  const out = new Float32Array(Math.round(0.08 * SR));
  for (let k = 0; k < out.length; k++) {
    const t = k / SR;
    out[k] = 0.5 * Math.exp(-t * 260) * (Math.sin(2 * Math.PI * 2400 * t) + 0.5 * Math.sin(2 * Math.PI * 5100 * t));
  }
  return out;
}

/** A quick whoosh between two scenes: noise through a low-pass that opens then closes, swelling in its middle. */
export function whoosh(): Float32Array {
  const out = new Float32Array(Math.round(0.4 * SR));
  // A fixed seed: the same sound at every render.
  let seed = 7;
  const noise = () => {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return seed / 1073741824 - 1;
  };
  let low = 0;
  for (let k = 0; k < out.length; k++) {
    const p = k / out.length;
    const swell = Math.sin(Math.PI * p) ** 2;
    // The filter opens with the swell: from a dull rush to an airy hiss.
    const a = 0.02 + 0.25 * swell;
    low += a * (noise() - low);
    out[k] = 0.6 * swell * low * 2.2;
  }
  return out;
}
