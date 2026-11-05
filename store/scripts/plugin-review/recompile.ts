import { Refused, type CompileOne } from '../../server/utils/compile';
import type { Compiler, JsCompiler } from '../../server/utils/checks';
import type { VersionFile } from '../../server/utils/versions';
import { engineSteps } from './engine';

// The review compiles the bundle again (ESC-324, docs/STORE.md §4.2, step 2): the `run.js` bundle of the archive it
// read, by `escouade-plugin build` on the user's machine, of the very versions `version.json` names (`compiler`:
// Javy, escouade-js, escouade-plugin), then compares the module with the one the store published. The same SHA-256:
// what was read is what runs. Another: the review stops there, « Refuser », as for an archive changed. Not done (no
// `escouade-plugin` here, another version of it, a bundle it refuses): nothing can be signed until it is. An
// `escouade-plugin` built without escouade-js, or with another one: the report says how to get the store's
// (`engine.ts`, ESC-343).

export type Recompiled =
  | { state: 'same'; sha256: string; compiler: JsCompiler }
  | { state: 'different'; published: string; local: string; compiler: JsCompiler }
  | { state: 'not-done'; why: string };

const isJs = (c: Compiler): c is JsCompiler => 'javy' in c;
const same = (a: JsCompiler, b: JsCompiler) => a.javy === b.javy && a.engine === b.engine && a.prelude === b.prelude;
const words = (c: JsCompiler) => `Javy ${c.javy}, escouade-js ${c.engine.slice(0, 12)}…, escouade-plugin ${c.prelude}`;

/**
 * Compiles `bundle` again with `one` and compares its module with `published` (the `wasm` of `version.json`). `one` is
 * null when there is no `escouade-plugin` here. `engine` says how to get the escouade-js of a fingerprint.
 */
export async function recompile(
  one: CompileOne | null,
  input: { manifest: Buffer; js: string; bundle: Buffer; firstParty?: boolean },
  module: NonNullable<VersionFile['wasm']>,
  engine: (sha256: string) => string = engineSteps,
): Promise<Recompiled> {
  const compiler = module.compiler;
  if (!isJs(compiler)) return { state: 'not-done', why: 'ce module vient de Rust : la revue ne le recompile pas encore' };
  const published = { sha256: module.sha256, compiler };
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
    if (/built without escouade-js/.test(said))
      return {
        state: 'not-done',
        why: `l’\`escouade-plugin\` de ce poste a été construit sans escouade-js ; ${engine(published.compiler.engine)}`,
      };
    return {
      state: 'not-done',
      why: e instanceof Refused ? `l’\`escouade-plugin\` de ce poste refuse le bundle : ${said}` : `la recompilation a échoué : ${said}`,
    };
  }
  const made = got.compiler;
  if (!isJs(made) || !same(made, published.compiler))
    return {
      state: 'not-done',
      why: !isJs(made)
        ? 'ce poste a compilé un module de Rust'
        : `l’\`escouade-plugin\` de ce poste compile avec ${words(made)} ; le store a compilé avec ${words(published.compiler)} : ${
            made.engine === published.compiler.engine
              ? 'installe cet escouade-plugin (ESCOUADE_PLUGIN) et relance la revue'
              : engine(published.compiler.engine)
          }`,
    };
  if (got.sha256 !== published.sha256) return { state: 'different', published: published.sha256, local: got.sha256, compiler: made };
  return { state: 'same', sha256: got.sha256, compiler: made };
}
