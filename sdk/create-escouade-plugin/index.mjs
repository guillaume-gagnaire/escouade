#!/usr/bin/env node
// npm create escouade-plugin: a new plugin of Escouade, from a template (ESC-181, PLG-9).
//
//   npm create escouade-plugin@latest [folder] -- [--kind program|declarative] [--id dev.you.name] [--name "Name"]
//
// `program` is a TypeScript program with its tests (a fact received, a permission decided); `declarative` is a
// theme, nothing that runs. Without --kind, it asks when it runs in a terminal, and takes `program` otherwise.
import { chmodSync, existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { userInfo } from 'node:os';
import { createInterface } from 'node:readline/promises';
import { fileURLToPath } from 'node:url';

/** The version of @escouade-dev/plugin a new plugin depends on (a test holds it to sdk/plugin/package.json). */
export const SDK_VERSION = '^0.1.0';
export const KINDS = ['program', 'declarative'];
const TEMPLATES = join(dirname(fileURLToPath(import.meta.url)), 'templates');
const ID = /^[a-z0-9]+(?:-[a-z0-9]+)*(?:\.[a-z0-9]+(?:-[a-z0-9]+)*)+$/;

/** A folder's name made an id's last word, and a name. */
const slug = (s) =>
  s
    .toLowerCase()
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'plugin';
const title = (s) => s.replace(/[-_.]+/g, ' ').replace(/^\w/, (c) => c.toUpperCase());

/** Writes the template `kind` into `folder`; gives the files written. */
export function create({ folder, kind = 'program', id, name, sdk = SDK_VERSION, author = '' }) {
  if (!KINDS.includes(kind)) throw new Error(`--kind ${kind}: one of ${KINDS.join(', ')}`);
  const word = slug(basename(folder));
  id ??= `dev.${slug(userInfo().username)}.${word}`;
  if (!ID.test(id)) throw new Error(`--id ${id}: words of a-z, 0-9 and -, joined by dots, like dev.you.${word}`);
  name ??= title(word);
  if (existsSync(folder) && readdirSync(folder).length) throw new Error(`${folder} is not empty`);
  const values = { id, name, package: word, sdk, year: String(new Date().getFullYear()), author: author || name };
  // In a JSON file, each value is escaped as a JSON string's content.
  const fill = (text, json) =>
    text.replace(/\{\{(\w+)\}\}/g, (_, k) => (json ? JSON.stringify(values[k] ?? '').slice(1, -1) : (values[k] ?? '')));
  const written = [];
  const copy = (from, to) => {
    mkdirSync(to, { recursive: true });
    for (const entry of readdirSync(from)) {
      const src = join(from, entry);
      // npm drops a .gitignore from a package: the template keeps it as _gitignore.
      const dst = join(to, entry === '_gitignore' ? '.gitignore' : entry);
      if (statSync(src).isDirectory()) copy(src, dst);
      else {
        writeFileSync(dst, fill(readFileSync(src, 'utf8'), entry.endsWith('.json')));
        written.push(dst);
      }
    }
  };
  copy(join(TEMPLATES, kind), folder);
  // The program of run on macOS and Linux.
  if (kind === 'program') chmodSync(join(folder, 'plugin'), 0o755);
  return written;
}

/** The command: gives its exit code. */
export async function main(argv) {
  const option = (n) => {
    const i = argv.indexOf(n);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const values = new Set(['--kind', '--id', '--name', '--sdk', '--author'].flatMap((n) => [n, option(n)]));
  let folderArg = argv.find((a) => !a.startsWith('--') && !values.has(a));
  let kind = option('--kind');
  if ((!folderArg || !kind) && process.stdin.isTTY) {
    const rl = createInterface({ input: process.stdin, output: process.stdout });
    folderArg ||= (await rl.question('Folder of the plugin (my-plugin): ')).trim() || 'my-plugin';
    if (!kind) {
      const answer = (await rl.question('A TypeScript program, or a declarative theme? (program/declarative, program): ')).trim();
      kind = answer.startsWith('d') ? 'declarative' : 'program';
    }
    rl.close();
  }
  const folder = resolve(folderArg ?? 'my-plugin');
  try {
    create({
      folder,
      kind: kind ?? 'program',
      id: option('--id'),
      name: option('--name'),
      sdk: option('--sdk'),
      author: option('--author'),
    });
  } catch (e) {
    console.error(`✗ ${e.message}`);
    return 1;
  }
  const rel = folderArg ?? 'my-plugin';
  console.log(`✓ ${rel}: a ${kind ?? 'program'} plugin of Escouade\n`);
  console.log(`  cd ${rel}\n  npm install`);
  if ((kind ?? 'program') === 'program') console.log('  npm test          the handlers, against the fake Escouade');
  console.log('  npm run validate  the manifest');
  console.log('  npm run bench     the conformance bench (with Escouade installed)');
  console.log('\n  https://escouade.dev/en/docs/plugins/sdk');
  return 0;
}
