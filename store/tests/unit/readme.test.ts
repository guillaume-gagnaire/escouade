import { describe, expect, it } from 'vitest';
import { imageInfo } from '../../server/utils/images';
import { DOC_PAGES_MAX, insidePath, renderDocs, renderMarkdown, titleOf } from '../../server/utils/readme';
import { readArchive } from '../../server/utils/tar';
import { manifest, png, tarball } from '../fake/plugins.mjs';

// A README is an author's text shown on the store's pages: what survives its rendering is a short list of tags, and
// nothing that runs, styles, frames, tracks or leaves for a third party (docs/STORE.md §7).
const REPO = 'sroux/escouade-ntfy';
const COMMIT = 'a41c9d2'.padEnd(40, '0');
const ctx = { from: 'README.md', repo: REPO, commit: COMMIT, images: new Map() };

/** A README booby-trapped every way the chief of the squad listed, and a few more. */
export const TRAPPED_README = [
  '# Ntfy',
  '',
  '<script>window.pwned = 1</script>',
  '<style>body { display: none }</style>',
  '<img src="x" onerror="window.pwned = 2">',
  '<img src="https://tracker.example/pixel.gif" alt="pixel">',
  '',
  '[clique](javascript:alert(1)) [data](data:text/html,<script>alert(1)</script>) [vb](vbscript:msgbox)',
  '',
  '<a href="JaVaScRiPt:alert(1)">mixed case</a> <a href="  javascript:alert(1)">spaced</a>',
  '<iframe src="https://evil.example"></iframe><object data="x"></object><embed src="x">',
  '<form action="https://evil.example"><input name="q"><button>Go</button></form>',
  '<p style="position:fixed;inset:0" class="hero" id="__nuxt" onclick="x()">styled</p>',
  '<svg><script>alert(1)</script></svg><math><mtext><script>alert(1)</script></mtext></math>',
  '<meta http-equiv="refresh" content="0;url=https://evil.example"><base href="https://evil.example/"><link rel="stylesheet" href="https://evil.example/x.css">',
  '<scr<script>ipt>alert(1)</script>',
  '',
  '## Usage',
  '',
  'Read [the setup](docs/setup.md), the [site](https://ntfy.sh) or [mail me](mailto:sam@example.dev).',
  '',
  '```yaml',
  'plugins: [ntfy]',
  '```',
].join('\n');

describe('the README', () => {
  it('keeps nothing that runs, styles or frames', () => {
    const { html } = renderMarkdown(TRAPPED_README, ctx);
    expect(html).not.toMatch(/<script|<style|<iframe|<object|<embed|<form|<input|<button|<svg|<math|<meta|<base|<link/i);
    expect(html).not.toMatch(/\son[a-z]+=/i);
    // A link refused by the markdown stays its text; no address of these schemes is left in an attribute.
    expect(html).not.toMatch(/="\s*(javascript|vbscript|data):/i);
    expect(html).not.toMatch(/\sstyle=|\sclass="hero"|\sid=/i);
    expect(html).not.toContain('pwned');
    expect(html).not.toContain('tracker.example');
    expect(html).not.toContain('display: none');
    // What is a text stays a text.
    expect(html).toContain('<h1>Ntfy</h1>');
    expect(html).toContain('styled');
    expect(html).toContain('<pre><code class="language-yaml">');
  });

  it('sends a link to a file of the repository to GitHub at the frozen commit, and marks every link', () => {
    const { html } = renderMarkdown(TRAPPED_README, ctx);
    expect(html).toContain(
      `<a href="https://github.com/${REPO}/blob/${COMMIT}/docs/setup.md" rel="nofollow ugc noopener noreferrer">the setup</a>`,
    );
    expect(html).toContain('<a href="https://ntfy.sh" rel="nofollow ugc noopener noreferrer">site</a>');
    expect(html).toContain('href="mailto:sam@example.dev"');
    // The links that were refused keep their words, without an address.
    expect(html).toContain('<a>mixed case</a> <a>spaced</a>');
    expect(html).toContain('[clique](javascript:alert(1))');
  });

  it('names only paths inside the plugin', () => {
    expect(insidePath('store/1.png', 'README.md', REPO)).toBe('store/1.png');
    expect(insidePath('./img/a.png?raw=1#x', 'docs/setup.md', REPO)).toBe('docs/img/a.png');
    expect(insidePath('/store/1.png', 'docs/setup.md', REPO)).toBe('store/1.png');
    expect(insidePath(`https://raw.githubusercontent.com/${REPO}/main/store/1.png`, 'README.md', REPO)).toBe('store/1.png');
    expect(insidePath(`https://github.com/${REPO}/blob/v1/store/1.png?raw=true`, 'README.md', REPO)).toBe('store/1.png');
    expect(insidePath('https://raw.githubusercontent.com/other/repo/main/x.png', 'README.md', REPO)).toBeNull();
    expect(insidePath('../../etc/passwd', 'docs/a.md', REPO)).toBeNull();
    expect(insidePath('%2e%2e/%2e%2e/x.png', 'README.md', REPO)).toBeNull();
    expect(insidePath('//evil.example/x.png', 'README.md', REPO)).toBeNull();
    expect(insidePath('data:image/png;base64,AAAA', 'README.md', REPO)).toBeNull();
    expect(insidePath('a\\..\\..\\x.png', 'README.md', REPO)).toBeNull();
  });

  it('copies the images of the repository, re-encoded, and drops the others', async () => {
    const files = {
      'escouade-plugin.json': JSON.stringify(manifest(REPO, { store: { docs: 'docs' } })),
      'README.fr.md': '# Ntfy\n\n![capture](store/1.png) ![ailleurs](https://example.com/x.png) ![absente](store/none.png)\n',
      'README.en.md': `# Ntfy\n\n<img src="https://raw.githubusercontent.com/${REPO}/main/store/1.png" alt="shot">\n`,
      'docs/setup.md': '# Setting up\n\n![Same](../store/1.png)\n',
      'docs/b-more.md': 'No heading.\n',
      'store/1.png': png(64, 32),
    };
    const read = readArchive(tarball(files));
    const kept: Buffer[] = [];
    const docs = await renderDocs(read, {
      repo: REPO,
      commit: COMMIT,
      keep: async (bytes, format) => {
        kept.push(bytes);
        return `${'f'.repeat(64)}.${format === 'jpeg' ? 'jpg' : format}`;
      },
    });
    // One copy for the three pages that show it, without the metadata of the original (its text chunk).
    expect(kept).toHaveLength(1);
    expect(kept[0]!.includes(Buffer.from('Somebody'))).toBe(false);
    expect(imageInfo(kept[0]!)).toMatchObject({ format: 'png', width: 64, height: 32 });
    expect(docs.images).toEqual([`${'f'.repeat(64)}.png`]);
    const img = `src="/files/${'f'.repeat(64)}.png" width="64" height="32" loading="lazy"`;
    expect(docs.readme.fr).toContain(`<img ${img} alt="capture" />`);
    expect(docs.readme.fr).not.toContain('example.com');
    expect(docs.readme.fr).not.toContain('absente');
    expect(docs.readme.en).toContain(`<img ${img} alt="shot" />`);
    expect(docs.pages.map((p) => [p.path, p.title])).toEqual([
      ['docs/b-more.md', 'b-more'],
      ['docs/setup.md', 'Setting up'],
    ]);
    expect(docs.pages[1]!.html).toContain(img);
  });

  it('shows README.md in both languages when it is the only one', async () => {
    const read = readArchive(tarball({ 'README.md': '# Only\n' }));
    const docs = await renderDocs(read, { repo: REPO, commit: COMMIT, keep: async () => 'x' });
    expect(docs.readme).toEqual({ fr: '<h1>Only</h1>\n', en: '<h1>Only</h1>\n' });
    expect(docs.pages).toEqual([]);
  });
});

describe('the documentation pages by language (ESC-366)', () => {
  const docsOf = async (files: Record<string, string | Buffer>, docs: unknown = 'docs') =>
    renderDocs(readArchive(tarball({ 'escouade-plugin.json': JSON.stringify(manifest(REPO, { store: { docs } })), ...files })), {
      repo: REPO,
      commit: COMMIT,
      keep: async (_bytes, format) => `${'e'.repeat(64)}.${format}`,
    });

  it('gives the pages of docs/fr and docs/en their language, in the order of their names', async () => {
    const docs = await docsOf({
      'README.md': '# Ntfy\n',
      'docs/fr/02-regler.md': '# Régler le sujet\n\nRetour à [l’installation](01-installer.md#avant).\n',
      'docs/fr/01-installer.md': '# Installer\n\n![Capture](../images/shot.png)\n\nPuis [régler](./02-regler.md).\n',
      'docs/en/01-install.md':
        '# Install\n\n![Shot](../images/shot.png)\n\nThen [set it up](02-set-up.md), or [in French](../fr/01-installer.md).\n',
      'docs/en/02-set-up.md': 'No heading here.\n\n```sh\n# not a title\n```\n',
      'docs/images/shot.png': png(40, 20),
      'docs/notes.md': '# Not shown: the folder has languages\n',
    });
    expect(docs.pages.map((p) => [p.lang, p.path, p.title])).toEqual([
      ['fr', 'docs/fr/01-installer.md', 'Installer'],
      ['fr', 'docs/fr/02-regler.md', 'Régler le sujet'],
      ['en', 'docs/en/01-install.md', 'Install'],
      // No heading, a `#` in a code block: the name of the file without its order.
      ['en', 'docs/en/02-set-up.md', 'set-up'],
    ]);
    const [fr1, fr2, en1] = docs.pages;
    // A link to another page opens its tab on the store; an image of the folder is copied, for both languages.
    expect(fr1!.html).toContain('href="?tab=docs&amp;page=docs%2Ffr%2F02-regler.md"');
    expect(fr2!.html).toContain('href="?tab=docs&amp;page=docs%2Ffr%2F01-installer.md"');
    expect(en1!.html).toContain('href="?tab=docs&amp;page=docs%2Fen%2F02-set-up.md"');
    expect(en1!.html).toContain('href="?tab=docs&amp;page=docs%2Ffr%2F01-installer.md"');
    const img = `src="/files/${'e'.repeat(64)}.png" width="40" height="20"`;
    expect(fr1!.html).toContain(img);
    expect(en1!.html).toContain(img);
    expect(docs.images).toEqual([`${'e'.repeat(64)}.png`]);
    expect(fr1!.html).not.toContain('rel=');
  });

  it('keeps a folder without languages as before: every page, for both, without a language', async () => {
    const docs = await docsOf({
      'README.md': '# Ntfy\n\nSee [the setup](docs/setup.md).\n',
      'docs/setup.md': '# Setting up\n\nBack to the [README](../README.md); the [source](../src/main.rs).\n',
      'docs/more/deep.md': '# Deeper\n',
      'src/main.rs': 'fn main() {}\n',
    });
    expect(docs.pages.map((p) => [p.path, p.title, p.lang])).toEqual([
      ['docs/more/deep.md', 'Deeper', undefined],
      ['docs/setup.md', 'Setting up', undefined],
    ]);
    expect(docs.pages.every((p) => !('lang' in p))).toBe(true);
    expect(docs.readme.fr).toContain('href="?tab=docs&amp;page=docs%2Fsetup.md"');
    expect(docs.pages[1]!.html).toContain('href="?"');
    // Any other file of the repository: on GitHub, at the commit the store froze.
    expect(docs.pages[1]!.html).toContain(`href="https://github.com/${REPO}/blob/${COMMIT}/src/main.rs"`);
  });

  it('shows the one language there is, and keeps DOC_PAGES_MAX pages per language', async () => {
    const many = Object.fromEntries(
      Array.from({ length: DOC_PAGES_MAX + 3 }, (_, i) => [`docs/en/${String(i).padStart(2, '0')}-p.md`, `# P${i}\n`]),
    );
    const fr = Object.fromEntries(
      Array.from({ length: DOC_PAGES_MAX + 1 }, (_, i) => [`docs/fr/${String(i).padStart(2, '0')}-p.md`, `# P${i}\n`]),
    );
    const onlyEn = await docsOf({ 'README.md': '# x\n', ...many });
    expect(onlyEn.pages).toHaveLength(DOC_PAGES_MAX);
    expect(onlyEn.pages.every((p) => p.lang === 'en')).toBe(true);
    expect(onlyEn.pages[0]!.title).toBe('P0');
    const both = await docsOf({ 'README.md': '# x\n', ...many, ...fr });
    expect(both.pages.filter((p) => p.lang === 'fr')).toHaveLength(DOC_PAGES_MAX);
    expect(both.pages.filter((p) => p.lang === 'en')).toHaveLength(DOC_PAGES_MAX);
  });

  it('reads the title from the first heading, else the name without its order', () => {
    expect(titleOf('Intro\n\n## **Set** `up`\n\n# Later\n', 'docs/x.md')).toBe('Set up');
    expect(titleOf('Nothing\n', 'docs/fr/03_regler-le-sujet.md')).toBe('regler-le-sujet');
    expect(titleOf('', 'docs/fr/01.md')).toBe('01');
  });

  it('reads no page from a folder that is not there', async () => {
    expect((await docsOf({ 'README.md': '# x\n' }, 'documentation')).pages).toEqual([]);
    expect((await docsOf({ 'README.md': '# x\n', 'x.md': '# x\n' }, 42)).pages).toEqual([]);
  });
});
