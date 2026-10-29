#!/usr/bin/env node
// escouade-plugin, the command of @escouade-dev/plugin (ESC-181, PLG-9).
//
// `validate` and `pack` run here, in Node: the manifest's checks are those of the store and the app
// (vendor/plugin-manifest.mjs, copied from schemas/). `test` (the conformance bench) and `dev` (the fake Escouade)
// are the app's own: they go to the escouade-plugin that ships beside Escouade, found by ESCOUADE_PLUGIN_BIN, the
// PATH or where Escouade installs it, so that a plugin is judged by the same bench as in the app.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, lstatSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, join, relative, resolve, sep } from 'node:path';
import { gzipSync } from 'node:zlib';
import { message, validateText } from '../vendor/plugin-manifest.mjs';
import { nativeBin } from './native.mjs';

const FILE = 'escouade-plugin.json';
const TARGET = /^(windows|macos|linux)-(x64|arm64)$/;
/** Never in an archive: the repository, the dependencies, the archives themselves. */
const SKIP = new Set(['.git', 'node_modules', 'packs']);

const args = process.argv.slice(2);
const option = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const lang = (option('--lang') ?? process.env.LC_ALL ?? process.env.LANG ?? Intl.DateTimeFormat().resolvedOptions().locale ?? 'en')
  .toLowerCase()
  .startsWith('fr')
  ? 'fr'
  : 'en';
const t = (fr, en) => (lang === 'fr' ? fr : en);

const USAGE = t(
  `escouade-plugin : écrire et tester un plugin d'Escouade.

  escouade-plugin validate [dossier]       le manifeste, champ par champ
  escouade-plugin test [dossier] [--json] [--measure [n]]
                                           le banc de conformité (celui d'Escouade)
  escouade-plugin dev [dossier] [--secret <clé>=<valeur>]...
                                           le faux Escouade : des faits, des décisions, des appels tapés
  escouade-plugin pack [dossier] [--target <système>-<arch>] [--out <dossier>]
                                           l'archive .tar.gz et son empreinte SHA-256
                                           (--target : windows-x64, macos-arm64, linux-x64…)

test et dev lancent l'escouade-plugin livré avec Escouade (ESCOUADE_PLUGIN_BIN pour un autre).
Options : --lang fr|en
`,
  `escouade-plugin: write and test an Escouade plugin.

  escouade-plugin validate [folder]        the manifest, field by field
  escouade-plugin test [folder] [--json] [--measure [n]]
                                           the conformance bench (Escouade's own)
  escouade-plugin dev [folder] [--secret <key>=<value>]...
                                           the fake Escouade: facts, decisions and calls you type
  escouade-plugin pack [folder] [--target <system>-<arch>] [--out <folder>]
                                           the .tar.gz archive and its SHA-256
                                           (--target: windows-x64, macos-arm64, linux-x64…)

test and dev run the escouade-plugin that ships with Escouade (ESCOUADE_PLUGIN_BIN for another).
Options: --lang fr|en
`,
);

/** The folder: the first word after the command that is not an option or an option's value. */
function folderOf() {
  const withValue = new Set(['--lang', '--target', '--out', '--secret', '--measure']);
  for (let i = 1; i < args.length; i++) {
    if (withValue.has(args[i])) {
      if (args[i] !== '--measure' || /^\d+$/.test(args[i + 1] ?? '')) i++;
      continue;
    }
    if (!args[i].startsWith('--')) return resolve(args[i]);
  }
  return process.cwd();
}

/** The manifest of `folder`, or null after saying what is wrong with it. */
function manifestOf(folder) {
  const path = join(folder, FILE);
  let text;
  try {
    text = readFileSync(path, 'utf8');
  } catch (e) {
    console.error(`${path}: ${e.message}`);
    return null;
  }
  const v = validateText(text);
  for (const f of v.ignored) console.error(t(`champ ignoré : ${f}`, `ignored field: ${f}`));
  if (!v.ok) {
    for (const e of v.errors) console.error(`✗ ${message(e, lang)}`);
    return null;
  }
  return JSON.parse(text);
}

function validate(folder) {
  const m = manifestOf(folder);
  if (!m) return 1;
  // What the manifest's run names must be in the folder (the store checks it too).
  const run = m.run ?? {};
  const missing = ['command', 'windows', 'macos', 'linux']
    .map((k) => run[k])
    .filter((p) => p && p !== '{escouade}' && !existsSync(join(folder, p)));
  for (const p of missing) console.error(t(`✗ run : ${p} n'est pas dans le dossier`, `✗ run: ${p} is not in the folder`));
  if (missing.length) return 1;
  console.log(t(`✓ ${m.id} ${m.version} : le manifeste est valide`, `✓ ${m.id} ${m.version}: the manifest holds`));
  return 0;
}

function native(command, folder) {
  const bin = nativeBin();
  if (!bin) {
    console.error(
      t(
        `escouade-plugin ${command} joue le banc d'Escouade, livré avec l'app : installe Escouade (https://escouade.dev), ou donne son chemin dans ESCOUADE_PLUGIN_BIN.`,
        `escouade-plugin ${command} plays Escouade's bench, which ships with the app: install Escouade (https://escouade.dev), or give its path in ESCOUADE_PLUGIN_BIN.`,
      ),
    );
    return 2;
  }
  const rest = args.slice(1).filter((a) => resolve(a) !== folder);
  const r = spawnSync(bin, [command, folder, ...rest, ...(args.includes('--lang') ? [] : ['--lang', lang])], { stdio: 'inherit' });
  if (r.error) {
    console.error(`${bin}: ${r.error.message}`);
    return 2;
  }
  return r.status ?? 1;
}

/** Every file of the plugin, sorted, as paths with `/`; a symbolic link refuses the archive. */
function filesOf(folder, out) {
  const files = [];
  const walk = (dir) => {
    for (const name of readdirSync(dir).sort()) {
      const path = join(dir, name);
      if (resolve(path) === out || (dir === folder && SKIP.has(name)) || name === '.git' || name === 'node_modules') continue;
      const s = lstatSync(path);
      if (s.isSymbolicLink())
        throw new Error(
          t(
            `${relative(folder, path)} est un lien symbolique : le store le refuse`,
            `${relative(folder, path)} is a symbolic link: the store refuses it`,
          ),
        );
      if (s.isDirectory()) walk(path);
      else if (s.isFile()) files.push(relative(folder, path).split(sep).join('/'));
    }
  };
  walk(folder);
  return files;
}

/** A ustar header: the same bytes for the same files, whatever the system and the hour. */
function header(name, size, mode) {
  const h = Buffer.alloc(512);
  let prefix = '';
  let base = name;
  if (Buffer.byteLength(name) > 100) {
    const cut = name.lastIndexOf('/', 154);
    prefix = name.slice(0, cut);
    base = name.slice(cut + 1);
    if (cut < 0 || Buffer.byteLength(base) > 100 || Buffer.byteLength(prefix) > 155) throw new Error(`${name}: path too long`);
  }
  const put = (text, at, len) => h.write(text, at, len, 'utf8');
  const octal = (n, at, len) => put(n.toString(8).padStart(len - 1, '0') + '\0', at, len);
  put(base, 0, 100);
  octal(mode, 100, 8);
  octal(0, 108, 8);
  octal(0, 116, 8);
  octal(size, 124, 12);
  octal(0, 136, 12);
  put('        ', 148, 8);
  put('0', 156, 1);
  put('ustar\0', 257, 6);
  put('00', 263, 2);
  put(prefix, 345, 155);
  let sum = 0;
  for (const b of h) sum += b;
  put(sum.toString(8).padStart(6, '0') + '\0 ', 148, 8);
  return h;
}

function pack(folder) {
  const m = manifestOf(folder);
  if (!m) return 1;
  const target = option('--target');
  if (target !== undefined && !TARGET.test(target)) {
    console.error(
      t(
        `--target ${target} : un système et une architecture, comme linux-x64`,
        `--target ${target}: a system and an architecture, like linux-x64`,
      ),
    );
    return 2;
  }
  const out = resolve(option('--out') ?? join(folder, 'packs'));
  const run = m.run ?? {};
  const programs = new Set(['command', 'windows', 'macos', 'linux'].map((k) => run[k]).filter(Boolean));
  let files;
  try {
    files = filesOf(folder, out);
  } catch (e) {
    console.error(`✗ ${e.message}`);
    return 1;
  }
  const parts = [];
  for (const f of files) {
    const bytes = readFileSync(join(folder, f));
    // Executable: a program of run, or a script that starts with #! (Windows keeps no such bit).
    const exec = programs.has(f) || bytes.subarray(0, 2).equals(Buffer.from('#!')) || (statSync(join(folder, f)).mode & 0o111) !== 0;
    parts.push(header(f, bytes.length, exec ? 0o755 : 0o644), bytes, Buffer.alloc((512 - (bytes.length % 512)) % 512));
  }
  parts.push(Buffer.alloc(1024));
  const archive = gzipSync(Buffer.concat(parts), { level: 9 });
  // gzip writes the system it ran on (byte 9): the same archive from Windows, macOS or Linux.
  archive[9] = 255;
  const name = `${m.id}-${m.version}${target ? `-${target}` : ''}.tar.gz`;
  mkdirSync(out, { recursive: true });
  const path = join(out, name);
  writeFileSync(path, archive);
  const sha256 = createHash('sha256').update(archive).digest('hex');
  writeFileSync(`${path}.sha256`, `${sha256}  ${name}\n`);
  if (archive.length > 50 * 1024 * 1024) {
    console.error(
      t(`✗ ${name} : ${archive.length} octets, au-delà des 50 Mo du store`, `✗ ${name}: ${archive.length} bytes, past the store's 50 MB`),
    );
    return 1;
  }
  const shown = relative(process.cwd(), path);
  console.log(shown.startsWith('..') ? path : shown || basename(path));
  console.log(t(`  ${files.length} fichiers, ${archive.length} octets`, `  ${files.length} files, ${archive.length} bytes`));
  console.log(`  sha256 ${sha256}`);
  return 0;
}

function main() {
  const command = args[0];
  if (!command) {
    process.stderr.write(USAGE);
    return 2;
  }
  if (['-h', '--help', 'help'].includes(command)) {
    process.stdout.write(USAGE);
    return 0;
  }
  const folder = folderOf();
  switch (command) {
    case 'validate':
      return validate(folder);
    case 'pack':
      return pack(folder);
    case 'test':
    case 'dev':
      return native(command, folder);
    default:
      process.stderr.write(USAGE);
      return 2;
  }
}

process.exitCode = main();
