// @escouade-dev/plugin/testing: a fake Escouade in memory, for a plugin's unit tests (ESC-181, PLG-9).
//
// It speaks the protocol to the plugin as the app does (the same lines, the same subjects as the conformance
// bench), without a process: `initialize`, facts, decisions, calls, `shutdown`; it serves `secrets/get` and keeps
// what the plugin logged. The bench itself (`escouade-plugin test`) plays the program as Escouade starts it.
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
import { RpcError, type Plugin } from './index.js';

export interface FakeOptions {
  /** The values of its secrets; a key of `allowed` without a value gets `null`, as when none is kept. */
  secrets?: Record<string, string | null>;
  /** Its `permissions.secrets`: any other key is refused (`-32001`); all keys of `secrets` when not given. */
  allowed?: string[];
  /** The version of Escouade it says it is. */
  version?: string;
  /** Its settings as the user set them (`settings/get`). */
  settings?: Record<string, string | number | boolean>;
  /** By project id, the settings its `.escouade.yml` gives the plugin: over the user's for `settings/get` of that project. */
  projectSettings?: Record<string, Record<string, string | number | boolean>>;
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

export class FakeEscouade {
  /** What it logged (`log`). */
  readonly logs: LogParams[] = [];
  /** The keys of the secrets it asked for, in order, refused ones too. */
  readonly secretsAsked: string[] = [];
  /** Its traces (what goes to its standard error). */
  readonly traces: string[] = [];
  /** Each line it wrote, parsed. */
  readonly received: unknown[] = [];
  /** Whether it answered `shutdown`. */
  ended = false;
  private next = 1;
  private readonly answers = new Map<number, (m: { result?: unknown; error?: { code: number; message: string } }) => void>();

  constructor(
    readonly plugin: Plugin,
    private readonly options: FakeOptions = {},
  ) {
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
      plugin: { id: 'dev.test.plugin', version: '0.0.0' },
      capabilities: { batch: true, secrets: true, log: true, attach: true },
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

  private serve(id: unknown, method: string, params: any) {
    if (method === 'settings/get') {
      const project = typeof params?.project === 'string' ? params.project : null;
      const own = project ? (this.options.projectSettings?.[project] ?? {}) : {};
      return { jsonrpc: '2.0', id, result: { ...(this.options.settings ?? {}), ...own } };
    }
    if (method !== 'secrets/get') {
      return { jsonrpc: '2.0', id, error: { code: ERRORS.methodNotFound, message: `unknown method ${method}` } };
    }
    const key = String(params?.key ?? '');
    this.secretsAsked.push(key);
    const secrets = this.options.secrets ?? {};
    const allowed = this.options.allowed ?? Object.keys(secrets);
    const own = allowed.some((p) => (p.endsWith('*') ? key.startsWith(p.slice(0, -1)) : p === key));
    if (!own) return { jsonrpc: '2.0', id, error: { code: ERRORS.notYours, message: `${key} is not one of its secrets` } };
    return { jsonrpc: '2.0', id, result: secrets[key] ?? null };
  }
}

/** A fake Escouade talking to `plugin`, in memory. */
export function connect(plugin: Plugin, options?: FakeOptions): FakeEscouade {
  return new FakeEscouade(plugin, options);
}
