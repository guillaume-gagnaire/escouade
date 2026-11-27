/**
 * The spaces this phone is paired with (MO-1, MO-4): each its name and how it is reached.
 *
 * - **Paired by the app** (MO-4, `pairing.svelte.ts`): the list kept on the phone (`localStorage`, `escouade.spaces`)
 *   holds their id, name and kind alone; their keys and token are the native side's, in the Keystore or the Keychain
 *   (SR1-15), and their address is asked of it at each connection (`startTunnel`).
 * - **Given to the page** (the e2e of the shell, a build of development): `window.__ESCOUADE_SPACES__` or
 *   `VITE_ESCOUADE_SPACES`, with their address and token.
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
  /** Kept by the app (MO-4): its address and token are the native side's, `url` and `token` empty here. */
  kept?: boolean;
}

/** A space the app paired, as its list keeps it: no address, no key, no token. */
export interface KeptSpace {
  id: string;
  name: string;
  here: boolean;
  kind: 'tunnel' | 'direct';
}

const LIST = 'escouade.spaces';

/** The spaces the app paired (a broken list is none). */
export function keptSpaces(store: Pick<Storage, 'getItem'> | null = safeStorage()): KeptSpace[] {
  try {
    const raw = JSON.parse(store?.getItem(LIST) ?? '[]');
    if (!Array.isArray(raw)) return [];
    return raw.filter(
      (s): s is KeptSpace =>
        !!s && typeof s.id === 'string' && !!s.id && typeof s.name === 'string' && (s.kind === 'tunnel' || s.kind === 'direct'),
    );
  } catch {
    return [];
  }
}

function write(list: KeptSpace[], store: Pick<Storage, 'setItem'> | null): void {
  try {
    store?.setItem(LIST, JSON.stringify(list));
  } catch {
    // A private window: the list lasts as long as the page.
  }
}

/** A space paired: kept (in place of one of the same id). */
export function keepSpace(space: KeptSpace, store: Storage | null = safeStorage()): void {
  write([...keptSpaces(store).filter((s) => s.id !== space.id), space], store);
}

/** A space forgotten from the list. */
export function dropSpace(id: string, store: Storage | null = safeStorage()): void {
  write(
    keptSpaces(store).filter((s) => s.id !== id),
    store,
  );
}

function safeStorage(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

/** Every space of the phone: those given to the page, then those the app paired. */
export function allSpaces(): PairedSpace[] {
  const given = pairedSpaces();
  const ids = new Set(given.map((s) => s.id));
  return [
    ...given,
    ...keptSpaces()
      .filter((s) => !ids.has(s.id))
      .map((s) => ({ id: s.id, name: s.name, here: s.here, url: '', token: '', kept: true })),
  ];
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
