import { existsSync, readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { CATALOGS, featuresFor } from '../app/data/catalogs';
import { SNAPSHOT, squadAt } from '../app/data/squad';

const {
  cards: CARDS,
  faq: { items: FAQ },
  install: { steps: STEPS },
} = CATALOGS.fr;
const FEATURES = featuresFor('fr');

const OUT = new URL('../.output/public/', import.meta.url);
const BASE = '/escouade/';
const VERSION = (JSON.parse(readFileSync(new URL('../../src-tauri/tauri.conf.json', import.meta.url), 'utf8')) as { version: string })
  .version;

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

/** Text as Vue writes it in HTML. */
const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

let html = '';
/** The page's styles: inlined, then linked. */
let css = '';
beforeAll(() => {
  const index = new URL('index.html', OUT);
  if (!existsSync(index)) throw new Error('Génère d’abord le site : npm run generate');
  html = readFileSync(index, 'utf8');
  const inline = [...html.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]);
  const linked = [...html.matchAll(/<link[^>]*rel="stylesheet"[^>]*href="([^"]+)"/g)].map((m) =>
    readFileSync(new URL(m[1].slice(BASE.length), OUT), 'utf8'),
  );
  css = [...inline, ...linked].join('\n');
});

describe('generated site', () => {
  it('is named and described for search engines and social networks', () => {
    expect(html).toContain('<title>Escouade — le poste de pilotage de tes agents Claude Code</title>');
    expect(html).toMatch(/<html[^>]*lang="fr"/);
    expect(html).toMatch(/<meta[^>]*name="description"[^>]*content="[^"]{40,}"/);
    expect(html).toMatch(/<meta[^>]*og:image[^>]*content="https:\/\/guillaume-gagnaire\.github\.io\/escouade\/images\/poster\.jpg"/);
  });

  it('presents every feature, the install steps and the questions', () => {
    const texts = [...FEATURES.map((f) => f.title), ...CARDS.map((c) => c.title), ...STEPS.map((s) => s.title), ...FAQ.map((f) => f.q)];
    for (const t of texts) expect(html, t).toContain(escape(t));
    expect(html).toContain('non affilié à Anthropic');
  });

  it('goes by its own name, Escouade, and no longer by the old one', () => {
    expect(html).toContain('>Escouade<');
    expect(html).not.toContain('Claude Code Manager');
    expect(html).not.toMatch(/\bCCM\b/);
  });

  it('downloads the latest release, links to the code and shows the version', () => {
    expect(html).toContain('href="https://github.com/guillaume-gagnaire/escouade/releases/latest"');
    expect(html).toContain('href="https://github.com/guillaume-gagnaire/escouade"');
    expect(html).toContain(`Version ${VERSION}`);
  });

  it('opens on a full-screen hero that says what Escouade is', () => {
    expect(html).toMatch(/<h1[^>]*>[\s\S]*?Une escouade de Claude\.[\s\S]*?Une seule fenêtre\.[\s\S]*?<\/h1>/);
    expect(html).toContain('Escouade, le poste de pilotage de tes agents Claude Code');
    expect(css).toMatch(/\.hero[^{]*\{[^}]*min-height:[^;}]*100svh/);
  });

  it('shows the squad at work before any script runs, and says its data is made up', () => {
    const still = squadAt(SNAPSHOT);
    for (const a of still.agents) expect(html, a.name).toContain(escape(a.name));
    expect(html).toContain(escape(still.agents.find((a) => a.question)!.question!.text));
    const t = still.agents.find((a) => a.ticket)!.ticket!;
    expect(html).toContain(`${t.key} · boucle ${t.loop}/${t.max}`);
    expect(html).toContain('données fictives');
  });

  it('lets the visitor pause the demonstration', () => {
    const buttons = [...html.matchAll(/<button[^>]*>/g)].map((m) => m[0]);
    expect(buttons.some((b) => /aria-pressed="(true|false)"/.test(b) && /aria-label="[^"]*pause/i.test(b))).toBe(true);
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
    const pause = [...html.matchAll(/<button[^>]*>/g)].map((m) => m[0]).find((b) => /aria-label="[^"]*pause/i.test(b));
    expect(pause).toContain('aria-pressed="true"');
  });

  it('sends the first install step to Claude Code’s own documentation', () => {
    expect(html).toContain('href="https://code.claude.com/docs/fr/overview"');
  });

  it('serves every local file under the GitHub Pages path', () => {
    const local = [...html.matchAll(/(?:src|href|poster)="(\/[^"]*)"/g)].map((m) => m[1]);
    expect(local.length).toBeGreaterThan(10);
    for (const ref of local) {
      expect(ref.startsWith(BASE), ref).toBe(true);
      const file = decodeURI(ref.slice(BASE.length).split(/[?#]/)[0]) || 'index.html';
      expect(existsSync(new URL(file, OUT)), ref).toBe(true);
    }
    for (const f of FEATURES) expect(html).toContain(`src="${BASE}${f.image}"`);
  });

  it('plays the presentation video on demand, with its poster', () => {
    expect(html).toContain(`src="${BASE}escouade.mp4"`);
    expect(html).toContain(`poster="${BASE}images/poster.jpg"`);
    expect(html).toMatch(/<video[^>]*preload="none"/);
  });

  it('subtitles the voice over in French, turned on from the player', () => {
    const track = html.match(/<track[^>]*>/)?.[0] ?? '';
    expect(track).toContain('kind="captions"');
    expect(track).toContain('srclang="fr"');
    expect(track).toContain(`src="${BASE}escouade.vtt"`);
    expect(track).not.toMatch(/\sdefault[\s>=/]/);
    const vtt = readFileSync(new URL('escouade.vtt', OUT), 'utf8');
    expect(vtt.startsWith('WEBVTT')).toBe(true);
    expect(vtt).toContain('Voici Escouade : le poste de pilotage de tous tes agents Claude Code.');
  });

  it('presents the board, the test launch and the editor among the features', () => {
    const ids = FEATURES.map((f) => f.id);
    expect(ids).toEqual(expect.arrayContaining(['tableau', 'test', 'editeur']));
    expect(html).toContain('Pilote auto');
  });

  it('presents the imports from Jira, Trello and GitHub, kept in sync, among the features', () => {
    const f = FEATURES.find((x) => x.id === 'integrations');
    expect(f, 'integrations').toBeDefined();
    const said = [f!.title, f!.text, ...f!.points].join(' ');
    for (const service of ['Jira', 'Trello', 'GitHub']) expect(said).toContain(service);
    expect(said).toMatch(/critères d’acceptation/);
    expect(said).toMatch(/statut/);
    expect(html).toContain(`src="${BASE}images/integrations.jpg"`);
  });

  it('calls the board by its name in the app, the Kanban', () => {
    expect(html).toContain('Kanban');
    // A Trello board is still a « tableau ».
    expect(html).not.toMatch(/\b(le|un|du) tableau\b(?! Trello)/i);
  });

  it('tells GitHub Pages not to run Jekyll on it', () => {
    expect(existsSync(new URL('.nojekyll', OUT))).toBe(true);
  });
});
