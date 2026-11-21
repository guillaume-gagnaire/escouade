// @escouade-dev/plugin/testing: a fake Escouade in memory, for a plugin's unit tests (ESC-181, PLG-9).
//
// It speaks the protocol to the plugin as the app does (the same lines, the same subjects as the conformance
// bench), without a process: `initialize`, facts, decisions, calls, `shutdown`; it serves `secrets/get`, `settings/get` and
// `files/read` (its `files`), and keeps what the plugin logged. Given the plugin's manifest (`manifest`), it serves as the
// app serves them (ESC-531): the profiles and their secrets (`profiles/list`, `profiles/get`, `secrets/get` with
// `profile`), the settings of each project by their ranks, and `proc/spawn` on `permissions.commands` with its jokers and
// `limits.processes`, a program being a record it keeps (`programs`), ended by `exit`. The bench itself
// (`escouade-plugin test`) plays the program as Escouade starts it.
import {
  ERRORS,
  PROTOCOL_VERSION,
  type Decision,
  type DecideParams,
  type Fact,
  type FactName,
  type InitializeResult,
  type LogParams,
} from './protocol.js';
import type { Manifest } from './manifest.js';
import { RpcError, type Plugin } from './index.js';

type Simple = string | number | boolean;

/** A profile the user made: its id (16 hexadecimal digits, drawn by Escouade), its simple fields and its secret fields. */
export interface FakeProfile {
  id: string;
  fields?: Record<string, Simple>;
  secrets?: Record<string, string>;
}

export interface FakeOptions {
  /** The values of its secrets; a key of `allowed` without a value gets `null`, as when none is kept. */
  secrets?: Record<string, string | null>;
  /** Its `permissions.secrets`: any other key is refused (`-32001`); the manifest's, else all keys of `secrets`. */
  allowed?: string[];
  /** The version of Escouade it says it is. */
  version?: string;
  /**
   * What it says it serves at `initialize`, over the app's own (`profiles` and `typedCommands` among them): `{ profiles:
   * false }` plays an Escouade older than `ESCOUADE_MIN`.
   */
  capabilities?: Record<string, boolean>;
  /**
   * Its manifest: what it serves is then judged as the app judges it (its secrets, its kinds of profile, the scope, the
   * `default` and `repo` of its settings, its `permissions.commands` and `limits.processes`).
   */
  manifest?: Manifest;
  /** Its settings as the user set them on its page (`settings/get`). */
  settings?: Record<string, Simple>;
  /** By project id, what its `.escouade.yml` gives the plugin (`plugins.<id>`): over the user's for `settings/get` of that project. */
  projectSettings?: Record<string, Record<string, Simple>>;
  /** By project id, the values this machine keeps for its settings of each project (`"scope": "project"`), profiles chosen among them. */
  projectValues?: Record<string, Record<string, Simple | null>>;
  /** The projects where the plugin is off (a private project it was not turned on in): their settings are only the defaults. */
  off?: string[];
  /** By kind (`contributes.profiles[].id`), the profiles the user made, in their order. */
  profiles?: Record<string, FakeProfile[]>;
  /** The files of its folder (`files/read`, `p.file`, `{path:plugin}`), by their path in it: `rules.json`, `data/words.txt`. */
  files?: Record<string, string>;
  /** The files of its temporary folder (`{path:temp}`); `fs/write` on the root `temp` adds to them. */
  temp?: Record<string, string>;
  /** What `fs/*` and `proc/*` answer, by method, over what it serves itself: the app's rules are held by the bench (`escouade-plugin test`). */
  host?: Record<string, (params: any) => unknown>;
}

/** A program the plugin runs, as the fake keeps it: what it asked, what it would run (its paths replaced), its pid. */
export interface FakeProgram {
  proc: number;
  pid: number;
  /** The words asked: the command, then its arguments. */
  asked: string[];
  /** The words it would run: those asked, a `{path:…}` replaced by its path under the fake's folder. */
  words: string[];
  dir: string;
  /** What the plugin wrote on its input. */
  input: string;
  /** Its input was closed. */
  closed: boolean;
}

/** Where the fake says its two folders are, for the paths a `{path:temp}` or `{path:plugin}` gives the program. */
export const FAKE_FOLDERS = { temp: '/escouade/plugins/temp/dev.test.plugin', plugin: '/escouade/plugins/dev.test.plugin' };

/**
 * Why Escouade refuses `files/read` of `path`, as the app judges it: a path that leaves the folder (absolute, `..`,
 * a drive), a hidden part (`.env`, `.git`), the manifest; null when it may be read.
 */
export function refusedPath(path: string): string | null {
  const parts = path.split(/[\\/]/);
  if (!path || /^[\\/]/.test(path) || /[\0:]/.test(path) || parts.some((p) => !p || p.startsWith('.'))) {
    return `${JSON.stringify(path)} is not a file of the plugin's folder`;
  }
  if (parts.length === 1 && parts[0] === 'escouade-plugin.json') return `${JSON.stringify(path)}: the manifest is not read by its plugin`;
  return null;
}

/** The project, the ticket and the agent of the bench's facts and decisions. */
export const SUBJECTS = {
  project: { id: 'bench-project', name: 'Banc', private: false },
  ticket: { id: 'bench-ticket', key: 'BEN-1', title: 'Un ticket du banc' },
  agent: { id: 'bench-agent', name: 'Agent du banc' },
};

type Extra<T> = { [K in keyof T]?: NonNullable<T[K]> extends object ? Partial<NonNullable<T[K]>> : T[K] } & Record<string, unknown>;

/** The fact `name` as the bench sends it, `extra` merged over it field by field. */
export function fact(name: FactName, extra: Extra<Fact> = {}): Fact {
  // What it says, as the core says it: a ticket of the bench moved to review for ticket.column.
  const data = name === 'ticket.column' ? { from: 'doing', to: 'review' } : {};
  const f: Fact = { ...structuredClone(SUBJECTS), name, version: 1, seq: 1, at: new Date().toISOString(), data };
  return merge(f, extra);
}

/** The request of a decision as the bench sends it (`npm test`, by Bash), `extra` merged over it. */
export function decisionRequest(extra: Extra<DecideParams> = {}): DecideParams {
  const d: DecideParams = {
    ...structuredClone(SUBJECTS),
    request: { id: 'bench-request', kind: 'command', tool: 'Bash', input: { command: 'npm test' } },
  };
  return merge(d, extra);
}

function merge<T>(base: T, extra: Record<string, unknown>): T {
  const out = base as Record<string, unknown>;
  for (const [k, v] of Object.entries(extra)) {
    const b = out[k];
    out[k] = v && typeof v === 'object' && !Array.isArray(v) && b && typeof b === 'object' ? { ...b, ...v } : v;
  }
  return base;
}

// ---- The app's rules (crates/escouade-plugins: runtime.rs, profiles.rs, procs.rs, files.rs) --------------------------

/** What the app says it serves at `initialize`. */
const SERVED = { batch: true, secrets: true, settings: true, log: true, attach: true, typedCommands: true, profiles: true };

/** How many programs a plugin has at once without asking, and the most the host gives (`limits.processes`). */
export const AT_ONCE = 4;
export const AT_ONCE_MAX = 16;

/** The programs at once of a plugin that declares `processes`: 4 without it, from 1 to 16. */
export function atOnce(processes: number | undefined): number {
  return processes === undefined ? AT_ONCE : Math.min(AT_ONCE_MAX, Math.max(1, Math.floor(processes)));
}

const matchesKey = (pattern: string, key: string) => (pattern.endsWith('*') ? key.startsWith(pattern.slice(0, -1)) : pattern === key);
const isProfileId = (s: string) => /^[0-9a-f]{16}$/.test(s);

type Field = Record<string, unknown> & { key?: string; type?: string; secret?: boolean };
interface Kind {
  id: string;
  nameField: string;
  fields: Field[];
}

function kindsOf(m: Manifest | undefined): Kind[] {
  const list = (m?.contributes as Record<string, unknown> | undefined)?.profiles;
  return Array.isArray(list) ? (list as Kind[]).filter((k) => typeof k?.id === 'string' && Array.isArray(k.fields)) : [];
}

function declaredOf(m: Manifest | undefined): Field[] {
  const list = (m?.contributes as Record<string, unknown> | undefined)?.settings;
  return Array.isArray(list) ? (list as Field[]) : [];
}

function secretKeysOf(m: Manifest | undefined): string[] {
  const list = (m?.contributes as Record<string, unknown> | undefined)?.secrets;
  return Array.isArray(list) ? (list as Field[]).map((s) => s.key).filter((k): k is string => typeof k === 'string') : [];
}

/** Whether `value` is of the setting's type: a `select` among its options when it lists them. */
function fits(decl: Field, value: unknown): boolean {
  switch (decl.type) {
    case 'string':
      return typeof value === 'string';
    case 'number':
      return typeof value === 'number';
    case 'boolean':
      return typeof value === 'boolean';
    case 'select': {
      if (typeof value !== 'string') return false;
      const options = decl.options;
      return !Array.isArray(options) || options.some((o) => (typeof o === 'string' ? o : (o as { value?: unknown })?.value) === value);
    }
    default:
      return false;
  }
}

/** A joker's form, or why `word` is not one (procs.rs, `whole` and `name`). */
function whole(word: string): number | string {
  if (!/^(0|[1-9][0-9]{0,9})$/.test(word)) return 'not a whole number of at most 10 digits, without sign nor leading zero';
  const n = Number(word);
  return n > 4294967295 ? 'over 4294967295' : n;
}

const DEVICES = ['CON', 'PRN', 'AUX', 'NUL'];
function windowsDevice(name: string): boolean {
  const stem = name.split('.')[0].toUpperCase();
  return DEVICES.includes(stem) || /^(COM|LPT)[0-9]$/.test(stem);
}

function nameWhy(word: string): string | null {
  if (!/^[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}$/.test(word)) return 'not 1 to 64 letters, digits, _ . - that start with a letter, a digit or _';
  return windowsDevice(word) ? 'a device of Windows' : null;
}

/** Why `relative` is not a relative path of a folder for `{path:…}` (files.rs, `file_beneath`), or null. */
function pathWhy(relative: string, hidden: boolean): string | null {
  for (const part of relative.split('/')) {
    const why = !part
      ? 'an empty part, or an absolute path'
      : !/^[A-Za-z0-9_.-]+$/.test(part)
        ? 'a part of other than letters, digits, _ . - (a drive, ~, \\, a space…)'
        : part === '.' || part === '..'
          ? 'a part . or ..'
          : part.startsWith('-')
            ? 'a part that starts with - (an option)'
            : !hidden && part.startsWith('.')
              ? 'a hidden part'
              : windowsDevice(part)
                ? 'a part that names a device of Windows'
                : null;
    if (why) return `not a relative path of its folder: ${why}`;
  }
  return null;
}

const JOKERS = ['{int}', '{port}', '{name}', '{pid}', '{path:temp}', '{path:plugin}'];

export class FakeEscouade {
  /** What it logged (`log`). */
  readonly logs: LogParams[] = [];
  /** The keys of the secrets it asked for, in order, refused ones too (`<key>@<profile>` for a profile's). */
  readonly secretsAsked: string[] = [];
  /** Its traces (what goes to its standard error). */
  readonly traces: string[] = [];
  /** Each line it wrote, parsed. */
  readonly received: unknown[] = [];
  /** Its programs under way, by their `proc`. */
  readonly programs = new Map<number, FakeProgram>();
  /** Every program it started, ended ones too, in order. */
  readonly spawned: FakeProgram[] = [];
  /** The calls refused, as the journal says them: `proc/spawn: …`, `secrets/get: …`. */
  readonly refusals: string[] = [];
  /** Whether it answered `shutdown`. */
  ended = false;
  private next = 1;
  private nextProc = 1;
  private nextPid = 41_000;
  private readonly temp: Record<string, string>;
  private readonly answers = new Map<number, (m: { result?: unknown; error?: { code: number; message: string } }) => void>();

  constructor(
    readonly plugin: Plugin,
    private readonly options: FakeOptions = {},
  ) {
    this.temp = { ...(options.temp ?? {}) };
    plugin.listen({
      write: (line) => this.read(line),
      trace: (text) => this.traces.push(text),
      end: () => (this.ended = true),
    });
  }

  /** Calls `method` and gives its result; an error answer throws an `RpcError`. */
  async call(method: string, params?: unknown): Promise<unknown> {
    const id = this.next++;
    const answer = new Promise<{ result?: unknown; error?: { code: number; message: string } }>((r) => this.answers.set(id, r));
    await this.plugin.receive(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
    const m = await answer;
    if (m.error) throw new RpcError(m.error.code, m.error.message);
    return m.result;
  }

  /** Sends a notification, and resolves once the plugin handled it. */
  async notify(method: string, params?: unknown): Promise<void> {
    await this.plugin.receive(JSON.stringify({ jsonrpc: '2.0', method, params }));
    await this.plugin.idle();
  }

  /** `initialize`, as Escouade sends it. */
  initialize(protocolVersion = PROTOCOL_VERSION): Promise<InitializeResult> {
    return this.call('initialize', {
      protocolVersion,
      escouade: { version: this.options.version ?? '2.1.0' },
      plugin: { id: this.options.manifest?.id ?? 'dev.test.plugin', version: this.options.manifest?.version ?? '0.0.0' },
      capabilities: { ...SERVED, ...this.options.capabilities },
    }) as Promise<InitializeResult>;
  }

  /** Sends the fact `name` (see `fact`), and resolves once its handlers are done. */
  fact(name: FactName, extra?: Extra<Fact>): Promise<void> {
    return this.notify('events/fact', fact(name, extra));
  }

  /** Asks for a decision (see `decisionRequest`). */
  decide(extra?: Extra<DecideParams>): Promise<Decision> {
    return this.call('permissions/decide', decisionRequest(extra)) as Promise<Decision>;
  }

  /** `shutdown`. */
  shutdown(): Promise<unknown> {
    return this.call('shutdown', null);
  }

  /** A program of the plugin writes `data`: `proc/out`, and resolves once the plugin read it. */
  out(proc: number, data: string, stream: 'stdout' | 'stderr' = 'stdout'): Promise<void> {
    // Its line handled, not what waits on the program: a handler that waits for its end is still running.
    return this.plugin.receive(JSON.stringify({ jsonrpc: '2.0', method: 'proc/out', params: { proc, stream, data } }));
  }

  /** A program of the plugin ends with `code` (`null`: killed): `proc/exit`, its place given back, once the plugin read it. */
  exit(proc: number, code: number | null = 0): Promise<void> {
    this.programs.delete(proc);
    return this.plugin.receive(JSON.stringify({ jsonrpc: '2.0', method: 'proc/exit', params: { proc, code } }));
  }

  private read(line: string): void {
    const parsed: unknown = JSON.parse(line);
    this.received.push(parsed);
    for (const m of (Array.isArray(parsed) ? parsed : [parsed]) as Array<Record<string, any>>) {
      if (m.method === undefined) {
        this.answers.get(m.id)?.(m);
        this.answers.delete(m.id);
      } else if (m.method === 'log') {
        this.logs.push(m.params);
      } else if (m.id !== undefined) {
        queueMicrotask(() => void this.plugin.receive(JSON.stringify(this.serve(m.id, m.method, m.params))));
      }
    }
  }

  private refuse(id: unknown, code: number, method: string, message: string) {
    this.refusals.push(`${method}: ${message}`);
    return { jsonrpc: '2.0', id, error: { code, message } };
  }

  private serve(id: unknown, method: string, params: any) {
    const served = this.options.host?.[method];
    if (served) {
      try {
        return { jsonrpc: '2.0', id, result: served(params) ?? null };
      } catch (e) {
        const code = e instanceof RpcError ? e.code : ERRORS.internal;
        return { jsonrpc: '2.0', id, error: { code, message: e instanceof Error ? e.message : String(e) } };
      }
    }
    const answer = (result: unknown) => ({ jsonrpc: '2.0', id, result: result ?? null });
    const invalid = (message: string) => this.refuse(id, ERRORS.invalidParams, method, message);
    switch (method) {
      case 'settings/get': {
        if (params?.project !== undefined && params?.project !== null && typeof params.project !== 'string') {
          return invalid("settings/get takes a project's id, or none");
        }
        return answer(this.settingsOf(typeof params?.project === 'string' ? params.project : null));
      }
      case 'files/read': {
        const path = typeof params?.path === 'string' ? params.path : null;
        if (path === null) return invalid('files/read takes a path');
        const refused = refusedPath(path);
        if (refused) return this.refuse(id, ERRORS.refused, method, refused);
        const files = this.options.files ?? {};
        const key = path.replace(/\\/g, '/');
        if (!Object.prototype.hasOwnProperty.call(files, key)) {
          return {
            jsonrpc: '2.0',
            id,
            error: { code: ERRORS.notFound, message: `no file ${JSON.stringify(path)} in the plugin's folder` },
          };
        }
        return answer(files[key]);
      }
      case 'fs/write': {
        if (params?.root !== 'temp' || typeof params?.path !== 'string') break;
        this.temp[params.path] = typeof params.text === 'string' ? params.text : atob(String(params.dataBase64 ?? ''));
        return answer(null);
      }
      case 'profiles/list':
      case 'profiles/get': {
        if (!this.serves('profiles')) break;
        const kind = kindsOf(this.options.manifest).find((k) => k.id === params?.kind);
        if (typeof params?.kind !== 'string') return invalid('takes a kind of profile');
        if (!kind) return invalid(`${JSON.stringify(params.kind)} is no kind of profile of this plugin (contributes.profiles)`);
        const made = this.options.profiles?.[kind.id] ?? [];
        if (method === 'profiles/list') return answer(made.map((p) => ({ id: p.id, name: this.nameOf(kind, p) })));
        if (typeof params?.profile !== 'string') return invalid("profiles/get takes a profile's id");
        const p = made.find((p) => p.id === params.profile);
        return answer(p ? this.simpleOf(kind, p.fields ?? {}) : null);
      }
      case 'secrets/get':
        return this.secret(id, params);
      case 'proc/spawn':
        if (this.options.manifest) return this.spawn(id, params);
        break;
      case 'proc/write': {
        const p = this.programs.get(params?.proc);
        if (!p) break;
        p.input += typeof params.data === 'string' ? params.data : '';
        p.closed ||= params.close === true;
        return answer(null);
      }
      case 'proc/kill': {
        if (!this.programs.has(params?.proc)) break;
        const proc = params.proc as number;
        queueMicrotask(() => void this.exit(proc, null));
        return answer(null);
      }
    }
    if (method.startsWith('proc/') && this.options.manifest) {
      return this.refuse(id, ERRORS.failed, method, `no program ${JSON.stringify(params?.proc)}`);
    }
    return { jsonrpc: '2.0', id, error: { code: ERRORS.methodNotFound, message: `unknown method ${method}` } };
  }

  private serves(capability: keyof typeof SERVED): boolean {
    return ({ ...SERVED, ...this.options.capabilities } as Record<string, boolean>)[capability] === true;
  }

  private nameOf(kind: Kind, p: FakeProfile): string {
    const v = p.fields?.[kind.nameField];
    return typeof v === 'string' && v.trim() ? v.trim() : p.id;
  }

  /** Its simple fields, held to those the kind declares, of their type: what a profile keeps. */
  private simpleOf(kind: Kind, values: Record<string, unknown>): Record<string, Simple> {
    const out: Record<string, Simple> = {};
    for (const f of kind.fields)
      if (f.secret !== true && typeof f.key === 'string' && fits(f, values[f.key])) out[f.key] = values[f.key] as Simple;
    return out;
  }

  private profileOf(id: string): [Kind, FakeProfile] | null {
    for (const kind of kindsOf(this.options.manifest)) {
      const p = (this.options.profiles?.[kind.id] ?? []).find((p) => p.id === id);
      if (p) return [kind, p];
    }
    return null;
  }

  /** `settings/get`, by the ranks of profiles.rs `settings_for`. */
  private settingsOf(project: string | null): Record<string, Simple | null> {
    const m = this.options.manifest;
    const file = project ? (this.options.projectSettings?.[project] ?? {}) : {};
    if (!m) return { ...(this.options.settings ?? {}), ...file };
    const decls = declaredOf(m);
    const ofProject = (d: Field) => d.scope === 'project';
    const mine: Record<string, Simple | null> = { ...(this.options.settings ?? {}) };
    for (const d of decls) if (ofProject(d) && d.key) delete mine[d.key];
    const here = project !== null && !(this.options.off ?? []).includes(project) ? project : null;
    const out = { ...mine };
    const secrets = secretKeysOf(m);
    if (here !== null)
      for (const [key, value] of Object.entries(file)) {
        const d = decls.find((d) => d.key === key);
        if (!d || secrets.includes(key) || d.repo === false || d.import !== undefined || !fits(d, value)) continue;
        out[key] = value;
      }
    const local = here !== null ? (this.options.projectValues?.[here] ?? {}) : {};
    const chosen = (kind: string, v: unknown) =>
      typeof v === 'string' && (this.options.profiles?.[kind] ?? []).some((p) => p.id === v) ? v : null;
    for (const d of decls) {
      if (typeof d.key !== 'string') continue;
      const kind = typeof d.of === 'string' ? d.of : '';
      if (!ofProject(d)) {
        if (d.type === 'profile') out[d.key] = chosen(kind, this.options.settings?.[d.key]);
        continue;
      }
      const v = local[d.key];
      if (d.type === 'profile') {
        if (here === null) out[d.key] = null;
        else if (v !== undefined) out[d.key] = chosen(kind, v);
        else {
          // The profile of an app's `profile` setting of the same kind, its « Profil par défaut ».
          const fallback = decls.find(
            (a) => !ofProject(a) && a.type === 'profile' && a.of === kind && chosen(kind, this.options.settings?.[a.key ?? '']),
          );
          out[d.key] = fallback ? chosen(kind, this.options.settings?.[fallback.key ?? '']) : null;
        }
      } else if (v !== undefined && fits(d, v)) out[d.key] = v;
      else if (!(d.key in out) && fits(d, d.default)) out[d.key] = d.default as Simple;
    }
    return out;
  }

  private secret(id: unknown, params: any) {
    const method = 'secrets/get';
    const key = typeof params?.key === 'string' ? params.key : null;
    if (key === null) return this.refuse(id, ERRORS.invalidParams, method, 'secrets/get takes a key');
    const profile = params?.profile;
    const m = this.options.manifest;
    const allowed = this.options.allowed ?? m?.permissions?.secrets ?? Object.keys(this.options.secrets ?? {});
    if (profile !== undefined && profile !== null && this.serves('profiles')) {
      this.secretsAsked.push(`${key}@${String(profile)}`);
      if (typeof profile !== 'string') return this.refuse(id, ERRORS.invalidParams, method, "secrets/get takes a profile's id, or none");
      if (!isProfileId(profile)) return this.refuse(id, ERRORS.invalidParams, method, `${JSON.stringify(profile)} is no profile's id`);
      const found = this.profileOf(profile);
      if (!found) return this.refuse(id, ERRORS.invalidParams, method, `no profile ${JSON.stringify(profile)}`);
      const [kind, p] = found;
      if (!kind.fields.some((f) => f.key === key && f.secret === true)) {
        return this.refuse(
          id,
          ERRORS.invalidParams,
          method,
          `${JSON.stringify(key)} is no secret field of the profiles ${JSON.stringify(kind.id)}`,
        );
      }
      const word = `profile:${kind.id}`;
      if (!allowed.includes(word)) {
        return this.refuse(
          id,
          ERRORS.notYours,
          method,
          `the secrets of the profiles ${JSON.stringify(kind.id)} are not this plugin's (permissions.secrets: ${word})`,
        );
      }
      return { jsonrpc: '2.0', id, result: p.secrets?.[key] ?? null };
    }
    this.secretsAsked.push(key);
    // A secret field of a profile asked without its profile: refused, never another secret given.
    const own = secretKeysOf(m).includes(key) || allowed.some((p) => !p.startsWith('profile:') && matchesKey(p, key));
    if (!own && this.serves('profiles') && kindsOf(m).some((k) => k.fields.some((f) => f.key === key && f.secret === true))) {
      return this.refuse(id, ERRORS.invalidParams, method, `${JSON.stringify(key)} is a field of a profile: secrets/get takes its profile`);
    }
    if (!allowed.some((p) => matchesKey(p, key))) {
      return this.refuse(id, ERRORS.notYours, method, `${key} is not one of its secrets`);
    }
    return { jsonrpc: '2.0', id, result: this.options.secrets?.[key] ?? null };
  }

  /** `proc/spawn` on `permissions.commands`, its jokers checked (procs.rs `allowed`), `limits.processes` held. */
  private spawn(id: unknown, params: any) {
    const method = 'proc/spawn';
    if (typeof params?.command !== 'string') return this.refuse(id, ERRORS.invalidParams, method, 'proc/spawn takes a command');
    if (params.args != null && (!Array.isArray(params.args) || params.args.some((a: unknown) => typeof a !== 'string'))) {
      return this.refuse(id, ERRORS.invalidParams, method, 'args is a list of words');
    }
    if (params.dir != null && params.dir !== 'agent' && params.dir !== 'project') {
      return this.refuse(id, ERRORS.refused, method, `dir is agent or project, never a path: ${JSON.stringify(params.dir)}`);
    }
    const asked: string[] = [params.command, ...(params.args ?? [])];
    const m = this.options.manifest!;
    const typed = this.serves('typedCommands');
    let best: [number, string] | null = null;
    let words: string[] | null = null;
    for (const entry of m.permissions?.commands ?? []) {
      const got = this.check(entry, asked, typed);
      if ('words' in got) {
        words = got.words;
        break;
      }
      if (got.far >= 2 && (!best || got.far > best[0])) best = [got.far, got.why];
    }
    if (!words) {
      const shown = asked.join(' ');
      return this.refuse(
        id,
        ERRORS.refused,
        method,
        best ? `${shown} is not in permissions.commands: ${best[1]}` : `${shown} is not in permissions.commands`,
      );
    }
    const n = atOnce((m.limits as { processes?: number } | undefined)?.processes);
    if (this.programs.size >= n) {
      return this.refuse(
        id,
        ERRORS.refused,
        method,
        `${n} ${n === 1 ? 'program' : 'programs'} at once at most for this plugin (limits.processes)`,
      );
    }
    const program: FakeProgram = {
      proc: this.nextProc++,
      pid: this.nextPid++,
      asked,
      words,
      dir: typeof params.dir === 'string' ? params.dir : 'temp',
      input: '',
      closed: false,
    };
    this.programs.set(program.proc, program);
    this.spawned.push(program);
    return { jsonrpc: '2.0', id, result: typed ? { proc: program.proc, pid: program.pid } : { proc: program.proc } };
  }

  /** The words an entry runs for `asked`, or how near it came and why not (procs.rs `Entry::check`). */
  private check(line: string, asked: string[], typed: boolean): { words: string[] } | { far: number; why: string } {
    let entry = line.split(/\s+/).filter(Boolean);
    const more = entry.length > 1 && entry[entry.length - 1] === '*';
    if (more) entry = entry.slice(0, -1);
    // A word no command matches: a brace it does not know (none, from an Escouade without the jokers).
    const NEVER = 'a word no command matches';
    let words = entry.map((w) => (/[{}]/.test(w) && !(typed && JOKERS.includes(w)) ? NEVER : w));
    // Never a bare `*`, nor a joker for a program: it would be any program.
    if (!words.length || words[0] === '*' || /[{}]/.test(entry[0])) words = [NEVER];
    const n = words.length;
    const fitsLength = asked.length === n || (more && asked.length > n) ? 1 : 0;
    const out: string[] = [];
    for (let i = 0; i < Math.min(n, asked.length); i++) {
      const w = words[i];
      const a = asked[i];
      const at = (why: string) => ({ far: 2 * i + fitsLength, why: `word ${i + 1} (${JSON.stringify(a)}) is no ${w}: ${why}` });
      if (!JOKERS.includes(w)) {
        if (w !== NEVER && w === a) {
          out.push(a);
          continue;
        }
        return { far: 2 * i + fitsLength, why: `word ${i + 1} is ${JSON.stringify(a)}, not ${JSON.stringify(w)}` };
      }
      if (a === '--' || a.startsWith('-')) return at('an option');
      if (w === '{int}' || w === '{port}' || w === '{pid}') {
        const v = whole(a);
        if (typeof v === 'string') return at(v);
        if (w === '{port}' && (v < 1 || v > 65535)) return at('out of 1 to 65535');
        if (w === '{pid}' && ![...this.programs.values()].some((p) => p.pid === v)) return at('no program it runs is alive with this pid');
        out.push(a);
      } else if (w === '{name}') {
        const why = nameWhy(a);
        if (why) return at(why);
        out.push(a);
      } else {
        const temp = w === '{path:temp}';
        const why = pathWhy(a, temp);
        if (why) return at(why);
        const files = temp ? this.temp : (this.options.files ?? {});
        if (!Object.prototype.hasOwnProperty.call(files, a)) return at('no such file in its folder');
        out.push(`${temp ? FAKE_FOLDERS.temp : FAKE_FOLDERS.plugin}/${a}`);
      }
    }
    if (asked.length < n || (!more && asked.length > n)) {
      return { far: 2 * Math.min(asked.length, n), why: `${asked.length} words where the entry has ${more ? 'at least ' : ''}${n}` };
    }
    return { words: [...out, ...asked.slice(n)] };
  }
}

/** A fake Escouade talking to `plugin`, in memory. */
export function connect(plugin: Plugin, options?: FakeOptions): FakeEscouade {
  return new FakeEscouade(plugin, options);
}
