// @escouade-dev/plugin: write a plugin of Escouade in TypeScript (ESC-181, PLG-9).
//
// A plugin with a program speaks JSON-RPC 2.0 to Escouade on its standard input and output, one message per
// line (https://escouade.dev/en/docs/plugins/api). `createPlugin()` gives a client that speaks it: it answers
// `initialize` and `shutdown`, serves batches, routes each fact to its handlers (`onEvent`), each permission to
// decide to its rule (`decidePermission`), and asks Escouade for secrets (`secret`) and journal lines (`log`).
import { createInterface } from 'node:readline';
import {
  ERRORS,
  PROTOCOL_VERSION,
  type Decision,
  type DecideParams,
  type Fact,
  type FactName,
  type InitializeParams,
  type InitializeResult,
  type LogParams,
  type ToEscouade,
} from './protocol.js';

export * from './protocol.js';
export type { Manifest } from './manifest.js';

/** An error to answer a call with: its code goes to Escouade as it is (`ERRORS`). */
export class RpcError extends Error {
  constructor(
    readonly code: number,
    message: string,
    readonly data?: unknown,
  ) {
    super(message);
    this.name = 'RpcError';
  }
}

/** Where the plugin's lines go, and where its traces go: the process's own, or a test's. */
export interface Connection {
  /** One line of the protocol, without its `\n`. */
  write(line: string): void;
  /** A trace, which Escouade puts in the plugin's journal. */
  trace?(text: string): void;
  /** Called after the answer to `shutdown`. */
  end?(): void;
}

/** A fact of `name`, or of a pattern ending in `*` (`ticket.*`), as the manifest's `permissions.events`. */
export type FactPattern = FactName | `${string}*`;
export type FactHandler = (fact: Fact, plugin: Plugin) => void | Promise<void>;
export type DecideHandler = (params: DecideParams, plugin: Plugin) => Decision | Promise<Decision>;
export type Handler = (params: unknown, plugin: Plugin) => unknown;

export interface PluginOptions {
  /** The capabilities it answers `initialize` with, beside `batch`. */
  capabilities?: Record<string, unknown>;
  /** How long its `onShutdown` handlers have, in milliseconds (Escouade waits 1 s for the answer). */
  shutdownMs?: number;
}

interface Message {
  jsonrpc?: string;
  id?: string | number | null;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
}

const matches = (pattern: string, name: string) => (pattern.endsWith('*') ? name.startsWith(pattern.slice(0, -1)) : pattern === name);

export class Plugin {
  /** What Escouade said at `initialize`: its version, the plugin's id and version, its capabilities. */
  escouade?: InitializeParams;
  private readonly facts: Array<[string, FactHandler]> = [];
  private decide?: DecideHandler;
  private readonly handlers = new Map<string, Handler>();
  private readonly closing: Array<() => void | Promise<void>> = [];
  private readonly starting: Array<(params: InitializeParams) => void | Promise<void>> = [];
  private readonly waiting = new Map<string, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
  private readonly busy = new Set<Promise<unknown>>();
  private next = 1;
  private conn?: Connection;

  constructor(private readonly options: PluginOptions = {}) {}

  /** Runs `handler` on each fact of `pattern` (the facts must be in the manifest's `permissions.events`). */
  onEvent(pattern: FactPattern | FactPattern[], handler: FactHandler): this {
    for (const p of Array.isArray(pattern) ? pattern : [pattern]) this.facts.push([p, handler]);
    return this;
  }

  /**
   * Decides the permissions agents ask for, before the user is asked (`permissions.decide` in the manifest).
   * Answer within 500 ms: past it, the user is asked, as without a plugin. A refusal of any plugin wins.
   */
  decidePermission(handler: DecideHandler): this {
    this.decide = handler;
    return this;
  }

  /** Serves another method of Escouade's (a later extension point); a thrown `RpcError` answers its code. */
  handle(method: string, handler: Handler): this {
    this.handlers.set(method, handler);
    return this;
  }

  /** Runs once Escouade said hello, before the answer to `initialize`. */
  onInitialize(handler: (params: InitializeParams) => void | Promise<void>): this {
    this.starting.push(handler);
    return this;
  }

  /** Runs when Escouade closes, before the answer to `shutdown`. */
  onShutdown(handler: () => void | Promise<void>): this {
    this.closing.push(handler);
    return this;
  }

  /** Its secret `key` (one of `permissions.secrets`), from the system's keychain; `null` when none is kept. */
  secret(key: string): Promise<string | null> {
    return this.call('secrets/get', { key });
  }

  /** A line of its journal, in Réglages › Plugins. */
  readonly log = {
    debug: (message: string) => this.notify('log', { level: 'debug', message }),
    info: (message: string) => this.notify('log', { level: 'info', message }),
    warn: (message: string) => this.notify('log', { level: 'warn', message }),
    error: (message: string) => this.notify('log', { level: 'error', message }),
  } satisfies Record<LogParams['level'], (message: string) => void>;

  /** Calls a method of Escouade's and waits for its answer; an error answer rejects with an `RpcError`. */
  call<M extends keyof ToEscouade>(method: M, params: ToEscouade[M]['params']): Promise<ToEscouade[M]['result']>;
  call(method: string, params?: unknown): Promise<unknown>;
  call(method: string, params?: unknown): Promise<unknown> {
    const id = `p${this.next++}`;
    return new Promise((resolve, reject) => {
      if (!this.conn) return reject(new Error('the plugin is not started'));
      this.waiting.set(id, { resolve, reject });
      this.send({ jsonrpc: '2.0', id, method, params });
    });
  }

  /** Sends a notification to Escouade. */
  notify<M extends keyof ToEscouade>(method: M, params: ToEscouade[M]['params']): void;
  notify(method: string, params?: unknown): void;
  notify(method: string, params?: unknown): void {
    this.send({ jsonrpc: '2.0', method, params });
  }

  /** Speaks the protocol on the process's standard input and output; ends the process after `shutdown`. */
  start(): Promise<void> {
    const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
    this.listen({
      write: (line) => process.stdout.write(line + '\n'),
      trace: (text) => process.stderr.write(text + '\n'),
      end: () => process.stdout.write('', () => process.exit(0)),
    });
    lines.on('line', (line) => void this.receive(line));
    return new Promise((resolve) => lines.on('close', () => resolve()));
  }

  /** Speaks the protocol on `conn` (`start()` does it on the standard streams; the tests on theirs). */
  listen(conn: Connection): this {
    this.conn = conn;
    return this;
  }

  /** Handles one line Escouade sent, and resolves once what it started is done (its handlers, its answers). */
  async receive(line: string): Promise<void> {
    const text = line.trim();
    if (!text) return;
    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch {
      return this.send({ jsonrpc: '2.0', id: null, error: { code: ERRORS.parse, message: 'not JSON' } });
    }
    const work = (async () => {
      if (Array.isArray(parsed)) {
        const answers = (await Promise.all(parsed.map((m) => this.one(m as Message)))).filter((a) => a !== undefined);
        if (answers.length) this.conn?.write(JSON.stringify(answers));
      } else {
        const answer = await this.one(parsed as Message);
        if (answer !== undefined) this.send(answer);
      }
    })();
    this.busy.add(work);
    try {
      await work;
    } finally {
      this.busy.delete(work);
    }
    if (Array.isArray(parsed) ? parsed.some((m) => (m as Message)?.method === 'shutdown') : (parsed as Message)?.method === 'shutdown') {
      this.conn?.end?.();
    }
  }

  /** Resolves once every line received so far is handled. */
  async idle(): Promise<void> {
    while (this.busy.size) await Promise.allSettled([...this.busy]);
  }

  private send(message: Message): void {
    this.conn?.write(JSON.stringify(message));
  }

  private trace(text: string): void {
    this.conn?.trace?.(text);
  }

  /** The answer to one message; none for a notification or an answer. */
  private async one(m: Message): Promise<Message | undefined> {
    if (!m || typeof m !== 'object' || m.jsonrpc !== '2.0') {
      return { jsonrpc: '2.0', id: null, error: { code: ERRORS.invalidRequest, message: 'not JSON-RPC 2.0' } };
    }
    if (m.method === undefined) {
      const w = m.id == null ? undefined : this.waiting.get(String(m.id));
      if (w) {
        this.waiting.delete(String(m.id));
        if (m.error) w.reject(new RpcError(m.error.code, m.error.message, m.error.data));
        else w.resolve(m.result ?? null);
      }
      return undefined;
    }
    const notification = m.id === undefined;
    try {
      const result = await this.serve(m.method, m.params);
      return notification ? undefined : { jsonrpc: '2.0', id: m.id, result: result ?? null };
    } catch (e) {
      const error =
        e instanceof RpcError
          ? { code: e.code, message: e.message, ...(e.data === undefined ? {} : { data: e.data }) }
          : { code: ERRORS.internal, message: e instanceof Error ? e.message : String(e) };
      if (!(e instanceof RpcError)) this.trace(`${m.method}: ${e instanceof Error ? (e.stack ?? e.message) : String(e)}`);
      return notification ? undefined : { jsonrpc: '2.0', id: m.id, error };
    }
  }

  private async serve(method: string, params: unknown): Promise<unknown> {
    switch (method) {
      case 'initialize': {
        this.escouade = params as InitializeParams;
        for (const h of this.starting) await h(this.escouade);
        return {
          protocolVersion: PROTOCOL_VERSION,
          capabilities: { batch: true, ...this.options.capabilities },
        } satisfies InitializeResult;
      }
      case 'shutdown': {
        const all = Promise.allSettled(this.closing.map((h) => h()));
        await Promise.race([all, new Promise((r) => setTimeout(r, this.options.shutdownMs ?? 800).unref?.())]);
        return null;
      }
      case 'events/fact': {
        const fact = params as Fact;
        const handlers = this.facts.filter(([p]) => typeof fact?.name === 'string' && matches(p, fact.name));
        const done = await Promise.allSettled(handlers.map(async ([, h]) => h(fact, this)));
        for (const d of done) if (d.status === 'rejected') this.trace(`events/fact ${fact.name}: ${d.reason?.stack ?? d.reason}`);
        return undefined;
      }
      case 'permissions/decide': {
        if (!this.decide) return { decision: 'ask' } satisfies Decision;
        return this.decide(params as DecideParams, this);
      }
      default: {
        const h = this.handlers.get(method);
        if (!h) throw new RpcError(ERRORS.methodNotFound, `unknown method ${method}`);
        return h(params, this);
      }
    }
  }
}

/** A plugin: add its handlers, then `start()` it. */
export function createPlugin(options?: PluginOptions): Plugin {
  return new Plugin(options);
}
