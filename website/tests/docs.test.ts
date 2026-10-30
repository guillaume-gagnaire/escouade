import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { CATALOGS } from '../app/data/catalogs';
import { DOC_GROUPS, DOC_SLUGS, docPath, docRoute, isDocSlug } from '../app/data/docs';
import type { Lang } from '../app/data/language';
import { renderDoc, slugify, type RenderedDoc } from '../app/data/render';
import { searchDocs, searchEntries } from '../app/data/search';

const CONTENT = new URL('../content/docs/', import.meta.url);
const PUBLIC = new URL('../public/', import.meta.url);
const LANGS: Lang[] = ['fr', 'en'];

/** A site path under a base URL that is not the root, to see that the pages' links keep it. */
const under = (path: string) => `/base${path}`;
const render = (source: string, lang: Lang = 'fr') => renderDoc(source, { lang, link: under, callouts: CATALOGS[lang].docs.callouts });
const page = (body: string) => `# Le titre\n\nLe chapeau, en une phrase.\n\n${body}`;

describe('the address of a page of the documentation', () => {
  it('is under /docs/ in French and /en/docs/ in English, with a slash at the end', () => {
    expect(docPath('fr')).toBe('/docs/');
    expect(docPath('en')).toBe('/en/docs/');
    expect(docPath('fr', 'managing-agents')).toBe('/docs/managing-agents/');
    expect(docPath('en', 'settings-network')).toBe('/en/docs/settings-network/');
  });

  it('knows the pages that exist', () => {
    expect(isDocSlug('autopilot')).toBe(true);
    expect(isDocSlug('pilote-auto')).toBe(false);
    expect(isDocSlug('')).toBe(false);
  });

  it('is where a link of a page leads the router, when it leads to another page of the documentation', () => {
    const at = (href: string, base = '/', current = '/docs/chat/') =>
      docRoute(new URL(href, 'https://site.dev/docs/chat/'), 'https://site.dev', base, current);
    expect(at('/docs/editor/')).toBe('/docs/editor/');
    expect(at('/en/docs/editor/#search')).toBe('/en/docs/editor/#search');
    expect(at('/escouade/docs/editor/', '/escouade/', '/docs/chat/')).toBe('/docs/editor/');
    // The same page (its own sections), the rest of the site, another site and another base are the browser’s.
    expect(at('#options')).toBeNull();
    expect(at('/docs/chat/#options')).toBeNull();
    expect(at('/docs/chat#options', '/', '/docs/chat')).toBeNull();
    expect(at('/')).toBeNull();
    expect(at('/images/board.jpg')).toBeNull();
    expect(at('https://github.com/docs/x/')).toBeNull();
    expect(at('/docs/editor/', '/escouade/')).toBeNull();
  });

  it('lists every page once, in the groups’ order', () => {
    expect(new Set(DOC_SLUGS).size).toBe(DOC_SLUGS.length);
    expect(DOC_SLUGS).toEqual(DOC_GROUPS.flatMap((g) => g.pages));
    expect(DOC_SLUGS[0]).toBe('introduction');
    for (const slug of DOC_SLUGS) expect(slug, slug).toMatch(/^(?:[a-z0-9]+(?:-[a-z0-9]+)*\/)?[a-z0-9]+(?:-[a-z0-9]+)*$/);
  });
});

describe('the anchor of a heading', () => {
  it('is its text in lowercase, without accents, with dashes between words', () => {
    expect(slugify('Modes de permission')).toBe('modes-de-permission');
    expect(slugify('Données locales')).toBe('donnees-locales');
    expect(slugify('Qu’est-ce qu’Escouade ?')).toBe('qu-est-ce-qu-escouade');
    expect(slugify('Ctrl+K : chercher')).toBe('ctrl-k-chercher');
    expect(slugify('« Pilote auto »')).toBe('pilote-auto');
    expect(slugify('Œuvre et Æther')).toBe('oeuvre-et-aether');
  });
});

describe('a page, rendered', () => {
  it('takes its title from its first line and its lede from the paragraph after it, as plain text', () => {
    const doc = render('# Les **agents**\n\nUn agent est une session de `Claude Code`.\n\n## Créer\n\nTexte.');
    expect(doc.title).toBe('Les agents');
    expect(doc.lede).toBe('Un agent est une session de Claude Code.');
    expect(doc.html).not.toContain('<h1');
    expect(doc.html).not.toContain('Un agent est une session');
    expect(doc.html).toContain('<p>Texte.</p>');
  });

  it('refuses a page without a title or without a lede', () => {
    expect(() => render('Pas de titre.\n\n## Section')).toThrow(/title/);
    expect(() => render('# Titre\n\n## Section tout de suite')).toThrow(/lede/);
  });

  it('gives each section an anchor, and lists the sections and their subsections for the page’s summary', () => {
    const doc = render(page('## Modes de permission\n\nA.\n\n### Le mode « Plan »\n\nB.\n\n## Données locales\n\nC.'));
    expect(doc.html).toContain('<h2 id="modes-de-permission">Modes de permission</h2>');
    expect(doc.html).toContain('<h3 id="le-mode-plan">Le mode « Plan »</h3>');
    expect(doc.headings).toEqual([
      { id: 'modes-de-permission', text: 'Modes de permission', depth: 2 },
      { id: 'le-mode-plan', text: 'Le mode « Plan »', depth: 3 },
      { id: 'donnees-locales', text: 'Données locales', depth: 2 },
    ]);
  });

  it('keeps the anchors of two sections of the same name apart', () => {
    const doc = render(page('## Options\n\n### Options\n\n## Options'));
    expect(doc.headings.map((h) => h.id)).toEqual(['options', 'options-2', 'options-3']);
  });

  it('turns a link to another page into its address in the page’s language, under the base URL', () => {
    const fr = render(page('Voir [le pilote auto](autopilot) et [ses quotas](usage#le-seuil).'), 'fr');
    expect(fr.html).toContain('<a href="/base/docs/autopilot/">le pilote auto</a>');
    expect(fr.html).toContain('<a href="/base/docs/usage/#le-seuil">ses quotas</a>');
    const en = render(page('See [autopilot](autopilot).'), 'en');
    expect(en.html).toContain('<a href="/base/en/docs/autopilot/">autopilot</a>');
    expect(en.links).toEqual([{ slug: 'autopilot', anchor: '' }]);
    expect(fr.links).toEqual([
      { slug: 'autopilot', anchor: '' },
      { slug: 'usage', anchor: 'le-seuil' },
    ]);
  });

  it('refuses a link to a page that does not exist', () => {
    expect(() => render(page('Voir [la lune](moon).'))).toThrow(/moon/);
  });

  it('refuses a link that would run a script, or that leads nowhere the site knows', () => {
    expect(() => render(page('[Clic](javascript:alert(1))'))).toThrow(/javascript/);
    expect(() => render(page('[Clic](data:text/html,x)'))).toThrow(/data:/);
    expect(render(page('[Écris-moi](mailto:a@b.c)')).html).toContain('<a href="mailto:a@b.c" rel="noopener">');
  });

  it('leaves the links to other sites and to the page’s own sections alone, and opens no new window', () => {
    const doc = render(page('[GitHub](https://github.com/guillaume-gagnaire/escouade) et [plus bas](#options).\n\n## Options'));
    expect(doc.html).toContain('<a href="https://github.com/guillaume-gagnaire/escouade" rel="noopener">GitHub</a>');
    expect(doc.html).toContain('<a href="#options">plus bas</a>');
    expect(doc.html).not.toContain('target=');
    expect(doc.links).toEqual([]);
    expect(doc.anchors).toEqual(['options']);
  });

  it('serves its images from the base URL, lazily, at their size', () => {
    const doc = render(page('![Le Kanban et ses colonnes](/images/board.jpg)'));
    expect(doc.html).toContain(
      '<img src="/base/images/board.jpg" alt="Le Kanban et ses colonnes" width="1920" height="960" loading="lazy" decoding="async">',
    );
    expect(doc.images).toEqual(['/images/board.jpg']);
  });

  it('turns a quote that opens with [!NOTE], [!TIP] or [!WARNING] into a callout named in the page’s language', () => {
    const doc = render(page('> [!TIP]\n> `Ctrl+J` ouvre le prochain agent.\n\n> [!WARNING]\n> Attention.\n\n> Une citation.'));
    expect(doc.html).toContain(
      `<div class="callout tip"><p class="callout-title">${CATALOGS.fr.docs.callouts.tip}</p><p><code class="key">Ctrl+J</code> ouvre le prochain agent.</p>\n</div>`,
    );
    expect(doc.html).toContain(`<div class="callout warning"><p class="callout-title">${CATALOGS.fr.docs.callouts.warning}</p>`);
    expect(doc.html).toContain('<blockquote>\n<p>Une citation.</p>\n</blockquote>');
    expect(render(page('> [!NOTE]\n> Ceci.'), 'en').html).toContain(`<p class="callout-title">${CATALOGS.en.docs.callouts.note}</p>`);
  });

  it('renders a callout like the rest of the page: its reference links, and a name alone on its line', () => {
    const doc = render(page('> [!NOTE]\n>\n> Voir [ici][r].\n\n[r]: https://example.com'));
    expect(doc.html).toContain(
      `<div class="callout note"><p class="callout-title">${CATALOGS.fr.docs.callouts.note}</p><p>Voir <a href="https://example.com" rel="noopener">ici</a>.</p>\n</div>`,
    );
  });

  it('never gives two headings the same anchor, even inside a quote', () => {
    const doc = render(page('## Options\n\n> ## Options\n\n## Options'));
    expect([...doc.html.matchAll(/id="([^"]+)"/g)].map((m) => m[1]).sort()).toEqual(['options', 'options-2', 'options-3']);
  });

  it('keeps a short code on one line, and lets a long one wrap', () => {
    const doc = render(page('`Ctrl+Maj+F` et `~/.escouade/conversations/<agent>.jsonl`'));
    expect(doc.html).toContain('<code class="key">Ctrl+Maj+F</code>');
    expect(doc.html).toContain('<code>~/.escouade/conversations/&lt;agent&gt;.jsonl</code>');
  });

  it('lets a wide table scroll on its own', () => {
    const doc = render(page('| A | B |\n|---|---|\n| 1 | 2 |'));
    expect(doc.html).toMatch(/^<div class="table"><table>[\s\S]*<\/table><\/div>$/m);
  });

  it('shows the HTML a page writes as text', () => {
    const doc = render(page('Un <script>alert(1)</script> et <b>gras</b>.'));
    expect(doc.html).not.toContain('<script>');
    expect(doc.html).toContain('&lt;script&gt;');
    // After a tag that opens a raw block in Markdown, the text is not escaped by marked itself.
    const raw = render(page('Un <code> <img src=x onerror=alert(1)// fin'));
    expect(raw.html).not.toContain('<img');
    expect(raw.html).toContain('&lt;img src=x onerror=alert(1)// fin');
  });

  it('gives the plain text of each section, to search in', () => {
    const doc = render(page('Avant.\n\n## Le **pilote** auto\n\nIl prend `1` à 6 [tickets](board).\n\n### Pause\n\nPrès du quota.'));
    expect(doc.sections).toEqual([
      { id: '', heading: '', text: 'Avant.' },
      { id: 'le-pilote-auto', heading: 'Le pilote auto', text: 'Il prend 1 à 6 tickets.' },
      { id: 'pause', heading: 'Pause', text: 'Près du quota.' },
    ]);
  });
});

describe('the search', () => {
  const docs = [
    {
      slug: 'managing-agents' as const,
      doc: render(
        '# Les agents\n\nUn agent est une session de Claude Code.\n\n## Modes de permission\n\nAuto, Plan, Bypass.\n\n## Créer un agent\n\n`Ctrl+N` crée un agent.',
      ),
    },
    {
      slug: 'autopilot' as const,
      doc: render('# Le pilote auto\n\nIl prend les tickets un à un.\n\n## Le quota\n\nIl se met en pause près de la limite des agents.'),
    },
  ];
  const entries = docs.flatMap(({ slug, doc }) => searchEntries(slug, doc));

  it('looks through each page and each of its sections', () => {
    expect(entries.map((e) => [e.slug, e.id, e.heading])).toEqual([
      ['managing-agents', '', ''],
      ['managing-agents', 'modes-de-permission', 'Modes de permission'],
      ['managing-agents', 'creer-un-agent', 'Créer un agent'],
      ['autopilot', '', ''],
      ['autopilot', 'le-quota', 'Le quota'],
    ]);
    expect(entries[0]).toMatchObject({ page: 'Les agents', text: 'Un agent est une session de Claude Code.' });
  });

  it('finds the words written with a typographic apostrophe when the keyboard types a straight one', () => {
    const doc = render('# L’interface\n\nCe qu’on voit d’abord.\n\n## L’en-tête\n\nLe nom de l’agent.');
    const hits = searchDocs(searchEntries('interface', doc), "l'agent");
    expect(hits.map((h) => h.id)).toEqual(['l-en-tete']);
    expect(hits[0].snippet.match).toBe('l’agent');
    expect(searchDocs(searchEntries('interface', doc), "qu'on").map((h) => h.id)).toEqual(['']);
  });

  it('finds every word asked, without minding the case or the accents', () => {
    expect(searchDocs(entries, 'CREER').map((h) => h.id)).toEqual(['creer-un-agent']);
    expect(searchDocs(entries, 'pilote pause').map((h) => [h.slug, h.id])).toEqual([['autopilot', 'le-quota']]);
    expect(searchDocs(entries, 'pilote lune')).toEqual([]);
    expect(searchDocs(entries, '   ')).toEqual([]);
  });

  it('puts what a heading says before what a page’s title says, and that before what a text says', () => {
    // A page’s introduction has the page’s title for its heading.
    expect(searchDocs(entries, 'agent').map((h) => [h.slug, h.id])).toEqual([
      ['managing-agents', 'creer-un-agent'],
      ['managing-agents', ''],
      ['managing-agents', 'modes-de-permission'],
      ['autopilot', 'le-quota'],
    ]);
  });

  it('keeps to a few results', () => {
    expect(searchDocs(entries, 'agent', 2)).toHaveLength(2);
  });

  it('shows the words around what it found, with the word itself apart', () => {
    const [hit] = searchDocs(entries, 'limite');
    expect(hit.snippet).toEqual({ before: 'Il se met en pause près de la ', match: 'limite', after: ' des agents.' });
    const [title] = searchDocs(entries, 'session');
    expect(title.snippet.match).toBe('session');
  });

  it('cuts a long text around what it found', () => {
    const long = render(`# Long\n\nUn chapeau assez long pour la page.\n\n## Section\n\n${'mot '.repeat(60)}cible${' fin'.repeat(60)}`);
    const [hit] = searchDocs(searchEntries('managing-agents', long), 'cible');
    expect(hit.snippet.match).toBe('cible');
    expect(hit.snippet.before.startsWith('…')).toBe(true);
    expect(hit.snippet.after.endsWith('…')).toBe(true);
    expect(hit.snippet.before.length + hit.snippet.after.length).toBeLessThan(200);
  });
});

/** The Markdown source of a page. */
const source = (lang: Lang, slug: string) => readFileSync(new URL(`${lang}/${slug}.md`, CONTENT), 'utf8');

/** The prose of a source: without its code blocks and its inline code. */
const prose = (s: string) => s.replace(/```[\s\S]*?```/g, '').replace(/`[^`\n]*`/g, '');

describe('the pages of the documentation', () => {
  const rendered = new Map<string, RenderedDoc>();
  const doc = (lang: Lang, slug: string) => {
    const key = `${lang}/${slug}`;
    if (!rendered.has(key)) rendered.set(key, render(source(lang, slug), lang));
    return rendered.get(key)!;
  };

  it('exist in both languages, and none is left out of the sidebar', () => {
    for (const lang of LANGS) {
      for (const slug of DOC_SLUGS) expect(existsSync(new URL(`${lang}/${slug}.md`, CONTENT)), `${lang}/${slug}.md`).toBe(true);
      const files = readdirSync(new URL(`${lang}/`, CONTENT), { recursive: true, encoding: 'utf8' })
        .map((f) => f.replaceAll('\\', '/'))
        .filter((f) => f.endsWith('.md'));
      expect(files.map((f) => f.slice(0, -3)).sort(), lang).toEqual([...DOC_SLUGS].sort());
    }
  });

  it('render, each with a title, a lede long enough to describe it, and sections', () => {
    for (const lang of LANGS) {
      for (const slug of DOC_SLUGS) {
        const d = doc(lang, slug);
        expect(d.title.length, `${lang}/${slug}`).toBeGreaterThan(2);
        expect(d.lede.length, `${lang}/${slug}: ${d.lede}`).toBeGreaterThan(40);
        expect(d.lede.length, `${lang}/${slug}: ${d.lede}`).toBeLessThan(320);
        expect(d.headings.length, `${lang}/${slug}`).toBeGreaterThan(0);
      }
    }
  });

  it('go no deeper than ###', () => {
    for (const lang of LANGS) for (const slug of DOC_SLUGS) expect(source(lang, slug), `${lang}/${slug}`).not.toMatch(/^#{4,} /m);
  });

  it('have the same sections, links and images in both languages', () => {
    for (const slug of DOC_SLUGS) {
      const [fr, en] = LANGS.map((lang) => doc(lang, slug));
      expect(
        en.headings.map((h) => h.depth),
        slug,
      ).toEqual(fr.headings.map((h) => h.depth));
      expect(
        en.links.map((l) => l.slug),
        slug,
      ).toEqual(fr.links.map((l) => l.slug));
      expect(en.images, slug).toEqual(fr.images.map((i) => i.replace(/^\/images\//, '/images/en/')));
    }
  });

  it('link only to sections that exist', () => {
    for (const lang of LANGS) {
      for (const slug of DOC_SLUGS) {
        const d = doc(lang, slug);
        for (const a of d.anchors)
          expect(
            d.headings.map((h) => h.id),
            `${lang}/${slug} → #${a}`,
          ).toContain(a);
        for (const l of d.links.filter((l) => l.anchor)) {
          expect(
            doc(lang, l.slug).headings.map((h) => h.id),
            `${lang}/${slug} → ${l.slug}#${l.anchor}`,
          ).toContain(l.anchor);
        }
      }
    }
  });

  it('show screenshots that exist, in their own language', () => {
    for (const lang of LANGS) {
      for (const slug of DOC_SLUGS) {
        for (const image of doc(lang, slug).images) {
          expect(existsSync(new URL(image.slice(1), PUBLIC)), `${lang}/${slug}: ${image}`).toBe(true);
          expect(image.startsWith('/images/en/'), `${lang}/${slug}: ${image}`).toBe(lang === 'en');
        }
      }
    }
  });

  it('write apostrophes, quotation marks and ellipses the typographic way, outside the code', () => {
    for (const lang of LANGS) {
      for (const slug of DOC_SLUGS) {
        const text = prose(source(lang, slug));
        const where = `${lang}/${slug}`;
        expect(text, where).not.toMatch(/['"]/);
        expect(text, where).not.toContain('...');
        if (lang === 'fr') {
          expect(text, where).not.toMatch(/[“”]/);
          // A non-breaking space inside the guillemets, so that « Pilote auto » never splits over two lines.
          expect(text, where).not.toMatch(/«(?! )|(?<! )»/);
        } else {
          expect(text, where).not.toMatch(/[«»]/);
        }
      }
    }
  });

  it('write the keys as the app shows them in the page’s language', () => {
    /** The shortcuts a page writes in code: `Ctrl+Maj+F`, `Entrée`. */
    const keys = (s: string) =>
      [...s.replace(/```[\s\S]*?```/g, '').matchAll(/`([^`\n]+)`/g)]
        .map((m) => m[1])
        .filter((k) => /^(?:[\p{L}\p{N}_⌘⌃⇧⌥←→↑↓…,.\-/]+\+)*[\p{L}\p{N}_⌘⌃⇧⌥←→↑↓…,.\-/]+$/u.test(k));
    for (const slug of DOC_SLUGS) {
      for (const k of keys(source('fr', slug))) expect(k, `fr/${slug}`).not.toMatch(/(?:^|\+)(?:Shift|Enter|Esc)(?:\+|$)/);
      for (const k of keys(source('en', slug))) expect(k, `en/${slug}`).not.toMatch(/(?:^|\+)(?:Maj|Entrée|Échap)(?:\+|$)/);
    }
  });

  it('write the English with American spellings and the app’s English words', () => {
    const said = DOC_SLUGS.map((slug) => prose(source('en', slug))).join('\n');
    expect(said).not.toMatch(/\b(labell|colour|favour|behaviour|catalogue|licence\b|cancell)/i);
    expect(said).not.toMatch(/\bauto-pilot\b|\bauto pilot\b/i);
    expect(said).not.toMatch(/\b(Validate|validate|validated|validating)\b/);
  });

  it('call the app by its name', () => {
    for (const lang of LANGS) expect(source(lang, 'introduction'), lang).toContain('Escouade');
  });

  it('are named in the sidebar of both languages', () => {
    for (const lang of LANGS) {
      const { docs } = CATALOGS[lang];
      expect(Object.keys(docs.pages).sort(), lang).toEqual([...DOC_SLUGS].sort());
      expect(Object.keys(docs.groups), lang).toEqual(DOC_GROUPS.map((g) => g.id));
      for (const slug of DOC_SLUGS) expect(docs.pages[slug].length, `${lang} ${slug}`).toBeLessThanOrEqual(32);
    }
  });
});

describe('the page of the voice settings', () => {
  const real = (lang: Lang) => render(source(lang, 'settings-voice'), lang);

  it('is in the settings group of the sidebar, named in both languages', () => {
    expect(DOC_GROUPS.find((g) => g.id === 'settings')?.pages).toContain('settings-voice');
    expect(CATALOGS.fr.docs.pages['settings-voice']).toBe('Voix');
    expect(CATALOGS.en.docs.pages['settings-voice']).toBe('Voice');
  });

  it('is found by the search, by the words of its settings', () => {
    for (const lang of LANGS) {
      const entries = searchEntries('settings-voice', real(lang));
      for (const word of ['Whisper', 'Piper', 'Fuze', 'ElevenLabs', 'ONNX'])
        expect(searchDocs(entries, word).length, `${lang} ${word}`).toBeGreaterThan(0);
    }
  });
});

describe('the page of the example plugins', () => {
  const REPO = new URL('../../', import.meta.url);
  // Each block of code under a path of plugin-examples/ (`plugin-examples/slack/src/plugin.ts`, then the block).
  const blocks = (lang: Lang) =>
    [...source(lang, 'plugins/examples').matchAll(/^`(plugin-examples\/[^`\s]+)`\n\n```\w*\n([\s\S]*?)\n```$/gm)].map(([, path, code]) => ({
      path,
      code,
    }));

  it('shows each example’s code as the repository holds it, the same in both languages', () => {
    for (const lang of LANGS) {
      const shown = blocks(lang);
      expect(shown.length, lang).toBeGreaterThanOrEqual(10);
      for (const { path, code } of shown) {
        const file = readFileSync(new URL(path, REPO), 'utf8').replace(/\r\n/g, '\n').replace(/\n+$/, '');
        expect(code, `${lang}: ${path} changed, copy it again into the page`).toBe(file);
      }
    }
    expect(blocks('en').map((b) => b.path)).toEqual(blocks('fr').map((b) => b.path));
  });

  it('shows every example of plugin-examples/', () => {
    const examples = readdirSync(new URL('plugin-examples/', REPO), { withFileTypes: true })
      .filter((d) => d.isDirectory() && existsSync(new URL(`plugin-examples/${d.name}/escouade-plugin.json`, REPO)))
      .map((d) => `plugin-examples/${d.name}/escouade-plugin.json`);
    expect(examples.length).toBe(4);
    for (const lang of LANGS) {
      const shown = blocks(lang).map((b) => b.path);
      for (const manifest of examples) expect(shown, lang).toContain(manifest);
    }
  });
});
