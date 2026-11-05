// The prelude of a JS/TS plugin compiled to WASM by Escouade (ESC-320, plugins/wasm.rs; reduced by ESC-321 to what
// the SDK does not do). It goes before the bundle of the plugin (dist/plugin.mjs, as committed) and gives it what Node
// gave it and the SDK does not, served by Escouade:
// - `globalThis.__esc`, the transport the SDK's `start()` finds here (instead of Node's stdin and stdout): `listen`
//   for the protocol's lines Escouade sends, `write` for those the plugin sends, `trace` for its journal;
// - `readFileSync` of `node:fs`, for the files at the root of the plugin's own folder only (rules.json,
//   channels.json), given by Escouade at the start, in the first line (`escouade/boot`); `p.file()` of the SDK reads
//   the others (`files/read`);
// - `fetch`, the standard one, served by Escouade (`net/fetch`), which lets out only the hosts of permissions.network;
// - `setTimeout` and `clearTimeout`, served by Escouade (`timer/set`), 10 ms at least;
// - `URL`, enough of it for `new URL('../rules.json', import.meta.url)`;
// - `console`, which goes to the journal (standard error), never into the protocol.
// `node:readline` is there for the SDK's import alone.
// Escouade calls the module's export `escouade-turn` each time it has written lines: the prelude hands them to the
// plugin, and the call returns once the plugin has nothing left to run (QuickJS's job queue empty): no thread.
const __esc = (globalThis.__esc ??= (() => {
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  let pending = new Uint8Array(0);
  const lines = [];
  const listeners = [];
  const waiting = new Map();
  let seq = 0;
  let files = {};
  let ended = false;
  // The timers of setTimeout, by the number it gave; the same, by the host's number once the host answered.
  const timers = new Map();
  const set = new Map();
  let handles = 0;

  function readAvailable() {
    const buf = new Uint8Array(65536);
    for (;;) {
      // Escouade's standard input never blocks: 0 is "nothing more for now".
      const n = Javy.IO.readSync(0, buf);
      if (!n || n <= 0) break;
      const merged = new Uint8Array(pending.length + n);
      merged.set(pending);
      merged.set(buf.subarray(0, n), pending.length);
      pending = merged;
    }
    let start = 0;
    for (let i = 0; i < pending.length; i++) {
      if (pending[i] === 10) {
        lines.push(dec.decode(pending.subarray(start, i)));
        start = i + 1;
      }
    }
    pending = pending.slice(start);
  }

  function out(fd, text) {
    Javy.IO.writeSync(fd, enc.encode(text));
  }

  /** A path of the plugin's folder, `/plugin/…`, from a path or a file URL; null for anything outside it. */
  function inFolder(p) {
    let path = typeof p === 'string' ? p : String(p && p.href ? p.href : p);
    if (path.startsWith('file://')) path = path.slice('file://'.length);
    if (!path.startsWith('/')) path = '/plugin/dist/' + path;
    const parts = [];
    for (const s of path.split('/')) {
      if (s === '' || s === '.') continue;
      if (s === '..') {
        if (!parts.length) return null;
        parts.pop();
      } else parts.push(s);
    }
    if (parts[0] !== 'plugin' || parts.length < 2) return null;
    return parts.slice(1).join('/');
  }

  function readFileSync(p) {
    const name = inFolder(p);
    if (name !== null && Object.prototype.hasOwnProperty.call(files, name)) return files[name];
    const e = new Error(`ENOENT: no such file of the plugin, '${String(p)}'`);
    e.code = 'ENOENT';
    throw e;
  }

  class Headers {
    constructor(h = {}) {
      this.map = {};
      const entries = h instanceof Headers ? Object.entries(h.map) : Array.isArray(h) ? h : Object.entries(h);
      for (const [k, v] of entries) this.map[String(k).toLowerCase()] = String(v);
    }
    get(k) {
      return this.map[String(k).toLowerCase()] ?? null;
    }
    has(k) {
      return String(k).toLowerCase() in this.map;
    }
    set(k, v) {
      this.map[String(k).toLowerCase()] = String(v);
    }
    forEach(cb) {
      for (const [k, v] of Object.entries(this.map)) cb(v, k, this);
    }
  }

  const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

  function toBase64(bytes) {
    let s = '';
    for (let i = 0; i < bytes.length; i += 3) {
      const n = (bytes[i] << 16) | ((bytes[i + 1] ?? 0) << 8) | (bytes[i + 2] ?? 0);
      s += LETTERS[n >> 18] + LETTERS[(n >> 12) & 63];
      s += i + 1 < bytes.length ? LETTERS[(n >> 6) & 63] : '=';
      s += i + 2 < bytes.length ? LETTERS[n & 63] : '=';
    }
    return s;
  }

  function fromBase64(text) {
    const clean = text.replace(/=+$/, '');
    const bytes = new Uint8Array(Math.floor((clean.length * 3) / 4));
    let bits = 0;
    let value = 0;
    let at = 0;
    for (const c of clean) {
      value = (value << 6) | LETTERS.indexOf(c);
      bits += 6;
      if (bits >= 8) {
        bits -= 8;
        bytes[at++] = (value >> bits) & 255;
      }
    }
    return bytes;
  }

  /** The bytes of a binary body; null for what is text. */
  function binary(body) {
    if (body instanceof ArrayBuffer) return new Uint8Array(body);
    if (ArrayBuffer.isView(body)) return new Uint8Array(body.buffer, body.byteOffset, body.byteLength);
    return null;
  }

  class Response {
    constructor(r) {
      this.status = r.status;
      this.ok = r.status >= 200 && r.status < 300;
      this.headers = new Headers(r.headers);
      this.body_ = r.body ?? null;
      this.bytes_ = r.bodyBase64 == null ? null : fromBase64(r.bodyBase64);
    }
    async text() {
      return this.bytes_ ? dec.decode(this.bytes_) : (this.body_ ?? '');
    }
    async json() {
      return JSON.parse(await this.text());
    }
    async bytes() {
      return this.bytes_ ?? enc.encode(this.body_ ?? '');
    }
    async arrayBuffer() {
      return (await this.bytes()).slice().buffer;
    }
  }

  /** A request to Escouade the prelude makes itself; `done` gets its answer as it is read, in the same turn. */
  function ask(method, params, done) {
    const id = `w${++seq}`;
    waiting.set(id, done);
    out(1, JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
  }

  /** A call to Escouade the prelude makes itself (`net/fetch`): its answer never reaches the plugin's listeners. */
  function call(method, params) {
    return new Promise((resolve, reject) => ask(method, params, { resolve, reject }));
  }

  function fetch(input, init = {}) {
    const headers = new Headers(init.headers ?? {}).map;
    const bytes = binary(init.body);
    return call('net/fetch', {
      url: String(input && input.href ? input.href : input),
      method: init.method ?? 'GET',
      headers,
      ...(bytes ? { bodyBase64: toBase64(bytes) } : { body: init.body == null ? null : String(init.body) }),
    }).then((r) => new Response(r));
  }

  /** The number is the prelude's: `clearTimeout` holds before the host's answer has come. */
  function setTimeout(fn, ms, ...args) {
    const handle = ++handles;
    timers.set(handle, { fn, args, timer: null });
    ask(
      'timer/set',
      { ms: Math.max(10, Number(ms) || 0) },
      {
        resolve(r) {
          const t = timers.get(handle);
          if (t) {
            t.timer = r.timer;
            set.set(r.timer, handle);
          } else out(1, JSON.stringify({ jsonrpc: '2.0', method: 'timer/cancel', params: { timer: r.timer } }) + '\n');
        },
        reject(e) {
          timers.delete(handle);
          out(2, `setTimeout: ${e.message}\n`);
        },
      },
    );
    return handle;
  }

  function clearTimeout(handle) {
    const t = timers.get(handle);
    if (!t) return;
    timers.delete(handle);
    if (t.timer !== null) {
      set.delete(t.timer);
      out(1, JSON.stringify({ jsonrpc: '2.0', method: 'timer/cancel', params: { timer: t.timer } }) + '\n');
    }
  }

  /** A timer of the host came due: what setTimeout was given runs, in this turn. */
  function fired(timer) {
    const handle = set.get(timer);
    set.delete(timer);
    const t = timers.get(handle);
    timers.delete(handle);
    if (t) t.fn(...t.args);
  }

  /** Just enough of URL for `new URL('../rules.json', import.meta.url)` and fetch's strings. */
  class URL {
    constructor(u, base) {
      u = String(u);
      if (/^[a-z]+:\/\//.test(u) || base === undefined) {
        this.href = u;
      } else {
        const b = String(base);
        const m = /^([a-z]+:\/\/[^/]*)(\/.*)?$/.exec(b);
        const dir = (m[2] ?? '/').replace(/[^/]*$/, '');
        const parts = [];
        for (const s of (u.startsWith('/') ? u : dir + u).split('/')) {
          if (s === '' || s === '.') continue;
          if (s === '..') parts.pop();
          else parts.push(s);
        }
        this.href = m[1] + '/' + parts.join('/');
      }
      const m = /^([a-z]+:)\/\/([^/]*)(\/[^?#]*)?/.exec(this.href);
      this.protocol = m ? m[1] : '';
      this.host = this.hostname = m ? m[2] : '';
      this.pathname = m ? (m[3] ?? '/') : this.href;
    }
    toString() {
      return this.href;
    }
  }

  const node = {
    'node:fs': { readFileSync },
    'node:readline': {
      createInterface() {
        throw new Error(
          'no standard input in WASM: build the plugin with @escouade-dev/plugin 0.3 or later, whose start() speaks to Escouade',
        );
      },
    },
  };

  const text = (args) =>
    args.map((a) => (typeof a === 'string' ? a : a instanceof Error ? (a.stack ?? a.message) : JSON.stringify(a))).join(' ') + '\n';

  return {
    booted: false,
    url: 'file:///plugin/dist/plugin.mjs',
    node(name) {
      return node[name];
    },
    /** The SDK's transport: `cb` gets each line Escouade sends, but the answers to the prelude's own calls. */
    listen(cb) {
      listeners.push(cb);
    },
    /** A line to Escouade, without its `\n`. */
    write(line) {
      out(1, line + '\n');
    },
    /** A line of its journal. */
    trace(line) {
      out(2, line + '\n');
    },
    /** After `shutdown`: nothing more is handed to the plugin. */
    end() {
      ended = true;
    },
    globals() {
      globalThis.URL ??= URL;
      globalThis.fetch = fetch;
      globalThis.Headers ??= Headers;
      globalThis.Response ??= Response;
      globalThis.setTimeout = setTimeout;
      globalThis.clearTimeout = clearTimeout;
      globalThis.console = {
        log: (...a) => out(2, text(a)),
        info: (...a) => out(2, text(a)),
        warn: (...a) => out(2, text(a)),
        error: (...a) => out(2, text(a)),
        debug: (...a) => out(2, text(a)),
      };
      // What a library reads of Node's process; no streams, no exit, no environment.
      globalThis.process ??= { env: {}, argv: [], platform: 'wasi' };
    },
    /** The first line Escouade writes: the plugin's files. */
    boot(bundle) {
      readAvailable();
      const first = lines.shift();
      const frame = first ? JSON.parse(first) : {};
      files = (frame['escouade/boot'] && frame['escouade/boot'].files) || {};
      this.globals();
      this.booted = true;
      try {
        bundle();
      } catch (e) {
        // Its reason in its journal (the last line of its standard error), then the instance crashes.
        out(2, `${(e && e.stack) || ''}\n${String(e)}\n`);
        throw e;
      }
    },
    /** Hands Escouade's lines to the plugin. */
    pump() {
      readAvailable();
      while (lines.length && !ended) {
        const line = lines.shift();
        let m;
        try {
          m = JSON.parse(line);
        } catch {
          m = null;
        }
        if (m && m.method === undefined && typeof m.id === 'string' && waiting.has(m.id)) {
          const w = waiting.get(m.id);
          waiting.delete(m.id);
          if (m.error) {
            const e = new Error(m.error.message);
            e.code = m.error.code;
            w.reject(e);
          } else w.resolve(m.result);
          continue;
        }
        if (m && m.method === 'timer/fired') {
          fired(m.params.timer);
          continue;
        }
        for (const cb of listeners) cb(line);
      }
    },
  };
})());
