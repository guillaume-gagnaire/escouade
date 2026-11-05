import { spawnSync } from 'node:child_process';
import { readdir, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import {
  compileTwice,
  compilerOf,
  inSandbox,
  Refused,
  whyNoBubblewrap,
  type CompileRust,
  type RustSandbox,
} from '../../server/utils/compile';

// Runs inside the store's image (scripts/image-test.mjs), regrouped with the store's own code: the compiler the image's
// variables name, as the store's start makes it, on the plugins of the repository (mounted at /plugins), which it
// compiles twice as the eleventh check does. Then a `run.wasm` plugin (ESC-336), the fixture mounted at /rust-build: its
// module, what is refused, and what its sandbox lets through. Without bubblewrap here, it says so and nothing compiles.

const one = compilerOf(process.env);
if (!one) throw new Error('no compiler: STORE_ESCOUADE_PLUGIN is not set');

const sandbox: RustSandbox = {
  bwrap: 'bwrap',
  root: process.env.STORE_COMPILE_ROOT!,
  program: '/usr/local/bin/escouade-plugin',
  toolchain: process.env.STORE_RUST_TOOLCHAIN!,
};
const why = whyNoBubblewrap(sandbox);
const can = !why;
// What a new /proc would have given here: the sandbox goes without one, because of what a container hides of its own.
const [exe0, args0, env0] = inSandbox(sandbox, '/tmp', false, {}, ['/bin/true']);
const tried = spawnSync(exe0, ['--proc', '/proc', ...args0], { encoding: 'utf8', env: env0 as NodeJS.ProcessEnv });
const withProc = tried.status === 0 ? null : tried.stderr.trim();
console.log(JSON.stringify({ bubblewrap: can, why, withProc }));
// image-test.mjs asks this alone of each set of options.
if (process.env.IMAGE_TEST_PROBE) process.exit(0);

for (const plugin of ['team-rules', 'slack']) {
  const folder = `/plugins/${plugin}`;
  const manifest = await readFile(`${folder}/escouade-plugin.json`);
  const js = (JSON.parse(manifest.toString('utf8')) as { run: { js: string } }).run.js;
  const bundle = await readFile(`${folder}/${js}`);
  const { compilation } = await compileTwice(one, { manifest, js, bundle, firstParty: true });
  console.log(JSON.stringify({ plugin, compilation }));
}

async function tree(dir: string, root = dir): Promise<[string, Buffer][]> {
  const found = await Promise.all(
    (await readdir(dir, { withFileTypes: true })).map(async (e): Promise<[string, Buffer][]> => {
      const at = join(dir, e.name);
      return e.isDirectory() ? tree(at, root) : [[relative(root, at), await readFile(at)]];
    }),
  );
  return found.flat();
}

const files = new Map(await tree('/rust-build'));
const rust = (more: [string, string][] = []): CompileRust => ({
  manifest: files.get('escouade-plugin.json')!,
  wasm: 'Cargo.toml',
  files: new Map([...files, ...more.map(([k, v]): [string, Buffer] => [k, Buffer.from(v)])]),
});

if (!can) {
  await one(rust()).then(
    () => console.log(JSON.stringify({ rust: 'compiled without a sandbox' })),
    (e: Error) => console.log(JSON.stringify({ rust: 'waits', said: e.message, plugin: e instanceof Refused })),
  );
} else {
  const started = Date.now();
  const { compilation } = await compileTwice(one, rust());
  console.log(JSON.stringify({ plugin: 'rust-build', compilation, seconds: Math.round((Date.now() - started) / 1000) }));

  /** What a compilation says of a plugin it must refuse; `Refused` alone counts, not a failure of the store. */
  const refusal = async (name: string, more: [string, string][]) => {
    const said = await one(rust(more)).then(
      () => 'compiled',
      (e: Error) => (e instanceof Refused ? e.message : `store: ${e.message}`),
    );
    console.log(JSON.stringify({ refused: name, said }));
  };
  const lib = files.get('src/lib.rs')!.toString();
  const lock = files.get('Cargo.lock')!.toString();
  await refusal('lock', [['Cargo.lock', lock.replace(/\[\[package\]\]\nname = "itoa"[\s\S]*?\n\n/, '')]]);
  await refusal('build.rs', [['build.rs', 'fn main() {}']]);
  await refusal('passwd', [['src/lib.rs', `${lib}\npub static P: &[u8] = include_bytes!("/etc/passwd");\n`]]);
  await refusal('store', [['src/lib.rs', `${lib}\npub static P: &[u8] = include_bytes!("/app/.output/server/index.mjs");\n`]]);

  // What the sandbox shows: the store's folders, a `/proc`, its variables and the network, from inside it.
  process.env.STORE_TEST_SECRET = 'seen';
  const inside = String.raw`
    echo "app $(test -e /app && echo there || echo absent)"
    echo "data $(test -e /data && echo there || echo absent)"
    echo "proc $(test -e /proc && echo there || echo absent)"
    echo "etc [$(ls /etc)]"
    echo "env [$(env | cut -d= -f1 | tr '\n' ' ')]"
    echo "net $(bash -c 'exec 3<>/dev/tcp/1.1.1.1/53' 2>&1 | grep -q 'Network is unreachable' && echo unreachable)"
  `;
  const [exe, args, env] = inSandbox(sandbox, '/tmp', false, { PATH: '/usr/bin:/bin' }, ['/bin/sh', '-c', inside]);
  const seen = spawnSync(exe, args, { encoding: 'utf8', env: env as NodeJS.ProcessEnv });
  console.log(JSON.stringify({ inside: seen.stdout.trim().split('\n'), code: seen.status, stderr: seen.stderr }));
}
