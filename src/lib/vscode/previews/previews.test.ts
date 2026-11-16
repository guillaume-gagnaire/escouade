import { describe, expect, it } from 'vitest';
import { parseTable, separatorOf } from './csv';
import { extOf, globOf, linkOf, NO_PREVIEW, opensInPreview, previewOf, resolveRel } from './kinds';
import { lineFor, renderPreview, scrollFor, slugOf } from './markdown';
import { cleanHtml } from './sanitize';

/** The HTML as an element, to look into it as the pane will. */
const dom = (html: string) => {
  const el = document.createElement('div');
  el.innerHTML = html;
  return el;
};

describe('the preview of a file, by its type', () => {
  it('chooses the preview of each family by its extension, any case', () => {
    for (const f of ['a.png', 'b.JPG', 'c.jpeg', 'd.gif', 'e.webp', 'f.avif', 'g.bmp', 'h.ico', 'logo.svg'])
      expect(previewOf(`assets/${f}`)?.kind, f).toBe('image');
    expect(previewOf('x/logo.svg')?.mime).toBe('image/svg+xml');
    expect(previewOf('son.mp3')).toEqual({ kind: 'audio', mime: 'audio/mpeg' });
    expect(previewOf('film.webm')).toEqual({ kind: 'video', mime: 'video/webm' });
    expect(previewOf('docs/Guide.PDF')?.kind).toBe('pdf');
    expect(previewOf('rapport.docx')?.kind).toBe('docx');
    expect(previewOf('README.md')?.kind).toBe('markdown');
    expect(previewOf('notes.markdown')?.kind).toBe('markdown');
    expect(previewOf('data.csv')?.kind).toBe('csv');
    expect(previewOf('data.tsv')?.kind).toBe('csv');
  });

  it('leaves the text editor the files it has no preview of, and says why for the documents', () => {
    for (const f of ['src/app.ts', 'Makefile', '.env', 'a.json', 'archive.zip', 'x.bin']) expect(previewOf(f), f).toBeNull();
    for (const f of ['a.odt', 'b.pptx', 'c.rtf', 'd.doc', 'e.xlsx', 'f.ods', 'g.tiff']) expect(previewOf(f)?.kind, f).toBe('none');
    expect(NO_PREVIEW.xlsx).toBe('sheet');
    expect(NO_PREVIEW.rtf).toBe('legacy');
  });

  it('opens in its preview every family but the tables, which open as text first', () => {
    expect(opensInPreview('image')).toBe(true);
    expect(opensInPreview('markdown')).toBe(true);
    expect(opensInPreview('csv')).toBe(false);
  });

  it('reads an extension from the last dot of the name only', () => {
    expect(extOf('a.b/c')).toBe('');
    expect(extOf('dir.v2/archive.tar.GZ')).toBe('gz');
    expect(extOf('C:\\x\\Image.PNG')).toBe('png');
    expect(extOf('.gitignore')).toBe('');
  });

  it('gives the editor resolver a glob of both cases', () => {
    const g = globOf('pdf');
    expect(g).toBe('**/*.{pdf,PDF}');
    expect(globOf('image')).toContain('svg');
    expect(globOf('image')).toContain('PNG');
    expect(globOf('none')).toContain('xlsx');
  });
});

describe('the links of a document', () => {
  it('leads to a file of the source, from the folder of the document', () => {
    expect(linkOf('docs/guide.md', 'other.md')).toEqual({ to: 'file', rel: 'docs/other.md' });
    expect(linkOf('docs/guide.md', '../src/app.ts#L12')).toEqual({ to: 'file', rel: 'src/app.ts', line: 12 });
    expect(linkOf('docs/guide.md', './a%20b.md#Mon-titre')).toEqual({ to: 'file', rel: 'docs/a b.md', anchor: 'Mon-titre' });
    expect(linkOf('README.md', 'img.png?raw=1')).toEqual({ to: 'file', rel: 'img.png' });
  });

  it('never out of the source, nor by another scheme', () => {
    expect(linkOf('README.md', '../secret.txt')).toEqual({ to: 'none' });
    expect(linkOf('a/b.md', '../../../etc/passwd')).toEqual({ to: 'none' });
    expect(linkOf('README.md', '/etc/passwd')).toEqual({ to: 'none' });
    expect(linkOf('README.md', 'C:\\Windows\\win.ini')).toEqual({ to: 'none' });
    expect(linkOf('README.md', 'javascript:alert(1)')).toEqual({ to: 'none' });
    expect(linkOf('README.md', 'file:///etc/passwd')).toEqual({ to: 'none' });
    expect(linkOf('README.md', '')).toEqual({ to: 'none' });
  });

  it('opens the web and the mail outside, an anchor in the document', () => {
    expect(linkOf('README.md', 'https://example.com/x')).toEqual({ to: 'outside', href: 'https://example.com/x' });
    expect(linkOf('README.md', 'mailto:a@b.c')).toEqual({ to: 'outside', href: 'mailto:a@b.c' });
    expect(linkOf('README.md', '#installation')).toEqual({ to: 'anchor', anchor: 'installation' });
  });

  it('resolves a relative path in the source', () => {
    expect(resolveRel('a/b/c.md', '../d.md')).toBe('a/d.md');
    expect(resolveRel('c.md', 'x\\y.md')).toBe('x/y.md');
    expect(resolveRel('c.md', '..')).toBeNull();
  });
});

describe('the sanitizing of a document’s HTML', () => {
  it('runs no script: no script element, no event attribute, no javascript address, no SVG nor MathML in the page', () => {
    const el = dom(
      cleanHtml(
        '<script>alert(1)</script><img src="x.png" onerror="alert(2)"><a href="javascript:alert(3)">a</a>' +
          '<svg><script>alert(4)</script><circle onload="alert(5)"/></svg><math><mi>x</mi></math>' +
          '<iframe src="https://evil.example"></iframe><object data="x.swf"></object><embed src="x">' +
          '<div onclick="alert(6)">d</div>',
      ),
    );
    expect(el.querySelector('script, svg, math, iframe, object, embed')).toBeNull();
    expect(el.innerHTML).not.toMatch(/onerror|onclick|onload|javascript:/i);
    expect(el.querySelector('a')?.getAttribute('href')).toBeNull();
  });

  it('styles nothing of the app: no style element nor attribute, no link, base, meta or form', () => {
    const el = dom(
      cleanHtml(
        '<style>body{display:none}</style><link rel="stylesheet" href="https://x/y.css"><base href="https://x/">' +
          '<meta http-equiv="refresh" content="0;url=https://x"><p style="position:fixed;inset:0">p</p>' +
          '<form action="https://x"><input type="text" name="q"><button>b</button></form>',
      ),
    );
    expect(el.querySelector('style, link, base, meta, form, button, input[type=text]')).toBeNull();
    expect(el.querySelector('[style]')).toBeNull();
    expect(el.textContent).toContain('p');
  });

  it('loads no image by itself: its address is kept aside, by where it leads', () => {
    const el = dom(
      cleanHtml(
        '<img src="https://tracker.example/pixel.png?u=1" alt="web"><img src="docs/schema.png" alt="local">' +
          '<img src="http://plain.example/a.png"><img src="file:///C:/x.png"><img src="/abs.png">' +
          '<img src="data:image/png;base64,iVBORw0KGgo=" alt="inline"><img src="data:text/html;base64,PHNjcmlwdD4=">' +
          '<img srcset="https://x/a.png 2x">',
      ),
    );
    const imgs = [...el.querySelectorAll('img')];
    expect(imgs.map((i) => i.getAttribute('src'))).toEqual([
      null,
      null,
      null,
      null,
      null,
      'data:image/png;base64,iVBORw0KGgo=',
      null,
      null,
    ]);
    expect(imgs[0].dataset.remoteSrc).toBe('https://tracker.example/pixel.png?u=1');
    expect(imgs[1].dataset.localSrc).toBe('docs/schema.png');
    expect(imgs[2].dataset.imgOff).toBe('http://plain.example/a.png');
    expect(imgs[3].hasAttribute('data-img-off')).toBe(true);
    expect(imgs[4].hasAttribute('data-img-off')).toBe(true);
    expect(imgs[6].hasAttribute('data-img-off')).toBe(true);
    expect(el.querySelector('[srcset]')).toBeNull();
  });

  it('never takes the pane’s marks from the document', () => {
    const el = dom(
      cleanHtml('<img data-remote-src="https://evil.example/x.png" data-local-src="../../x.png"><span data-show-img>s</span>'),
    );
    expect(el.querySelector('img')?.getAttribute('data-remote-src')).toBeNull();
    expect(el.querySelector('img')?.hasAttribute('data-img-off')).toBe(true);
    expect(el.querySelector('[data-show-img]')).toBeNull();
  });

  it('keeps the links, without a target, and prefixes the ids', () => {
    const el = dom(cleanHtml('<a href="https://example.com" target="_blank">w</a><h2 id="location">t</h2>'));
    const a = el.querySelector('a')!;
    expect(a.getAttribute('href')).toBe('https://example.com');
    expect(a.hasAttribute('target')).toBe(false);
    expect(a.getAttribute('rel')).toBe('noopener noreferrer');
    expect(el.querySelector('h2')?.id).toBe('user-content-location');
  });

  it('keeps a task of a list, never to be ticked', () => {
    const el = dom(cleanHtml('<ul><li><input type="checkbox" checked> fait</li></ul>'));
    const box = el.querySelector('input')!;
    expect(box.type).toBe('checkbox');
    expect(box.disabled).toBe(true);
  });
});

describe('the Markdown preview', () => {
  it('renders GitHub’s Markdown, each block marked with the source line it starts at', () => {
    const el = dom(
      renderPreview('# Titre\n\nUn paragraphe.\n\n- un\n- deux\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```ts\nconst x = 1;\n```\n'),
    );
    expect(el.querySelector('h1')?.textContent).toBe('Titre');
    expect([...el.querySelectorAll('[data-line]')].map((e) => [e.tagName, (e as HTMLElement).dataset.line])).toEqual([
      ['H1', '0'],
      ['P', '2'],
      ['UL', '4'],
      ['TABLE', '7'],
      ['PRE', '11'],
    ]);
    expect(el.querySelector('pre')?.dataset.lang).toBe('ts');
  });

  it('gives each title its anchor, as GitHub does, the same title numbered', () => {
    const el = dom(renderPreview('## Mise en route !\n\n## Mise en route !\n'));
    expect([...el.querySelectorAll('h2')].map((h) => h.id)).toEqual(['user-content-mise-en-route-', 'user-content-mise-en-route--1']);
    expect(slugOf('Hello, World')).toBe('hello-world');
  });

  it('shows a front matter as its YAML, and counts its lines', () => {
    const el = dom(renderPreview('---\ntitle: x\n---\n# T\n'));
    expect(el.querySelector('pre.front-matter')?.textContent).toBe('title: x');
    expect(el.querySelector('h1')?.dataset.line).toBe('3');
  });

  it('keeps the raw HTML of a README, sanitized: a script in it does nothing', () => {
    const el = dom(
      renderPreview(
        '<p align="center"><img src="logo.png" onerror="alert(1)"></p>\n\n<script>alert(2)</script>\n\n<details><summary>Plus</summary>\n\nCaché\n\n</details>\n',
      ),
    );
    expect(el.querySelector('script')).toBeNull();
    expect(el.innerHTML).not.toMatch(/onerror/);
    expect(el.querySelector('img')?.dataset.localSrc).toBe('logo.png');
    expect(el.querySelector('details summary')?.textContent).toBe('Plus');
  });

  it('loads no image of the web, written in Markdown', () => {
    const el = dom(renderPreview('![pixel](https://tracker.example/p.png)\n'));
    const img = el.querySelector('img')!;
    expect(img.getAttribute('src')).toBeNull();
    expect(img.dataset.remoteSrc).toBe('https://tracker.example/p.png');
    expect(img.alt).toBe('pixel');
  });

  it('follows the source’s scroll between its blocks, both ways', () => {
    const blocks = [
      { line: 0, top: 0 },
      { line: 10, top: 200 },
      { line: 20, top: 300 },
    ];
    expect(scrollFor(0, blocks)).toBe(0);
    expect(scrollFor(5, blocks)).toBe(100);
    expect(scrollFor(15, blocks)).toBe(250);
    expect(scrollFor(40, blocks)).toBe(300);
    expect(lineFor(100, blocks)).toBe(5);
    expect(lineFor(250, blocks)).toBe(15);
    expect(scrollFor(3, [])).toBe(0);
  });
});

describe('a table of a CSV file', () => {
  it('reads quoted fields, doubled quotes and line breaks in a field', () => {
    expect(parseTable('a,b\n"x, y","il dit ""oui"""\n"deux\nlignes",3\n', ',').rows).toEqual([
      ['a', 'b'],
      ['x, y', 'il dit "oui"'],
      ['deux\nlignes', '3'],
    ]);
  });

  it('guesses the separator, a tab for a TSV, and skips a BOM', () => {
    expect(separatorOf('a;b;c\n1;2;3', false)).toBe(';');
    expect(separatorOf('a,b\n', false)).toBe(',');
    expect(separatorOf('a,b', true)).toBe('\t');
    expect(parseTable('\uFEFFa;b\r\n1;2', ';').rows).toEqual([
      ['a', 'b'],
      ['1', '2'],
    ]);
  });

  it('keeps a number of rows, and counts the rest', () => {
    const t = parseTable('1\n2\n3\n4\n', ',', 2);
    expect(t.rows).toEqual([['1'], ['2']]);
    expect(t.more).toBe(2);
  });
});
