#!/usr/bin/env node
// The version of Escouade that serves the contract of the plugins of ESC-524 to ESC-531 (the profiles and the settings of
// each project, the jokers of permissions.commands, limits.processes): the `escouade.min` a plugin says when it takes one
// of them. Its one source is `x-escouadeMin.version` of schemas/plugin-protocol-1.json; the files below say it too, and
// scripts/escouade-min.test.ts fails when one of them says another.
//
//   node scripts/escouade-min.mjs            says it
//   node scripts/escouade-min.mjs 2.2.6      writes 2.2.6 everywhere it is said (the chef, at the release)
//
// Then: `npx prettier --write` on what changed is not needed (the same length of words), and the bundles of the test
// plugins are those esbuild makes again (sdk/plugin/src/both.test.ts checks them).
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const SOURCE = 'schemas/plugin-protocol-1.json';

/** Every file that says it, the source first. */
export const FILES = [
  SOURCE,
  'website/public/schemas/plugin-protocol-1.json',
  'crates/escouade-plugin-protocol/src/lib.rs',
  'crates/escouade-plugin-cli/src/manifest.rs',
  'crates/escouade-plugin-cli/src/cli.rs',
  'crates/escouade-plugin-kit/CHANGELOG.md',
  'crates/escouade-plugins/src/bench/fake.rs',
  'crates/escouade-plugins/src/bench/mod.rs',
  'crates/escouade-plugins/src/plugins_bench_tests.rs',
  'schemas/plugin-1.json',
  'website/public/schemas/plugin-1.json',
  'schemas/plugin-manifest.mjs',
  'sdk/plugin/vendor/plugin-manifest.mjs',
  'sdk/plugin/src/protocol.ts',
  'sdk/plugin/src/manifest.ts',
  'sdk/plugin/CHANGELOG.md',
  'sdk/plugin/bin/escouade-plugin.mjs',
  'sdk/plugin/src/both.test.ts',
  'sdk/doc-examples/ts/src/profiles.ts',
  'sdk/doc-examples/rust/src/profiles.rs',
  'src-tauri/src/plugins_contract_tests.rs',
  'tests/fixtures/plugins/sdk-contract/escouade-plugin.json',
  'tests/fixtures/plugins/sdk-contract/src/plugin.ts',
  'tests/fixtures/plugins/sdk-contract/dist/plugin.mjs',
  'tests/fixtures/plugins/sdk-both/dist/plugin.mjs',
  'tests/fixtures/plugins/sdk-net/dist/plugin.mjs',
  'docs/PLUGINS.md',
  ...['fr', 'en'].flatMap((lang) =>
    ['api', 'calls', 'capabilities', 'manifest', 'project-credentials', 'rust', 'sdk', 'testing', 'versioning'].map(
      (page) => `website/content/docs/${lang}/plugins/${page}.md`,
    ),
  ),
];

/** The version the source says. */
export function current(root = ROOT) {
  return JSON.parse(readFileSync(join(root, SOURCE), 'utf8'))['x-escouadeMin'].version;
}

const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** `text` with the version `from` written `to`, and its anchors (`2-2-5`) with it; a longer version (`2.2.50`) untouched. */
export function rewrite(text, from, to) {
  const word = (v) => new RegExp(`(?<![\\d.])${escape(v)}(?![\\d]|\\.\\d)`, 'g');
  return text.replace(word(from), to).replace(word(from.replace(/\./g, '-')), to.replace(/\./g, '-'));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const from = current();
  const to = process.argv[2];
  if (!to) {
    console.log(from);
  } else if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(to)) {
    console.error(`${to} is no version (2.2.6)`);
    process.exit(2);
  } else {
    for (const f of FILES) {
      const path = join(ROOT, f);
      if (!existsSync(path)) continue;
      const text = readFileSync(path, 'utf8');
      const next = rewrite(text, from, to);
      if (next !== text) writeFileSync(path, next);
    }
    console.log(`${from} → ${to} in ${FILES.length} files`);
  }
}
