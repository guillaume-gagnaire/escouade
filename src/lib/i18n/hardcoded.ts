// The guard against texts written in the code: what the interface shows passes through `t`, and a test fails on a
// text left in a component or a module (any file of `src/`, the exceptions of `hardcoded-allow.ts` apart). Read by the
// tests only, on the sources as written.
//   - markup: a text node with a letter; an attribute read by people (`title`, `aria-label`, and the props of this
//     app's components that are shown: `desc`, `hint`…) with a letter, any other attribute with an accented letter,
//     any prop of a component with two words (`message="Rien a faire"`); a string an expression of the markup shows
//     (`{ok ? 'Oui' : 'Non'}`);
//   - scripts and modules: a string with a character of French (an accent, ’, « »), anywhere but in comments; any
//     call to the deprecated `plural()`, whose words have no accent (`plural(n, 'ticket', 'tickets')`).
// A guard, not a proof: a French string without accents in a script goes through. `hardcoded-allow.ts` lists the
// texts that are no translation (proper nouns, symbols).
import { parse } from 'svelte/compiler';

export interface Finding {
  /** Path from the root of the repository, with `/`. */
  file: string;
  line: number;
  /** `text`: shown in the markup; `attribute`: the value of an attribute; `string`: a string of a script. */
  kind: 'text' | 'attribute' | 'string';
  /** The text, its spaces collapsed; `${…}` for what a template string puts in it. */
  text: string;
}

/** A text the guard lets through: in `file` (or `*`, any file), exactly `text`, for the reason `why`. */
export interface Allowed {
  file: string;
  text: string;
  why: string;
}

/** A character only French writes, among those of the interface. */
const FRENCH = /[àâäçéèêëîïôöùûüÿœæÀÂÄÇÉÈÊËÎÏÔÖÙÛÜŸŒÆ’«»]/;
const LETTER = /\p{L}/u;

/** The attributes people read, and the props this app's components show (`<Row desc>`, `<Dropdown caption>`): a letter in them is a text. */
const READ = new Set([
  'title',
  'aria-label',
  'aria-description',
  'aria-roledescription',
  'aria-valuetext',
  'placeholder',
  'alt',
  'label',
  'desc',
  'note',
  'hint',
  'caption',
]);

/** Two words: in a prop of a component, a sentence (`class` and `style` aside, which hold names). */
const WORDS = /\p{L}\s+\p{L}/u;
const NAMES = new Set(['class', 'style', 'id']);

/** The fields of the markup's nodes that hold markup (an expression's are read for strings only). */
const MARKUP = ['fragment', 'nodes', 'consequent', 'alternate', 'body', 'fallback', 'pending', 'then', 'catch'];

type Node = { type: string; start: number; end: number; [field: string]: unknown };

const isNode = (v: unknown): v is Node => !!v && typeof v === 'object' && typeof (v as Node).type === 'string';

/** The texts written in the code of `source`, a `.svelte` component or a `.ts` module. */
export function scan(file: string, source: string): Finding[] {
  const component = file.endsWith('.svelte');
  // A module is read as the script of a component (same parser, same nodes). « </script> » in one of its strings
  // would close it: written otherwise here. Neither adds a line: the lines are those of the source.
  const parsed = component ? source : `<script lang="ts">${source.split('</script').join('<\\/script')}</script>`;
  const found = new Map<string, Finding & { at: number }>();
  const lineAt = (at: number) => parsed.slice(0, at).split('\n').length;
  const add = (kind: Finding['kind'], raw: string, at: number) => {
    const text = raw.replace(/\s+/g, ' ').trim();
    const start = at + raw.length - raw.trimStart().length;
    const key = `${start}:${text}`;
    // The same string seen as shown and as accented is one finding.
    if (!found.has(key)) found.set(key, { file, line: lineAt(start), kind, text, at: start });
  };

  /** Every string of a script or of the markup's expressions with a French character. */
  const strings = (node: unknown): void => {
    if (Array.isArray(node)) return node.forEach(strings);
    if (!isNode(node)) return;
    if (node.type === 'Literal' && typeof node.value === 'string') {
      if (FRENCH.test(node.value)) add('string', node.value, node.start);
      return;
    }
    if (node.type === 'TemplateLiteral' && FRENCH.test(template(node))) add('string', template(node), node.start);
    // `plural()` writes French only (`n > 1`): each call is a text to extract, accented or not.
    if (node.type === 'CallExpression' && isNode(node.callee) && node.callee.type === 'Identifier' && node.callee.name === 'plural') {
      add('string', parsed.slice(node.start, node.end), node.start);
    }
    for (const [field, v] of Object.entries(node)) if (field !== 'metadata' && field !== 'loc' && v && typeof v === 'object') strings(v);
  };

  /** The strings an expression shows: itself, the branches of a condition, the parts of a `+`, a template string. */
  const shown = (e: unknown): void => {
    if (!isNode(e)) return;
    switch (e.type) {
      case 'Literal':
        if (typeof e.value === 'string' && LETTER.test(e.value)) add('text', e.value, e.start);
        return;
      case 'TemplateLiteral':
        if (LETTER.test(template(e).replaceAll('${…}', ''))) add('text', template(e), e.start);
        return (e.expressions as unknown[]).forEach(shown);
      case 'ConditionalExpression':
        shown(e.consequent);
        return shown(e.alternate);
      case 'LogicalExpression':
        shown(e.left);
        return shown(e.right);
      case 'BinaryExpression':
        if (e.operator === '+') {
          shown(e.left);
          shown(e.right);
        }
        return;
      case 'TSAsExpression':
      case 'TSSatisfiesExpression':
      case 'TSNonNullExpression':
        return shown(e.expression);
    }
  };

  /** An attribute of an element, or a prop of a component (`<Row>`, `<settings.Row>`). */
  const attribute = (a: Node, component: boolean) => {
    if (a.value === true) return;
    const name = a.name as string;
    const read = READ.has(name);
    for (const part of Array.isArray(a.value) ? a.value : [a.value]) {
      if (!isNode(part)) continue;
      if (part.type === 'Text') {
        const data = part.data as string;
        const text = read ? LETTER.test(data) : FRENCH.test(data) || (component && !NAMES.has(name) && WORDS.test(data));
        if (text) add('attribute', data, part.start);
      } else if (part.type === 'ExpressionTag' && read) shown(part.expression);
    }
  };

  const markup = (node: unknown): void => {
    if (Array.isArray(node)) return node.forEach(markup);
    if (!isNode(node)) return;
    if (node.type === 'Text') {
      if (LETTER.test(node.data as string)) add('text', node.data as string, node.start);
    } else if (node.type === 'ExpressionTag' || node.type === 'HtmlTag') shown(node.expression);
    else {
      for (const a of (node.attributes as unknown[] | undefined) ?? [])
        if (isNode(a) && a.type === 'Attribute') attribute(a, node.type === 'Component');
      for (const field of MARKUP) markup(node[field]);
    }
  };

  const ast = parse(parsed, { modern: true });
  strings([ast.module, ast.instance, ast.fragment]);
  if (component) markup(ast.fragment);
  return [...found.values()].sort((a, b) => a.at - b.at).map(({ at: _, ...f }) => f);
}

/** A template string as written, `${…}` for each of its expressions. */
function template(node: Node): string {
  return (node.quasis as { value: { cooked: string | null; raw: string } }[]).map((q) => q.value.cooked ?? q.value.raw).join('${…}');
}

/** The findings no exception lets through. */
export function notAllowed(found: Finding[], allowed: Allowed[]): Finding[] {
  return found.filter((f) => !allowed.some((a) => a.text === f.text && (a.file === '*' || a.file === f.file)));
}
