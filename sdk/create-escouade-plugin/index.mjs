#!/usr/bin/env node
// npm create escouade-plugin: a new plugin of Escouade, from a template (ESC-181, PLG-9).
//
//   npm create escouade-plugin@latest [folder] -- [--kind program|declarative] [--id dev.you.name] [--name "Name"]
//
// In a folder of a git repository already there (several plugins in one repository), the manifest names the repository
// and the plugin's tags name its folder: `<folder>/v0.1.0`, never a bare `v…` (docs/STORE.md §3.6).
//
// `program` is a TypeScript program with its tests (a fact received, a permission decided); `declarative` is a
// theme, nothing that runs. Without --kind, it asks when it runs in a terminal, and takes `program` otherwise.
import { chmodSync, existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, relative, resolve, sep } from 'node:path';
import { userInfo } from 'node:os';
import { createInterface } from 'node:readline/promises';
import { fileURLToPath } from 'node:url';

/** The version of @escouade-dev/plugin a new plugin depends on (a test holds it to sdk/plugin/package.json). */
export const SDK_VERSION = '^0.2.0';
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

/** The git repository `folder` would be in (the first parent with a `.git`), or null. */
export function repositoryOf(folder) {
  for (let dir = resolve(folder, '..'); ; dir = dirname(dir)) {
    if (existsSync(join(dir, '.git'))) return dir;
    if (dirname(dir) === dir) return null;
  }
}

/** The GitHub address of a repository's `origin`, from its .git/config; null if it is none. */
function githubOf(root) {
  try {
    const config = readFileSync(join(root, '.git', 'config'), 'utf8');
    const url = /\[remote "origin"\][^[]*?url\s*=\s*(\S+)/.exec(config)?.[1] ?? '';
    const m = /github\.com[:/]([\w.-]+)\/([\w.-]+?)(?:\.git)?$/.exec(url);
    return m ? `https://github.com/${m[1]}/${m[2]}` : null;
  } catch {
    return null;
  }
}

/**
 * The tag of version `version` of a plugin in the folder `path` of its repository (docs/STORE.md §3.6, ESC-313): at
 * its root `v1.2.3`; in a folder of a repository with several, `<folder>/v1.2.3` (never a bare `v…`, which may be the
 * repository's own releases).
 */
export function tagOf(path, version) {
  return path ? `${path}/v${version}` : `v${version}`;
}

/** The folder of `folder` in its git repository, '' outside one. */
const folderIn = (folder) => {
  const root = repositoryOf(folder);
  return root ? relative(root, resolve(folder)).split(sep).join('/') : '';
};

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
  // In a folder of a repository already there (several plugins in one repository, ESC-313): the manifest names the
  // repository, and its tags name the folder.
  const root = repositoryOf(folder);
  const path = folderIn(folder);
  const github = root ? githubOf(root) : null;
  if (github) {
    const at = join(folder, 'escouade-plugin.json');
    const m = JSON.parse(readFileSync(at, 'utf8'));
    writeFileSync(at, `${JSON.stringify({ ...m, repository: github }, null, 2)}\n`);
  }
  const tag = tagOf(path, '0.1.0');
  const how = path
    ? `This plugin is the folder \`${path}\` of its repository. Its versions are tags that name the folder, never a bare \`v…\`: \`${tag}\` (or \`${basename(path)}@0.1.0\`). Submit it on https://store.escouade.dev/submit, folder \`${path}\`.`
    : 'Its versions are the repository’s tags, `v0.1.0` for the `version` of its manifest. Submit it on https://store.escouade.dev/submit.';
  const readme = join(folder, 'README.md');
  writeFileSync(
    readme,
    `${readFileSync(readme, 'utf8').trimEnd()}\n\n## Publish\n\n${how}\n\n\`\`\`sh\ngit tag ${tag} && git push origin ${tag}\n\`\`\`\n`,
  );
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
  const path = folderIn(folder);
  console.log(`\n  Publish: git tag ${tagOf(path, '0.1.0')}, then https://store.escouade.dev/submit${path ? ` (folder ${path})` : ''}`);
  console.log('\n  https://escouade.dev/en/docs/plugins/sdk');
  return 0;
}
