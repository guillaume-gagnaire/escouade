// The catalogs by language, and what the pages build from them.
import type { Catalog, FeatureText } from './catalog';
import { en } from './en';
import { fr } from './fr';
import type { Lang } from './language';
import { FEATURE_SHOTS, imageOf, type FeatureId } from './site';

export const CATALOGS: Record<Lang, Catalog> = { fr, en };

export const catalogOf = (lang: Lang): Catalog => CATALOGS[lang];

export interface Feature extends FeatureText {
  id: FeatureId;
  /** In public/. */
  image: string;
}

/** The features of the page of `lang`, in the page's order: its texts, and the screenshot it shows for each. */
export function featuresFor(lang: Lang): Feature[] {
  return FEATURE_SHOTS.map(({ id, file }) => ({ id, image: imageOf(lang, file), ...CATALOGS[lang].features.items[id] }));
}
