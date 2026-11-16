// A Markdown file as its preview shows it (ESC-473): marked (GitHub's flavour), its raw HTML kept but sanitized
// (`cleanHtml`: no script, no style, no image loaded by itself), each block marked with the line of the source it starts
// at (`data-line`, 0-based) so that the preview follows the source's scroll, each title with an id for its anchors
// (`#mon-titre`, as GitHub writes them). The code blocks are `pre[data-lang]`, colored by the conversation's highlighter.

import { Marked, type Token, type Tokens, type TokensList } from 'marked';
import { cleanHtml } from './sanitize';

const escapeHtml = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** A title's anchor, as GitHub makes it: lower case, its punctuation off, its spaces dashes. */
export function slugOf(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/<[^>]*>/g, '')
    .replace(/[^\p{L}\p{N}\s_-]/gu, '')
    .replace(/\s/g, '-');
}

function marked(slugs: Map<string, number>) {
  return new Marked({
    gfm: true,
    breaks: false,
    renderer: {
      heading(this: { parser: { parseInline(t: Token[]): string } }, { tokens, depth, text }: Tokens.Heading) {
        const base = slugOf(text) || 'section';
        const n = slugs.get(base) ?? 0;
        slugs.set(base, n + 1);
        const id = n ? `${base}-${n}` : base;
        return `<h${depth} id="${escapeHtml(id)}">${this.parser.parseInline(tokens)}</h${depth}>\n`;
      },
      code({ text, lang }: Tokens.Code) {
        const l = (lang ?? '').trim().split(/\s+/)[0] ?? '';
        return `<pre data-lang="${escapeHtml(l)}"><code>${escapeHtml(text)}</code></pre>\n`;
      },
    },
  });
}

/** The front matter of a file (`---` … `---` at its top): shown as its YAML, not as a rule and a title. */
const FRONT = /^---\r?\n([\s\S]*?)\r?\n---[ \t]*(\r?\n|$)/;

const lines = (s: string) => (s.match(/\n/g) ?? []).length;

/** The first element of `html` marked with the source line `line`. */
function marked0(html: string, line: number): string {
  return html.replace(/^(\s*<[a-z][a-z0-9-]*)/i, `$1 data-line="${line}"`);
}

/** `text` rendered and sanitized, its blocks marked with their source lines. */
export function renderPreview(text: string): string {
  const slugs = new Map<string, number>();
  const md = marked(slugs);
  let out = '';
  let line = 0;
  let rest = text;
  const front = FRONT.exec(text);
  if (front) {
    out += `<pre data-lang="yaml" data-line="0" class="front-matter"><code>${escapeHtml(front[1])}</code></pre>\n`;
    line = lines(front[0]);
    rest = text.slice(front[0].length);
  }
  const tokens = md.lexer(rest);
  for (const token of tokens) {
    if (token.type !== 'space') {
      const one = Object.assign([token], { links: tokens.links }) as TokensList;
      out += marked0(md.parser(one), line);
    }
    line += lines(token.raw);
  }
  return cleanHtml(out);
}

/**
 * The scroll top that shows the source line `line` (0-based, may be a fraction) at the top of the preview: between the
 * blocks that start at or before it and after it, by their place in it (`blocks`: their lines and their tops, in order).
 */
export function scrollFor(line: number, blocks: { line: number; top: number }[]): number {
  if (!blocks.length) return 0;
  let i = 0;
  while (i + 1 < blocks.length && blocks[i + 1].line <= line) i++;
  const a = blocks[i];
  if (line <= a.line || i + 1 >= blocks.length) return a.top;
  const b = blocks[i + 1];
  return a.top + ((b.top - a.top) * (line - a.line)) / (b.line - a.line);
}

/** The source line shown at the top of the preview scrolled to `top`: the inverse of `scrollFor`. */
export function lineFor(top: number, blocks: { line: number; top: number }[]): number {
  if (!blocks.length) return 0;
  let i = 0;
  while (i + 1 < blocks.length && blocks[i + 1].top <= top) i++;
  const a = blocks[i];
  if (top <= a.top || i + 1 >= blocks.length) return a.line;
  const b = blocks[i + 1];
  return a.line + ((b.line - a.line) * (top - a.top)) / (b.top - a.top);
}
