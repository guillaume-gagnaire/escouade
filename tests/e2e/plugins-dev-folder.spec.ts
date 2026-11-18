import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

// Issue #43 (ESC-500): « En développement › Charger un dossier… » of a plugin in Rust. Its module is where
// `escouade-plugin build` writes it, `packs/<id>-<version>.wasm`, never asked for at the root of the folder; a folder
// not built yet says what to run instead of a path that is not there. Here the module is the committed one of the
// fixture tests/fixtures/plugins/reactor, put where `build` would (the real build is played by
// crates/escouade-plugins/src/plugins_rust_build_tests.rs). At both sizes: ESCOUADE_E2E_VIEWPORT=1024x728 for CI's.

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const FIXTURE = path.join(ROOT, 'tests', 'fixtures', 'plugins', 'reactor');
const BUILT = path.join('packs', 'dev.test.reactor-1.0.0.wasm');

/** The plugin in Rust of the fixture, in a folder of the test's own as its author has it; built: its module in packs/. */
function rustPlugin(dir: string, built: boolean) {
  fs.mkdirSync(dir, { recursive: true });
  for (const f of ['escouade-plugin.json', 'Cargo.toml', 'Cargo.lock']) fs.copyFileSync(path.join(FIXTURE, f), path.join(dir, f));
  if (built) {
    fs.mkdirSync(path.join(dir, 'packs'), { recursive: true });
    fs.copyFileSync(path.join(FIXTURE, 'plugin.wasm'), path.join(dir, BUILT));
  }
}

/** The folder dialog answered with `dir`, in the page, on the `fetch` of Tauri's IPC: no native window opens. */
async function pickAnswers(page: Page, dir: string) {
  await page.evaluate((answer) => {
    const w = window as unknown as { esc500Pick?: string; fetch: typeof fetch };
    const first = w.esc500Pick === undefined;
    w.esc500Pick = answer;
    if (!first) return;
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.includes('plugin:dialog|open'))
        return Promise.resolve(
          new Response(JSON.stringify(w.esc500Pick), { headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } }),
        );
      return fetch(input, init);
    };
  }, dir);
}

/** Nothing of the view runs past its width. */
async function fits(page: Page) {
  const over = await page.evaluate(() => {
    const view = document.querySelector('.plugins-work');
    return view ? view.scrollWidth - view.clientWidth : 0;
  });
  expect(over).toBeLessThanOrEqual(1);
}

test('a folder in Rust loads with the module escouade-plugin build wrote, and one not built says what to run', async ({ app }) => {
  test.setTimeout(120_000);
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  const side = page.getByRole('complementary', { name: 'Plugins' });
  await side.getByRole('button', { name: /En développement/ }).click();
  await expect(page.getByText('Aucun dossier en développement.')).toBeVisible();

  // Not built: refused, with the command to run and where it writes; nothing loaded.
  const bare = path.join(app.root, 'mon-plugin-rust');
  rustPlugin(bare, false);
  await pickAnswers(page, bare);
  await page.getByRole('button', { name: 'Charger un dossier…' }).click();
  const alert = page.locator('.plugins-work').getByRole('alert');
  await expect(alert).toContainText("Ce plugin n'a pas encore de module : construis-le d'abord");
  await expect(alert).toContainText(`escouade-plugin build ${bare}`);
  await expect(alert).toContainText('packs/dev.test.reactor-1.0.0.wasm');
  await expect(alert).not.toContainText('No such file');
  await expect(page.getByRole('dialog')).toBeHidden();
  await fits(page);
  await expect(page.getByText('Aucun dossier en développement.')).toBeVisible();

  // Built: the window shows it, it loads in place, and runs from its module of packs/.
  const built = path.join(app.root, 'mon-plugin-construit');
  rustPlugin(built, true);
  await pickAnswers(page, built);
  await page.getByRole('button', { name: 'Charger un dossier…' }).click();
  const dialog = page.getByRole('dialog', { name: 'Charger Reactor en développement ?' });
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Charger', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByRole('heading', { name: /Reactor/ })).toBeVisible();
  await page.getByRole('tab', { name: 'Journal' }).click();
  const journal = page.getByRole('tabpanel', { name: 'Journal' });
  await journal.getByRole('button', { name: 'Démarrer' }).click();
  await expect(journal).toContainText('En marche', { timeout: 60_000 });
  await fits(page);
  expect(fs.existsSync(path.join(built, 'plugin.wasm'))).toBe(false);
});
