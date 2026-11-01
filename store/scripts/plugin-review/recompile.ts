import { Refused, type CompileOne } from '../../server/utils/compile';
import type { Compiler } from '../../server/utils/checks';
import type { VersionFile } from '../../server/utils/versions';

// The review compiles the bundle again (ESC-324, docs/STORE.md §4.2, step 2): the `run.js` bundle of the archive it
// read, by `escouade-plugin build` on the user's machine, of the very versions `version.json` names (`compiler`:
// Javy, escouade-js, escouade-plugin), then compares the module with the one the store published. The same SHA-256:
// what was read is what runs. Another: the review stops there, « Refuser », as for an archive changed. Not done (no
// `escouade-plugin` here, another version of it, a bundle it refuses): nothing can be signed until it is.

export type Recompiled =
  | { state: 'same'; sha256: string; compiler: Compiler }
  | { state: 'different'; published: string; local: string; compiler: Compiler }
  | { state: 'not-done'; why: string };

const same = (a: Compiler, b: Compiler) => a.javy === b.javy && a.engine === b.engine && a.prelude === b.prelude;
const words = (c: Compiler) => `Javy ${c.javy}, escouade-js ${c.engine.slice(0, 12)}…, escouade-plugin ${c.prelude}`;

/**
 * Compiles `bundle` again with `one` and compares its module with `published` (the `wasm` of `version.json`). `one` is
 * null when there is no `escouade-plugin` here.
 */
export async function recompile(
  one: CompileOne | null,
  input: { manifest: Buffer; js: string; bundle: Buffer; firstParty?: boolean },
  published: NonNullable<VersionFile['wasm']>,
): Promise<Recompiled> {
  if (!one)
    return {
      state: 'not-done',
      why: `pas d’\`escouade-plugin\` sur ce poste pour recompiler (ESCOUADE_PLUGIN, le PATH ou src-tauri/target) ; il faut celui de version.json : ${words(published.compiler)}`,
    };
  let got;
  try {
    got = await one(input);
  } catch (e) {
    const said = e instanceof Error ? e.message : String(e);
    return {
      state: 'not-done',
      why: e instanceof Refused ? `l’\`escouade-plugin\` de ce poste refuse le bundle : ${said}` : `la recompilation a échoué : ${said}`,
    };
  }
  if (!same(got.compiler, published.compiler))
    return {
      state: 'not-done',
      why: `l’\`escouade-plugin\` de ce poste compile avec ${words(got.compiler)} ; le store a compilé avec ${words(published.compiler)} : installe cet escouade-plugin (ESCOUADE_PLUGIN) et relance la revue`,
    };
  if (got.sha256 !== published.sha256)
    return { state: 'different', published: published.sha256, local: got.sha256, compiler: got.compiler };
  return { state: 'same', sha256: got.sha256, compiler: got.compiler };
}
