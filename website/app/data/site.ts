// What the site says in every language: its links, its version, its screenshots and its video.
// Its texts are in fr.ts and en.ts.
import type { Lang } from './language';

export const REPO = 'https://github.com/guillaume-gagnaire/escouade';
export const DOWNLOAD = `${REPO}/releases/latest`;
/** Public address of the site, for links shared on social networks. */
export const SITE = 'https://guillaume-gagnaire.github.io/escouade/';

/** The features, in the order of the page, with the screenshot (a file of the images folder) that shows each. */
export const FEATURE_SHOTS = [
  { id: 'agents', file: 'agents.jpg' },
  { id: 'chat', file: 'chat.jpg' },
  { id: 'notifications', file: 'notifications.jpg' },
  { id: 'git', file: 'git.jpg' },
  { id: 'editeur', file: 'editor.jpg' },
  { id: 'tableau', file: 'board.jpg' },
  { id: 'test', file: 'test.jpg' },
  { id: 'integrations', file: 'integrations.jpg' },
  { id: 'lancement', file: 'launch.jpg' },
  { id: 'stats', file: 'stats.jpg' },
  { id: 'remote', file: 'remote.jpg' },
] as const;

export type FeatureId = (typeof FEATURE_SHOTS)[number]['id'];

/**
 * The folder of public/ holding the screenshots (and the video's poster) each language's page shows. The
 * English page borrows the French ones for now: giving it its own is a matter of pointing `en` at
 * 'images/en/' once the captures are there.
 */
export const IMAGE_DIR: Record<Lang, string> = { fr: 'images/', en: 'images/' };

/** A file of public/ for the page of `lang`. */
export const imageOf = (lang: Lang, file: string) => `${IMAGE_DIR[lang]}${file}`;

export interface Track {
  srclang: string;
  label: string;
  /** In public/. */
  src: string;
  /** Turned on without the visitor asking. */
  default?: boolean;
}

/**
 * The presentation video and the subtitle tracks each page offers. Its voice is French: the English page
 * has nothing else to offer yet than the French subtitles.
 */
export const VIDEO: Record<Lang, { src: string; tracks: Track[] }> = {
  fr: { src: 'escouade.mp4', tracks: [{ srclang: 'fr', label: 'Français', src: 'escouade.vtt' }] },
  en: { src: 'escouade.mp4', tracks: [{ srclang: 'fr', label: 'Français', src: 'escouade.vtt' }] },
};
