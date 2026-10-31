// src/plugin.ts
import { readFileSync } from "node:fs";

// ../../sdk/plugin/dist/index.js
import { createInterface } from "node:readline";

// ../../sdk/plugin/dist/protocol.js
var PROTOCOL_VERSION = 1;
var ERRORS = {
  /** A line that is not JSON. */
  parse: -32700,
  /** A message that is not JSON-RPC 2.0. */
  invalidRequest: -32600,
  /** A method the other side does not serve. */
  methodNotFound: -32601,
  /** Parameters that do not hold. */
  invalidParams: -32602,
  /** The handler failed. */
  internal: -32603,
  /** A secret that is not one of the plugin's own. */
  notYours: -32001,
  /** The system's keychain cannot be reached. */
  keychain: -32002
};

// ../../sdk/plugin/dist/index.js
var RpcError = class extends Error {
  code;
  data;
  constructor(code, message, data) {
    super(message);
    this.code = code;
    this.data = data;
    this.name = "RpcError";
  }
};
var matches = (pattern, name) => pattern.endsWith("*") ? name.startsWith(pattern.slice(0, -1)) : pattern === name;
var Plugin = class {
  options;
  /** What Escouade said at `initialize`: its version, the plugin's id and version, its capabilities. */
  escouade;
  facts = [];
  decide;
  handlers = /* @__PURE__ */ new Map();
  closing = [];
  starting = [];
  waiting = /* @__PURE__ */ new Map();
  busy = /* @__PURE__ */ new Set();
  next = 1;
  conn;
  constructor(options = {}) {
    this.options = options;
  }
  /** Runs `handler` on each fact of `pattern` (the facts must be in the manifest's `permissions.events`). */
  onEvent(pattern, handler) {
    for (const p of Array.isArray(pattern) ? pattern : [pattern])
      this.facts.push([p, handler]);
    return this;
  }
  /**
   * Decides the permissions agents ask for, before the user is asked (`permissions.decide` in the manifest).
   * Answer within 500 ms: past it, the user is asked, as without a plugin. A refusal of any plugin wins.
   */
  decidePermission(handler) {
    this.decide = handler;
    return this;
  }
  /** Serves another method of Escouade's (a later extension point); a thrown `RpcError` answers its code. */
  handle(method, handler) {
    this.handlers.set(method, handler);
    return this;
  }
  /** Runs once Escouade said hello, before the answer to `initialize`. */
  onInitialize(handler) {
    this.starting.push(handler);
    return this;
  }
  /** Runs when Escouade closes, before the answer to `shutdown`. */
  onShutdown(handler) {
    this.closing.push(handler);
    return this;
  }
  /** Its secret `key` (one of `permissions.secrets`), from the system's keychain; `null` when none is kept. */
  secret(key) {
    return this.call("secrets/get", { key });
  }
  /**
   * Its settings (`contributes.settings`) as the user set them; with `project` (the `project.id` of a fact), those the
   * project's `.escouade.yml` gives it (`plugins.<id>`) over them, where it is on.
   */
  settings(project) {
    return this.call("settings/get", project ? { project } : {});
  }
  /** A line of its journal, in Réglages › Plugins. */
  log = {
    debug: (message) => this.notify("log", { level: "debug", message }),
    info: (message) => this.notify("log", { level: "info", message }),
    warn: (message) => this.notify("log", { level: "warn", message }),
    error: (message) => this.notify("log", { level: "error", message })
  };
  call(method, params) {
    const id = `p${this.next++}`;
    return new Promise((resolve, reject) => {
      if (!this.conn)
        return reject(new Error("the plugin is not started"));
      this.waiting.set(id, { resolve, reject });
      this.send({ jsonrpc: "2.0", id, method, params });
    });
  }
  notify(method, params) {
    this.send({ jsonrpc: "2.0", method, params });
  }
  /** Speaks the protocol on the process's standard input and output; ends the process after `shutdown`. */
  start() {
    const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
    this.listen({
      write: (line) => process.stdout.write(line + "\n"),
      trace: (text) => process.stderr.write(text + "\n"),
      end: () => process.stdout.write("", () => process.exit(0))
    });
    lines.on("line", (line) => void this.receive(line));
    return new Promise((resolve) => lines.on("close", () => resolve()));
  }
  /** Speaks the protocol on `conn` (`start()` does it on the standard streams; the tests on theirs). */
  listen(conn) {
    this.conn = conn;
    return this;
  }
  /** Handles one line Escouade sent, and resolves once what it started is done (its handlers, its answers). */
  async receive(line) {
    const text = line.trim();
    if (!text)
      return;
    let parsed;
    try {
      parsed = JSON.parse(text);
    } catch {
      return this.send({ jsonrpc: "2.0", id: null, error: { code: ERRORS.parse, message: "not JSON" } });
    }
    const work = (async () => {
      if (Array.isArray(parsed)) {
        const answers = (await Promise.all(parsed.map((m) => this.one(m)))).filter((a) => a !== void 0);
        if (answers.length)
          this.conn?.write(JSON.stringify(answers));
      } else {
        const answer = await this.one(parsed);
        if (answer !== void 0)
          this.send(answer);
      }
    })();
    this.busy.add(work);
    try {
      await work;
    } finally {
      this.busy.delete(work);
    }
    if (Array.isArray(parsed) ? parsed.some((m) => m?.method === "shutdown") : parsed?.method === "shutdown") {
      this.conn?.end?.();
    }
  }
  /** Resolves once every line received so far is handled. */
  async idle() {
    while (this.busy.size)
      await Promise.allSettled([...this.busy]);
  }
  send(message) {
    this.conn?.write(JSON.stringify(message));
  }
  trace(text) {
    this.conn?.trace?.(text);
  }
  /** The answer to one message; none for a notification or an answer. */
  async one(m) {
    if (!m || typeof m !== "object" || m.jsonrpc !== "2.0") {
      return { jsonrpc: "2.0", id: null, error: { code: ERRORS.invalidRequest, message: "not JSON-RPC 2.0" } };
    }
    if (m.method === void 0) {
      const w = m.id == null ? void 0 : this.waiting.get(String(m.id));
      if (w) {
        this.waiting.delete(String(m.id));
        if (m.error)
          w.reject(new RpcError(m.error.code, m.error.message, m.error.data));
        else
          w.resolve(m.result ?? null);
      }
      return void 0;
    }
    const notification = m.id === void 0;
    try {
      const result = await this.serve(m.method, m.params);
      return notification ? void 0 : { jsonrpc: "2.0", id: m.id, result: result ?? null };
    } catch (e) {
      const error = e instanceof RpcError ? { code: e.code, message: e.message, ...e.data === void 0 ? {} : { data: e.data } } : { code: ERRORS.internal, message: e instanceof Error ? e.message : String(e) };
      if (!(e instanceof RpcError))
        this.trace(`${m.method}: ${e instanceof Error ? e.stack ?? e.message : String(e)}`);
      return notification ? void 0 : { jsonrpc: "2.0", id: m.id, error };
    }
  }
  async serve(method, params) {
    switch (method) {
      case "initialize": {
        this.escouade = params;
        for (const h of this.starting)
          await h(this.escouade);
        return {
          protocolVersion: PROTOCOL_VERSION,
          capabilities: { batch: true, ...this.options.capabilities }
        };
      }
      case "shutdown": {
        const all = Promise.allSettled(this.closing.map((h) => h()));
        await Promise.race([all, new Promise((r) => setTimeout(r, this.options.shutdownMs ?? 800).unref?.())]);
        return null;
      }
      case "events/fact": {
        const fact = params;
        const handlers = this.facts.filter(([p]) => typeof fact?.name === "string" && matches(p, fact.name));
        const done = await Promise.allSettled(handlers.map(async ([, h]) => h(fact, this)));
        for (const d of done)
          if (d.status === "rejected")
            this.trace(`events/fact ${fact.name}: ${d.reason?.stack ?? d.reason}`);
        return void 0;
      }
      case "permissions/decide": {
        if (!this.decide)
          return { decision: "ask" };
        return this.decide(params, this);
      }
      default: {
        const h = this.handlers.get(method);
        if (!h)
          throw new RpcError(ERRORS.methodNotFound, `unknown method ${method}`);
        return h(params, this);
      }
    }
  }
};
function createPlugin(options) {
  return new Plugin(options);
}

// src/plugin.ts
function readRules() {
  return JSON.parse(readFileSync(new URL("../rules.json", import.meta.url), "utf8"));
}
function commandsOf(line) {
  return line.split(/&&|\|\||[;|&\n]/).map((c) => c.trim().replace(/\s+/g, " ")).filter(Boolean);
}
var HIDDEN = /[`$<>\\]/;
var startsWith = (command, rule) => command === rule || command.startsWith(rule + " ");
function decide(rules, line) {
  const commands = commandsOf(line);
  for (const rule of rules.deny) {
    const { command, message } = typeof rule === "string" ? { command: rule, message: void 0 } : rule;
    if (commands.some((c) => startsWith(c, command))) {
      return { decision: "deny", message: message ?? `The team's rules forbid \`${command}\`.` };
    }
  }
  if (!HIDDEN.test(line) && commands.length > 0 && commands.every((c) => rules.allow.some((rule) => startsWith(c, rule)))) {
    return { decision: "allow" };
  }
  return { decision: "ask" };
}
function teamRules(rules = readRules()) {
  return createPlugin().decidePermission(({ request }) => {
    const line = request.input?.command;
    if (request.kind !== "command" || typeof line !== "string") return { decision: "ask" };
    return decide(rules, line);
  });
}

// src/main.ts
void teamRules().start();
