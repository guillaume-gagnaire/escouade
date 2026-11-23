#!/usr/bin/env node
// Checks that no crate of a published build has the `testing` feature on (SEC-18 of the security audit, ESC-594).
// `testing` holds what only the tests need (secrets kept in memory, fake services, harness programs not looked up): it
// must come only through dev-dependencies. A normal dependency that turned it on would put it in the published binary
// without anything else saying so. Run by the CI (Windows lint) and by the release, before the build:
//   node scripts/no-testing.mjs
// It reads `cargo tree` of the app's workspace (every binary the release builds: escouade, escouade-hook,
// escouade-plugin), for every target, without the dev-dependencies, features shown.

import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

/** The arguments of the `cargo tree` this check reads: normal and build dependencies only, all targets. */
export const TREE = ['tree', '--workspace', '--locked', '--target', 'all', '-e', 'normal,build,features', '--prefix', 'none'];

/** The crates that `cargo tree` (features shown) says have `testing` on, sorted, each once. */
export function withTesting(tree) {
  const found = new Set();
  for (const line of String(tree).split(/\r?\n/)) {
    const m = /^\s*(\S+) feature "testing"/.exec(line);
    if (m) found.add(m[1]);
  }
  return [...found].sort();
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const run = spawnSync('cargo', TREE, {
    cwd: path.join(ROOT, 'src-tauri'),
    encoding: 'utf8',
    maxBuffer: 256 * 1024 * 1024,
  });
  if (run.status !== 0) {
    console.error(run.stderr);
    console.error('::error::cargo tree failed');
    process.exit(1);
  }
  const crates = withTesting(run.stdout);
  if (crates.length > 0) {
    for (const c of crates) {
      console.error(`::error::${c} has the \`testing\` feature on in a normal build: it would enter the published app`);
    }
    process.exit(1);
  }
  console.log('No crate of the build has the `testing` feature on.');
}
