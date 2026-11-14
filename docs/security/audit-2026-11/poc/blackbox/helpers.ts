// What the black-box tests of the audit share: the ports they were given, raw HTTP requests (a header forged as a
// browser or another program would), the test instance's processes and what they listen on, and the notes each test
// keeps for the report.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';

const BASE = Number(process.env.AUDIT_PORT_BASE ?? 0);
if (!BASE) throw new Error('AUDIT_PORT_BASE: give the first of the four ports the tests may use');

/** The ports of the tests: the MCP server of the test instance, the fake update server, a decoy page, a spare. */
export const PORTS = { mcp: BASE, updates: BASE + 1, decoy: BASE + 2, spare: BASE + 3 };

const OUT = process.env.AUDIT_OUT ?? path.join(process.cwd(), 'audit-out');

/**
 * Keeps what a test saw, for the report: `<AUDIT_OUT>/<name>.json`. Never a token (`redact` first), and never the name
 * of who ran the tests: the home folder is written `<profil>`, in each of the spellings JSON gives it.
 */
export function keep(name: string, seen: unknown) {
  fs.mkdirSync(OUT, { recursive: true });
  let text = JSON.stringify(seen, null, 2);
  const home = os.homedir();
  const doubled = home.split('\\').join('\\\\');
  for (const spelling of [doubled.split('\\').join('\\\\'), doubled, home, home.split('\\').join('/')]) {
    text = text.split(spelling).join('<profil>');
  }
  text = text.split(os.userInfo().username).join('<utilisateur>');
  fs.writeFileSync(path.join(OUT, `${name}.json`), `${text}\n`);
}

/** `text` without the tokens given (those of the test instance): the notes and the logs quoted never hold one. */
export function redact(text: string, ...tokens: string[]): string {
  let out = text;
  for (const t of tokens.filter((t) => t.length > 8)) out = out.split(t).join('<jeton>');
  return out;
}

export interface Raw {
  status: number;
  headers: Record<string, string>;
  body: string;
}

/** One HTTP request written as it is to 127.0.0.1:`port`: the status, the headers and the body that came back. */
export function raw(port: number, request: string, timeout = 15_000): Promise<Raw> {
  return new Promise((resolve, reject) => {
    const socket = net.connect({ host: '127.0.0.1', port });
    const chunks: Buffer[] = [];
    const done = () => {
      const text = Buffer.concat(chunks).toString('utf8');
      const [head, ...rest] = text.split('\r\n\r\n');
      const lines = head.split('\r\n');
      const status = Number(lines[0]?.split(' ')[1] ?? 0);
      const headers: Record<string, string> = {};
      for (const l of lines.slice(1)) {
        const at = l.indexOf(':');
        if (at > 0) headers[l.slice(0, at).trim().toLowerCase()] = l.slice(at + 1).trim();
      }
      resolve({ status, headers, body: rest.join('\r\n\r\n') });
    };
    socket.setTimeout(timeout, () => {
      socket.destroy();
      done();
    });
    socket.on('connect', () => socket.write(request));
    socket.on('data', (c) => chunks.push(c));
    socket.on('end', done);
    socket.on('close', done);
    socket.on('error', (e) => (chunks.length ? done() : reject(e)));
  });
}

/** A JSON-RPC POST to the MCP endpoint, its headers as given (`host` and `authorization` are the caller's to forge). */
export function mcpRequest(port: number, headers: Record<string, string>, body: object | string, target = '/mcp', method = 'POST') {
  const text = typeof body === 'string' ? body : JSON.stringify(body);
  const all = {
    'content-type': 'application/json',
    accept: 'application/json, text/event-stream',
    'content-length': String(Buffer.byteLength(text)),
    connection: 'close',
    ...headers,
  };
  const head = Object.entries(all)
    .filter(([, v]) => v !== '')
    .map(([k, v]) => `${k}: ${v}`)
    .join('\r\n');
  return raw(port, `${method} ${target} HTTP/1.1\r\n${head}\r\n\r\n${text}`);
}

/** A tool of Escouade's MCP server called with `token`: what it answered (its text, parsed when it is JSON). */
export async function tool(port: number, token: string, name: string, args: object) {
  const r = await mcpRequest(
    port,
    { host: `127.0.0.1:${port}`, authorization: `Bearer ${token}` },
    { jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name, arguments: args } },
  );
  let json: { result?: { isError?: boolean; content?: { text?: string }[] }; error?: { message: string } } | null = null;
  try {
    json = JSON.parse(r.body);
  } catch {
    // Not JSON: a refusal's plain text.
  }
  const text = json?.result?.content?.map((c) => c.text ?? '').join('\n') ?? json?.error?.message ?? r.body;
  let value: unknown = text;
  try {
    value = JSON.parse(text);
  } catch {
    // A sentence.
  }
  return { status: r.status, isError: !!json?.result?.isError || !!json?.error, text, value };
}

function powershell(script: string): string {
  return execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8', timeout: 60_000 });
}

/**
 * The processes of the test instance: the app started on the data folder `data` (found by its WebView2's own folder,
 * which is the test's), and everything it started. Never another Escouade's.
 */
export function instanceProcesses(root: string): { pid: number; parent: number; name: string }[] {
  const all = JSON.parse(
    powershell('Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,CommandLine | ConvertTo-Json -Compress'),
  ) as { ProcessId: number; ParentProcessId: number; Name: string; CommandLine: string | null }[];
  const mark = path.basename(root).toLowerCase();
  const webviews = all.filter((p) => p.Name.toLowerCase() === 'msedgewebview2.exe' && (p.CommandLine ?? '').toLowerCase().includes(mark));
  // The app is the parent of its WebView2's browser process.
  const apps = new Set(
    webviews.map((w) => w.ParentProcessId).filter((pid) => all.some((p) => p.ProcessId === pid && p.Name.toLowerCase() === 'escouade.exe')),
  );
  const ours = new Set<number>(apps);
  for (let grew = true; grew;) {
    grew = false;
    for (const p of all) {
      if (!ours.has(p.ProcessId) && ours.has(p.ParentProcessId)) {
        ours.add(p.ProcessId);
        grew = true;
      }
    }
  }
  return all.filter((p) => ours.has(p.ProcessId)).map((p) => ({ pid: p.ProcessId, parent: p.ParentProcessId, name: p.Name }));
}

/** What the processes `pids` listen on: TCP (address and port) and UDP. */
export function listeners(pids: number[]): { protocol: string; address: string; port: number; pid: number }[] {
  if (!pids.length) return [];
  const list = pids.join(',');
  const tcp = JSON.parse(
    powershell(
      `@(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object { @(${list}) -contains $_.OwningProcess } | Select-Object LocalAddress,LocalPort,OwningProcess) | ConvertTo-Json -Compress`,
    ) || '[]',
  ) as { LocalAddress: string; LocalPort: number; OwningProcess: number }[];
  const udp = JSON.parse(
    powershell(
      `@(Get-NetUDPEndpoint -ErrorAction SilentlyContinue | Where-Object { @(${list}) -contains $_.OwningProcess } | Select-Object LocalAddress,LocalPort,OwningProcess) | ConvertTo-Json -Compress`,
    ) || '[]',
  ) as { LocalAddress: string; LocalPort: number; OwningProcess: number }[];
  return [
    ...[tcp].flat().map((t) => ({ protocol: 'tcp', address: t.LocalAddress, port: t.LocalPort, pid: t.OwningProcess })),
    ...[udp].flat().map((u) => ({ protocol: 'udp', address: u.LocalAddress, port: u.LocalPort, pid: u.OwningProcess })),
  ];
}

/** The memory the process `pid` holds (its private bytes), in MiB. */
export function memoryMb(pid: number): number {
  return Math.round(Number(powershell(`(Get-Process -Id ${pid}).PrivateMemorySize64`).trim()) / (1 << 20));
}

/** A command of the app called from its own window, as its interface does. */
export function invoke<T>(page: Page, command: string, args: object = {}): Promise<T> {
  return page.evaluate(
    ([c, a]) =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(c, a),
    [command, args] as [string, object],
  ) as Promise<T>;
}

/** Every file under `dir`, with its size. */
export function filesUnder(dir: string): { file: string; bytes: number }[] {
  const out: { file: string; bytes: number }[] = [];
  const walk = (d: string) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.isFile()) out.push({ file: p, bytes: fs.statSync(p).size });
    }
  };
  if (fs.existsSync(dir)) walk(dir);
  return out;
}

/** The text files under `dir` (2 MiB at most each) that hold `needle`. */
export function filesHolding(dir: string, needle: string): string[] {
  return filesUnder(dir)
    .filter((f) => f.bytes <= 2 << 20)
    .filter((f) => fs.readFileSync(f.file).includes(needle))
    .map((f) => path.relative(dir, f.file));
}
