// The prelude of a JS/TS plugin compiled to WASM by Escouade (ESC-319, prototype). It goes before the plugin's bundle
// (dist/plugin.mjs, as committed) and gives it what Node gave it, served by Escouade instead:
// - the protocol's lines on the standard input and output, as Node's process: `process.stdout.write`, and
//   `createInterface` of `node:readline` whose 'line' events are the lines Escouade sends;
// - `readFileSync` of `node:fs`, for the files of the plugin's own folder only (rules.json, channels.json), given by
//   Escouade at the start, in the first line (`escouade/boot`);
// - `fetch`, served by Escouade (`net/fetch`), which lets out only the hosts of `permissions.network`;
// - `console`, which goes to the journal (standard error), never into the protocol.
// Escouade calls the module's export `on-line` each time it has written lines: the prelude hands them to the plugin,
// and the call returns once the plugin has nothing left to run (QuickJS's job queue empty): no thread, no timer.
const __esc = (globalThis.__esc ??= (() => {
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  let pending = new Uint8Array(0);
  const lines = [];
  const lineHandlers = [];
  const closeHandlers = [];
  const waiting = new Map();
  let seq = 0;
  let files = {};
  let ended = false;

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

  function write(fd, text) {
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
      for (const [k, v] of Object.entries(h)) this.map[k.toLowerCase()] = String(v);
    }
    get(k) {
      return this.map[k.toLowerCase()] ?? null;
    }
  }

  class Response {
    constructor(r) {
      this.status = r.status;
      this.ok = r.status >= 200 && r.status < 300;
      this.headers = new Headers(r.headers);
      this.body_ = r.body ?? '';
    }
    async text() {
      return this.body_;
    }
    async json() {
      return JSON.parse(this.body_);
    }
  }

  function call(method, params) {
    const id = `w${++seq}`;
    return new Promise((resolve, reject) => {
      waiting.set(id, { resolve, reject });
      write(1, JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
    });
  }

  function fetch(input, init = {}) {
    const headers = init.headers instanceof Headers ? init.headers.map : (init.headers ?? {});
    return call('net/fetch', {
      url: String(input && input.href ? input.href : input),
      method: init.method ?? 'GET',
      headers,
      body: init.body == null ? null : String(init.body),
    }).then((r) => new Response(r));
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
        const rl = {
          on(event, cb) {
            if (event === 'line') lineHandlers.push(cb);
            else if (event === 'close') closeHandlers.push(cb);
            return rl;
          },
          close() {},
        };
        return rl;
      },
    },
  };

  const text = (args) => args.map((a) => (typeof a === 'string' ? a : JSON.stringify(a))).join(' ') + '\n';

  return {
    booted: false,
    url: 'file:///plugin/dist/plugin.mjs',
    node(name) {
      return node[name];
    },
    globals() {
      globalThis.URL ??= URL;
      globalThis.fetch = fetch;
      globalThis.Headers ??= Headers;
      globalThis.Response ??= Response;
      // No timer yet in the prototype: a timeout never fires (the SDK uses one only to bound `shutdown`).
      globalThis.setTimeout ??= () => ({ unref() {} });
      globalThis.clearTimeout ??= () => {};
      globalThis.console = {
        log: (...a) => write(2, text(a)),
        info: (...a) => write(2, text(a)),
        warn: (...a) => write(2, text(a)),
        error: (...a) => write(2, text(a)),
        debug: (...a) => write(2, text(a)),
      };
      globalThis.process = {
        env: {},
        platform: 'wasi',
        argv: [],
        stdin: {},
        stdout: {
          write(s, cb) {
            write(1, String(s));
            if (typeof cb === 'function') cb();
            return true;
          },
        },
        stderr: {
          write(s) {
            write(2, String(s));
            return true;
          },
        },
        exit() {
          ended = true;
        },
      };
    },
    /** The first line Escouade writes: the plugin's files. */
    boot(bundle) {
      readAvailable();
      const first = lines.shift();
      const frame = first ? JSON.parse(first) : {};
      files = (frame['escouade/boot'] && frame['escouade/boot'].files) || {};
      this.globals();
      this.booted = true;
      bundle();
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
          if (m.error) w.reject(new Error(m.error.message));
          else w.resolve(m.result);
          continue;
        }
        for (const cb of lineHandlers) cb(line);
      }
    },
  };
})());
