// The projects' logos (ESC-167), as `data:` URLs for their tiles: asked of the backend once per logo file (a new logo
// is a new file), kept for the rest of the run. Shown in an `<img>`: an SVG's scripts never run.

import { open } from '@tauri-apps/plugin-dialog';
import { onServer } from './connection.svelte';
import { t } from './i18n';
import { api } from './ipc';
import { baseName, bytesOfDataUrl } from './server-moves.svelte';
import { local } from './transport';
import { app } from './state.svelte';
import type { Project } from './types';

const urls = $state<Record<string, string | null>>({});
/** Asked already: not state, read and written while a tile is drawn. */
const asked = new Set<string>();

const keyOf = (p: Pick<Project, 'id' | 'logo'>) => `${p.id}/${p.logo}`;

/** The project's logo, null while it has none or until the backend gave it. */
export function logoUrl(p: Pick<Project, 'id' | 'logo'>): string | null {
  if (!p.logo) return null;
  const key = keyOf(p);
  if (!asked.has(key)) {
    asked.add(key);
    api
      .projectLogo(p.id)
      .then((url) => (urls[key] = url))
      .catch(() => (urls[key] = null));
  }
  return urls[key] ?? null;
}

/** The logo the window has already (the preview of the file chosen), for the project saved with it. */
export function knowLogo(p: Pick<Project, 'id' | 'logo'>, url: string) {
  if (!p.logo) return;
  asked.add(keyOf(p));
  urls[keyOf(p)] = url;
}

/** The extensions « Choisir une image… » offers; the backend knows an image by its bytes all the same. */
export const LOGO_EXTENSIONS = ['png', 'jpg', 'jpeg', 'webp', 'svg', 'ico'];

/**
 * « Choisir une image… »: a file chosen, read as a logo would be taken; null when none was chosen or it is refused (too
 * heavy, unreadable, of another format), which a toast says.
 */
export async function pickLogo(): Promise<{ path: string; url: string } | null> {
  const path = await open({
    multiple: false,
    title: t('settings.project.logoTitle'),
    filters: [{ name: t('settings.project.logoFilter'), extensions: LOGO_EXTENSIONS }],
  });
  if (typeof path !== 'string') return null;
  // The image is this computer's: read here, whichever core the window speaks to (ESC-538).
  const url = await app.run(onServer() ? local<string>('read_logo', { path }) : api.readLogo(path));
  return url ? { path, url } : null;
}

/**
 * The logo chosen (`path` of this computer, its preview `url`) saved as the project's, or removed (null). On a server,
 * the image is sent as bytes, never its path, which is this computer's (ESC-538).
 */
export function saveLogo(id: string, path: string | null, url?: string | null) {
  if (path && url && onServer()) return api.setProjectLogoBytes(id, baseName(path), bytesOfDataUrl(url));
  return api.setProjectLogo(id, path);
}

/** For the tests: nothing known. */
export function forgetLogos() {
  asked.clear();
  for (const k of Object.keys(urls)) delete urls[k];
}
