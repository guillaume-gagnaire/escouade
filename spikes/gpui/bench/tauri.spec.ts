// La même mesure que le prototype GPUI (ESC-365, docs/GPUI.md §5), sur l'app d'aujourd'hui : l'exe installé
// (ESCOUADE_E2E_EXE), lancé par la fixture des e2e sur un dossier de données jetable (sandbox : ni instance unique, ni
// état de fenêtre de l'utilisateur). Rien de l'app ne change : la page est instrumentée de l'extérieur, comme le fait
// tests/e2e/terminal-latency.spec.ts.
//
//   ESCOUADE_E2E_EXE=%LOCALAPPDATA%\Escouade\escouade.exe ESCOUADE_E2E_CDP_PORT=<port> ESCOUADE_E2E_SHOW=1
//     npx playwright test -c spikes/gpui/bench/playwright.config.ts
//
// Mesures, écrites dans spikes/gpui/results/tauri.jsonl :
// - fenêtre : du lancement du process (juste avant spawn) au first-contentful-paint de la page, sur 1 + RESTARTS lancements ;
// - repos : mémoire de l'interface (escouade.exe et ses msedgewebview2) et de tout l'arbre, au démarrage, puis avec un
//   projet et un terminal, puis avec dix terminaux ;
// - gros flux : la commande du prototype (`gpui-spike.exe --flood N` puis la sentinelle) écrite au PowerShell du terminal
//   par la commande `term_write`, comme le prototype l'écrit à son PTY ; du write à la sentinelle reçue par la page, puis à
//   la frame qui suit ; le plus long retard d'un setInterval de 20 ms pendant le flux (la fluidité de la page) ; le CPU des
//   process de l'interface. Un terminal (100 000 lignes), puis dix à la fois (20 000 lignes chacun).

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { test, expect, addProject } from '../../../tests/e2e/fixture';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const RESULTS = path.join(HERE, '..', 'results');
const OUT = path.join(RESULTS, process.env.ESC365_OUT ?? 'tauri.jsonl');
const SPIKE = path.join(HERE, '..', 'target', 'release', 'gpui-spike.exe');
const MEM = path.join(HERE, 'mem.ps1');
const SHOT = path.join(HERE, 'shot.ps1');
const RESTARTS = Number(process.env.ESC365_RESTARTS ?? 9);
const FLOODS = Number(process.env.ESC365_FLOODS ?? 3);

function record(o: object) {
  fs.mkdirSync(RESULTS, { recursive: true });
  fs.appendFileSync(OUT, JSON.stringify(o) + '\n');
  console.log(JSON.stringify(o));
}

function ps(file: string, ...args: string[]): string {
  return execFileSync('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', file, ...args], {
    encoding: 'utf8',
    maxBuffer: 64 << 20,
  });
}

/** Le pid d'escouade.exe de ce test : le parent du msedgewebview2 « browser » dont la ligne de commande cite son dossier. */
function mainPid(root: string): number {
  const script =
    "Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | ForEach-Object { '{0}|{1}|{2}' -f $_.ProcessId, $_.ParentProcessId, $_.CommandLine }";
  const text = execFileSync('powershell.exe', ['-NoProfile', '-Command', script], { encoding: 'utf8', maxBuffer: 64 << 20 });
  const tag = path.basename(root);
  for (const line of text.split(/\r?\n/)) {
    const m = /^(\d+)\|(\d+)\|(.*)$/.exec(line);
    if (m && m[3].includes(tag) && !/--type=/.test(m[3])) return Number(m[2]);
  }
  throw new Error('escouade.exe de ce test introuvable');
}

function memory(root: string) {
  const out = ps(MEM, '-Root', String(mainPid(root)))
    .trim()
    .split(/\r?\n/);
  return JSON.parse(out[out.length - 1]);
}

/** CPU (ms) de l'interface : escouade.exe et ses msedgewebview2. */
function uiCpu(main: number): number {
  const script = `$all = Get-CimInstance Win32_Process; $ids = @(${main}); $more = $true; while ($more) { $more = $false; foreach ($p in $all) { if ($ids -contains [int]$p.ParentProcessId -and -not ($ids -contains [int]$p.ProcessId) -and $p.Name -match 'msedgewebview2') { $ids += [int]$p.ProcessId; $more = $true } } }; ($all | Where-Object { $ids -contains [int]$_.ProcessId } | Measure-Object -Property UserModeTime -Sum).Sum + ($all | Where-Object { $ids -contains [int]$_.ProcessId } | Measure-Object -Property KernelModeTime -Sum).Sum`;
  return Number(execFileSync('powershell.exe', ['-NoProfile', '-Command', script], { encoding: 'utf8' }).trim()) / 1e4;
}

async function firstPaint(page: Page): Promise<number> {
  const until = Date.now() + 30_000;
  while (Date.now() < until) {
    const at = await page
      .evaluate(() => {
        if (location.protocol === 'about:') return null;
        const e = performance.getEntriesByName('first-contentful-paint')[0];
        return e ? performance.timeOrigin + e.startTime : null;
      })
      .catch(() => null);
    if (at !== null) return at;
    await new Promise((r) => setTimeout(r, 100));
  }
  return NaN;
}

/* eslint-disable @typescript-eslint/no-explicit-any */
/** Les terminaux ouverts (leur id et leur canal) et, par canal, les octets reçus et l'heure de la sentinelle. */
function instrument() {
  const w = window as any;
  if (w.__esc365) return;
  const now = () => performance.timeOrigin + performance.now();
  const ti = w.__TAURI_INTERNALS__;
  const b: any = { now, spawns: [], bytes: {}, done: {}, painted: {}, tail: {}, maxLag: 0 };
  const calls = new Map<number, any>();
  const fetch0 = window.fetch;
  window.fetch = function (this: unknown, input: any, init?: any) {
    const m = /^(?:https?:\/\/ipc\.localhost|ipc:\/\/localhost)\/([^?]+)/.exec(
      typeof input === 'string' ? input : String(input?.url ?? input),
    );
    if (m && init?.headers?.get && decodeURIComponent(m[1]) === 'term_spawn') {
      const rec = { channel: Number(String(JSON.parse(init.body).output).replace('__CHANNEL__:', '')), termId: null };
      b.spawns.push(rec);
      calls.set(Number(init.headers.get('Tauri-Callback')), rec);
    }
    return fetch0.apply(this, arguments as any);
  } as typeof fetch;
  const dec = new TextDecoder();
  const wrap = (id: number, fn: (data: any) => unknown) =>
    function (this: unknown, data: any) {
      const rec = calls.get(id);
      if (rec) {
        calls.delete(id);
        rec.termId = data?.id;
        return fn.call(this, data);
      }
      const message = data?.message;
      if (message instanceof ArrayBuffer) {
        b.bytes[id] = (b.bytes[id] ?? 0) + message.byteLength;
        const text = (b.tail[id] ?? '') + dec.decode(message);
        if (b.armed && b.done[id] === undefined && text.includes('FLOOD-DONE')) {
          b.done[id] = now();
          try {
            return fn.call(this, data);
          } finally {
            setTimeout(() => requestAnimationFrame(() => (b.painted[id] = now())), 0);
          }
        }
        b.tail[id] = text.slice(-16);
      }
      return fn.call(this, data);
    };
  const set = ti.callbacks.set;
  ti.callbacks.set = function (this: Map<number, unknown>, id: number, fn: (data: any) => unknown) {
    return set.call(this, id, wrap(id, fn));
  };
  for (const [id, fn] of [...ti.callbacks.entries()]) set.call(ti.callbacks, id, wrap(id, fn));
  let last = now();
  setInterval(() => {
    const t = now();
    if (b.armed) b.maxLag = Math.max(b.maxLag, t - last - 20);
    last = t;
  }, 20);
  w.__esc365 = b;
}
/* eslint-enable @typescript-eslint/no-explicit-any */

const bench = <T>(page: Page, f: (b: any) => T) => page.evaluate(f as any, undefined) as Promise<T>; // eslint-disable-line @typescript-eslint/no-explicit-any

async function newTerminal(page: Page): Promise<{ termId: string; channel: number }> {
  const before = await bench(page, () => (window as any).__esc365.spawns.length); // eslint-disable-line @typescript-eslint/no-explicit-any
  // Le bouton est dans la barre du projet : y revenir si une autre vue s'est ouverte.
  const button = page.getByRole('button', { name: 'Nouveau terminal' });
  if (!(await button.isVisible())) await page.locator('nav.rail .list button').first().click();
  await button.click();
  const item = page
    .getByRole('menuitem')
    .filter({ hasText: /PowerShell/ })
    .first();
  await item.click();
  await expect
    .poll(() => page.evaluate((i) => (window as any).__esc365.spawns[i]?.termId ?? null, before), { timeout: 60_000 }) // eslint-disable-line @typescript-eslint/no-explicit-any
    .toBeTruthy();
  const s = await page.evaluate((i) => (window as any).__esc365.spawns[i], before); // eslint-disable-line @typescript-eslint/no-explicit-any
  // Le shell a écrit son invite.
  await expect
    .poll(() => page.evaluate((ch) => (window as any).__esc365.bytes[ch] ?? 0, s.channel), { timeout: 60_000 }) // eslint-disable-line @typescript-eslint/no-explicit-any
    .toBeGreaterThan(0);
  return s;
}

/** Le gros flux dans `terms`, à la fois ; rend le temps jusqu'à la dernière sentinelle reçue, puis peinte. */
async function flood(page: Page, root: string, terms: { termId: string; channel: number }[], lines: number) {
  const main = mainPid(root);
  await page.evaluate(() => {
    const b = (window as any).__esc365; // eslint-disable-line @typescript-eslint/no-explicit-any
    b.done = {};
    b.painted = {};
    b.tail = {};
    b.maxLag = 0;
    b.armed = true;
  });
  const cpu0 = uiCpu(main);
  const sent = await page.evaluate(
    async ({ terms, cmd }) => {
      const w = window as any; // eslint-disable-line @typescript-eslint/no-explicit-any
      const t = w.__esc365.now();
      await Promise.all(terms.map((x: any) => w.__TAURI_INTERNALS__.invoke('term_write', { id: x.termId, data: cmd }))); // eslint-disable-line @typescript-eslint/no-explicit-any
      return t;
    },
    { terms, cmd: `& '${SPIKE}' --flood ${lines}; Write-Output ('FLOOD'+'-DONE')\r` },
  );
  await expect
    .poll(
      () =>
        page.evaluate(
          (chs) => chs.every((c: number) => (window as any).__esc365.painted[c] !== undefined), // eslint-disable-line @typescript-eslint/no-explicit-any
          terms.map((t) => t.channel),
        ),
      { timeout: 300_000, intervals: [250] },
    )
    .toBe(true);
  const r = await page.evaluate(
    (chs) => {
      const b = (window as any).__esc365; // eslint-disable-line @typescript-eslint/no-explicit-any
      b.armed = false;
      return {
        done: Math.max(...chs.map((c: number) => b.done[c])),
        painted: Math.max(...chs.map((c: number) => b.painted[c])),
        maxLag: b.maxLag,
      };
    },
    terms.map((t) => t.channel),
  );
  await page.waitForTimeout(500);
  const cpu = uiCpu(main) - cpu0;
  return {
    parsed_ms: Math.round(r.done - sent),
    painted_ms: Math.round(r.painted - sent),
    max_loop_lag_ms: Math.round(r.maxLag),
    ui_cpu_ms: Math.round(cpu),
  };
}

test('mesures de l’app d’aujourd’hui (ESC-365)', async ({ app }) => {
  test.setTimeout(30 * 60_000);
  fs.mkdirSync(RESULTS, { recursive: true });
  const exe = process.env.ESCOUADE_E2E_EXE ?? 'debug';
  const windows = [(await firstPaint(app.page)) - app.launchedAt()];
  for (let i = 0; i < RESTARTS; i++) {
    const page = await app.restart();
    windows.push((await firstPaint(page)) - app.launchedAt());
  }
  const sorted = [...windows].sort((a, b) => a - b);
  record({
    kind: 'startup',
    exe,
    launches: windows.length,
    median: Math.round(sorted[Math.floor((sorted.length - 1) / 2)]),
    min: Math.round(sorted[0]),
    max: Math.round(sorted.at(-1)!),
    all: windows.map(Math.round),
  });

  const page = app.page;
  await page.waitForTimeout(10_000);
  record({ kind: 'rest-start', exe, mem: memory(app.root) });

  await page.evaluate(instrument);
  await addProject(page, app.repo);
  const first = await newTerminal(page);
  await page.waitForTimeout(10_000);
  record({ kind: 'rest-project-1-terminal', exe, mem: memory(app.root) });
  ps(SHOT, '-ProcessId', String(mainPid(app.root)), '-Out', path.join(RESULTS, 'tauri-1-terminal.png'));

  const terms = [first];
  for (let i = 1; i < 10; i++) terms.push(await newTerminal(page));
  await page.waitForTimeout(10_000);
  record({ kind: 'rest-10-terminals', exe, mem: memory(app.root) });

  // Le flux dans un terminal : le dernier ouvert, celui qu'on voit.
  const shown = terms[terms.length - 1];
  for (let i = 1; i <= FLOODS; i++) {
    const f = await flood(page, app.root, [shown], 100_000);
    record({ kind: 'flood', exe, run: i, lines: 100_000, result: f, after: memory(app.root) });
    if (i === 1) ps(SHOT, '-ProcessId', String(mainPid(app.root)), '-Out', path.join(RESULTS, 'tauri-apres-flux.png'));
  }
  for (let i = 1; i <= FLOODS; i++) {
    const f = await flood(page, app.root, terms, 20_000);
    record({ kind: 'ten-flood', exe, run: i, lines_each: 20_000, result: f, after: memory(app.root) });
  }
  ps(SHOT, '-ProcessId', String(mainPid(app.root)), '-Out', path.join(RESULTS, 'tauri-10-terminaux.png'));
});
