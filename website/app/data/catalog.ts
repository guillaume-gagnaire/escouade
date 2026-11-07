// The shape of the texts of a language. fr.ts and en.ts must both fit it, so that a page cannot lack a
// sentence in one language; tests/i18n.test.ts also checks the lists' lengths and the {parameters}.
import type { DocGroupId, DocSlug } from './docs';
import type { Lang } from './language';
import type { FeatureId, StageId } from './site';
import type { Status } from './squad';

/** A sentence in its singular and its plural form. Both take `{count}`, and may take other parameters. */
export interface Plural {
  one: string;
  other: string;
}

export interface FeatureText {
  title: string;
  text: string;
  /** What the screenshot shows. */
  alt: string;
}

export interface Card {
  title: string;
  text: string;
}

/** A step of the cycle: its name, what it is about, and its features that have no screenshot of their own. */
export interface Stage {
  title: string;
  text: string;
  points: Card[];
}

export interface Step {
  title: string;
  text: string;
  link?: { label: string; href: string };
}

export interface Question {
  q: string;
  a: string;
  /** A link said after the answer. */
  link?: { label: string; href: string };
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
    roadmap: string;
    faq: string;
    github: string;
    /** The link to the documentation, in the header and the footer. */
    docs: string;
    /** The link to the plugin store, in the header and the footer. */
    plugins: string;
    /** The header's short download button. */
    download: string;
    /** The language switch's name, for screen readers. */
    language: string;
  };
  /** The ids of the sections the header’s links point to (the other sections keep the same id everywhere). */
  anchors: { features: string; install: string; roadmap: string };
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
  /** The demonstration of the queue, staged as a product screen: what it proves, in a title, a sentence and three facts. */
  spotlight: { title: string; text: string; points: string[] };
  video: { title: string };
  features: {
    title: string;
    lead: string;
    /** The chapters: what each step of the cycle is about, and what else it holds, one line each. */
    stages: Record<StageId, Stage>;
    items: Record<FeatureId, FeatureText>;
  };
  /** What else the app does, whatever the step, one line each. */
  cards: Card[];
  install: { title: string; steps: Step[] };
  /** What comes next: the next version and what it brings, with links to its detailed plan and to what is out. */
  roadmap: {
    title: string;
    lead: string;
    /** Takes `{version}`. */
    next: string;
    /** An item may lead somewhere: the plugins, to their store. */
    items: Step[];
    plan: string;
    changelog: string;
  };
  faq: { title: string; items: Question[] };
  /** The page's last word: one more chance to download. */
  close: { title: string };
  footer: { github: string; discord: string; support: string; license: string; privacy: string; legal: string; note: string };
  /** For an address that leads nowhere. */
  notFound: { title: string; text: string; home: string };
  /** For any other error the page itself runs into. */
  failure: { title: string; home: string };
  /** The demonstration: an Escouade window and its agents. */
  demo: {
    status: Record<Status, string>;
    agents: string;
    /** Takes `{loop}` and `{max}`. */
    loop: string;
    /** Take `{count}` (the criteria met) and `{total}`. */
    criteria: Plural;
    /** The other tab of the sidebar, beside the agents. */
    kanban: string;
    /** The project's tiles on the left: what a screen reader hears of them. */
    projects: string;
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
  /** The documentation's own words, around its pages (which are Markdown files, in content/docs/). */
  docs: {
    /** Its name, over its home. */
    title: string;
    /** Under the title of its home. */
    lead: string;
    /** The home's name and description for search engines and the browser's tab. */
    meta: { title: string; description: string };
    /** A page's name for search engines and the browser's tab. Takes `{title}`. */
    pageTitle: string;
    /** The groups of pages, in the sidebar and on the home: their name and what they cover. */
    groups: Record<DocGroupId, Card>;
    /** The pages' names in the sidebar, shorter than their titles. */
    pages: Record<DocSlug, string>;
    /** The sidebar's name, for screen readers. */
    nav: string;
    /** The button that opens the sidebar on a narrow screen. */
    menu: string;
    /** Over the page's summary. */
    toc: string;
    previous: string;
    next: string;
    edit: string;
    /** The names of the callouts a page may hold. */
    callouts: { note: string; tip: string; warning: string };
    search: {
      /** The header's button, and the window's name. */
      button: string;
      placeholder: string;
      /** Takes `{query}`. */
      empty: string;
      /** How to move through the results, under them. */
      keys: string;
      /** When the pages could not be loaded to search them. */
      failed: string;
      close: string;
    };
  };
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
