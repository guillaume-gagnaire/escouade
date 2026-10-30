// The startup measures of INT-0, the reference measures (docs/PLUGINS.md §4.8): how long the app takes to show its window,
// and its first agent to be ready, before any integration runs in a process of its own, then after. A measure, not a test of
// the app: skipped unless ESCOUADE_BENCH is set (`npm run bench:int0` sets it).
//
//   ESCOUADE_BENCH=1 [ESCOUADE_BENCH_LABEL=before] [ESCOUADE_BENCH_LAUNCHES=10] [ESCOUADE_E2E_EXE=…]
//     npx playwright test tests/e2e/startup-bench.spec.ts
//
// Each launch is the app started again on the same data folder (one project, its agent, which answered once before):
// - window: from the launch of its process (just before `spawn`) to the first paint of its page (`first-contentful-paint`,
//   the page's own clock, `performance.timeOrigin`, compared with the node clock as it is: both are the machine's);
// - first agent ready: from the start of its harness's process (the fake CLI, resumed on its session) to its answer to
//   `initialize`, as the core measures it (`<harness> ready in N ms` of app.log): what a harness in a process of its own
//   (INT-9) changes. The agent is shown by a click on its card, which may start its process before its message;
// - message to answer: from the message sent (its Enter, in the page) to the answer shown, as the page saw it (a
//   MutationObserver): its process started by it, or already started by the click (then much shorter);
// - launch to answer: from the launch to that answer, Playwright's attaching to the window included (it polls every 500 ms:
//   a bound, not a measure).
// The first launch, on a new data folder, is measured for its window only. The results go to
// bench-results/int0-startup-<label>.json (Playwright empties test-results/ at each run).

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { test, expect, addProject, send } from './fixture';

test.skip(!process.env.ESCOUADE_BENCH, 'a measure to run by hand: ESCOUADE_BENCH=1 (npm run bench:int0)');

const LABEL = process.env.ESCOUADE_BENCH_LABEL || 'run';
const LAUNCHES = Number(process.env.ESCOUADE_BENCH_LAUNCHES || 10);

/**
 * When the app's page first painted, in ms since the epoch. Playwright may attach before the WebView2 navigated to it (the
 * context of an evaluation is then destroyed): asked again until the app's page answers.
 */
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

/** Sends `text` to the agent shown; when it was sent and when its answer showed, in ms since the epoch. */
async function answered(page: Page, text: string): Promise<{ sent: number; seen: number }> {
  const want = `Bonjour, tu as dit : ${text}`;
  const box = page.getByRole('textbox').last();
  await box.fill(text);
  await page.evaluate((want) => {
    const w = window as unknown as { __int0: { sent?: number; seen?: number } };
    w.__int0 = {};
    const now = () => performance.timeOrigin + performance.now();
    document.addEventListener(
      'keydown',
      (e) => {
        if (e.key === 'Enter' && w.__int0.sent === undefined) w.__int0.sent = now();
      },
      { capture: true, once: true },
    );
    const seen = new MutationObserver(() => {
      if (document.body.textContent?.includes(want)) {
        w.__int0.seen = now();
        seen.disconnect();
      }
    });
    seen.observe(document.body, { childList: true, subtree: true, characterData: true });
  }, want);
  await box.press('Enter');
  await expect(page.getByText(want)).toBeVisible();
  return page.evaluate(() => (window as unknown as { __int0: { sent: number; seen: number } }).__int0);
}

function stat(xs: number[]) {
  const s = xs.filter(Number.isFinite).sort((a, b) => a - b);
  if (!s.length) return { n: 0 };
  const at = (q: number) => s[Math.min(s.length - 1, Math.max(0, Math.ceil(q * s.length) - 1))];
  return { n: s.length, median: at(0.5), max: s[s.length - 1], min: s[0] };
}

test('time to the window and to the first agent ready', async ({ app }) => {
  test.setTimeout(LAUNCHES * 60_000 + 60_000);
  const first = { window: (await firstPaint(app.page)) - app.launchedAt() };
  await addProject(app.page, app.repo);
  await send(app.page, 'Bonjour');
  await expect(app.page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();

  const launches: { window: number; agentReady: number; launchToAnswer: number; processReady: number }[] = [];
  const log = path.join(app.data, 'app.log');
  const logSize = () => (fs.existsSync(log) ? fs.statSync(log).size : 0);
  for (let i = 0; i < LAUNCHES; i++) {
    const from = logSize();
    const page = await app.restart();
    const launched = app.launchedAt();
    const painted = await firstPaint(page);
    await page.locator('.card').first().click();
    const { sent, seen } = await answered(page, `mesure ${i + 1}`);
    // What the core said of its harness's process this launch: started before the message (warm) or by it, and ready in.
    const said = fs.readFileSync(log).subarray(from).toString('utf8');
    const ready = Number(said.match(/ready in (\d+) ms/)?.[1] ?? NaN);
    launches.push({ window: painted - launched, agentReady: seen - sent, launchToAnswer: seen - launched, processReady: ready });
    const told = said
      .split('\n')
      .filter((l) => /agent .*: (starting|.* ready in)/.test(l))
      .map((l) => `  ${l.slice(0, 140)}`);
    console.log(
      `launch ${i + 1}: window ${(painted - launched).toFixed(0)} ms, first agent ready ${(seen - sent).toFixed(0)} ms, ` +
        `launch to answer ${(seen - launched).toFixed(0)} ms, process ready in ${ready} ms\n${told.join('\n')}`,
    );
  }
  const report = {
    label: LABEL,
    exe: process.env.ESCOUADE_E2E_EXE ?? 'debug build',
    firstLaunch: first,
    launches,
    window: stat(launches.map((l) => l.window)),
    processReady: stat(launches.map((l) => l.processReady)),
    agentReady: stat(launches.map((l) => l.agentReady)),
    launchToAnswer: stat(launches.map((l) => l.launchToAnswer)),
  };
  const out = path.join(process.cwd(), 'bench-results', `int0-startup-${LABEL}.json`);
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, JSON.stringify(report, null, 1));
  console.log(
    `first launch, window: ${first.window.toFixed(0)} ms\n${JSON.stringify(report.window)}\n${JSON.stringify(report.processReady)}\n${out}`,
  );
  expect(report.window.n).toBe(LAUNCHES);
  expect(report.agentReady.n).toBe(LAUNCHES);
  expect(report.processReady.n).toBe(LAUNCHES);
});
