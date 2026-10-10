// The types of the catalogs. The French catalog is the source: its texts, written `as const`, give the keys `t`
// accepts and the parameters each one needs; the English catalog is checked against it, key by key.
import type { Catalog } from './fr';

/** A text that depends on a count, chosen by `Intl.PluralRules`: French says « 0 fichier », English « 0 files ». */
export interface PluralLeaf {
  readonly one: string;
  readonly other: string;
}

/** A zone of a catalog, or a group in it: texts, plurals and groups, by key. */
export type Tree = { readonly [key: string]: string | PluralLeaf | Tree };

/** The `{name}` placeholders of a text. */
export type Vars<S> = S extends `${string}{${infer P}}${infer R}` ? P | Vars<R> : never;

/** The placeholders of a leaf: all forms of a plural, which always has `count` besides. */
type LeafVars<V> = V extends PluralLeaf ? Vars<V['one']> | Vars<V['other']> | 'count' : Vars<V>;

/** [dotted path, leaf] for every leaf of a tree. */
type Entries<T, P extends string> = {
  [K in keyof T & string]: T[K] extends string | PluralLeaf ? [`${P}${K}`, T[K]] : Entries<T[K], `${P}${K}.`>;
}[keyof T & string];

/** A catalog flattened: its leaves by dotted path (`editor.tabs.closeOthers`). Computed once for all the calls. */
export type Flat<T> = {
  [E in Entries<T, ''> as E extends [infer P extends string, unknown] ? P : never]: E extends [string, infer V] ? V : never;
};

type FrFlat = Flat<Catalog>;

/** A key of the catalog: the dotted path of one of its texts. */
export type Key = keyof FrFlat;

/** The values a leaf's placeholders take: a number for the count of a plural, a text or a number otherwise. */
export type Params<V> = { [P in LeafVars<V>]: P extends 'count' ? (V extends PluralLeaf ? number : string | number) : string | number };

/**
 * The parameters `t` takes for a key: none for a text without placeholders, all of them otherwise. Distributed
 * over `K`: an unknown key then fails as such (« not assignable to Key »), not as a count of arguments.
 */
export type Args<K extends Key> = K extends Key ? ([LeafVars<FrFlat[K]>] extends [never] ? [] : [params: Params<FrFlat[K]>]) : never;

/** The parameters `tRich` takes: the count of a plural, and any of the others (the rest is left to `Rich`). */
export type RichArgs<K extends Key> = K extends Key
  ? FrFlat[K] extends PluralLeaf
    ? [params: { count: number } & Partial<Params<FrFlat[K]>>]
    : [LeafVars<FrFlat[K]>] extends [never]
      ? []
      : [params?: Partial<Params<FrFlat[K]>>]
  : never;

// --- the English catalog, checked against the French one -------------------------------------------------------

/** Exactly the same placeholders. */
type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

/**
 * What the translation of the French `F` must be, for the given `E` (intersected with it: what is right stays `E`,
 * what is wrong meets a type that names the mistake in the error).
 */
type CheckNode<E, F> = F extends string
  ? E extends string
    ? Same<Vars<E>, Vars<F>> extends true
      ? string
      : { readonly placeholdersMustBe: Vars<F> }
    : string
  : F extends PluralLeaf
    ? E extends PluralLeaf
      ? // Compared over all forms: English « one file » may leave out the `{count}` French « {count} fichier » needs for 0.
        Same<Exclude<LeafVars<E>, 'count'>, Exclude<LeafVars<F>, 'count'>> extends true
        ? PluralLeaf & { [K in Exclude<keyof E, keyof PluralLeaf>]: { readonly notAPluralForm: K } }
        : { readonly placeholdersMustBe: Exclude<LeafVars<F>, 'count'> }
      : PluralLeaf
    : Check<E, F>;

/** Every key of `F`, checked, and none other. */
export type Check<E, F> = { [K in keyof F]: K extends keyof E ? CheckNode<E[K], F[K]> : F[K] } & {
  [K in Exclude<keyof E, keyof F>]: { readonly notInFrenchCatalog: K };
};

/** A function that returns its catalog as is, once checked against `F` (any French catalog: the tests use small ones). */
export function catalogChecker<F>() {
  return <const E extends Tree>(catalog: E & Check<E, F>): E => catalog;
}

/** Declares the English catalog: a key missing, a key in excess or other placeholders than the French ones fail to compile. */
export const defineCatalog = catalogChecker<Catalog>();

/** Declares a zone of the English catalog, checked against the same zone of the French one: errors show at their line. */
export function defineZone<Z extends keyof Catalog, const E extends Tree>(_zone: Z, zone: E & Check<E, Catalog[Z]>): E {
  return zone;
}
