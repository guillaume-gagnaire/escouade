import { readFile } from 'node:fs/promises';
import { compileTwice, compilerOf } from '../../server/utils/compile';

// Runs inside the store's image (scripts/image-test.mjs), regrouped with the store's own code: the compiler the image's
// variables name, as the store's start makes it, on the plugins of the repository (mounted at /plugins), which it
// compiles twice as the eleventh check does.

const one = compilerOf(process.env);
if (!one) throw new Error('no compiler: STORE_ESCOUADE_PLUGIN is not set');

for (const plugin of ['team-rules', 'slack']) {
  const folder = `/plugins/${plugin}`;
  const manifest = await readFile(`${folder}/escouade-plugin.json`);
  const js = (JSON.parse(manifest.toString('utf8')) as { run: { js: string } }).run.js;
  const bundle = await readFile(`${folder}/${js}`);
  const { compilation } = await compileTwice(one, { manifest, js, bundle, firstParty: true });
  console.log(JSON.stringify({ plugin, compilation }));
}
