// The page of a launch in a pane of the split screen (ESC-419): its bar drawn with the app's controls (captured in both
// themes, at the window's size and wide), then the page put beside its conversation from its « … » menu, resized by
// the handle, closed by its ×, put back by a drag of its chip and from the agent's menu. Its webview follows its pane:
// where the window places it is its pane's room, at its size (the page sees its width), hidden (placed nowhere) while a
// menu is over it, while a content is dragged and when its pane closes; its address and width are kept. ESCOUADE_E2E_SHOTS=<folder>: the captures.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test, windowScale } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

type Bounds = { x: number; y: number; width: number; height: number } | null;

/** Where the window asks the backend to put the page (`devpage_place`), call by call: read from the IPC it sends. */
async function watchPlace(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as { esc419Placed: unknown[] };
    w.esc419Placed = [];
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.endsWith('/devpage_place')) {
        try {
          w.esc419Placed.push(JSON.parse(String(init?.body ?? '{}')).bounds ?? null);
        } catch {
          // A body of another shape: not one of ours.
        }
      }
      return fetch(input, init);
    };
  });
  return () => page.evaluate(() => (window as unknown as { esc419Placed: Bounds[] }).esc419Placed.at(-1));
}

/** The box of `el`, rounded as the window rounds the page's. */
const box = (el: Locator) =>
  el.evaluate((n) => {
    const r = n.getBoundingClientRect();
    return { x: Math.round(r.left), y: Math.round(r.top), width: Math.round(r.width), height: Math.round(r.height) };
  });

/** The theme of the app, as « Thème » sets it. */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

/** The keys of the panes shown, in order. */
const keys = (page: Page) => page.getByTestId('pane').evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.paneKey));

/** The captures of the page's view: the window, and its bar alone, in both themes. */
async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.mouse.move(2, 2);
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${t}.png`) });
    const bar = page.locator('.devpage .bar').first();
    if (await bar.isVisible()) await bar.screenshot({ path: path.join(SHOTS, `${name}-bar-${t}.png`) });
  }
  await theme(page, 'dark');
}

test('the page of a launch goes into a pane beside its conversation, follows it, is resized and closed, and keeps its state', async ({
  app,
}) => {
  test.skip(process.platform !== 'win32', 'WebView2: what the page sees of its size is read on Windows');
  test.setTimeout(180_000);
  const { page } = app;
  const widths = path.join(app.data, 'largeurs.txt');
  // The developed app: its page says its width at each change.
  fs.writeFileSync(
    path.join(app.repo, 'serveur.js'),
    `const fs = require('node:fs');
const port = Number(process.env.ESCOUADE_PORT_BASE) + 1;
const PAGE = \`<!doctype html><title>demo</title><body style="font:16px system-ui;margin:24px"><h1>Utilisateurs</h1><p>La page de la branche.</p><script>
const say = () => fetch('/etat?w=' + innerWidth);
addEventListener('resize', say);
say();
</script>\`;
require('node:http').createServer((req, res) => {
  if (req.url.startsWith('/etat?')) {
    const q = new URL(req.url, 'http://x').searchParams;
    fs.appendFileSync(${JSON.stringify(widths)}, q.get('w') + '\\n');
    return res.end('ok');
  }
  res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
  res.end(PAGE);
}).listen(port, 'localhost');
`,
  );
  execFileSync('git', ['add', '-A'], { cwd: app.repo });
  execFileSync('git', ['commit', '-qm', 'Serveur de test'], { cwd: app.repo });
  const lastOf = (file: string) => () => (fs.existsSync(file) ? fs.readFileSync(file, 'utf8').trim().split('\n').at(-1) : null);
  const width = lastOf(widths);

  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Page [ok] [recette]');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  await review.getByRole('button', { name: 'Tester' }).click();
  const dialog = page.getByRole('dialog', { name: 'Tester DEM-1' });
  await dialog.getByRole('button', { name: 'Lancer' }).click();
  await expect(dialog).toHaveCount(0, { timeout: 60_000 });
  const pageView = page.getByRole('button', { name: 'La page de son lancement, dans Escouade' });
  await expect(pageView).toHaveAttribute('aria-pressed', 'true');
  const address = page.getByRole('textbox', { name: 'Adresse de la page' });
  await expect(address).toHaveValue(/^http:\/\/localhost:\d+\/fonction$/);
  const url = await address.inputValue();
  const placed = await watchPlace(page);
  const scale = await windowScale(page);

  // Its bar, as the window's size draws it, then wide.
  const size = page.viewportSize()!;
  await shots(page, `devpage-${size.width}`);
  if (size.width < 1400) {
    await page.setViewportSize({ width: 1440, height: 900 });
    await shots(page, 'devpage-1440');
    await page.setViewportSize(size);
  }

  // A tablet's width, kept when it goes into its pane.
  const more = page.getByRole('button', { name: 'Plus d’actions sur la page' });
  await more.click();
  await page.getByRole('menuitemradio', { name: /^Tablette/ }).click();
  // Its menu over it: the page hides, and comes back once it closes.
  await more.click();
  await expect(page.getByRole('menu')).toBeVisible();
  await expect.poll(placed).toBeNull();
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'devpage-menu.png') });
  // Beside its conversation, from its « … ».
  await page.getByRole('menuitem', { name: 'Ouvrir à côté' }).click();
  const agentId = await page.locator('[data-pane-key^="agent:"]').first().getAttribute('data-pane-key');
  const id = agentId!.slice('agent:'.length);
  await expect.poll(() => keys(page)).toEqual([`agent:${id}`, `page:${id}`]);
  const pane = page.locator(`[data-pane-key="page:${id}"]`);
  // Its conversation on the left shows its messages, not the page; the page on the right, at the same address and width.
  await expect(page.locator(`[data-pane-key="agent:${id}"] .devpage`)).toHaveCount(0);
  await expect(pane.getByRole('textbox', { name: 'Adresse de la page' })).toHaveValue(url);
  await expect(pane.getByText(/^(768 px|Rétréci à \d+ px)$/)).toBeVisible();
  await pane.getByRole('button', { name: 'Plus d’actions sur la page' }).click();
  await page.getByRole('menuitemradio', { name: /^Pleine largeur/ }).click();

  // Its webview is its pane's room: where, and how wide the page sees itself.
  const room = pane.locator('.room');
  const at = async () => {
    const r = await box(room);
    const b = await placed();
    return (
      !!b && Math.abs(b.x - r.x) <= 1 && Math.abs(b.y - r.y) <= 1 && Math.abs(b.width - r.width) <= 1 && Math.abs(b.height - r.height) <= 1
    );
  };
  await expect.poll(at, { timeout: 15_000 }).toBe(true);
  const seesRoom = async () => Math.abs(Number(width()) - (await box(room)).width * scale) <= 2;
  await expect.poll(seesRoom, { timeout: 15_000 }).toBe(true);
  const before = (await box(room)).width;
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'devpage-pane.png') });

  // The handle pulled to the right: the page's pane narrows, and the page with it.
  const handle = page.getByRole('separator').first();
  await handle.focus();
  for (let i = 0; i < 5; i++) await handle.press('ArrowRight');
  await expect.poll(async () => (await box(room)).width).toBeLessThan(before - 5);
  await expect.poll(at, { timeout: 15_000 }).toBe(true);
  await expect.poll(seesRoom, { timeout: 15_000 }).toBe(true);
  if (SHOTS) {
    for (const t of ['light', 'dark'] as const) {
      await theme(page, t);
      await page.waitForTimeout(150);
      await page.screenshot({ path: path.join(SHOTS, `devpage-pane-${t}.png`) });
    }
  }

  // Closed by its ×: its conversation alone, the page hidden, its view « Page » there again.
  await pane.hover();
  await pane.getByTestId('pane-close').click();
  await expect(page.getByTestId('pane')).toHaveCount(0);
  await expect.poll(placed).toBeNull();
  await expect(pageView).toHaveAttribute('aria-pressed', 'false');

  // Back from the agent's menu: « Ouvrir la page à côté ».
  await page.getByRole('button', { name: 'Actions de l’agent' }).click();
  await page.getByRole('menuitem', { name: 'Ouvrir la page à côté' }).click();
  await expect.poll(() => keys(page)).toEqual([`agent:${id}`, `page:${id}`]);
  await expect(pane.getByRole('textbox', { name: 'Adresse de la page' })).toHaveValue(url);
  await expect.poll(at, { timeout: 15_000 }).toBe(true);
  // Ctrl+Shift+W closes it, as any pane.
  await expect(pane).toHaveAttribute('data-pane-active', 'true');
  await page.keyboard.press('Control+Shift+W');
  await expect(page.getByTestId('pane')).toHaveCount(0);

  // Its view « Page » dragged onto the bottom of its conversation: one above the other.
  await pageView.click();
  await expect(address).toHaveValue(url);
  const chip = pageView;
  const work = page.locator('.work').first();
  const cb = (await chip.boundingBox())!;
  const wb = (await work.boundingBox())!;
  await page.mouse.move(cb.x + cb.width / 2, cb.y + cb.height / 2);
  await page.mouse.down();
  await page.mouse.move(cb.x + cb.width / 2 + 12, cb.y + cb.height / 2 + 12, { steps: 3 });
  // While it is dragged, the page is hidden (the compass is drawn where it was).
  await page.mouse.move(wb.x + wb.width / 2, wb.y + wb.height * 0.9, { steps: 12 });
  await expect(page.getByTestId('pane-drop')).toBeVisible();
  await expect.poll(placed).toBeNull();
  await page.mouse.up();
  await expect.poll(() => keys(page)).toEqual([`agent:${id}`, `page:${id}`]);
  const pb = (await pane.boundingBox())!;
  const ab = (await page.locator(`[data-pane-key="agent:${id}"]`).boundingBox())!;
  expect(pb.y).toBeGreaterThan(ab.y + ab.height - 1);
  await expect.poll(at, { timeout: 15_000 }).toBe(true);
});
