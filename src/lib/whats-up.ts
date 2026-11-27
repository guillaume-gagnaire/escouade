// What's up ? (index.html, public/whats-up/): the window of a version's news, once per user after an update.
import { THEME_KEY } from './appearance';
import { locale, type Lang } from './i18n/locale.svelte';
import index from '../../public/whats-up/index.json';

/** Where the window keeps the last note seen (`whats-up:<app>`, app="Escouade" in index.html). */
export const SEEN_KEY = 'whats-up:Escouade';

/** Seen before any note: nothing is newer, the window stays shut. */
const AFTER_ALL = '9999-12-31';

/** The notes, known from the build (no fetch): the same ids in every language. */
export const NOTES: readonly string[] = index.ids;

/** The notes kept in public/whats-up at most: their captures go into the app's binary. */
export const MAX_NOTES = 3;

/** Where the notes of `lang` are: French at the folder's root, each other language in its own folder. */
export const notesFolder = (lang: Lang) => (lang === 'fr' ? '/whats-up/' : `/whats-up/${lang}/`);

/** Sortable key of a note id, as whats-up.js sorts them: `2026-11-27` < `2026-11-27-2` < … < `2026-11-27-10`. */
export const noteKey = (id: string) => {
  const suffix = id.slice(11);
  return `${id.slice(0, 10)}#${String(/^\d+$/.test(suffix) ? Number(suffix) : 1).padStart(4, '0')}`;
};

/**
 * What the window counts as seen when it starts (its `vu`), at once, before anything is read: null to let it read its
 * own storage. At the window's very first start (no theme, no note seen), past notes are no news: the newest counts as
 * seen and is kept now, so a reload a moment later stays shut too. After skipped versions, the one before the newest
 * counts as seen: the newest note alone opens, not every note since the last one seen.
 */
export function seenAtStart(storage: Pick<Storage, 'getItem' | 'setItem'> = localStorage, notes: readonly string[] = NOTES): string | null {
  const sorted = [...notes].sort((a, b) => (noteKey(a) < noteKey(b) ? -1 : 1));
  const latest = sorted.at(-1);
  let seen: string | null;
  let first: boolean;
  try {
    seen = storage.getItem(SEEN_KEY);
    first = seen === null && storage.getItem(THEME_KEY) === null;
  } catch {
    return null;
  }
  if (first) {
    try {
      if (latest) storage.setItem(SEEN_KEY, latest);
    } catch {
      // No storage: shut for this start all the same.
    }
    return latest ?? AFTER_ALL;
  }
  if (seen === null || sorted.length < 2) return null;
  const before = sorted.at(-2)!;
  return noteKey(before) > noteKey(seen) ? before : null;
}

/**
 * Puts the window in the page once the interface knows its language (the settings'): its notes are in that language,
 * and so are its own words. `template` holds the `<whats-up>` of index.html: it connects, and reads its attributes,
 * only now (public/whats-up/whats-up.js is left as copied).
 */
export function showWhatsUp(template: HTMLTemplateElement | null, vu: string | null, lang: Lang = locale.ui): Element | null {
  const el = template?.content.firstElementChild?.cloneNode(true) as Element | undefined;
  if (!el) return null;
  el.setAttribute('src', notesFolder(lang));
  el.setAttribute('lang', lang);
  if (vu) el.setAttribute('vu', vu);
  template!.replaceWith(el);
  return el;
}
