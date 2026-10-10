// The texts of the interface, in French or English. `t('zone.group.key', params)` gives the text of a key in the
// language of the interface: it reads `locale.ui`, so a template or a `$derived` that calls it follows a change of
// language at once. A label computed once when a module loads would not: read texts where they are shown.
import { en } from './en';
import { fr } from './fr';
import { locale, type Lang } from './locale.svelte';
import type { Args, Key, PluralLeaf, Tree } from './types';

export { intlLocale, locale, setLang, type Lang } from './locale.svelte';
export type { Key, RichPart, RichProps } from './types';

type Leaf = string | PluralLeaf;
type Values = Record<string, string | number | undefined>;

const catalogs: Record<Lang, Tree> = { fr, en };

/** Each catalog flattened once, at its first text: a key is a lookup, not a walk. */
const flat = new Map<Lang, Map<string, Leaf>>();

function flatten(tree: Tree, prefix: string, out: Map<string, Leaf>): Map<string, Leaf> {
  for (const [k, v] of Object.entries(tree)) {
    if (typeof v === 'string' || isPlural(v)) out.set(prefix + k, v);
    else flatten(v, `${prefix}${k}.`, out);
  }
  return out;
}

const isPlural = (v: Leaf | Tree): v is PluralLeaf =>
  typeof (v as PluralLeaf).one === 'string' && typeof (v as PluralLeaf).other === 'string';

function leaf(lang: Lang, key: string): Leaf | undefined {
  let leaves = flat.get(lang);
  if (!leaves) flat.set(lang, (leaves = flatten(catalogs[lang], '', new Map())));
  return leaves.get(key);
}

const rules = new Map<Lang, Intl.PluralRules>();

/** The form of a plural for `count`. Only « one » and « other » are written: French « many » (a million) is « other ». */
function form(lang: Lang, text: Leaf, count: unknown): string {
  if (typeof text === 'string') return text;
  let r = rules.get(lang);
  if (!r) rules.set(lang, (r = new Intl.PluralRules(lang)));
  return typeof count === 'number' && r.select(count) === 'one' ? text.one : text.other;
}

/** Puts the values in their placeholders; one without a value stays as written (for `Rich`, or to be seen). */
function fill(text: string, values: Values | undefined): string {
  if (!values) return text;
  return text.replace(/\{(\w+)\}/g, (placeholder, name: string) => {
    const v = values[name];
    return v === undefined ? placeholder : String(v);
  });
}

function translate(lang: Lang, key: string, values: Values | undefined): string {
  // The English catalog cannot lack a key (it does not compile): French is only a safety net.
  const text = leaf(lang, key) ?? leaf('fr', key);
  return text === undefined ? key : fill(form(lang, text, values?.count), values);
}

/** The text of `key` in the language of the interface, its placeholders filled with `params` (required when it has some). */
export function t<K extends Key>(key: K, ...params: Args<K>): string {
  return translate(locale.ui, key, params[0] as Values | undefined);
}

/** The text of `key` in `lang`, whatever the language of the interface. */
export function tIn<K extends Key>(lang: Lang, key: K, ...params: Args<K>): string {
  return translate(lang, key, params[0] as Values | undefined);
}

/**
 * The text of `key` with its placeholders as written, for `Rich` to fill (with snippets, which are no text): the
 * form of a plural chosen by `count`, and `{count}` written.
 */
export function tRaw(key: Key, count?: number): string {
  return translate(locale.ui, key, count === undefined ? undefined : { count });
}
