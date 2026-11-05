import { readFileSync } from 'node:fs';
import { posix } from 'node:path';
import type { Lead } from './leads';

// The leads of a `run.wasm` plugin (ESC-337, docs/PLUGINS.md §8.5.5): what `findLeads` does not see in Rust. The store has
// refused most of it already (`rust::check`): a lead here says that something got past it, or where to read. Nothing runs.

/** The closed list of what may run in native at a compilation, as `escouade-plugin` holds it (docs/PLUGINS.md §8.17). */
export interface Native {
  name: string;
  version: string;
  checksum: string;
}

export const readNativeList = (): Native[] =>
  JSON.parse(readFileSync(new URL('../../../crates/escouade-plugin-cli/src/rust_native.json', import.meta.url), 'utf8')) as Native[];

const CRATES_IO = 'registry+https://github.com/rust-lang/crates.io-index';
const cut = (s: string) => {
  const t = s.trim();
  return t.length > 160 ? `${t.slice(0, 157)}…` : t;
};
const lineAt = (text: string, index: number) => text.slice(0, index).split('\n').length;

const isCode = (file: string) => file.endsWith('.rs');

/** A path a Rust source names: from its folder, absolute or climbing out of the plugin, as the compiler would read it. */
function leavesPlugin(file: string, path: string): string | null {
  if (/^(?:[\\/]|[A-Za-z]:|~)/.test(path)) return 'chemin absolu : il lit un fichier de la machine qui compile';
  const to = posix.normalize(posix.join(posix.dirname(file), path.replace(/\\\\?/g, '/')));
  return to === '..' || to.startsWith('../') ? 'chemin qui sort du dossier du plugin' : null;
}

/** `include_bytes!`, `include_str!`, `include!` and `#[path]`. */
function includeLeads(file: string, text: string): Lead[] {
  const out: Lead[] = [];
  const literal = /^\s*(?:r#*)?"((?:[^"\\]|\\.)*)"\s*,?\s*\)/;
  for (const m of text.matchAll(/\b(include_bytes|include_str|include)\s*!\s*\(/g)) {
    const line = lineAt(text, m.index);
    const shown = cut(text.split('\n')[line - 1]!);
    const path = literal.exec(text.slice(m.index + m[0].length))?.[1];
    if (path === undefined)
      out.push({ kind: 'includes', file, line, text: shown, why: `\`${m[1]}!\` d’un chemin calculé : on ne lit pas ce qu’il inclut` });
    else {
      const why = leavesPlugin(file, path);
      if (why) out.push({ kind: 'includes', file, line, text: shown, why: `\`${m[1]}!\` : ${why}` });
    }
  }
  for (const m of text.matchAll(/#\s*\[\s*path\s*=\s*"((?:[^"\\]|\\.)*)"/g)) {
    const why = leavesPlugin(file, m[1]!);
    const line = lineAt(text, m.index);
    if (why) out.push({ kind: 'includes', file, line, text: cut(text.split('\n')[line - 1]!), why: `\`#[path]\` : ${why}` });
  }
  for (const m of text.matchAll(/\binclude(?:_bytes|_str)?\s+as\s+\w+/g)) {
    const line = lineAt(text, m.index);
    out.push({ kind: 'includes', file, line, text: cut(text.split('\n')[line - 1]!), why: 'macro d’inclusion renommée' });
  }
  return out;
}

const UNSAFE: [RegExp, string][] = [
  [/\bunsafe\b/, '`unsafe`'],
  [/\bextern\s+"[^"]*"|\bextern\s+crate\b/, '`extern`'],
  [/#\s*\[\s*(?:unsafe\s*\(\s*)?(?:link\b|link_name\b|link_section\b|no_mangle\b|export_name\b)/, 'symbole ou lien choisi à la main'],
  [/\b(?:global_)?asm\s*!/, 'assembleur'],
  [/\btransmute\b/, '`transmute`'],
];

const SECRET_AT_BUILD = /\b(?:option_)?env\s*!\s*\(/;
const BIDI = /[‪-‮⁦-⁩]/;
const BYTES = /(?:0x[0-9a-f]{2}\s*,\s*){16}/i;

/** The lines of one source: `unsafe`, what it links, what the compilation's environment puts in it, what cannot be read. */
function sourceLeads(file: string, text: string): Lead[] {
  const out: Lead[] = [];
  text.split('\n').forEach((raw, i) => {
    const line = i + 1;
    if (BIDI.test(raw))
      out.push({
        kind: 'obscure',
        file,
        line,
        text: cut(raw),
        why: 'caractère qui inverse le sens du texte : le code lu n’est pas le code compilé',
      });
    if (BYTES.test(raw)) out.push({ kind: 'obscure', file, line, text: cut(raw), why: 'suite d’octets écrite à la main' });
    if (/^\s*\/\//.test(raw)) return;
    for (const [re, why] of UNSAFE) if (re.test(raw)) out.push({ kind: 'unsafe', file, line, text: cut(raw), why });
    if (SECRET_AT_BUILD.test(raw))
      out.push({ kind: 'secrets', file, line, text: cut(raw), why: 'variable lue à la compilation : sa valeur entre dans le module' });
  });
  return out;
}

/** What a `Cargo.toml` of the plugin says that would run or come from elsewhere; the store refuses it all. */
function manifestLeads(file: string, text: string): Lead[] {
  const out: Lead[] = [];
  text.split('\n').forEach((raw, i) => {
    const s = raw.replace(/#.*$/, '');
    const line = i + 1;
    const say = (kind: Lead['kind'], why: string) => out.push({ kind, file, line, text: cut(raw), why });
    if (/^\s*proc-macro\s*=\s*true/.test(s)) say('native', 'macro procédurale : elle tourne en natif à la compilation');
    if (/^\s*build\s*=/.test(s)) say('native', 'script de compilation nommé par le manifeste');
    if (/^\s*\[\s*(?:target\..*\.)?build-dependencies/.test(s)) say('native', 'dépendances d’un script de compilation');
    if (/^\s*\[\s*(?:patch|replace)\b/.test(s)) say('dependencies', '`[patch]` ou `[replace]`');
    if (/\bgit\s*=\s*"/.test(s)) say('dependencies', 'dépendance `git`');
    if (/\bregistry\s*=\s*"/.test(s)) say('dependencies', 'dépendance d’un autre registre');
    const path = /\bpath\s*=\s*"([^"]*)"/.exec(s)?.[1];
    if (path !== undefined && leavesPlugin(file, path)) say('dependencies', 'dépendance `path` qui sort du dossier du plugin');
  });
  return out;
}

export interface Package {
  name: string;
  version: string;
  source?: string;
  checksum?: string;
  line: number;
}

/** The `[[package]]` of a `Cargo.lock`. */
export function readLock(text: string): Package[] {
  const out: Package[] = [];
  let at: Package | null = null;
  text.split('\n').forEach((raw, i) => {
    if (raw.trim() === '[[package]]') {
      at = { name: '', version: '', line: i + 1 };
      out.push(at);
      return;
    }
    const m = /^(name|version|source|checksum)\s*=\s*"([^"]*)"/.exec(raw);
    if (m && at) at[m[1] as 'name'] = m[2]!;
  });
  return out;
}

/** The dependencies of a `Cargo.lock`, one by one, against crates.io and the closed list. */
function lockLeads(file: string, text: string, native: Native[]): Lead[] {
  const packages = readLock(text);
  const out: Lead[] = [];
  let listed = 0;
  let others = 0;
  for (const p of packages) {
    const shown = `${p.name} ${p.version}`;
    if (!p.source) continue;
    const say = (why: string) => out.push({ kind: 'dependencies', file, line: p.line, text: shown, why });
    if (p.source !== CRATES_IO) say(`source hors de crates.io : ${p.source}`);
    else if (!p.checksum) say('pas de somme de contrôle');
    const same = native.filter((n) => n.name === p.name);
    if (same.some((n) => n.version === p.version && n.checksum === p.checksum)) listed++;
    else {
      others++;
      say(
        same.length
          ? 'sur la liste fermée (`rust_native.json`) sous une autre version ou une autre somme : ne doit tourner qu’en WASM'
          : 'hors de la liste fermée (`rust_native.json`) : ne doit tourner qu’en WASM, code à relire',
      );
    }
  }
  const local = packages.length - listed - others;
  out.unshift({
    kind: 'dependencies',
    file,
    why: `${packages.length} paquets : ${listed} sur la liste fermée, ${others} hors liste, ${local} du plugin`,
  });
  return out;
}

/** What `Cargo.lock` adds or changes since the version before: code that runs and was not read then. */
export function lockChanges(before: string, now: string, file = 'Cargo.lock'): Lead[] {
  const known = new Map(readLock(before).map((p) => [p.name, p]));
  return readLock(now).flatMap((p): Lead[] => {
    const was = known.get(p.name);
    if (was && was.version === p.version && was.source === p.source && was.checksum === p.checksum) return [];
    return [
      {
        kind: 'dependencies',
        file,
        line: p.line,
        text: `${p.name} ${p.version}`,
        why: was
          ? `change depuis la version d’avant (${was.version}) : ce diff est à lire`
          : 'nouvelle depuis la version d’avant : son code est à lire',
      },
    ];
  });
}

/** One import of a module. */
export interface Import {
  module: string;
  name: string;
  kind: number;
}

/** The imports of a WASM module, from its import section; null for what is not one. */
export function moduleImports(bytes: Buffer): Import[] | null {
  if (bytes.length < 8 || bytes.readUInt32LE(0) !== 0x6d736100) return null;
  let at = 8;
  const leb = () => {
    let value = 0;
    for (let shift = 0; shift < 35; shift += 7) {
      if (at >= bytes.length) throw new RangeError('end of the module');
      const b = bytes[at++]!;
      value += (b & 0x7f) * 2 ** shift;
      if (!(b & 0x80)) return value;
    }
    throw new RangeError('a number too long');
  };
  const name = () => {
    const size = leb();
    if (at + size > bytes.length) throw new RangeError('end of the module');
    const text = bytes.toString('utf8', at, at + size);
    at += size;
    return text;
  };
  try {
    while (at < bytes.length) {
      const id = bytes[at++]!;
      const size = leb();
      if (at + size > bytes.length) return null;
      if (id !== 2) {
        at += size;
        continue;
      }
      const count = leb();
      const out: Import[] = [];
      for (let i = 0; i < count; i++) {
        const module = name();
        const field = name();
        const kind = bytes[at++]!;
        out.push({ module, name: field, kind });
        // What follows names a type: a function's, or the limits of a table, a memory, a global.
        if (kind === 0) leb();
        else if (kind === 1) {
          at++;
          if (leb() & 1) leb();
          leb();
        } else if (kind === 2) {
          const flags = leb();
          leb();
          if (flags & 1) leb();
        } else if (kind === 3) at += 2;
        else return null;
      }
      return out;
    }
    return [];
  } catch {
    return null;
  }
}

/** The host's methods a module can name, with the permission each family needs (docs/PLUGINS.md §8.4). */
const FAMILIES: { prefix: string[]; permission: string; given: (p: Record<string, unknown>) => boolean }[] = [
  { prefix: ['net', 'ws'], permission: 'permissions.network', given: (p) => filled(p.network) },
  { prefix: ['fs'], permission: 'permissions.files', given: (p) => filled(p.files) },
  { prefix: ['proc'], permission: 'permissions.commands', given: (p) => filled(p.commands) },
];
const filled = (v: unknown) => Array.isArray(v) && v.length > 0;
const METHOD =
  /(?<![\w/.-])(?:net\/(?:fetch|stream|credit|cancel)|ws\/(?:open|send|close)|fs\/(?:read|write|stat|list|canon)|proc\/(?:spawn|write|kill))(?![\w/.-])/g;
/** What wasmtime gives a module of WASI p1 here that reaches something: no folder is open, no socket exists. */
const WASI_REACH = /^(?:path_|sock_|fd_prestat)/;

/** The module's imports, and the methods written in it, against what the manifest declares. */
export function moduleLeads(file: string, bytes: Buffer, permissions: Record<string, unknown>): Lead[] {
  const imports = moduleImports(bytes);
  if (!imports) return [{ kind: 'obscure', file, why: 'les imports du module ne se lisent pas : ce n’est pas un module WASM bien formé' }];
  const out: Lead[] = [];
  const say = (why: string, text?: string) => out.push({ kind: 'imports', file, why, ...(text ? { text } : {}) });
  say(`${imports.length} imports : ${[...new Set(imports.map((i) => i.module))].join(', ') || 'aucun'}`);
  for (const i of imports) {
    const shown = `${i.module}.${i.name}`;
    if (i.module === 'escouade' ? i.name !== 'call' : i.module !== 'wasi_snapshot_preview1')
      say('import que l’hôte d’Escouade ne sert pas : le module ne se chargerait pas', shown);
    else if (WASI_REACH.test(i.name))
      say('appel WASI vers des fichiers ou des sockets : l’hôte n’ouvre aucun dossier ni aucune socket', shown);
  }
  const methods = [...new Set(bytes.toString('latin1').match(METHOD) ?? [])].sort();
  for (const family of FAMILIES) {
    const mine = methods.filter((m) => family.prefix.includes(m.split('/')[0]!));
    if (mine.length && !family.given(permissions))
      say(`le module nomme ${mine.join(', ')} sans ${family.permission} : ces appels seraient refusés (-32003)`);
    if (!mine.length && family.given(permissions))
      say(
        `${family.permission} est déclaré, et le module ne nomme aucune de ses méthodes : à quoi sert-il ? (un nom calculé ne se voit pas ici)`,
      );
  }
  return out;
}

/** The leads of a `run.wasm` plugin: `files` as the archive holds them, `module` the one the store published, if any. */
export function rustLeads(
  files: Map<string, Buffer>,
  module: Buffer | null,
  permissions: Record<string, unknown>,
  native: Native[],
): Lead[] {
  const out: Lead[] = [];
  for (const [file, bytes] of [...files].sort(([a], [b]) => a.localeCompare(b))) {
    const name = file.split('/').pop()!;
    if (name === 'build.rs') out.push({ kind: 'native', file, why: 'script de compilation : il tourne en natif à la compilation' });
    if (/^rust-toolchain(?:\.toml)?$/.test(name) || /(?:^|\/)\.cargo\//.test(file))
      out.push({ kind: 'native', file, why: 'configuration de cargo ou de la chaîne d’outils : le store la refuse' });
    if (name === 'Cargo.lock') out.push(...lockLeads(file, bytes.toString('utf8'), native));
    if (name === 'Cargo.toml') out.push(...manifestLeads(file, bytes.toString('utf8')));
    if (isCode(file)) {
      const text = bytes.toString('utf8');
      out.push(...includeLeads(file, text), ...sourceLeads(file, text));
    }
  }
  if (module) out.push(...moduleLeads('plugin.wasm', module, permissions));
  return out;
}
