import { existsSync, readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { CATALOGS, featuresFor } from '../app/data/catalogs';
import { fmt } from '../app/data/catalog';
import type { Lang } from '../app/data/language';
import { SITE, VIDEO, imageOf } from '../app/data/site';
import { SNAPSHOT, squadAt } from '../app/data/squad';
import { escapeText, headOf, leaves, tags } from './helpers';

const OUT = new URL('../.output/public/', import.meta.url);
const BASE = '/escouade/';
const VERSION = (JSON.parse(readFileSync(new URL('../../src-tauri/tauri.conf.json', import.meta.url), 'utf8')) as { version: string })
  .version;

/** The two pages the site generates: where each lives, and its address once online. */
const PAGES: { lang: Lang; file: string; url: string; other: Lang }[] = [
  { lang: 'fr', file: 'index.html', url: SITE, other: 'en' },
  { lang: 'en', file: 'en/index.html', url: `${SITE}en/`, other: 'fr' },
];

/** What only one of the pages says, in the other one’s words. */
const SAYS = {
  fr: {
    autopilot: 'Pilote auto',
    criteria: /critères d’acceptation/,
    status: /statut/,
    affiliation: 'non affilié à Anthropic',
    docs: '/docs/fr/overview',
  },
  en: {
    autopilot: 'Autopilot',
    criteria: /acceptance criteria/,
    status: /status/,
    affiliation: 'not affiliated with Anthropic',
    docs: '/docs/en/overview',
  },
};

/** `css` without the blocks opening with `open` (their braces balanced). */
function withoutBlocks(css: string, open: RegExp): string {
  let out = '';
  let from = 0;
  for (const m of css.matchAll(open)) {
    if (m.index < from) continue;
    out += css.slice(from, m.index);
    let depth = 1;
    let i = m.index + m[0].length;
    for (; i < css.length && depth; i++) depth += css[i] === '{' ? 1 : css[i] === '}' ? -1 : 0;
    from = i;
  }
  return out + css.slice(from);
}

describe('generated site', () => {
  it('generates one page per language, and keeps Jekyll away from them', () => {
    for (const p of PAGES) expect(existsSync(new URL(p.file, OUT)), p.file).toBe(true);
    expect(existsSync(new URL('.nojekyll', OUT))).toBe(true);
  });
});

describe.each(PAGES)('generated site ($lang)', ({ lang, file, url, other }) => {
  const text = CATALOGS[lang];
  const says = SAYS[lang];
  const features = featuresFor(lang);
  const video = VIDEO[lang];
  let html = '';
  /** The page's styles: inlined, then linked. */
  let css = '';
  /** The content of the `<meta>` tag called `key`. */
  const meta = (key: string) => tags(headOf(html), 'meta').find((t) => t.name === key || t.property === key)?.content;

  beforeAll(() => {
    const index = new URL(file, OUT);
    if (!existsSync(index)) throw new Error('Génère d’abord le site : npm run generate');
    html = readFileSync(index, 'utf8');
    const inline = [...html.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]);
    const linked = [...html.matchAll(/<link[^>]*rel="stylesheet"[^>]*href="([^"]+)"/g)].map((m) =>
      readFileSync(new URL(m[1].slice(BASE.length), OUT), 'utf8'),
    );
    css = [...inline, ...linked].join('\n');
  });

  it('is named and described for search engines, in its language', () => {
    expect(html).toContain(`<title>${escapeText(text.meta.title)}</title>`);
    expect(html).toMatch(new RegExp(`<html[^>]*lang="${lang}"`));
    expect(meta('description')).toBe(text.meta.description);
    expect(text.meta.description.length).toBeGreaterThan(40);
  });

  it('is presented as such on social networks, with Open Graph and Twitter tags', () => {
    const image = `${SITE}${imageOf(lang, 'poster.jpg')}`;
    expect(meta('og:title')).toBe(text.meta.title);
    expect(meta('og:description')).toBe(text.meta.description);
    expect(meta('og:type')).toBe('website');
    expect(meta('og:url')).toBe(url);
    expect(meta('og:image')).toBe(image);
    expect(meta('og:site_name')).toBe('Escouade');
    expect(meta('og:locale')).toBe(text.meta.locale);
    expect(meta('og:locale:alternate')).toBe(CATALOGS[other].meta.locale);
    expect(meta('twitter:card')).toBe('summary_large_image');
    expect(meta('twitter:title')).toBe(text.meta.title);
    expect(meta('twitter:description')).toBe(text.meta.description);
    expect(meta('twitter:image')).toBe(image);
  });

  it('points search engines to both versions, and to itself as the one to index', () => {
    const links = tags(headOf(html), 'link');
    const alternates = links.filter((l) => l.rel === 'alternate' && l.hreflang);
    expect(alternates.map((l) => [l.hreflang, l.href]).sort()).toEqual(
      [
        ['fr', SITE],
        ['en', `${SITE}en/`],
        ['x-default', SITE],
      ].sort(),
    );
    expect(links.filter((l) => l.rel === 'canonical').map((l) => l.href)).toEqual([url]);
  });

  it('presents every feature, the install steps and the questions', () => {
    const texts = [
      ...features.map((f) => f.title),
      ...text.cards.map((c) => c.title),
      ...text.install.steps.map((s) => s.title),
      ...text.faq.items.map((f) => f.q),
    ];
    expect(texts.length).toBe(11 + 6 + 3 + 8);
    for (const t of texts) expect(html, t).toContain(escapeText(t));
    expect(html).toContain(says.affiliation);
  });

  it('says nothing in the other language', () => {
    const mine = new Set(leaves(text).values());
    let checked = 0;
    for (const [path, s] of leaves(CATALOGS[other])) {
      if (s.length < 15 || mine.has(s)) continue;
      expect(html, `${lang}: ${path}`).not.toContain(escapeText(s));
      checked++;
    }
    expect(checked).toBeGreaterThan(100);
  });

  it('goes by its own name, Escouade, and no longer by the old one', () => {
    expect(html).toContain('>Escouade<');
    expect(html).not.toContain('Claude Code Manager');
    expect(html).not.toMatch(/\bCCM\b/);
  });

  it('downloads the latest release, links to the code and shows the version', () => {
    expect(html).toContain('href="https://github.com/guillaume-gagnaire/escouade/releases/latest"');
    expect(html).toContain('href="https://github.com/guillaume-gagnaire/escouade"');
    expect(html).toContain(escapeText(fmt(text.hero.meta, { version: VERSION })));
    expect(html).toContain(`Version ${VERSION}`);
  });

  it('opens on a full-screen hero that says what Escouade is', () => {
    const h1 = html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/)?.[1] ?? '';
    expect(h1).toContain(text.hero.line1);
    expect(h1).toContain(text.hero.line2);
    expect(html).toContain(escapeText(text.hero.lede));
    expect(text.hero.lede.startsWith('Escouade, ')).toBe(true);
    expect(css).toMatch(/\.hero[^{]*\{[^}]*min-height:[^;}]*100svh/);
  });

  it('shows the squad at work before any script runs, and says its data is made up', () => {
    const still = squadAt(SNAPSHOT, [], lang);
    for (const a of still.agents) expect(html, a.name).toContain(escapeText(a.name));
    expect(html).toContain(escapeText(still.agents.find((a) => a.question)!.question!.text));
    const t = still.agents.find((a) => a.ticket)!.ticket!;
    expect(html).toContain(`${t.key} · ${fmt(text.demo.loop, { loop: t.loop, max: t.max })}`);
    for (const a of still.agents) if (a.result) expect(html, a.result).toContain(escapeText(a.result));
    expect(html).toContain(escapeText(text.demo.caption));
  });

  it('lets the visitor pause the demonstration', () => {
    const buttons = tags(html, 'button');
    expect(buttons.some((b) => b['aria-pressed'] && b['aria-label'] === text.demo.pause && /pause/i.test(b['aria-label']))).toBe(true);
  });

  it('keeps still for visitors who ask for less motion, and until its script runs', () => {
    // Every animation of the hero lives in a block for visitors who accept motion.
    const outside = withoutBlocks(css, /@media\s*\(prefers-reduced-motion:\s*no-preference\)\s*\{/g);
    for (const name of ['unfold', 'logo', 'square', 'spark', 'appear', 'rise', 'blink', 'pulse']) {
      // Minified: `animation:1.3s … unfold-<scope>`.
      const uses = new RegExp(`animation:[^;}]*\\b${name}(-[0-9a-f]{8})?[;}]`);
      expect(css, name).toMatch(uses);
      expect(outside, name).not.toMatch(uses);
    }
    // Before any script, the demo is still and its button says so.
    expect(html).toMatch(/<figure[^>]*class="[^"]*\bstill\b/);
    const pause = tags(html, 'button').find((b) => b['aria-label'] === text.demo.pause);
    expect(pause?.['aria-pressed']).toBe('true');
  });

  it('sends the first install step to Claude Code’s own documentation, in its language', () => {
    const link = text.install.steps[0].link!;
    expect(link.href).toContain(says.docs);
    expect(html).toContain(`href="${link.href}"`);
    expect(html).toContain(escapeText(link.label));
  });

  it('serves every local file under the GitHub Pages path', () => {
    const local = [...html.matchAll(/(?:src|href|poster)="(\/[^"]*)"/g)].map((m) => m[1]);
    expect(local.length).toBeGreaterThan(10);
    for (const ref of local) {
      expect(ref.startsWith(BASE), ref).toBe(true);
      const file = decodeURI(ref.slice(BASE.length).split(/[?#]/)[0]) || 'index.html';
      expect(existsSync(new URL(file, OUT)), ref).toBe(true);
    }
    for (const f of features) expect(html).toContain(`src="${BASE}${f.image}"`);
  });

  it('plays the presentation video on demand, with its poster', () => {
    expect(html).toContain(`src="${BASE}${video.src}"`);
    expect(html).toContain(`poster="${BASE}${imageOf(lang, 'poster.jpg')}"`);
    expect(html).toMatch(/<video[^>]*preload="none"/);
  });

  it('offers the subtitle tracks of its video, turned on from the player unless one is the default', () => {
    const tracks = tags(html, 'track');
    expect(tracks.map((t) => [t.kind, t.srclang, t.label, t.src])).toEqual(
      video.tracks.map((t) => ['captions', t.srclang, t.label, `${BASE}${t.src}`]),
    );
    const raw = html.match(/<track[^>]*>/g) ?? [];
    expect(raw.map((t) => /\sdefault[\s>=/]/.test(t))).toEqual(video.tracks.map((t) => !!t.default));
    for (const t of video.tracks) {
      const vtt = readFileSync(new URL(t.src, OUT), 'utf8');
      expect(vtt.startsWith('WEBVTT')).toBe(true);
    }
    // The voice over is French, and so are its subtitles for now.
    expect(readFileSync(new URL('escouade.vtt', OUT), 'utf8')).toContain(
      'Voici Escouade : le poste de pilotage de tous tes agents Claude Code.',
    );
  });

  it('presents the board, the test launch and the editor among the features', () => {
    const ids = features.map((f) => f.id);
    expect(ids).toEqual(expect.arrayContaining(['tableau', 'test', 'editeur']));
    expect(html).toContain(says.autopilot);
  });

  it('presents the imports from Jira, Trello and GitHub, kept in sync, among the features', () => {
    const f = features.find((x) => x.id === 'integrations');
    expect(f, 'integrations').toBeDefined();
    const said = [f!.title, f!.text, ...f!.points].join(' ');
    for (const service of ['Jira', 'Trello', 'GitHub']) expect(said).toContain(service);
    expect(said).toMatch(says.criteria);
    expect(said).toMatch(says.status);
    expect(html).toContain(`src="${BASE}${f!.image}"`);
  });

  it('calls the board by its name in the app, the Kanban', () => {
    expect(html).toContain('Kanban');
  });

  it.runIf(lang === 'fr')('never calls the Kanban a « tableau », except for a Trello board', () => {
    expect(html).not.toMatch(/\b(le|un|du) tableau\b(?! Trello)/i);
  });

  it('points the header’s links at sections that exist', () => {
    const header = html.match(/<header[\s\S]*?<\/header>/)?.[0] ?? '';
    const targets = tags(header, 'a')
      .map((a) => a.href)
      .filter((h) => h?.startsWith('#'));
    expect(targets).toEqual(['#top', '#video', `#${text.anchors.features}`, `#${text.anchors.install}`, '#faq']);
    for (const target of targets) expect(html, target).toContain(`id="${target.slice(1)}"`);
  });
});
