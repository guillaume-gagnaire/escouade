#!/usr/bin/env node
// ESC-444 (study): the bench of the editors on the three systems. For each mode (CodeMirror, the VS Code view, the VS Code
// view with the Node extension host), it starts the built app on a throwaway data folder and a test project, with
// ESCOUADE_VSCODE_STUDY: the page runs its phases itself (src/lib/vscode/study.ts) and logs them. Beside it, this script
// samples the memory of the app's processes (the app and its webview's: WebView2, WebKitGTK's, WKWebView's) and takes a
// screenshot at the phase `shot`. It prints a table, and writes results.json in --out.
//
//   node scripts/study/editor-bench.mjs --exe <app> --out <folder> [--modes codemirror,vscode,vscode-node]
//        [--server <VS Code server folder>] [--rest 8000] [--runs 1] [--shots] [--hidden]
//
// Memory: the resident set (RSS) of each process of the app's tree summed (on Linux, also the proportional set, PSS,
// which counts a shared page once); on macOS, WebKit's processes (com.apple.WebKit.*) are launchd's children, counted by
// name. On Windows, the working set and the private bytes of the tree.

import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, a, i, all) => {
    if (a.startsWith('--')) acc.push([a.slice(2), all[i + 1] && !all[i + 1].startsWith('--') ? all[i + 1] : true]);
    return acc;
  }, []),
);
const EXE = path.resolve(args.exe ?? '');
const OUT = path.resolve(args.out ?? 'study-out');
const MODES = String(args.modes ?? 'codemirror,vscode').split(',');
const REST = Number(args.rest ?? 8000);
const RUNS = Number(args.runs ?? 1);
const SERVER = args.server ? path.resolve(args.server) : null;
const SHOTS = !!args.shots;
/** The window hidden (a run on a desktop in use): WebView2 told to draw it all the same, as the e2e tests do. */
const HIDDEN = !!args.hidden;
const WIN = process.platform === 'win32';
const MAC = process.platform === 'darwin';

if (!fs.existsSync(EXE)) throw new Error(`no app at ${EXE}`);
fs.mkdirSync(OUT, { recursive: true });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const git = (cwd, ...a) => execFileSync('git', a, { cwd, stdio: 'pipe' });

/** A git repository: package.json, tsconfig.json, src/app.ts with an error, and src/f1.ts to f10.ts of 200 lines each. */
function makeProject(dir) {
  fs.mkdirSync(path.join(dir, 'src'), { recursive: true });
  git(dir, 'init', '-q', '-b', 'main');
  git(dir, 'config', 'user.email', 'bench@test');
  git(dir, 'config', 'user.name', 'bench');
  fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify({ name: 'demo-api', private: true }, null, 2));
  fs.writeFileSync(
    path.join(dir, 'tsconfig.json'),
    JSON.stringify({ compilerOptions: { strict: true, target: 'es2022', module: 'esnext', noEmit: true }, include: ['src'] }, null, 2),
  );
  fs.writeFileSync(path.join(dir, 'src', 'app.ts'), "export const a: number = 'pas un nombre';\n");
  fs.writeFileSync(path.join(dir, 'notes.md'), '# Notes\n\nThis file has a mistake: recieve.\n');
  for (let i = 1; i <= 10; i++) {
    const lines = [`// Fichier ${i}`, `export interface Item${i} { id: number; name: string; tags: string[] }`];
    for (let j = 0; j < 40; j++)
      lines.push(
        `export function f${i}_${j}(items: Item${i}[], q: string): Item${i}[] {`,
        `  return items.filter((x) => x.name.includes(q) && x.id % ${j + 2} === 0);`,
        `}`,
        '',
      );
    fs.writeFileSync(path.join(dir, 'src', `f${i}.ts`), lines.join('\n'));
  }
  git(dir, 'add', '-A');
  git(dir, 'commit', '-qm', 'init');
}

// ---------- memory of the app's processes ----------

function linuxProcs() {
  const out = [];
  for (const d of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(d)) continue;
    try {
      const stat = fs.readFileSync(`/proc/${d}/stat`, 'utf8');
      const ppid = Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1]);
      const name = stat.slice(stat.indexOf('(') + 1, stat.lastIndexOf(')'));
      let rss = 0;
      let pss = 0;
      try {
        const roll = fs.readFileSync(`/proc/${d}/smaps_rollup`, 'utf8');
        rss = Number(/^Rss:\s+(\d+)/m.exec(roll)?.[1] ?? 0) * 1024;
        pss = Number(/^Pss:\s+(\d+)/m.exec(roll)?.[1] ?? 0) * 1024;
      } catch {
        const st = fs.readFileSync(`/proc/${d}/status`, 'utf8');
        rss = Number(/^VmRSS:\s+(\d+)/m.exec(st)?.[1] ?? 0) * 1024;
      }
      out.push({ pid: Number(d), ppid, name, rss, pss });
    } catch {
      // Gone meanwhile.
    }
  }
  return out;
}

function psProcs() {
  const text = execFileSync('ps', ['-axo', 'pid=,ppid=,rss=,comm='], { encoding: 'utf8' });
  return text
    .split('\n')
    .filter(Boolean)
    .map((l) => {
      const m = /^\s*(\d+)\s+(\d+)\s+(\d+)\s+(.*)$/.exec(l);
      return m ? { pid: +m[1], ppid: +m[2], rss: +m[3] * 1024, name: m[4] } : null;
    })
    .filter(Boolean);
}

function winProcs() {
  const ps =
    'Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,WorkingSetSize,PrivatePageCount | ConvertTo-Json -Compress';
  const json = execFileSync('powershell.exe', ['-NoProfile', '-Command', ps], { encoding: 'utf8', maxBuffer: 64 << 20 });
  return JSON.parse(json).map((p) => ({
    pid: p.ProcessId,
    ppid: p.ParentProcessId,
    name: p.Name,
    rss: Number(p.WorkingSetSize),
    priv: Number(p.PrivatePageCount),
  }));
}

/** The app's processes: its tree from `root`, and on macOS WebKit's, by name. */
function sample(root) {
  const all = WIN ? winProcs() : MAC ? psProcs() : linuxProcs();
  const kids = new Map();
  for (const p of all) kids.set(p.ppid, [...(kids.get(p.ppid) ?? []), p]);
  const tree = [];
  const seen = new Set();
  const walk = (pid) => {
    for (const k of kids.get(pid) ?? []) {
      if (seen.has(k.pid)) continue;
      seen.add(k.pid);
      tree.push(k);
      walk(k.pid);
    }
  };
  const self = all.find((p) => p.pid === root);
  if (self) tree.push(self);
  walk(root);
  if (MAC) for (const p of all) if (/com\.apple\.WebKit/.test(p.name) && !seen.has(p.pid)) tree.push(p);
  const sum = (k, list = tree) => list.reduce((a, p) => a + (p[k] ?? 0), 0);
  // Its parts: the core (the app's own process), the webview's processes, and the programs it started (language
  // servers, the Node extension host, the fake CLI).
  const webview = tree.filter((p) => /msedgewebview2|WebKit|WebContent/i.test(p.name));
  const core = tree.filter((p) => p.pid === root);
  const others = tree.filter((p) => !webview.includes(p) && !core.includes(p));
  const key = MAC || WIN ? 'rss' : 'pss';
  return {
    at: Date.now(),
    n: tree.length,
    rss: sum('rss'),
    pss: sum('pss'),
    priv: sum('priv'),
    core: sum(key, core),
    webview: sum(key, webview),
    servers: sum(key, others),
    names: tree.map((p) => p.name),
  };
}

// ---------- screenshot ----------

function screenshot(file) {
  try {
    if (MAC) execFileSync('screencapture', ['-x', file]);
    else if (WIN) {
      const ps = `Add-Type -AssemblyName System.Windows.Forms,System.Drawing; $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $i=New-Object System.Drawing.Bitmap $b.Width,$b.Height; $g=[System.Drawing.Graphics]::FromImage($i); $g.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size); $i.Save('${file}')`;
      execFileSync('powershell.exe', ['-NoProfile', '-Command', ps]);
    } else execFileSync('import', ['-window', 'root', file]);
    return true;
  } catch (e) {
    console.warn(`screenshot failed: ${e.message}`);
    return false;
  }
}

// ---------- one run ----------

async function run(mode, n) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), `esc444-${mode}-`));
  const data = path.join(dir, 'data');
  const project = path.join(dir, 'demo-api');
  fs.mkdirSync(data, { recursive: true });
  makeProject(project);
  const fake = path.join(ROOT, 'tests', 'fixtures', WIN ? 'fake-claude.cmd' : 'fake-claude');
  fs.writeFileSync(
    path.join(data, 'settings.json'),
    JSON.stringify({
      claudePath: fake,
      sound: false,
      osNotifications: false,
      idleStopMinutes: 0,
      language: 'fr',
      telemetry: false,
      nameAsked: true,
      accountsVersion: 1,
      autoUpdate: false,
    }),
  );
  const logFile = path.join(dir, 'study.log');
  const tsls = path.join(ROOT, 'node_modules', '.bin', WIN ? 'typescript-language-server.cmd' : 'typescript-language-server');
  const env = {
    ...process.env,
    ESCOUADE_DATA_DIR: data,
    ESCOUADE_VSCODE_STUDY: logFile,
    ESCOUADE_VSCODE_STUDY_OPTS: JSON.stringify({ project, mode, rest: REST }),
    ESCOUADE_STUDY_TS_LSP: tsls,
    FAKE_CLAUDE_LOG: path.join(dir, 'fake-claude.jsonl'),
    CLAUDE_CONFIG_DIR: path.join(dir, 'claude'),
    FAKE_CLAUDE_HOME: path.join(dir, 'home'),
    WEBVIEW2_USER_DATA_FOLDER: path.join(dir, 'webview'),
    ...(SERVER ? { ESCOUADE_STUDY_VSCODE_SERVER: SERVER } : {}),
  };
  delete env.ESCOUADE_HIDDEN_WINDOW;
  if (HIDDEN) {
    env.ESCOUADE_HIDDEN_WINDOW = '1';
    env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS =
      '--disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-background-timer-throttling';
  }
  const started = Date.now();
  const child = spawn(EXE, [], { env, stdio: ['ignore', 'pipe', 'pipe'] });
  const stderr = [];
  child.stderr.on('data', (d) => stderr.push(String(d)));
  child.stdout.on('data', (d) => stderr.push(String(d)));
  let exited = null;
  child.on('exit', (code, sig) => (exited = { code, sig }));

  const samples = [];
  let shotTaken = null;
  const lines = () =>
    fs.existsSync(logFile)
      ? fs
          .readFileSync(logFile, 'utf8')
          .split('\n')
          .filter(Boolean)
          .map((l) => ({ at: Number(l.split('\t')[0]), text: l.split('\t').slice(1).join('\t') }))
      : [];
  const deadline = started + 6 * 60_000;
  while (!exited && Date.now() < deadline) {
    try {
      samples.push(sample(child.pid));
    } catch (e) {
      console.warn(`sample failed: ${e.message}`);
    }
    const l = lines();
    if (SHOTS && !shotTaken && l.some((x) => x.text === 'shot')) {
      const file = path.join(OUT, `${process.platform}-${mode}-${n}.png`);
      if (screenshot(file)) shotTaken = file;
    }
    if (l.some((x) => x.text === 'done')) break;
    await sleep(WIN ? 400 : 250);
  }
  await sleep(1500);
  if (!exited) child.kill();
  await sleep(500);

  const log = lines();
  /** The memory at the end of a rest: the samples of its last 3 s, averaged. */
  const at = (name) => {
    const m = log.find((x) => x.text === `mark ${name}`);
    if (!m) return null;
    const s = samples.filter((x) => x.at <= m.at && x.at >= m.at - 3000);
    if (!s.length) return null;
    const avg = (k) => Math.round(s.reduce((a, x) => a + x[k], 0) / s.length / 2 ** 20);
    return {
      rss: avg('rss'),
      pss: avg('pss') || null,
      priv: avg('priv') || null,
      core: avg('core'),
      webview: avg('webview'),
      servers: avg('servers'),
      procs: s.at(-1).n,
    };
  };
  const num = (prefix) => {
    const x = log.find((l) => l.text.startsWith(prefix + ' '));
    return x ? Number(x.text.split(' ')[1]) : null;
  };
  const result = {
    os: `${process.platform} ${os.release()}`,
    mode,
    run: n,
    open_ms: num('open'),
    ten_ms: num('ten'),
    idleApp: at('idle-app'),
    restEditor: at('rest-editor'),
    restTen: at('rest-ten'),
    peakRss: Math.round(Math.max(0, ...samples.map((s) => s.rss)) / 2 ** 20),
    errors: num('errors'),
    failed: log.find((l) => l.text.startsWith('failed'))?.text ?? null,
    marks: log.find((l) => l.text.startsWith('marks'))?.text ?? null,
    lsp: log.find((l) => l.text.startsWith('lsp '))?.text ?? null,
    extensions: log.filter((l) => l.text.startsWith('ext ') || l.text.startsWith('cspell ')).map((l) => l.text),
    pageErrors: log.filter((l) => l.text.startsWith('error ')).map((l) => l.text.slice(6)),
    env: log.find((l) => l.text.startsWith('env'))?.text ?? null,
    exited,
    shot: shotTaken,
    processes: [...new Set(samples.at(-1)?.names ?? [])],
  };
  if (!log.some((l) => l.text === 'done')) result.stderr = stderr.join('').slice(-3000);
  try {
    fs.rmSync(dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 500 });
  } catch {
    // A file still held: left in the temporary folder.
  }
  return result;
}

const results = [];
for (const mode of MODES) {
  if (mode === 'vscode-node' && !SERVER) {
    console.log('vscode-node: skipped (no --server)');
    continue;
  }
  for (let n = 1; n <= RUNS; n++) {
    console.log(`${mode} #${n}…`);
    const r = await run(mode, n);
    results.push(r);
    console.log(JSON.stringify(r));
  }
}
fs.writeFileSync(path.join(OUT, `results-${process.platform}.json`), JSON.stringify(results, null, 2));

const mb = (x, k = 'rss') => (x?.[k] != null ? `${x[k]} Mo` : '—');
const parts = (x) => (x ? `${x.core} / ${x.webview} / ${x.servers}` : '—');
console.log(
  `\n| Système | Mode | Ouverture | 10 fichiers | Repos (app) | Repos (éditeur) | Repos (10 fichiers) | dont cœur / webview / serveurs | Pic | Erreurs |`,
);
console.log(`|---|---|---|---|---|---|---|---|---|---|`);
for (const r of results)
  console.log(
    `| ${process.platform} | ${r.mode} | ${r.open_ms ?? '—'} ms | ${r.ten_ms ?? '—'} ms | ${mb(r.idleApp)} | ${mb(r.restEditor)} | ${mb(r.restTen)} | ${parts(r.restTen)} | ${r.peakRss} Mo | ${r.errors ?? '—'}${r.failed ? ` (${r.failed})` : ''} |`,
  );
