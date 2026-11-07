import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Runner } from '../../scripts/plugin-review/attest';
import { engineFolder, engineHelp, engineSteps, fetchEngine } from '../../scripts/plugin-review/engine';
import { recompile } from '../../scripts/plugin-review/recompile';
import { Refused, type CompileOne } from '../../server/utils/compile';

// escouade-js on the machine of the review (ESC-343): the review says how to get the one the store compiled with, and
// fetches it from the artifacts of main's CI, keeping nothing but this very fingerprint.

const sum = (b: Buffer) => createHash('sha256').update(b).digest('hex');
const GOOD = Buffer.from('the good escouade-js');
const OTHER = Buffer.from('another escouade-js');
const ENGINE = sum(GOOD);
const REPO = 'guillaume-gagnaire/escouade';

let root: string;
let at = 0;
beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), 'plugin-review-engine-'));
});
afterAll(() => rmSync(root, { recursive: true, force: true }));
const folder = () => join(root, `target-${at++}`);

/** A fake `gh`: `artifacts` as the API lists them, `modules` the file each run's artifact holds. */
function fakeGh(artifacts: unknown[], modules: Record<number, Buffer>, seen: string[][] = []): Runner {
  return (args) => {
    seen.push(args);
    if (args[0] === 'api') return { status: 0, stdout: JSON.stringify({ artifacts }), stderr: '' };
    const id = Number(args[2]);
    const module = modules[id];
    if (!module) return { status: 1, stdout: '', stderr: 'no artifact' };
    writeFileSync(join(args[args.indexOf('--dir') + 1]!, 'escouade-js.wasm'), module);
    return { status: 0, stdout: '', stderr: '' };
  };
}
const artifact = (id: number, branch = 'main', expired = false) => ({ expired, workflow_run: { id, head_branch: branch } });

describe('fetching escouade-js', () => {
  it('takes the first run of main whose artifact has the fingerprint version.json names', () => {
    const seen: string[][] = [];
    const to = folder();
    const got = fetchEngine(ENGINE, to, REPO, fakeGh([artifact(30), artifact(20), artifact(10)], { 30: OTHER, 20: GOOD, 10: GOOD }, seen));
    expect(got).toEqual({ state: 'fetched', run: 20 });
    expect(readFileSync(join(to, 'escouade-js.wasm'))).toEqual(GOOD);
    expect(seen[0]).toEqual(['api', `repos/${REPO}/actions/artifacts?name=escouade-js&per_page=100`]);
    expect(seen[1]).toEqual(['run', 'download', '30', '--repo', REPO, '--name', 'escouade-js', '--dir', expect.any(String)]);
    expect(seen).toHaveLength(3);
  });

  it('keeps nothing of another fingerprint, and leaves out other branches and expired artifacts', () => {
    const seen: string[][] = [];
    const to = folder();
    const got = fetchEngine(
      ENGINE,
      to,
      REPO,
      fakeGh([artifact(1, 'ticket/x'), artifact(2, 'main', true), artifact(3)], { 3: OTHER }, seen),
    );
    expect(got.state).toBe('not-found');
    expect(existsSync(join(to, 'escouade-js.wasm'))).toBe(false);
    expect(seen.filter((a) => a[0] === 'run').map((a) => a[2])).toEqual(['3']);
  });

  it('asks nothing when it is already there', () => {
    const to = folder();
    fetchEngine(ENGINE, to, REPO, fakeGh([artifact(1)], { 1: GOOD }));
    expect(fetchEngine(ENGINE, to, REPO, () => { throw new Error('gh called'); })).toEqual({ state: 'here' }); // prettier-ignore
  });

  it('says the way by hand when `gh` is not there or fails', () => {
    const none: Runner = () => ({ status: null, stdout: '', stderr: '', error: new Error('ENOENT') });
    const said = engineHelp(ENGINE, folder(), REPO, none);
    expect(said).toContain('`gh` introuvable');
    expect(said).toContain(engineSteps(ENGINE));
    const refused: Runner = () => ({ status: 1, stdout: '', stderr: 'gh auth login' });
    expect(engineHelp(ENGINE, folder(), REPO, refused)).toContain('gh auth login');
  });

  it('says what is left once it is fetched: cargo build', () => {
    const said = engineHelp(ENGINE, folder(), REPO, fakeGh([artifact(7)], { 7: GOOD }));
    expect(said).toContain('exécution 7');
    expect(said).toContain('empreinte vérifiée');
    expect(said).toContain('cd src-tauri && cargo build --bin escouade-plugin');
  });

  it('goes in src-tauri/escouade-js/target, where build.rs reads it', () => {
    expect(engineFolder('/r').replace(/\\/g, '/')).toMatch(/\/r\/src-tauri\/escouade-js\/target$/);
  });
});

describe('the recompilation says how to get escouade-js', () => {
  const compiler = { javy: '9.1.0', engine: ENGINE, prelude: '2.1.2' };
  const published = { sha256: 'a'.repeat(64), size: 1, compiler };
  const input = { manifest: Buffer.from('{}'), js: 'dist/plugin.mjs', bundle: Buffer.from('x') };

  it('an escouade-plugin built without it', async () => {
    const one: CompileOne = async () => {
      throw new Refused('dist/plugin.mjs does not compile to WASM: this Escouade was built without escouade-js (scripts/escouade-js.sh)');
    };
    const asked: string[] = [];
    const got = await recompile(one, input, published, (sha) => (asked.push(sha), 'la marche à suivre'));
    expect(asked).toEqual([ENGINE]);
    expect(got).toEqual({
      state: 'not-done',
      why: 'l’`escouade-plugin` de ce poste a été construit sans escouade-js ; la marche à suivre',
    });
  });

  it('an escouade-plugin built with another one, by default the way by hand; another version of the tool, not this', async () => {
    const other: CompileOne = async () => ({
      bytes: Buffer.alloc(1),
      sha256: 'b'.repeat(64),
      compiler: { ...compiler, engine: 'c'.repeat(64) },
    });
    const got = await recompile(other, input, published);
    expect(got.state).toBe('not-done');
    expect((got as { why: string }).why).toContain(engineSteps(ENGINE));

    const tool: CompileOne = async () => ({ bytes: Buffer.alloc(1), sha256: 'b'.repeat(64), compiler: { ...compiler, prelude: '2.1.9' } });
    const said = await recompile(tool, input, published, () => {
      throw new Error('the engine is the same');
    });
    expect((said as { why: string }).why).toContain('installe cet escouade-plugin');
  });

  it('a bundle the tool refuses stays the plugin’s', async () => {
    const one: CompileOne = async () => {
      throw new Refused('import of node:net refused');
    };
    const got = await recompile(one, input, published);
    expect((got as { why: string }).why).toBe('l’`escouade-plugin` de ce poste refuse le bundle : import of node:net refused');
  });
});
