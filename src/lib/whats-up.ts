// What's up ? (index.html, public/whats-up/): the window of a version's news, once per user after an update.
import { THEME_KEY } from './appearance';
import { locale, type Lang } from './i18n/locale.svelte';

/** Where the window keeps the last note seen (`whats-up:<app>`, app="Escouade" in index.html). */
export const SEEN_KEY = 'whats-up:Escouade';

/** Seen before any note: nothing is newer, the window stays shut. */
const AFTER_ALL = '9999-12-31';

async function latestNote(): Promise<string | null> {
  const { ids } = (await (await fetch('/whats-up/index.json')).json()) as { ids: string[] };
  return ids.at(-1) ?? null;
}

/**
 * At the window's very first start (nothing kept: no theme, no note seen), the notes of past versions are not news: the
 * window stays shut, and the next start counts from the last note. A fresh WebView profile (each end-to-end test) is
 * such a start. Called before public/whats-up/whats-up.js defines the element, which reads `vu` as it connects.
 */
export async function skipPastNews(
  el: Element | null,
  storage: Pick<Storage, 'getItem' | 'setItem'> = localStorage,
  latest: () => Promise<string | null> = latestNote,
): Promise<void> {
  let first: boolean;
  try {
    first = storage.getItem(THEME_KEY) === null && storage.getItem(SEEN_KEY) === null;
  } catch {
    return;
  }
  if (!first || !el) return;
  el.setAttribute('vu', AFTER_ALL);
  const id = await latest().catch(() => null);
  if (!id) return;
  try {
    storage.setItem(SEEN_KEY, id);
  } catch {
    // No storage: shut for this start all the same.
  }
}

/**
 * The notes are written in French alone for now (`lang="fr"` in index.html): an interface in English keeps the window
 * shut, and keeps nothing, so a note in English opens once there is one. The language is the one the window starts in
 * (the system's, the default setting) before the backend says the settings'.
 */
export function skipOtherLanguage(el: Element | null, lang: Lang = locale.ui): void {
  if (el && lang !== 'fr') el.setAttribute('vu', AFTER_ALL);
}
