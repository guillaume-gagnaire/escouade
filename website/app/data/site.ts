// What the site says in every language: its links, its version, its screenshots and its video.
// Its texts are in fr.ts and en.ts.
import { LANGS, type Lang } from './language';

export const REPO = 'https://github.com/guillaume-gagnaire/escouade';
/** The Discord server of Escouade's community. */
export const DISCORD = 'https://discord.gg/m3KYtN8gxP';
/**
 * The page to offer a coffee to support Escouade (a Stripe payment link, any amount), in the page’s language: in the
 * footer, the FAQ and the documentation.
 */
export const SUPPORT: Record<Lang, string> = {
  fr: 'https://buy.stripe.com/8x2fZh20z7rM87067200001',
  en: 'https://buy.stripe.com/00w7sLcFdaDYaf8cvq00000',
};
/**
 * The plugin store, in the page’s language: French at its root, English under /en. In the header, the footer and
 * the roadmap.
 */
export const STORE: Record<Lang, string> = {
  fr: 'https://store.escouade.dev/',
  en: 'https://store.escouade.dev/en',
};
export const DOWNLOAD = `${REPO}/releases/latest`;
/** The version the roadmap announces, whose detailed plan is `docs/TODO-<it>.md`. A test fails once it is out. */
export const NEXT = '2.2';
export const PLAN = `${REPO}/blob/main/docs/TODO-${NEXT}.md`;
/** What is already out, in the page’s language: CHANGELOG.md in French, CHANGELOG.en.md in English. */
export const CHANGELOG: Record<Lang, string> = {
  fr: `${REPO}/blob/main/CHANGELOG.md`,
  en: `${REPO}/blob/main/CHANGELOG.en.md`,
};
/** Public address of the site, for links shared on social networks. */
export const SITE = 'https://escouade.dev/';

/** Public address of the page of `lang`. */
export const pageUrl = (lang: Lang) => `${SITE}${LANGS.find((l) => l.code === lang)!.path}`;

/**
 * The steps of developing with agents, the chapters of the page's features: Escouade is a whole development
 * environment, from the ticket to the merge, and the page tells it in that order.
 */
export const STAGES = ['plan', 'code', 'review', 'ship', 'steer'] as const;

export type StageId = (typeof STAGES)[number];

/**
 * The features, in the order of the page, with the step they belong to and the screenshot (a file of the images
 * folder) that shows each. A step may have none: it then says what it does in its points alone.
 */
export const FEATURE_SHOTS = [
  { id: 'tableau', stage: 'plan', file: 'board.jpg' },
  { id: 'integrations', stage: 'plan', file: 'integrations.jpg' },
  { id: 'agents', stage: 'code', file: 'agents.jpg' },
  { id: 'chat', stage: 'code', file: 'chat.jpg' },
  { id: 'editeur', stage: 'code', file: 'editor.jpg' },
  { id: 'git', stage: 'review', file: 'git.jpg' },
  { id: 'test', stage: 'review', file: 'test.jpg' },
  { id: 'lancement', stage: 'review', file: 'launch.jpg' },
  { id: 'notifications', stage: 'steer', file: 'notifications.jpg' },
  { id: 'stats', stage: 'steer', file: 'stats.jpg' },
  { id: 'remote', stage: 'steer', file: 'remote.jpg' },
] as const satisfies readonly { id: string; stage: StageId; file: string }[];

export type FeatureId = (typeof FEATURE_SHOTS)[number]['id'];

/** The feature whose screenshot the hero shows whole: the code under review, more than a conversation. */
export const HERO_SHOT: FeatureId = 'git';

/**
 * The folder of public/ holding the screenshots (and the video's poster) each language's page shows: the real app on
 * a made-up demo, in the language of the page (`npm run site-shots` at the root makes both, tests/site-shots/README.md;
 * the poster comes from the video, `npm run site-images` in video/).
 */
export const IMAGE_DIR: Record<Lang, string> = { fr: 'images/', en: 'images/en/' };

/** A file of public/ for the page of `lang`. */
export const imageOf = (lang: Lang, file: string) => `${IMAGE_DIR[lang]}${file}`;

export interface Track {
  /** `captions` write the voice in its own language, `subtitles` translate it. */
  kind: 'captions' | 'subtitles';
  srclang: string;
  label: string;
  /** In public/. */
  src: string;
  /** Turned on without the visitor asking. */
  default?: boolean;
}

/**
 * The presentation video and the subtitle tracks each page offers. Its voice is French: the English page turns
 * on the English subtitles (same moments, translated) and still offers the French ones.
 */
export const VIDEO: Record<Lang, { src: string; tracks: Track[] }> = {
  fr: { src: 'escouade.mp4', tracks: [{ kind: 'captions', srclang: 'fr', label: 'Français', src: 'escouade.vtt' }] },
  en: {
    src: 'escouade.mp4',
    tracks: [
      { kind: 'subtitles', srclang: 'en', label: 'English', src: 'escouade.en.vtt', default: true },
      { kind: 'captions', srclang: 'fr', label: 'Français', src: 'escouade.vtt' },
    ],
  },
};
