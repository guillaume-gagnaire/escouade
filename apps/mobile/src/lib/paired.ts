/**
 * The spaces this phone is paired with (MO-1): each its name, the address of its remote API and the device's token.
 *
 * MO-4 pairs them by QR code and keeps the token in the Keystore or the Keychain. Until then, a space is given to the
 * page alone: by `window.__ESCOUADE_SPACES__` (the e2e put it there before the page loads), or, in a build of
 * development only, by `VITE_ESCOUADE_SPACES` (`npm run dev:server` pairs a local `escouade-server` and says it). The
 * app on a phone is given neither: it opens on « Escouade sur ce téléphone ».
 */

/** A space the phone knows. */
export interface PairedSpace {
  /** Its id on the phone: `here` for the computer, else the server's. */
  id: string;
  /** Its name, as the phone shows it (« Ce poste », « Travail »). */
  name: string;
  /** The computer (its plate is a screen), not a server (a letter). */
  here: boolean;
  /** The address of its remote API: `ws://127.0.0.1:<port>/<secret>/remote` through the tunnel's proxy (MO-3). */
  url: string;
  /** The device's token. */
  token: string;
}

declare global {
  interface Window {
    __ESCOUADE_SPACES__?: unknown;
  }
}

/** The spaces in `raw`, as far as they are well formed (a broken one is dropped, never guessed). */
export function parseSpaces(raw: unknown): PairedSpace[] {
  let list = raw;
  if (typeof list === 'string') {
    try {
      list = JSON.parse(list);
    } catch {
      return [];
    }
  }
  if (!Array.isArray(list)) return [];
  const out: PairedSpace[] = [];
  const seen = new Set<string>();
  for (const s of list) {
    if (!s || typeof s !== 'object') continue;
    const { id, name, here, url, token } = s as Record<string, unknown>;
    if (typeof id !== 'string' || !id || seen.has(id)) continue;
    if (typeof name !== 'string' || !name.trim()) continue;
    if (typeof url !== 'string' || !/^wss?:\/\//.test(url)) continue;
    if (typeof token !== 'string' || !token) continue;
    seen.add(id);
    out.push({ id, name: name.trim(), here: here === true, url, token });
  }
  return out;
}

/** The spaces given to this page: the e2e's, else a build of development's. */
export function pairedSpaces(win: Pick<Window, '__ESCOUADE_SPACES__'> = window, dev: string | undefined = devSpaces()): PairedSpace[] {
  if (win.__ESCOUADE_SPACES__ !== undefined) return parseSpaces(win.__ESCOUADE_SPACES__);
  return dev ? parseSpaces(dev) : [];
}

function devSpaces(): string | undefined {
  return import.meta.env.DEV ? (import.meta.env.VITE_ESCOUADE_SPACES as string | undefined) : undefined;
}
