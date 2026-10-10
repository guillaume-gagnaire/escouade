import { describe, expect, it } from 'vitest';
import { en } from './en';
import { fr } from './fr';

type Leaf = string | { one: string; other: string };

/** Every leaf of a catalog, by its dotted path. */
function leaves(tree: object, prefix = '', out = new Map<string, unknown>()): Map<string, unknown> {
  for (const [k, v] of Object.entries(tree)) {
    const path = prefix + k;
    if (typeof v === 'string' || isPlural(v)) out.set(path, v);
    else if (v && typeof v === 'object') leaves(v, path + '.', out);
    else out.set(path, v);
  }
  return out;
}

const isPlural = (v: unknown): v is { one: string; other: string } =>
  !!v && typeof v === 'object' && 'one' in v && 'other' in v && typeof v.one === 'string';

/** The `{name}` placeholders of a leaf (all forms of a plural), `count` aside: a plural always has it. */
function placeholders(leaf: Leaf): string[] {
  const texts = typeof leaf === 'string' ? [leaf] : [leaf.one, leaf.other];
  const names = texts.flatMap((s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]));
  return [...new Set(names)].filter((n) => typeof leaf === 'string' || n !== 'count').sort();
}

const frLeaves = leaves(fr);
const enLeaves = leaves(en);

describe('catalogs', () => {
  it('have the same zones in both languages', () => expect(Object.keys(en).sort()).toEqual(Object.keys(fr).sort()));

  it('have exactly the same keys in both languages', () => {
    expect([...enLeaves.keys()].sort()).toEqual([...frLeaves.keys()].sort());
  });

  it('have the same placeholders for each key', () => {
    for (const [key, frLeaf] of frLeaves) {
      expect({ key, names: placeholders(enLeaves.get(key) as Leaf) }).toEqual({ key, names: placeholders(frLeaf as Leaf) });
    }
  });

  it('write each plural as { one, other } in both languages, and nothing else as an object', () => {
    for (const [key, leaf] of [...frLeaves, ...enLeaves]) {
      if (typeof leaf === 'string') continue;
      expect({ key, forms: Object.keys(leaf as object).sort() }).toEqual({ key, forms: ['one', 'other'] });
      expect({ key, plural: isPlural(enLeaves.get(key)) && isPlural(frLeaves.get(key)) }).toEqual({ key, plural: true });
    }
  });

  it('have no empty text', () => {
    for (const [key, leaf] of [...frLeaves, ...enLeaves]) {
      const texts = typeof leaf === 'string' ? [leaf] : isPlural(leaf) ? [leaf.one, leaf.other] : [leaf];
      for (const s of texts) expect({ key, empty: typeof s !== 'string' || s.trim() === '' }).toEqual({ key, empty: false });
    }
  });

  it('use braces for placeholders only', () => {
    // A stray « { » would be a placeholder for the types and plain text for `t`: neither is meant.
    for (const [key, leaf] of [...frLeaves, ...enLeaves]) {
      const texts = typeof leaf === 'string' ? [leaf] : [(leaf as { one: string }).one, (leaf as { other: string }).other];
      for (const s of texts) expect({ key, stray: /[{}]/.test(s.replace(/\{\w+\}/g, '')) }).toEqual({ key, stray: false });
    }
  });

  it('leave the name « k » to the key of Rich', () => {
    // `<Rich k="…">` takes its key in `k`, and a prop per placeholder: a `{k}` would be both.
    for (const [key, leaf] of frLeaves) expect({ key, k: placeholders(leaf as Leaf).includes('k') }).toEqual({ key, k: false });
  });

  it('name their keys in camelCase', () => {
    for (const key of frLeaves.keys()) for (const part of key.split('.')) expect({ key, part }).toEqual({ key, part: camel(part) });
  });
});

const camel = (s: string) => (/^[a-z][a-zA-Z0-9]*$/.test(s) ? s : `not camelCase: ${s}`);
