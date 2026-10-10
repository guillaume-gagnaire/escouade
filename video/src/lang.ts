// The language of the reconstructed app: the voice stays French, the interface the pictures show can be English.
// A scene reads its language from here; the French is the default, so a picture that asks for nothing is French.

import { createContext, useContext, type ReactNode } from 'react';
import { fr as comma } from './anim';

export type Lang = 'fr' | 'en';
export const LANGS: readonly Lang[] = ['fr', 'en'];

export const LangContext = createContext<Lang>('fr');
export const useLang = (): Lang => useContext(LangContext);

/** `tr(fr, en)`: the text in the language of the picture. */
export type Tr = (fr: string, en: string) => string;
export const trOf =
  (lang: Lang): Tr =>
  (fr, en) =>
    lang === 'en' ? en : fr;

/** Numbers and amounts as the app writes them in each language (src/lib/format.ts of the app). */
export interface Fmt {
  lang: Lang;
  tr: Tr;
  /** A value that depends on the language: where the English text, being shorter or longer, puts a spotlight or a pointer. */
  pick: <T>(fr: T, en: T) => T;
  /** `tr` for markup: the French keeps its own text nodes (« ~{n} modifiés »), so that its pictures stay the same to the pixel. */
  trx: (fr: ReactNode, en: ReactNode) => ReactNode;
  /** Thousands of tokens: « 24,8 k » in French, « 24.8k » in English. */
  tok: (thousands: number, digits?: number) => string;
  /** « 2,41 $ » in French, « $2.41 » in English. */
  usd: (amount: number, digits?: number) => string;
  /** « 22 % » in French, « 22% » in English. */
  pct: (n: number) => string;
  /** Millions of tokens: « 28,10 M » in French, « 28.10M » in English. */
  million: (m: number, digits?: number) => string;
  /** A whole number with its thousands separated. */
  int: (n: number) => string;
  /** The right form of a noun for `n`: French writes « 0 fichier », English « 0 files ». */
  plural: (n: number, frOne: string, frMany: string, enOne: string, enMany: string) => string;
}

export function fmtOf(lang: Lang): Fmt {
  const en = lang === 'en';
  return {
    lang,
    tr: trOf(lang),
    pick: (french, english) => (en ? english : french),
    trx: (french, english) => (en ? english : french),
    tok: (k, digits = 0) => (en ? `${k.toFixed(digits)}k` : `${comma(k, digits)} k`),
    usd: (x, digits = 2) => (en ? `$${x.toFixed(digits)}` : `${comma(x, digits)} $`),
    pct: (n) => (en ? `${n}%` : `${n} %`),
    million: (m, digits = 2) => (en ? `${m.toFixed(digits)}M` : `${comma(m, digits)} M`),
    int: (n) => Math.round(n).toLocaleString(en ? 'en-US' : 'fr-FR'),
    plural: (n, frOne, frMany, enOne, enMany) => (en ? (n === 1 ? enOne : enMany) : n > 1 ? frMany : frOne),
  };
}

/** The language of the picture being drawn, with its formatters: `const { tr, usd } = useFmt()`. */
export const useFmt = (): Fmt => fmtOf(useLang());
