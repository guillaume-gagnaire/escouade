// The shape of the texts of a language. fr.ts and en.ts must both fit it, so that a page cannot lack a
// sentence in one language; tests/i18n.test.ts also checks the lists' lengths and the {parameters}.
import type { Lang } from './language';
import type { FeatureId } from './site';
import type { Status } from './squad';

/** A sentence in its singular and its plural form. Both take `{count}`, and may take other parameters. */
export interface Plural {
  one: string;
  other: string;
}

export interface FeatureText {
  title: string;
  text: string;
  points: string[];
  /** What the screenshot shows. */
  alt: string;
}

export interface Card {
  title: string;
  text: string;
}

export interface Step {
  title: string;
  text: string;
  link?: { label: string; href: string };
}

export interface Question {
  q: string;
  a: string;
}

/** What the demo of the hero makes up in each language: the agents' names, a question, what the agents did. */
export interface SquadText {
  names: { auth: string; e2e: string; docs: string; login: string };
  /** The tickets the ticket agent takes in turn, as the end of their agent's name. */
  ticketSlugs: string[];
  question: { text: string; options: string[] };
  results: { files: Plural; tests: Plural; met: string };
}

export interface Catalog {
  meta: {
    title: string;
    description: string;
    /** For Open Graph: `fr_FR`, `en_US`. */
    locale: string;
  };
  header: {
    /** The navigation's name, for screen readers. */
    sections: string;
    video: string;
    features: string;
    install: string;
    faq: string;
    github: string;
    /** The language switch's name, for screen readers. */
    language: string;
  };
  /** The ids of the sections the header’s links point to (the other sections keep the same id everywhere). */
  anchors: { features: string; install: string };
  hero: {
    line1: string;
    line2: string;
    lede: string;
    download: string;
    github: string;
    /** Takes `{version}`. */
    meta: string;
    video: string;
  };
  features: { eyebrow: string; title: string; lead: string; items: Record<FeatureId, FeatureText> };
  cards: Card[];
  install: { eyebrow: string; title: string; steps: Step[] };
  faq: { eyebrow: string; title: string; items: Question[] };
  footer: { github: string; license: string; note: string };
  /** The windows of the hero. */
  demo: {
    status: Record<Status, string>;
    agents: string;
    /** Takes `{loop}` and `{max}`. */
    loop: string;
    /** Take `{count}` (the criteria met) and `{total}`. */
    criteria: Plural;
    tokens: string;
    /** Take `{count}`. */
    running: string;
    waiting: string;
    done: Plural;
    /** Takes `{amount}`. */
    today: string;
    pause: string;
    caption: string;
    /** Read by screen readers after the caption, which it continues. */
    description: string;
  };
  squad: SquadText;
}

/** `template` with its `{parameters}` replaced. A parameter that is missing is a mistake, not a text to show. */
export function fmt(template: string, params: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (_, name: string) => {
    if (!(name in params)) throw new Error(`Missing parameter {${name}} for “${template}”`);
    return String(params[name]);
  });
}

/** The form of `entry` that goes with `count` in `lang`, with `{count}` filled in. */
export function plural(lang: Lang, entry: Plural, count: number, params: Record<string, string | number> = {}): string {
  const form = new Intl.PluralRules(lang).select(count) === 'one' ? entry.one : entry.other;
  return fmt(form, { count, ...params });
}
