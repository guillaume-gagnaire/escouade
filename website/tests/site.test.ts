import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { beforeAll, describe, expect, it } from 'vitest';
import { CATALOGS, featuresFor } from '../app/data/catalogs';
import { fmt } from '../app/data/catalog';
import { DOC_GROUPS, DOC_SLUGS, docPath, docSource, type DocSlug } from '../app/data/docs';
import { LANG_KEY, type Lang } from '../app/data/language';
import { renderDoc } from '../app/data/render';
import { FEATURE_SHOTS, HERO_SHOT, NEXT, REPO, SITE, STAGES, SUPPORT, VIDEO, imageOf } from '../app/data/site';
import { SNAPSHOT, squadAt } from '../app/data/squad';
import { escapeText, headOf, leaves, tags } from './helpers';

const OUT = new URL('../.output/public/', import.meta.url);
const BASE = '/';
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
    // A Trello board is still a « tableau »: only the Kanban has a name of its own.
    board: /\b(le|un|du) tableau\b(?! Trello)/i,
    criteria: /critères d’acceptation/,
    status: /statut/,
    affiliation: 'non affilié à Anthropic',
  },
  en: {
    autopilot: 'Autopilot',
    // A Trello board is still a “board”: only the Kanban has a name of its own.
    board: /\b(the|a|your|each) board\b/i,
    criteria: /acceptance criteria/,
    status: /status/,
    affiliation: 'not affiliated with Anthropic',
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

describe('the roadmap', () => {
  it('announces a version that is not out yet: once it is, the roadmap says what comes after it', () => {
    const parts = (v: string) => v.split('.').map(Number);
    const [nextMajor, nextMinor] = parts(NEXT);
    const [major, minor] = parts(VERSION);
    expect(nextMajor > major || (nextMajor === major && nextMinor > minor), `${NEXT} after ${VERSION}`).toBe(true);
    expect(existsSync(new URL(`../../docs/TODO-${NEXT}.md`, import.meta.url)), `docs/TODO-${NEXT}.md`).toBe(true);
  });
});

describe('generated site', () => {
  it('generates one page per language', () => {
    for (const p of PAGES) expect(existsSync(new URL(p.file, OUT)), p.file).toBe(true);
  });

  it('generates the page the server shows for an unknown address', () => {
    const page = new URL('404.html', OUT);
    expect(existsSync(page)).toBe(true);
    expect(readFileSync(page, 'utf8')).toContain('<div id="__nuxt">');
  });
});

describe('the server', () => {
  const conf = readFileSync(new URL('../nginx.conf', import.meta.url), 'utf8');

  it('sends the former address and www to the same page of the site, for good', () => {
    const host = new URL(SITE).host;
    expect(conf).toContain(`server_name escouade.gagnaire.dev www.${host};`);
    expect(conf).toContain(`return 301 https://${host}$request_uri;`);
  });

  it('keeps no access log, in the redirect and in the site (the reverse proxy in front already logs)', () => {
    expect(conf.match(/^\s*access_log off;$/gm)).toHaveLength(2);
  });

  it('answers an unknown address with the generated 404 page', () => {
    expect(conf).toContain('error_page 404 /404.html;');
  });

  it('serves the copy of the store’s revocation list as JSON and its signature as text, asked again each time', () => {
    const block = (path: string) => conf.slice(conf.indexOf(`location = ${path} {`)).split(/\n {4}\}/)[0]!;
    expect(block('/revocations.json')).toContain('default_type application/json;');
    expect(block('/revocations.json.minisig')).toContain('default_type text/plain;');
    for (const path of ['/revocations.json', '/revocations.json.minisig'])
      expect(block(path)).toContain('add_header Cache-Control "public, no-cache";');
  });
});

/** What the browser makes of the French root’s inline script, on a visit with these languages, kept choice and address. */
function visit(script: string, v: { languages?: string[]; language?: string; stored?: string; search?: string; storageThrows?: boolean }) {
  const kept = new Map<string, string>(v.stored === undefined ? [] : [[LANG_KEY, v.stored]]);
  const moved: string[] = [];
  const context = {
    URLSearchParams,
    navigator: { languages: v.languages, language: v.language },
    location: { search: v.search ?? '', replace: (to: string) => void moved.push(to) },
    localStorage: {
      getItem: (k: string) => {
        if (v.storageThrows) throw new DOMException('The operation is insecure.', 'SecurityError');
        return kept.get(k) ?? null;
      },
      setItem: (k: string, value: string) => {
        if (v.storageThrows) throw new DOMException('The operation is insecure.', 'SecurityError');
        kept.set(k, value);
      },
    },
  };
  runInNewContext(script, context);
  return { moved, kept: kept.get(LANG_KEY) };
}

describe('the French root’s first visit', () => {
  let html = '';
  let english = '';
  beforeAll(() => {
    html = readFileSync(new URL('index.html', OUT), 'utf8');
    english = readFileSync(new URL('en/index.html', OUT), 'utf8');
  });
  /** The inline scripts (those without a `src`) of the page’s head. */
  const inline = (page: string) =>
    [...headOf(page).matchAll(/<script(?![^>]*\ssrc=)(?![^>]*type="(?:importmap|application\/json)")[^>]*>([\s\S]*?)<\/script>/g)].map(
      (m) => m[1],
    );
  const script = () => inline(html).find((s) => s.includes(LANG_KEY)) ?? '';

  it('is a small script in the French page’s head, and only there', () => {
    expect(script().length).toBeGreaterThan(100);
    expect(script().length).toBeLessThan(2000);
    expect(inline(english).filter((s) => s.includes(LANG_KEY))).toEqual([]);
    expect(english).not.toContain(LANG_KEY);
  });

  it('sends a browser with no French among its languages to the English page, under the base URL', () => {
    expect(visit(script(), { languages: ['en-US', 'en'] }).moved).toEqual([`${BASE}en/`]);
    expect(visit(script(), { languages: ['de-DE', 'es'] }).moved).toEqual([`${BASE}en/`]);
  });

  it('leaves a browser that has French among its languages where it is', () => {
    expect(visit(script(), { languages: ['fr-FR', 'en-US'] }).moved).toEqual([]);
    expect(visit(script(), { languages: ['en-US', 'fr'] }).moved).toEqual([]);
  });

  it('respects the language the visitor kept', () => {
    expect(visit(script(), { languages: ['en-US'], stored: 'fr' }).moved).toEqual([]);
    expect(visit(script(), { languages: ['fr-FR'], stored: 'en' }).moved).toEqual([`${BASE}en/`]);
    expect(visit(script(), { languages: ['en-US'], stored: 'klingon' }).moved).toEqual([`${BASE}en/`]);
  });

  it('respects ?lang=fr, and keeps it as the visitor’s choice', () => {
    const v = visit(script(), { languages: ['en-US'], search: '?lang=fr' });
    expect(v.moved).toEqual([]);
    expect(v.kept).toBe('fr');
    expect(visit(script(), { languages: ['en-US'], stored: 'en', search: '?utm=x&lang=fr' }).moved).toEqual([]);
  });

  it('goes to English, and remembers it, when ?lang=en asks for it', () => {
    const v = visit(script(), { languages: ['fr-FR'], search: '?lang=en' });
    expect(v.moved).toEqual([`${BASE}en/`]);
    expect(v.kept).toBe('en');
  });

  it('does not keep what the browser says as if the visitor had chosen it', () => {
    expect(visit(script(), { languages: ['en-US'] }).kept).toBeUndefined();
  });

  it('still works with a browser that only has its one language, or none, or no usable storage', () => {
    expect(visit(script(), { language: 'en-GB' }).moved).toEqual([`${BASE}en/`]);
    expect(visit(script(), { language: 'fr-FR' }).moved).toEqual([]);
    expect(visit(script(), {}).moved).toEqual([]);
    expect(visit(script(), { languages: ['en-US'], storageThrows: true }).moved).toEqual([`${BASE}en/`]);
    expect(visit(script(), { languages: ['en-US'], storageThrows: true, search: '?lang=fr' }).moved).toEqual([]);
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
      ...STAGES.flatMap((id) => text.features.stages[id].points.map((p) => p.title)),
      ...text.cards.map((c) => c.title),
      ...text.install.steps.map((s) => s.title),
      ...text.faq.items.map((f) => f.q),
    ];
    expect(texts.length).toBe(11 + 16 + 6 + 3 + 12);
    for (const t of texts) expect(html, t).toContain(escapeText(t));
    expect(html).toContain(says.affiliation);
  });

  it('says what comes next: the next version, what it brings, its detailed plan and what is already out', () => {
    const r = text.roadmap;
    expect(html).toContain(`id="${text.anchors.roadmap}"`);
    expect(tags(html, 'a').some((a) => a.href === `#${text.anchors.roadmap}`)).toBe(true);
    expect(r.items).toHaveLength(3);
    for (const s of [r.title, fmt(r.next, { version: NEXT }), ...r.items.flatMap((i) => [i.title, i.text])]) {
      expect(html, s).toContain(escapeText(s));
    }
    const hrefs = tags(html, 'a').map((a) => a.href);
    expect(hrefs).toContain(`${REPO}/blob/main/docs/TODO-${NEXT}.md`);
    expect(hrefs).toContain(`${REPO}/blob/main/${lang === 'en' ? 'CHANGELOG.en.md' : 'CHANGELOG.md'}`);
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

  it('opens on a hero that says what Escouade is, over the app’s own window', () => {
    const h1 = html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/)?.[1] ?? '';
    expect(h1).toContain(text.hero.line1);
    expect(h1).toContain(text.hero.line2);
    expect(html).toContain(escapeText(text.hero.lede));
    const start = html.indexOf('id="top"');
    const hero = html.slice(start, html.indexOf('</section>', start));
    expect(hero).toContain(`src="${BASE}${imageOf(lang, FEATURE_SHOTS.find((s) => s.id === HERO_SHOT)!.file)}"`);
  });

  it('tells the whole development cycle, step by step, from planning to steering', () => {
    const at = (s: string) => html.indexOf(escapeText(s));
    const cycle = html.match(/<ol class="cycle"[^>]*>([\s\S]*?)<\/ol>/)?.[1] ?? '';
    for (const id of STAGES) expect(cycle, id).toContain(escapeText(text.features.stages[id].title));
    for (const [i, id] of STAGES.entries()) {
      const stage = text.features.stages[id];
      expect(at(stage.text), id).toBeGreaterThan(0);
      expect(html).toContain(`${escapeText(stage.title)}</h3>`);
      if (i) expect(at(stage.text), `${id} after ${STAGES[i - 1]}`).toBeGreaterThan(at(text.features.stages[STAGES[i - 1]].text));
    }
    // Every step has something to show: its screenshots, or its points alone.
    for (const id of STAGES) expect(features.some((f) => f.stage === id) || text.features.stages[id].points.length >= 4, id).toBe(true);
    // The demonstration proves the last step, steering: it comes after it, before the install steps.
    const demo = html.indexOf('section signal');
    expect(demo).toBeGreaterThan(at(text.features.stages.steer.text));
    expect(demo).toBeLessThan(at(text.install.title));
  });

  it('says what Escouade is without leaning on the name of the engine it runs today', () => {
    const story = [
      text.meta.title,
      text.meta.description,
      text.hero.line1,
      text.hero.line2,
      text.hero.lede,
      text.features.title,
      text.features.lead,
      ...STAGES.flatMap((id) => [text.features.stages[id].title, text.features.stages[id].text]),
      text.close.title,
      text.docs.meta.description,
    ];
    for (const s of story) expect(s).not.toMatch(/Claude/);
    expect(text.meta.description).toContain('ADE');
  });

  it('shows the squad at work before any script runs, and says its data is made up', () => {
    const still = squadAt(SNAPSHOT, [], lang);
    for (const a of still.agents) expect(html, a.name).toContain(escapeText(a.name));
    expect(html).toContain(escapeText(still.agents.find((a) => a.question)!.question!.text));
    const t = still.agents.find((a) => a.ticket)!.ticket!;
    expect(html).toMatch(new RegExp(`>${t.key}</b>\\s*<span[^>]*>${fmt(text.demo.loop, { loop: t.loop, max: t.max })}<`));
    for (const a of still.agents) expect(html, a.name).toContain(`>${a.harness}</span>`);
    for (const a of still.agents) if (a.result) expect(html, a.result).toContain(escapeText(a.result));
    expect(html).toContain(escapeText(text.demo.caption));
  });

  it('lets the visitor pause the demonstration', () => {
    const buttons = tags(html, 'button');
    expect(buttons.some((b) => b['aria-pressed'] && b['aria-label'] === text.demo.pause && /pause/i.test(b['aria-label']))).toBe(true);
  });

  it('keeps still for visitors who ask for less motion, and until its script runs', () => {
    // Every animation of the page lives in a block for visitors who accept motion.
    const outside = withoutBlocks(css, /@media\s*\(prefers-reduced-motion:\s*no-preference\)\s*\{/g);
    for (const name of ['rise', 'lift', 'blink', 'pulse']) {
      // Minified: `animation:1.2s … blink-<scope>`.
      // The name comes last, or first when the timing holds a var() the minifier cannot reorder.
      const uses = new RegExp(`animation:[^;}]*\\b${name}(-[0-9a-f]{8})?(?=[\\s;}])`);
      expect(css, name).toMatch(uses);
      expect(outside, name).not.toMatch(uses);
    }
    // Before any script, the demo is still and its button says so.
    expect(html).toMatch(/<figure[^>]*class="[^"]*\bstill\b/);
    const pause = tags(html, 'button').find((b) => b['aria-label'] === text.demo.pause);
    expect(pause?.['aria-pressed']).toBe('true');
  });

  it('sends the first install step to the page that compares the harnesses, in its language', () => {
    const link = text.install.steps[0].link!;
    expect(link.href).toBe(docPath(lang, 'harness'));
    expect(html).toContain(`href="${BASE}${link.href.slice(1)}"`);
    expect(html).toContain(escapeText(link.label));
  });

  it('serves every local file from the site’s root', () => {
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

  it('shows the screenshots and the poster of its own language, and none of the other', () => {
    const dir = lang === 'fr' ? 'images/' : 'images/en/';
    for (const { file } of FEATURE_SHOTS) expect(html, file).toContain(`src="${BASE}${dir}${file}"`);
    expect(html).toContain(`poster="${BASE}${dir}poster.jpg"`);
    expect(meta('og:image')).toBe(`${SITE}${dir}poster.jpg`);
    expect(meta('twitter:image')).toBe(`${SITE}${dir}poster.jpg`);
    const images = [...html.matchAll(new RegExp(`(?:src|poster)="(${BASE}images/[^"]+\\.jpg)"`, 'g'))].map((m) => m[1]);
    // The hero shows the agents' screenshot too: each picture counts once.
    expect(new Set(images).size).toBe(FEATURE_SHOTS.length + 1);
    for (const image of images) expect(image.startsWith(`${BASE}${dir}`), image).toBe(true);
    // The app’s window in the pictures says what the page does: no French on the English page.
    if (lang === 'en') expect(html).not.toMatch(/images\/(?!en\/)[a-z]+\.jpg/);
  });

  it('turns on the subtitles of the language of the page, which the English page still offers in French', () => {
    // The French ones write the voice as it is said (captions); the English ones translate it (subtitles).
    const expected = {
      fr: [['captions', 'fr', 'Français', false]],
      en: [
        ['subtitles', 'en', 'English', true],
        ['captions', 'fr', 'Français', false],
      ],
    }[lang];
    const raw = html.match(/<track[^>]*>/g) ?? [];
    expect(
      raw.map((t) => [tags(t, 'track')[0].kind, tags(t, 'track')[0].srclang, tags(t, 'track')[0].label, /\sdefault[\s>=/]/.test(t)]),
    ).toEqual(expected);
    if (lang === 'en') expect(tags(html, 'track').map((t) => t.src)).toEqual([`${BASE}escouade.en.vtt`, `${BASE}escouade.vtt`]);
  });

  it('offers the subtitle tracks of its video, turned on from the player unless one is the default', () => {
    const tracks = tags(html, 'track');
    expect(tracks.map((t) => [t.kind, t.srclang, t.label, t.src])).toEqual(
      video.tracks.map((t) => [t.kind, t.srclang, t.label, `${BASE}${t.src}`]),
    );
    const raw = html.match(/<track[^>]*>/g) ?? [];
    expect(raw.map((t) => /\sdefault[\s>=/]/.test(t))).toEqual(video.tracks.map((t) => !!t.default));
    for (const t of video.tracks) {
      const vtt = readFileSync(new URL(t.src, OUT), 'utf8');
      expect(vtt.startsWith('WEBVTT')).toBe(true);
    }
    // The voice over is French; the French subtitles say it, the English ones translate it.
    expect(readFileSync(new URL('escouade.vtt', OUT), 'utf8')).toContain(
      'Voici Escouade : le poste de pilotage de tous tes agents Claude Code.',
    );
    expect(readFileSync(new URL('escouade.en.vtt', OUT), 'utf8')).toContain('Meet Escouade: the cockpit for all your Claude Code agents.');
  });

  it('presents the board, the test launch and the editor among the features', () => {
    const ids = features.map((f) => f.id);
    expect(ids).toEqual(expect.arrayContaining(['tableau', 'test', 'editeur']));
    expect(html).toContain(says.autopilot);
  });

  it('presents the imports from Jira, Trello and GitHub, kept in sync, among the features', () => {
    const f = features.find((x) => x.id === 'integrations');
    expect(f, 'integrations').toBeDefined();
    const said = [f!.title, f!.text].join(' ');
    for (const service of ['Jira', 'Trello', 'GitHub']) expect(said).toContain(service);
    expect(said).toMatch(says.criteria);
    expect(said).toMatch(says.status);
    expect(html).toContain(`src="${BASE}${f!.image}"`);
  });

  it('calls the board by its name in the app, the Kanban', () => {
    expect(html).toContain('Kanban');
  });

  it('never calls the Kanban a board, except for a Trello board', () => {
    expect(html).not.toMatch(says.board);
  });

  it('lets the visitor switch language with real links, and marks the language shown', () => {
    const header = html.match(/<header[\s\S]*?<\/header>/)?.[0] ?? '';
    const nav = header.match(/<nav[^>]*class="lang"[^>]*>[\s\S]*?<\/nav>/)?.[0] ?? '';
    expect(tags(nav, 'nav')[0]['aria-label']).toBe(text.header.language);
    const links = tags(nav, 'a');
    // The French link asks for French explicitly, so that the first visit’s redirection leaves it alone.
    expect(links.map((a) => [a.lang, a.hreflang, a.href, a.title])).toEqual([
      ['fr', 'fr', `${BASE}?lang=fr`, 'Français'],
      ['en', 'en', `${BASE}en/`, 'English'],
    ]);
    expect(links.map((a) => a['aria-current'])).toEqual(lang === 'fr' ? ['page', undefined] : [undefined, 'page']);
    expect([...nav.matchAll(/<a[^>]*>([^<]*)<\/a>/g)].map((m) => m[1])).toEqual(['FR', 'EN']);
  });

  it('leads to the documentation in its language, from the header and from the footer', () => {
    const header = html.match(/<header[\s\S]*?<\/header>/)?.[0] ?? '';
    const footer = html.match(/<footer class="foot[\s\S]*?<\/footer>/)?.[0] ?? '';
    for (const part of [header, footer]) {
      const link = tags(part, 'a').find((a) => a.href === `${BASE}${docPath(lang).slice(1)}`);
      expect(link, part.slice(0, 80)).toBeDefined();
    }
    expect(header).toContain(`>${escapeText(text.header.docs)}</a>`);
  });

  it('points the header’s links at sections that exist', () => {
    const header = html.match(/<header[\s\S]*?<\/header>/)?.[0] ?? '';
    const targets = tags(header, 'a')
      .map((a) => a.href)
      .filter((h) => h?.startsWith('#'));
    expect(targets).toEqual([
      '#top',
      '#video',
      `#${text.anchors.features}`,
      `#${text.anchors.install}`,
      `#${text.anchors.roadmap}`,
      '#faq',
    ]);
    for (const target of targets) expect(html, target).toContain(`id="${target.slice(1)}"`);
  });
});

/** The pages of the documentation the site generates, in both languages. */
const DOC_PAGES = (['fr', 'en'] as const).flatMap((lang) => [
  { lang, slug: undefined, other: lang === 'fr' ? 'en' : 'fr' },
  ...DOC_SLUGS.map((slug) => ({ lang, slug, other: lang === 'fr' ? 'en' : 'fr' })),
]) as { lang: Lang; slug?: DocSlug; other: Lang }[];

describe.each(DOC_PAGES)('the documentation ($lang, $slug)', ({ lang, slug, other }) => {
  const text = CATALOGS[lang];
  const path = docPath(lang, slug).slice(1);
  let html = '';
  const meta = (key: string) => tags(headOf(html), 'meta').find((t) => t.name === key || t.property === key)?.content;
  const links = (kind: string) => tags(headOf(html), 'link').filter((l) => l.rel === kind);
  /** The page's own title and lede, from its Markdown; the home's come from the catalog. */
  const page = () => {
    if (!slug) return { title: text.docs.meta.title, description: text.docs.meta.description };
    const source = readFileSync(new URL(`../${docSource(lang, slug)}`, import.meta.url), 'utf8');
    const doc = renderDoc(source, { lang, link: (p) => p, callouts: text.docs.callouts });
    return { title: fmt(text.docs.pageTitle, { title: doc.title }), description: doc.lede, h1: doc.title };
  };

  beforeAll(() => {
    const file = new URL(`${path}index.html`, OUT);
    if (!existsSync(file)) throw new Error(`${path}index.html : génère d’abord le site (npm run generate)`);
    html = readFileSync(file, 'utf8');
  });

  it('is named and described for search engines, in its language', () => {
    const p = page();
    expect(html).toContain(`<title>${escapeText(p.title)}</title>`);
    expect(html).toMatch(new RegExp(`<html[^>]*lang="${lang}"`));
    expect(meta('description')).toBe(p.description);
    expect(meta('og:title')).toBe(p.title);
    expect(meta('og:url')).toBe(`${SITE}${path}`);
    if (p.h1) expect(html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/)?.[1]).toBe(escapeText(p.h1));
  });

  it('points search engines to itself and to its version in the other language', () => {
    expect(links('canonical').map((l) => l.href)).toEqual([`${SITE}${path}`]);
    const alternates = Object.fromEntries(links('alternate').map((l) => [l.hreflang, l.href]));
    expect(alternates).toEqual({
      fr: `${SITE}${docPath('fr', slug).slice(1)}`,
      en: `${SITE}${docPath('en', slug).slice(1)}`,
      'x-default': `${SITE}${docPath('fr', slug).slice(1)}`,
    });
  });

  it('lists every page in its sidebar, by group, and marks the one shown', () => {
    const nav = html.match(/<nav id="docs-nav"[\s\S]*?<\/nav>/)?.[0] ?? '';
    expect(tags(nav, 'nav')[0]['aria-label']).toBe(text.docs.nav);
    const a = tags(nav, 'a');
    expect(a.map((l) => l.href)).toEqual(DOC_SLUGS.map((s) => `${BASE}${docPath(lang, s).slice(1)}`));
    for (const g of DOC_GROUPS) expect(nav).toContain(escapeText(text.docs.groups[g.id].title));
    expect(a.filter((l) => l['aria-current'] === 'page').map((l) => l.href)).toEqual(slug ? [`${BASE}${path}`] : []);
  });

  it('switches language to the same page', () => {
    const nav = html.match(/<nav[^>]*class="lang"[^>]*>[\s\S]*?<\/nav>/)?.[0] ?? '';
    expect(tags(nav, 'a').map((l) => l.href)).toEqual([
      `${BASE}${docPath('fr', slug).slice(1)}?lang=fr`,
      `${BASE}${docPath('en', slug).slice(1)}`,
    ]);
  });

  it('sends a first visit from a browser without French to the English page, from the French one only', () => {
    const redirect = headOf(html).includes(LANG_KEY);
    expect(redirect).toBe(lang === 'fr');
    if (redirect) expect(headOf(html)).toContain(`location.replace(${JSON.stringify(`${BASE}${docPath('en', slug).slice(1)}`)})`);
  });

  it('serves every local file and links to pages that exist', () => {
    const local = [...html.matchAll(/(?:src|href)="(\/[^"]*)"/g)].map((m) => m[1]);
    expect(local.length).toBeGreaterThan(DOC_SLUGS.length);
    for (const ref of local) {
      const file = decodeURI(ref.slice(BASE.length).split(/[?#]/)[0]);
      expect(existsSync(new URL(file.endsWith('/') || !file ? `${file}index.html` : file, OUT)), ref).toBe(true);
    }
  });

  it('says where to edit it, and leads to the pages before and after it', () => {
    if (!slug) {
      for (const s of DOC_SLUGS) expect(html, s).toContain(`href="${BASE}${docPath(lang, s).slice(1)}"`);
      return;
    }
    expect(html).toContain(`href="${REPO}/edit/main/website/${docSource(lang, slug)}"`);
    const at = DOC_SLUGS.indexOf(slug);
    const pager = html.slice(html.indexOf('class="pager"'));
    for (const s of [DOC_SLUGS[at - 1], DOC_SLUGS[at + 1]].filter(Boolean)) {
      expect(pager, s).toContain(`href="${BASE}${docPath(lang, s).slice(1)}"`);
    }
  });

  it('calls the documentation by its name in the header, and links back to the site’s home', () => {
    const header = html.match(/<header[\s\S]*?<\/header>/)?.[0] ?? '';
    expect(header).toContain(`href="${BASE}${lang === 'en' ? 'en/' : ''}"`);
    expect(header).toContain(escapeText(text.docs.title));
    expect(header).toContain(escapeText(text.docs.search.button));
    expect(html).not.toContain(escapeText(CATALOGS[other].docs.toc));
  });
});

describe('the page to offer a coffee', () => {
  /** Every page the site generates, but the empty one the server shows for an unknown address. */
  const pages = () =>
    (readdirSync(OUT, { recursive: true }) as string[])
      .map((f) => f.replaceAll('\\', '/'))
      .filter((f) => f.endsWith('.html') && !/^(404|200)\.html$/.test(f));
  const langOf = (html: string) => html.match(/<html[^>]*lang="(\w+)"/)?.[1] as Lang;
  const stripe = (part: string) =>
    tags(part, 'a')
      .filter((a) => a.href?.startsWith('https://buy.stripe.com/'))
      .map((a) => a.href);

  beforeAll(() => {
    if (!existsSync(new URL('index.html', OUT))) throw new Error('Génère d’abord le site : npm run generate');
  });

  it('is in the footer of every page, the one of its language', () => {
    const all = pages();
    expect(all.length).toBeGreaterThan(80);
    for (const file of all) {
      const html = readFileSync(new URL(file, OUT), 'utf8');
      const footer = html.match(/<footer class="foot[\s\S]*?<\/footer>/)?.[0];
      expect(footer, file).toBeDefined();
      expect(stripe(footer!), file).toEqual([SUPPORT[langOf(html)]]);
      expect(langOf(html), file).toBe(file.startsWith('en/') ? 'en' : 'fr');
    }
  });

  it.each(PAGES)('is in the answer of the FAQ that says it is free ($lang)', ({ lang, file }) => {
    const html = readFileSync(new URL(file, OUT), 'utf8');
    const faq = html.match(/<section id="faq"[\s\S]*?<\/section>/)?.[0] ?? '';
    expect(stripe(faq)).toEqual([SUPPORT[lang]]);
    // The answer that says it is free and open source, and that one alone.
    const linked = CATALOGS[lang].faq.items.filter((q) => q.link);
    expect(linked.map((q) => q.link!.href)).toEqual([SUPPORT[lang]]);
    expect(linked[0].a).toContain('GPL-3.0');
  });

  it.each(['fr', 'en'] as const)('has its section in the introduction of the documentation (%s)', (lang) => {
    const html = readFileSync(new URL(`${docPath(lang, 'introduction').slice(1)}index.html`, OUT), 'utf8');
    const article = html.split(/<footer class="foot/)[0];
    expect(article).toContain(lang === 'fr' ? 'id="soutenir-escouade"' : 'id="support-escouade"');
    expect(stripe(article)).toEqual([SUPPORT[lang]]);
  });

  it('is written in site.ts alone: the pages and the documentation name it support:', () => {
    const SOURCES = ['app/', 'content/'].map((d) => new URL(`../${d}`, import.meta.url));
    for (const dir of SOURCES)
      for (const f of readdirSync(dir, { recursive: true }) as string[]) {
        const path = f.replaceAll('\\', '/');
        if (!/\.(ts|vue|md)$/.test(path) || path === 'data/site.ts') continue;
        expect(readFileSync(new URL(path, dir), 'utf8'), path).not.toContain('buy.stripe.com');
      }
  });
});
