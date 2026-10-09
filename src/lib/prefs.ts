// Preferences kept in the WebView's localStorage. Until 0.1.4 they were under `ccm.*`: a
// preference found there is moved to its `escouade.*` key the first time it is read.

const key = (name: string) => `escouade.${name}`;
const oldKey = (name: string) => `ccm.${name}`;

export function readPref(name: string): string | null {
  try {
    const value = localStorage.getItem(key(name));
    if (value !== null) return value;
    const old = localStorage.getItem(oldKey(name));
    if (old !== null) {
      localStorage.setItem(key(name), old);
      localStorage.removeItem(oldKey(name));
    }
    return old;
  } catch {
    return null;
  }
}

export function writePref(name: string, value: string) {
  try {
    localStorage.setItem(key(name), value);
  } catch {
    // Storage unavailable: the choice lasts until the app closes.
  }
}

/** Forgets a preference, including the key it had before the rename (which would be taken over again). */
export function removePref(name: string) {
  try {
    localStorage.removeItem(key(name));
    localStorage.removeItem(oldKey(name));
  } catch {
    // Storage unavailable: nothing was kept.
  }
}
