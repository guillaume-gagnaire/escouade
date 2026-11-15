// The Workflow tab (ESC-194) compared to the preview (design/app-redesign/index.html, `#loops`, `#loops-history`,
// `#loops-simple`, `#loops-empty`, and `#loops-next` with the steps done and « Ensuite » of ESC-244) in both themes at 1440 × 900 and at CI's 1028 × 779 (a panel narrower than 420 px:
// the pills under their step, the tabs closer, the count of the tab gone); then at the runner's 1024 × 728, nothing of the
// tab sticks out, in both themes and in English. The app is driven to the preview's state by the fake agent's facts
// (`faits-workflow`): a GSD project whose open agent went back from Verify to Plan, two more agents, a project of one
// prompt run over and over, a project with none. The captures go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const PREVIEW = path.join(ROOT, 'design', 'app-redesign', 'index.html');
const PREVIEW_FONT = path.join(ROOT, 'website', 'node_modules', '@fontsource-variable', 'inter', 'opsz.css');
const FIXTURES = path.join(ROOT, 'tests', 'fixtures', 'workflows');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900, name: '' },
  { width: 1028, height: 779, name: '-small' },
];
const TINT = 'oklch(0.72 0.12 48)';

function look(el: Locator) {
  return el.evaluate((node) => {
    const s = getComputedStyle(node);
    const r = node.getBoundingClientRect();
    return {
      width: Math.round(r.width),
      height: Math.round(r.height),
      background: s.backgroundColor,
      color: s.color,
      shadow: s.boxShadow,
      radius: s.borderRadius,
      padding: s.padding,
      margin: s.margin,
      border: `${s.borderTopWidth} ${s.borderTopStyle} ${s.borderTopColor}`,
      font: `${s.fontWeight} ${s.fontSize} ${s.fontFamily.split(',')[0]}`,
      outline: `${s.outlineStyle} ${s.outlineWidth}`,
    };
  });
}
const lookBut = (el: Locator, ...keys: string[]) =>
  look(el).then((l) => Object.fromEntries(Object.entries(l).filter(([k]) => !keys.includes(k))));
/** The look of both, but `but`, the same; one missing is said at once (not after the test's whole time). */
async function same(what: string, a: Locator, p: Locator, but: string[]) {
  const [na, np] = [await a.count(), await p.count()];
  if (!na || !np) return expect.soft([na, np], `${what}: in the app, in the preview`).toEqual([1, 1]);
  expect.soft(await lookBut(a.first(), ...but), what).toLookLike(await lookBut(p.first(), ...but));
}
const place = (el: Locator) =>
  el.evaluate((node) => {
    const r = node.getBoundingClientRect();
    return [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)];
  });
const stroke = (el: Locator) =>
  el.evaluate((node) => {
    const s = getComputedStyle(node);
    return [s.stroke, s.strokeWidth, s.strokeDasharray];
  });

const still = (page: Page) =>
  page.evaluate((p) => {
    if (!document.getElementById('still')) {
      const style = document.createElement('style');
      style.id = 'still';
      style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
      document.head.append(style);
    }
    document.documentElement.style.setProperty('--p', p);
  }, TINT);
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

function repoNamed(root: string, name: string, lay?: (dir: string) => void): string {
  const dir = path.join(root, name);
  fs.mkdirSync(dir, { recursive: true });
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(dir, 'README.md'), `# ${name}\n`);
  lay?.(dir);
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

/** A folder of the fixtures copied in `to`, its `@~/.claude/` made the project's. */
function copy(from: string, to: string) {
  for (const e of fs.readdirSync(from, { withFileTypes: true })) {
    const src = path.join(from, e.name);
    const dst = path.join(to, e.name);
    if (e.isDirectory()) {
      fs.mkdirSync(dst, { recursive: true });
      copy(src, dst);
    } else fs.writeFileSync(dst, fs.readFileSync(src, 'utf8').split('@~/.claude/').join('@.claude/'));
  }
}

/** What was laid out in the repository, committed: the project is added as a clean one. */
function commitAll(repo: string) {
  const git = (...args: string[]) =>
    execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', ...args], { cwd: repo, stdio: 'pipe' });
  git('add', '-A');
  git('commit', '-qm', 'workflow');
}

const door = (cmd: string, facts: object[]) => `${cmd} faits-workflow: ${JSON.stringify(facts)}`;
async function play(page: Page, text: string) {
  const ends = await page.getByText('Fait.', { exact: true }).count();
  await send(page, text);
  await expect(page.getByText('Fait.', { exact: true })).toHaveCount(ends + 1);
}
/** The open agent's name, once its first message named it (`agent-N` until then). */
async function openName(page: Page): Promise<string> {
  const name = page.locator('.card[aria-current="true"] .name');
  await expect(name).not.toHaveText(/^agent-\d+$/);
  return (await name.textContent())!.trim();
}
const tile = (page: Page, name: string) => page.getByRole('tab', { name: new RegExp(`^${name}`) });

async function previewAt(preview: Page, route: string, t: 'dark' | 'light') {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await still(preview);
}

test('the Workflow tab looks as the preview draws it, in both themes, at both sizes', async ({ app }) => {
  test.setTimeout(420_000);
  test.skip(
    !fs.existsSync(chromium.executablePath()) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await page.setViewportSize({ width: 1440, height: 900 });
  copy(path.join(FIXTURES, 'gsd', 'home'), app.repo);
  copy(path.join(FIXTURES, 'gsd', 'project'), app.repo);
  commitAll(app.repo);
  await addProject(page, app.repo);

  // The first agent, as pagination-users: through the phase, back from Verify to Plan, on Verify again.
  await play(page, door('/gsd:discuss-phase 3', [{ write: '.planning/CONTEXT.md', text: '# Contexte' }]));
  const a1 = await openName(page);
  await play(page, door('/gsd:plan-phase 3', [{ agent: 'gsd-planner' }, { agent: 'gsd-plan-checker' }]));
  await play(page, door('/gsd:execute-phase 3', [{ agent: 'gsd-executor' }, { agent: 'gsd-executor' }]));
  await play(page, door('/gsd:verify-work 3', [{ agent: 'gsd-verifier' }, { say: '2 écarts : le curseur n’est pas validé' }]));
  await play(page, door('/gsd:plan-phase 3 --gaps', [{ agent: 'gsd-planner' }]));
  await play(page, door('/gsd:execute-phase 3', [{ agent: 'gsd-executor' }]));
  await play(page, door('/gsd:verify-work 3', [{ agent: 'gsd-verifier' }]));
  // Two more, on Execute and on Plan.
  for (const cmd of ['/gsd:execute-phase 3', '/gsd:plan-phase 3']) {
    await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
    // Its own conversation, empty, before its first turn is counted.
    await expect(page.getByText('Fait.', { exact: true })).toHaveCount(0);
    await play(page, door(cmd, [{ wait: 50 }]));
  }
  await page.locator('.card').filter({ hasText: a1 }).click();
  await page.getByRole('button', { name: 'Conversation et fichiers côte à côte' }).click();
  const side = page.locator('aside.git-side.docked');
  await side.getByRole('tab', { name: /^Workflow/ }).click();
  const tab = side.getByTestId('workflow');
  await expect(tab.locator('.ev.back')).toHaveCount(1);
  await expect(tab.locator('.ev.on')).toHaveCount(1);

  // A loop of one prompt (studio-web) and a project with none (infra).
  const ralph = repoNamed(app.root, 'studio-web', (dir) => copy(path.join(FIXTURES, 'ralph', 'project'), dir));
  await addProject(page, ralph);
  await play(
    page,
    `Relis PROMPT.md et fais la prochaine tâche faits-workflow: ${JSON.stringify([{ write: 'fix_plan.md', text: '- [x] Export PNG' }])}`,
  );
  // The next turn runs, as the preview's: its tests for a long while.
  await send(
    page,
    `Relis PROMPT.md et fais la prochaine tâche faits-workflow: ${JSON.stringify([{ bash: 'npm test' }, { wait: 600_000 }])}`,
  );
  await addProject(page, repoNamed(app.root, 'infra'));

  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await theme(page, t);
        await still(page);
        const where = `${t} ${size.width}×${size.height}`;
        const shot = async (name: string) => {
          if (!SHOTS) return;
          await page.screenshot({ path: path.join(SHOTS, `app-${name}${size.name}-${t}.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `preview-${name}${size.name}-${t}.png`) });
        };

        // ---------- #loops: the rail, the agents, the history ----------
        await tile(page, 'demo-api').click();
        await page.locator('.card').filter({ hasText: a1 }).click();
        await side.getByRole('tab', { name: /^Workflow/ }).click();
        if ((await tab.locator('.wf-graph.folded').count()) > 0) await tab.getByRole('button', { name: 'Déplier les étapes' }).click();
        await page.mouse.move(size.width / 2, 20);
        await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
        await previewAt(preview, 'loops', t);
        const pv = preview.locator('#side-loop');
        const pside = preview.locator('#side');
        await shot('loops');
        expect.soft(await place(side), `panel's place, ${where}`).toLookLike(await place(pside));
        const tabs = side.locator('.utabs');
        const ptabs = pside.locator('.utabs');
        expect.soft(await lookBut(tabs, 'width'), `tabs, ${where}`).toLookLike(await lookBut(ptabs, 'width'));
        const sel = side.getByRole('tab', { name: /^Workflow/ });
        const psel = preview.locator('#tab-loop');
        expect.soft(await lookBut(sel, 'width'), `the Workflow tab, ${where}`).toLookLike(await lookBut(psel, 'width'));
        expect.soft(await sel.locator('.n').isVisible(), `its count shown, ${where}`).toBe(await psel.locator('.n').isVisible());
        expect.soft((await place(sel))[0], `the Workflow tab's place, ${where}`).toBe((await place(psel))[0]);

        // The head: the menu, the caption, the doors.
        const pairs: [string, Locator, Locator, string[]][] = [
          ['the menu', tab.locator('.wf-top .select'), pv.locator('.wf-top .select'), ['width']],
          ['the caption', tab.locator('.wf-cap').first(), pv.locator('.wf-cap').first(), ['width']],
          ['the doors', tab.locator('.wf-gates'), pv.locator('.wf-gates'), ['width', 'height']],
          ['a door', tab.locator('.wf-gates .gate').first(), pv.locator('.wf-gates .gate').first(), ['width']],
          ['« +n »', tab.locator('.wf-gates .gate.more'), pv.locator('.wf-gates .gate.more'), ['width']],
          // The rail.
          ['the rail', tab.locator('.wf-graph'), pv.locator('.wf-graph'), ['width', 'height']],
          ['the open agent’s step', tab.locator('.wf-step.here'), pv.locator('.wf-step.here'), ['width', 'height']],
          ['its node', tab.locator('.wf-step.here .wf-node'), pv.locator('.wf-step.here .wf-node'), []],
          [
            'a busy node',
            tab.locator('.wf-step.busy:not(.here) .wf-node').first(),
            pv.locator('.wf-step.busy:not(.here) .wf-node').first(),
            [],
          ],
          ['an idle node', tab.locator('.wf-step:not(.busy) .wf-node').first(), pv.locator('.wf-step:not(.busy) .wf-node').first(), []],
          ['a step’s name', tab.locator('.wf-name').first(), pv.locator('.wf-name').first(), ['width']],
          ['a step’s door', tab.locator('.wf-line .gate').first(), pv.locator('.wf-line .gate').first(), ['width']],
          ['a detail', tab.locator('.wf-detail').nth(1), pv.locator('.wf-detail').nth(1), ['width', 'height']],
          ['a subagent', tab.locator('.wf-detail .sa').first(), pv.locator('.wf-detail .sa').first(), ['width']],
          ['a way back', tab.locator('.wf-detail .back'), pv.locator('.wf-detail .back'), ['width']],
          ['the open agent’s pill', tab.locator('.who[aria-current="true"]'), pv.locator('.who[aria-current="true"]'), ['width']],
          ['another pill', tab.locator('.who:not([aria-current])').first(), pv.locator('.who:not([aria-current])').first(), ['width']],
          ['the end', tab.locator('.wf-end'), pv.locator('.wf-end'), ['width', 'height']],
          // Where the agent read is in its pass (ESC-244): the steps done ticked, the exit next, the way back « ou retour ».
          [
            'a step done',
            tab.locator('.wf-step.done:not(.busy)').first(),
            pv.locator('.wf-step.done:not(.busy)').first(),
            ['width', 'height'],
          ],
          ['its node ticked', tab.locator('.wf-step.done .wf-node').first(), pv.locator('.wf-step.done .wf-node').first(), []],
          ['its tick', tab.locator('.wf-step.done .wf-node svg').first(), pv.locator('.wf-step.done .wf-node svg').first(), []],
          ['its name', tab.locator('.wf-step.done .wf-name').first(), pv.locator('.wf-step.done .wf-name').first(), ['width']],
          ['the other end of the choice', tab.locator('.wf-step.alt .wf-name'), pv.locator('.wf-step.alt .wf-name'), ['width']],
          ['« ou retour »', tab.locator('.wf-next.alt'), pv.locator('.wf-next.alt'), ['width']],
          ['« Ensuite » on the end', tab.locator('.wf-end.next .wf-next'), pv.locator('.wf-end.next .wf-next'), ['width']],
          ['an agent’s next', tab.locator('.wf-ag .nx').first(), pv.locator('.wf-ag .nx').first(), ['width']],
          // The agents in the workflow.
          ['a heading', tab.locator('.wf-h').first(), pv.locator('.wf-h').first(), ['width', 'height']],
          ['its count', tab.locator('.wf-h .n').first(), pv.locator('.wf-h .n').first(), ['width']],
          ['the agent read', tab.locator('.wf-ag[aria-current="true"]'), pv.locator('.wf-ag[aria-current="true"]'), ['width', 'height']],
          [
            'another agent',
            tab.locator('.wf-ag:not([aria-current])').first(),
            pv.locator('.wf-ag:not([aria-current])').first(),
            ['width', 'height'],
          ],
          ['its numbers', tab.locator('.wf-ag .nums').first(), pv.locator('.wf-ag .nums').first(), ['width']],
          ['its step', tab.locator('.wf-ag .where b').first(), pv.locator('.wf-ag .where b').first(), ['width']],
          ['a barrette done', tab.locator('.pips i.done').first(), pv.locator('.pips i.done').first(), []],
          ['a barrette now', tab.locator('.pips i.on').first(), pv.locator('.pips i.on').first(), []],
          // The history.
          ['the strip', tab.locator('.wf-strip'), pv.locator('.wf-strip'), ['width']],
          ['now on the strip', tab.locator('.wf-strip i.on'), pv.locator('.wf-strip i.on'), ['width']],
          ['a way back on it', tab.locator('.wf-strip i.bad'), pv.locator('.wf-strip i.bad'), ['width']],
          ['the line now', tab.locator('.ev.on'), pv.locator('.ev.on'), ['width', 'height']],
          ['the line that went back', tab.locator('.ev.bad .what b'), pv.locator('.ev.bad .what b'), ['width']],
          ['the way back', tab.locator('.ev.back'), pv.locator('.ev.back'), ['width', 'height']],
          ['an hour', tab.locator('.ev .at').first(), pv.locator('.ev .at').first(), ['width']],
          [
            'a dot',
            tab.locator('.ev:not(.on):not(.bad):not(.back) .dot').first(),
            pv.locator('.ev:not(.on):not(.bad):not(.back) .dot').first(),
            [],
          ],
          ['its numbers', tab.locator('.ev.on .num'), pv.locator('.ev.on .num'), ['width']],
        ];
        for (const [what, a, p, but] of pairs) await same(`${what}, ${where}`, a, p, but);
        expect
          .soft(await stroke(tab.locator('.wf-edges path:not(.outer)')), `the way back drawn, ${where}`)
          .toLookLike(await stroke(pv.locator('.wf-edges path.inner')));
        expect
          .soft(await stroke(tab.locator('.wf-edges path.outer')), `the next round drawn, ${where}`)
          .toLookLike(await stroke(pv.locator('.wf-edges path.outer')));
        // The pills under their step when the panel is narrow, beside it when it is wide: as the preview lays them.
        const under = async (root: Locator) => {
          const p = (await root.locator('.wf-step.here .who').first().boundingBox())!;
          const n = (await root.locator('.wf-step.here .wf-node').boundingBox())!;
          return p.y > n.y + n.height;
        };
        expect.soft(await under(tab), `the pills under their step, ${where}`).toBe(await under(pv));
        const [, gy] = await place(tab.locator('.wf-graph'));
        const [, pgy] = await place(pv.locator('.wf-graph'));
        expect.soft(gy, `the rail's top, ${where}`).toBe(pgy);

        // ---------- #loops-next: the agent on Execute chosen in the list, Verify next (ESC-244) ----------
        await tab
          .locator('.wf-ag')
          .filter({ has: page.locator('.where b', { hasText: 'Execute phase' }) })
          .click();
        await expect(tab.locator('.wf-step.here .wf-name')).toHaveText('Execute phase');
        await page.mouse.move(size.width / 2, 20);
        await previewAt(preview, 'loops-next', t);
        await shot('loops-next');
        for (const [what, a, p, but] of [
          ['the next step', tab.locator('.wf-step.next'), pv.locator('.wf-step.next'), ['width', 'height']],
          ['its name underlined', tab.locator('.wf-step.next .wf-name'), pv.locator('.wf-step.next .wf-name'), ['width']],
          ['its node', tab.locator('.wf-step.next .wf-node'), pv.locator('.wf-step.next .wf-node'), []],
          ['« Ensuite »', tab.locator('.wf-step.next .wf-next'), pv.locator('.wf-step.next .wf-next'), ['width']],
          ['the end, not next', tab.locator('.wf-end'), pv.locator('.wf-end'), ['width', 'height']],
        ] as [string, Locator, Locator, string[]][])
          await same(`${what}, ${where}`, a, p, but);
        expect.soft(await tab.locator('.wf-step.next .wf-name').textContent(), `the step next, ${where}`).toBe('Verify work');
        await tab.locator('.wf-ag').filter({ hasText: a1 }).click();
        await expect(tab.locator('.wf-end.next')).toHaveCount(1);

        // ---------- #loops-history: folded ----------
        await tab.getByRole('button', { name: 'Replier les étapes' }).click();
        await previewAt(preview, 'loops-history', t);
        await shot('loops-history');
        expect
          .soft(await lookBut(tab.locator('.wf-graph.folded .wf-step.here'), 'width'), `a folded step, ${where}`)
          .toLookLike(await lookBut(pv.locator('.wf-graph.folded .wf-step.here'), 'width'));
        expect
          .soft(await lookBut(tab.locator('.wf-graph.folded .wf-step').first(), 'width'), `an idle folded step, ${where}`)
          .toLookLike(await lookBut(pv.locator('.wf-graph.folded .wf-step').first(), 'width'));
        expect.soft(await tab.locator('.wf-gates').count(), `no doors folded, ${where}`).toBe(await pv.locator('.wf-gates').count());
        await tab.getByRole('button', { name: 'Déplier les étapes' }).click();

        // ---------- #loops-simple: a loop of one prompt ----------
        await tile(page, 'studio-web').click();
        await side.getByRole('tab', { name: /^Workflow/ }).click();
        await expect(tab.locator('.wf-ring')).toBeVisible();
        await previewAt(preview, 'loops-simple', t);
        await shot('loops-simple');
        for (const [what, a, p, but] of [
          ['the ring', tab.locator('.wf-ring'), pv.locator('.wf-ring'), ['width', 'height']],
          ['its text', tab.locator('.wf-ring .txt b'), pv.locator('.wf-ring .txt b'), ['width']],
          ['a turn now', tab.locator('.wf-turns i.on'), pv.locator('.wf-turns i.on'), ['width']],
          ['the next turn', tab.locator('.wf-turns i.next'), pv.locator('.wf-turns i.next'), ['width']],
          [
            'a turn to come',
            tab.locator('.wf-turns i.todo:not(.next)').first(),
            pv.locator('.wf-turns i.todo:not(.next)').first(),
            ['width'],
          ],
          ['a turn done', tab.locator('.wf-turns i.ok').first(), pv.locator('.wf-turns i.ok').first(), ['width']],
        ] as [string, Locator, Locator, string[]][])
          await same(`${what}, ${where}`, a, p, but);

        // ---------- #loops-empty: no workflow ----------
        await tile(page, 'infra').click();
        await side.getByRole('tab', { name: /^Workflow/ }).click();
        await expect(tab.locator('.wf-empty')).toBeVisible();
        await previewAt(preview, 'loops-empty', t);
        await shot('loops-empty');
        for (const [what, a, p, but] of [
          ['the empty state', tab.locator('.wf-empty'), pv.locator('.wf-empty'), ['width', 'height']],
          ['its icon', tab.locator('.wf-empty .ic'), pv.locator('.wf-empty .ic'), []],
          ['its title', tab.locator('.wf-empty h3'), pv.locator('.wf-empty h3'), ['width', 'height']],
          ['its text', tab.locator('.wf-empty p').first(), pv.locator('.wf-empty p').first(), ['width', 'height']],
          ['what was looked for', tab.locator('.wf-looked'), pv.locator('.wf-looked'), ['width', 'height']],
          ['a button', tab.locator('.wf-empty .btn').first(), pv.locator('.wf-empty .btn').first(), ['width']],
        ] as [string, Locator, Locator, string[]][])
          await same(`${what}, ${where}`, a, p, but);
        expect
          .soft((await place(tab.locator('.wf-empty h3')))[0], `the title's left, ${where}`)
          .toBe((await place(pv.locator('.wf-empty h3')))[0]);
      }
  } finally {
    await browser.close();
  }

  // At the runner's size, in both themes and in English: nothing of the tab sticks out of the panel.
  await page.setViewportSize({ width: 1024, height: 728 });
  await tile(page, 'demo-api').click();
  await page.locator('.card').filter({ hasText: a1 }).click();
  await side.getByRole('tab', { name: /^Workflow/ }).click();
  const outside = () =>
    side.evaluate((aside) => {
      const box = aside.getBoundingClientRect();
      const wf = aside.querySelector('.wf') as HTMLElement;
      const out = [...aside.querySelectorAll('.wf *')]
        .filter((el) => {
          const r = el.getBoundingClientRect();
          return r.width > 0 && (r.right > box.right + 0.5 || r.left < box.left - 0.5);
        })
        .map((el) => `${el.tagName}.${el.className}`);
      return { out, scrolls: wf.scrollWidth > wf.clientWidth };
    });
  for (const project of ['demo-api', 'studio-web', 'infra']) {
    await tile(page, project).click();
    await side.getByRole('tab', { name: /^Workflow/ }).click();
    await expect(side.getByTestId('workflow').locator('.wf-top, .wf-empty').first()).toBeVisible();
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      expect.soft(await outside(), `nothing out of ${project}'s tab, ${t} 1024×728`).toLookLike({ out: [], scrolls: false });
    }
  }
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the tab fit in their place, at the runner’s size and at 1028 px', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    copy(path.join(FIXTURES, 'gsd', 'home'), app.repo);
    copy(path.join(FIXTURES, 'gsd', 'project'), app.repo);
    commitAll(app.repo);
    await addProject(page, app.repo, { lang: 'en' });
    await play(page, door('/gsd:plan-phase 3', [{ agent: 'gsd-planner' }]));
    await page.getByRole('button', { name: 'New agent', exact: true }).first().click();
    await expect(page.getByText('Fait.', { exact: true })).toHaveCount(0);
    await play(page, door('/gsd:execute-phase 3', [{ agent: 'gsd-executor' }]));
    const a2 = await openName(page);
    await page.keyboard.press('Control+Shift+B');
    const side = page.locator('aside.git-side');
    await side.getByRole('tab', { name: /^Workflow/ }).click();
    const tab = side.getByTestId('workflow');
    for (const size of [
      { width: 1024, height: 728 },
      { width: 1028, height: 779 },
    ]) {
      await page.setViewportSize(size);
      await expect(tab.getByText('Entry points')).toBeVisible();
      await expect(tab.getByText('Agents in GSD')).toBeVisible();
      await expect(tab.getByText(`History of ${a2}`)).toBeVisible();
      await expect(tab.locator('.wf-cap').first()).toContainText('4 steps · phase 3 of 6 from .planning/STATE.md');
      // What is cut short, or overflows its box: none of it, but the names of the workflow's own, cut by design.
      const cut = await side.evaluate((root) =>
        [...root.querySelectorAll<HTMLElement>('.wf *')]
          .filter(
            (el) =>
              el.offsetParent &&
              el.children.length === 0 &&
              !el.closest('.caption, .gate, .who, .where .t, .nm, .wf-h .n') &&
              (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
          )
          .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
      );
      expect(cut, `${size.width}×${size.height}`).toEqual([]);
      for (const b of await side.locator('.utabs [role="tab"]:visible').all())
        expect((await b.boundingBox())!.height).toBeLessThanOrEqual(41);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `workflow-${size.width}-en.app.png`) });
    }
  });
});
