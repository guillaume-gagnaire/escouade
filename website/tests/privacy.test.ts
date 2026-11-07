import { existsSync, readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { CATALOGS } from '../app/data/catalogs';
import { DOC_SLUGS } from '../app/data/docs';
import { LANG_KEY, pathIn, type Lang } from '../app/data/language';
import { privacyPath, privacySource } from '../app/data/privacy';
import { renderDoc } from '../app/data/render';
import { REPO, SITE } from '../app/data/site';
import { escapeText, headOf, tags } from './helpers';

const OUT = new URL('../.output/public/', import.meta.url);
const BASE = '/';
const LANGS: Lang[] = ['fr', 'en'];

/** Every event the app sends, with the props it adds (the app’s own list: src-tauri/src/telemetry.rs). */
const EVENTS = [
  'app_started',
  'app_active',
  'project_created',
  'agent_created',
  'message_sent',
  'ticket_created',
  'ticket_started',
  'ticket_approved',
  'committed',
  'merged',
  'git_sync',
  'branch_created',
  'terminal_opened',
  'launch_started',
  'test_run_started',
  'claude_code_view',
  'file_saved',
  'code_searched',
  'mcp_enabled',
  'integration_connected',
  'tickets_imported',
  'account_created',
  'remote_control_enabled',
  'dictation_used',
  'voice_conversation',
  'plugin_installed',
  'pr_opened',
  'pr_merged',
  'ci_fix_started',
  'pr_review_started',
];
/** The values some props take, written on the page as `value`. */
const VALUES = [
  'user',
  'autopilot',
  'fetch',
  'pull',
  'push',
  'jira',
  'trello',
  'github',
  'linear',
  'gitlab',
  'clickup',
  'azure',
  'bitbucket',
  'gitea',
  'other',
  'agent',
  'ticket',
  'project',
  'global',
];
const PROPS = ['worktree', 'by', 'op', 'service', 'from', 'from_store', 'forge'];

const source = (lang: Lang) => readFileSync(new URL(`../${privacySource(lang)}`, import.meta.url), 'utf8');
const render = (lang: Lang) => renderDoc(source(lang), { lang, link: (p) => p, callouts: CATALOGS[lang].docs.callouts });

describe('the addresses of the privacy page', () => {
  it('are the same page in both languages', () => {
    expect(privacyPath('fr')).toBe('/privacy/');
    expect(privacyPath('en')).toBe('/en/privacy/');
    expect(pathIn('en', '/privacy/')).toBe('/en/privacy/');
    expect(pathIn('fr', '/en/privacy/')).toBe('/privacy/');
  });
});

describe('the privacy page’s Markdown', () => {
  it('has a title and a lede in both languages', () => {
    expect(render('fr').title).toBe('Confidentialité');
    expect(render('en').title).toBe('Privacy');
    for (const lang of LANGS) expect(render(lang).lede.length, lang).toBeGreaterThan(40);
  });

  it('has the same sections, links and anchors in both languages', () => {
    const [fr, en] = LANGS.map(render);
    expect(en.headings.map((h) => h.depth)).toEqual(fr.headings.map((h) => h.depth));
    expect(en.links.map((l) => l.slug)).toEqual(fr.links.map((l) => l.slug));
  });

  it('links only to documentation pages and sections that exist', () => {
    for (const lang of LANGS) {
      const doc = render(lang);
      for (const l of doc.links) expect(DOC_SLUGS, `${lang} → ${l.slug}`).toContain(l.slug);
      for (const a of doc.anchors)
        expect(
          doc.headings.map((h) => h.id),
          `${lang} → #${a}`,
        ).toContain(a);
    }
  });

  it('names every event the app sends, and the props it adds', () => {
    for (const lang of LANGS) {
      const text = source(lang);
      for (const e of EVENTS) expect(text, `${lang}: ${e}`).toContain(`\`${e}\``);
      for (const p of PROPS) expect(text, `${lang}: ${p}`).toContain(`\`${p}\``);
      for (const v of VALUES) expect(text, `${lang}: ${v}`).toContain(`\`${v}\``);
    }
  });

  it('lists exactly the events of the app’s code', () => {
    const code = readFileSync(new URL('../../src-tauri/src/telemetry.rs', import.meta.url), 'utf8');
    const names = [...code.matchAll(/Event::\w+(?: \{ \.\. \})? => "(\w+)",/g)].map((m) => m[1]);
    expect(names.length).toBeGreaterThan(20);
    expect([...names].sort()).toEqual([...EVENTS].sort());
    for (const lang of LANGS) {
      const listed = [...source(lang).matchAll(/^\| `(\w+)` \|/gm)].map((m) => m[1]);
      expect([...listed].sort(), lang).toEqual([...names].sort());
    }
  });

  it('says that nothing of the code hosts goes to Escouade, and where the tokens and the calls go', () => {
    for (const [lang, facts] of [
      [
        'fr',
        [
          '**rien ne part vers les serveurs d’Escouade**',
          '**Tes jetons restent sur ta machine**',
          '**Les appels vont directement de l’app à l’hébergeur**',
        ],
      ],
      [
        'en',
        [
          '**nothing goes to Escouade’s servers**',
          '**Your tokens stay on your machine**',
          '**The calls go straight from the app to the host**',
        ],
      ],
    ] as const)
      for (const fact of facts) expect(source(lang), lang).toContain(fact);
  });

  it('names the file that holds the install id, the server and the source code', () => {
    for (const lang of LANGS) {
      const text = source(lang);
      expect(text).toContain('~/.escouade/telemetry-id');
      expect(text).toContain('aptabase.gagnaire.dev');
      expect(text).toContain('https://aptabase.gagnaire.dev/api/v0/events');
      expect(text).toContain(`${REPO}/blob/main/src-tauri/src/telemetry.rs`);
      expect(text).toContain(`${REPO}/issues`);
      expect(text).toContain('ESCOUADE_DATA_DIR');
      expect(text).toContain('escouade-lang');
    }
  });

  it('claims no anonymity, and says the data is pseudonymous personal data', () => {
    for (const lang of LANGS) {
      const text = source(lang);
      expect(text, lang).not.toMatch(/anonym/i);
      expect(text, lang).toMatch(lang === 'fr' ? /pseudonymes/ : /pseudonymous personal data/);
    }
  });

  it('gives the controller, the legal basis, the recipients, the server logs, the rights and the CNIL', () => {
    for (const lang of LANGS) {
      const text = source(lang).replace(/ /g, ' ');
      for (const fact of [
        'GAGNAIRE DEVELOPPEMENT, SAS, 2 rue du Taillis, 33620 Cavignac',
        '(mailto:guillaume@gagnaire.dev)',
        '6.1.a',
        'article 82',
        '6.1.f',
        'Scaleway SAS',
        'Traefik',
        'User-Agent',
        'https://www.cnil.fr/fr/plaintes',
        '3 place de Fontenoy, TSA 80715, 75334 Paris Cedex 07',
      ])
        expect(text, `${lang}: ${fact}`).toContain(fact);
      expect(text, lang).toContain(lang === 'fr' ? '**consentement**' : '**consent**');
      expect(text, lang).toContain(lang === 'fr' ? '25 mois' : '25 months');
      expect(text, lang).not.toMatch(/182|5 ans|5 years/);
      // The proxy's access logs: one year at most.
      expect(text, lang).toMatch(lang === 'fr' ? /un an au plus/ : /one year at most/);
      expect(text, lang).not.toMatch(/20 Mo|20 MB/);
    }
  });

  it('writes the typography the way the documentation does', () => {
    const prose = (s: string) => s.replace(/```[\s\S]*?```/g, '').replace(/`[^`\n]*`/g, '');
    for (const lang of LANGS) {
      const text = prose(source(lang));
      expect(text, lang).not.toMatch(/['"]/);
      expect(text, lang).not.toContain('...');
      expect(source(lang), lang).not.toMatch(/^#{4,} /m);
      if (lang === 'fr') expect(text).not.toMatch(/«(?! )|(?<! )»/);
      else expect(text).not.toMatch(/[«»]/);
    }
  });
});

describe.each(LANGS)('the privacy page (%s)', (lang) => {
  const other: Lang = lang === 'fr' ? 'en' : 'fr';
  const text = CATALOGS[lang];
  const path = privacyPath(lang).slice(1);
  let html = '';
  const meta = (key: string) => tags(headOf(html), 'meta').find((t) => t.name === key || t.property === key)?.content;
  const links = (kind: string) => tags(headOf(html), 'link').filter((l) => l.rel === kind);

  beforeAll(() => {
    const file = new URL(`${path}index.html`, OUT);
    if (!existsSync(file)) throw new Error(`${path}index.html : génère d’abord le site (npm run generate)`);
    html = readFileSync(file, 'utf8');
  });

  it('is titled and described for search engines, in its language', () => {
    const doc = render(lang);
    expect(html).toContain(`<title>${escapeText(`${doc.title} · Escouade`)}</title>`);
    expect(html).toMatch(new RegExp(`<html[^>]*lang="${lang}"`));
    expect(meta('description')).toBe(doc.lede);
    expect(meta('og:title')).toBe(`${doc.title} · Escouade`);
    expect(meta('og:url')).toBe(`${SITE}${path}`);
    expect(html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/)?.[1]).toBe(escapeText(doc.title));
  });

  it('points search engines to itself and to its version in the other language', () => {
    expect(links('canonical').map((l) => l.href)).toEqual([`${SITE}${path}`]);
    expect(Object.fromEntries(links('alternate').map((l) => [l.hreflang, l.href]))).toEqual({
      fr: `${SITE}privacy/`,
      en: `${SITE}en/privacy/`,
      'x-default': `${SITE}privacy/`,
    });
  });

  it('shows its content: the events, the system, and the links to the documentation', () => {
    for (const e of EVENTS) expect(html, e).toMatch(new RegExp(`<code[^>]*>${e}</code>`));
    expect(render(lang).headings.some((h) => h.depth === 2 && /jamais|never/.test(h.text))).toBe(true);
    const hrefs = tags(html.match(/<main[\s\S]*<\/main>/)?.[0] ?? '', 'a').map((a) => a.href);
    expect(hrefs).toContain(`${BASE}${lang === 'en' ? 'en/' : ''}docs/settings-about/`);
    expect(hrefs).toContain(`${REPO}/blob/main/src-tauri/src/telemetry.rs`);
    expect(html).toContain(`href="${REPO}/edit/main/website/${privacySource(lang)}"`);
  });

  it('switches language to the same page', () => {
    const nav = html.match(/<nav[^>]*class="lang"[^>]*>[\s\S]*?<\/nav>/)?.[0] ?? '';
    expect(tags(nav, 'a').map((l) => l.href)).toEqual([`${BASE}privacy/?lang=fr`, `${BASE}en/privacy/`]);
    expect(tags(nav, 'a').map((l) => l['aria-current'])).toEqual(lang === 'fr' ? ['page', undefined] : [undefined, 'page']);
  });

  it('sends a first visit from a browser without French to the English page, from the French one only', () => {
    const redirect = headOf(html).includes(LANG_KEY);
    expect(redirect).toBe(lang === 'fr');
    if (redirect) expect(headOf(html)).toContain(`location.replace(${JSON.stringify(`${BASE}en/privacy/`)})`);
  });

  it('has the site’s header and footer, and no cookie or analytics script', () => {
    const header = html.match(/<header[\s\S]*?<\/header>/)?.[0] ?? '';
    expect(tags(header, 'a').find((a) => a.href === `${BASE}${lang === 'en' ? 'en/' : ''}`)).toBeDefined();
    expect(html).toContain('<footer class="foot');
    expect(html).not.toMatch(/document\.cookie|gtag\(|plausible|google-analytics|matomo|aptabase\.js/i);
  });

  it('serves every local file and links to pages that exist', () => {
    const local = [...html.matchAll(/(?:src|href)="(\/[^"]*)"/g)].map((m) => m[1]);
    expect(local.length).toBeGreaterThan(10);
    for (const ref of local) {
      const file = decodeURI(ref.slice(BASE.length).split(/[?#]/)[0]);
      expect(existsSync(new URL(file.endsWith('/') || !file ? `${file}index.html` : file, OUT)), ref).toBe(true);
    }
  });

  it('is linked from the footer of the home page, in its language', () => {
    const home = readFileSync(new URL(lang === 'en' ? 'en/index.html' : 'index.html', OUT), 'utf8');
    const footer = home.match(/<footer class="foot[\s\S]*?<\/footer>/)?.[0] ?? '';
    const link = tags(footer, 'a').find((a) => a.href === `${BASE}${path}`);
    expect(link).toBeDefined();
    expect(footer).toContain(`>${escapeText(text.footer.privacy)}</a>`);
    expect(footer).not.toContain(escapeText(CATALOGS[other].footer.privacy));
  });
});

describe('the rest of the site', () => {
  it('links to the privacy page from the footer and the text of the pages of the documentation about the statistics, in its language', () => {
    for (const lang of LANGS) {
      for (const slug of ['settings-about', 'data', 'installation']) {
        const html = readFileSync(new URL(`${lang === 'en' ? 'en/' : ''}docs/${slug}/index.html`, OUT), 'utf8');
        const footer = html.match(/<footer class="foot[\s\S]*?<\/footer>/)?.[0] ?? '';
        expect(
          tags(footer, 'a').map((a) => a.href),
          `${lang}/${slug}`,
        ).toContain(`${BASE}${privacyPath(lang).slice(1)}`);
        const main = html.match(/<main[\s\S]*<\/main>/)?.[0] ?? '';
        expect(
          tags(main, 'a').map((a) => a.href),
          `${lang}/${slug}`,
        ).toContain(`${BASE}${privacyPath(lang).slice(1)}`);
      }
    }
  });

  it('documents the statistics in the “About” settings and the local data, in both languages', () => {
    for (const lang of LANGS) {
      const about = readFileSync(new URL(`../content/docs/${lang}/settings-about.md`, import.meta.url), 'utf8');
      const data = readFileSync(new URL(`../content/docs/${lang}/data.md`, import.meta.url), 'utf8');
      expect(about, lang).toContain(lang === 'fr' ? 'Statistiques d’usage' : 'Usage statistics');
      expect(about, lang).toContain(lang === 'fr' ? 'rien qui t’identifie' : 'nothing that identifies you');
      for (const doc of [about, data]) expect(doc, lang).not.toMatch(/anonym/i);
      expect(data, lang).toContain('`telemetry-id`');
    }
  });
});
