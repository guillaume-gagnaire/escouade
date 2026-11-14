// Audit 2026-11, black box, « Tout mettre à jour »: an update from the store that nobody reviewed has, asked alone, a
// window that says so in words. What does « Tout mettre à jour » do of it, among the others? It never goes in the one
// window for all: it has its own, with its warning. The store is the project's fake one
// (tests/fixtures/store), on a port of the test's: a theme whose 1.0.0 is verified and whose 1.1.0 is not, and a second
// plugin so that there are two updates.
import { addProject, expect, test } from '../../../../../tests/e2e/fixture';
import { startFakeStore } from '../../../../../tests/fixtures/store/fake-store.mjs';
import { invoke, keep, PORTS } from './helpers';

let fake: { url: string; key: string; close: () => Promise<void> } | undefined;

test.beforeAll(async () => {
  fake = await startFakeStore(PORTS.spare);
  process.env.ESCOUADE_STORE_URL = fake!.url;
  process.env.ESCOUADE_STORE_MIRROR_URL = `${fake!.url}/mirror`;
  process.env.ESCOUADE_STORE_TEST_KEY = fake!.key;
});

test.afterAll(async () => {
  await fake?.close();
  delete process.env.ESCOUADE_STORE_URL;
  delete process.env.ESCOUADE_STORE_MIRROR_URL;
  delete process.env.ESCOUADE_STORE_TEST_KEY;
});

const THEME = 'dev.exemple.high-contrast';
const SLACK = 'dev.exemple.slack';
const WARNING = 'Personne n’a relu cette version';

test('« Tout mettre à jour » and an update nobody reviewed: its own window, with its warning', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  await addProject(page, app.repo);
  // Installed as the window installs them, each at its first version: the theme's is verified.
  const first: Record<string, boolean> = {};
  for (const id of [THEME, SLACK]) {
    const read = await invoke<{ token: string; verified: boolean }>(page, 'plugin_inspect_store', { id, version: '1.0.0' });
    first[id] = read.verified;
    await invoke(page, 'plugin_install', { token: read.token });
  }
  expect(first[THEME], 'the theme’s 1.0.0 is verified').toBe(true);
  // The second plugin's next version, which asks for one more host, is published now.
  const released = await fetch(`${fake!.url}/_fake/release`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ id: SLACK, version: '1.1.0' }),
  });
  expect(released.status).toBe(200);

  // The updates' page, the store read again (it is read at most every 30 s).
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page.waitForTimeout(31_000);
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Mises à jour/ })
    .click();
  const work = page.locator('.plugins-work');
  const look = work.getByRole('button', { name: 'Chercher maintenant' });
  await look.click();
  await expect(look).toBeEnabled();
  const line = work.locator('.ups .up', { hasText: 'Contraste élevé' });
  await expect(line).toContainText('v1.0.0 → v1.1.0');

  const says = (text: string) => ({
    'nobody reviewed this version (« Personne n’a relu cette version »)': text.includes(WARNING),
    'a tag « Non vérifié »': text.includes('Non vérifié'),
    'the seal « Vérifié par Escouade »': text.includes('Vérifié par Escouade'),
  });
  const read = async (dialog: ReturnType<typeof page.getByRole>) => ({
    text: (await dialog.innerText()).replace(/\s+/g, ' ').trim(),
    buttons: (await dialog.getByRole('button').allInnerTexts()).map((b) => b.trim()).filter(Boolean),
  });
  const version = async () => (await invoke<{ id: string; version: string }[]>(page, 'plugins_list')).find((p) => p.id === THEME)?.version;

  // Asked alone: its own window. Read, then closed without installing.
  await line.getByRole('button', { name: 'Mettre à jour' }).click();
  const own = page.getByRole('dialog', { name: /Contraste élevé/ });
  await expect(own).toBeVisible();
  const alone = await read(own);
  await own.getByRole('button', { name: 'Annuler' }).click();
  await expect(own).toBeHidden();
  // Alone, the window says in words that nobody reviewed the version.
  expect(alone.text).toContain(WARNING);

  // Asked with the others.
  await work.getByRole('button', { name: /^Tout mettre à jour/ }).click();
  // No window for all holds it. Each window that comes is read, then closed without
  // installing: the one of the version nobody reviewed says so in words.
  const windows: { title: string; says: ReturnType<typeof says>; buttons: string[]; text: string }[] = [];
  for (let n = 0; n < 3; n++) {
    const dialog = page.getByRole('dialog');
    const shown = await dialog
      .first()
      .waitFor({ state: 'visible', timeout: 15_000 })
      .then(
        () => true,
        () => false,
      );
    if (!shown) break;
    const title = (await dialog.first().getByRole('heading').first().innerText()).trim();
    const seen = await read(dialog.first());
    windows.push({ title, says: says(seen.text), buttons: seen.buttons, text: seen.text.slice(0, 600) });
    await dialog.first().getByRole('button', { name: 'Annuler' }).click();
    await expect(page.getByRole('dialog', { name: title })).toBeHidden();
  }
  const installed = await version();
  keep('plugins-update-all', {
    theUpdate: 'a theme whose 1.0.0 is verified and whose 1.1.0 is not; it asks for nothing more',
    askedAlone: { says: says(alone.text), buttons: alone.buttons, text: alone.text.slice(0, 600) },
    windowsOfUpdateAll: windows,
    installedAfterClosingThem: installed,
  });
  // No window for all: nothing here is both verified and quiet.
  expect(windows.filter((w) => /Mettre à jour \d+ plugin/.test(w.title))).toEqual([]);
  // The version nobody reviewed came in its own window, with the words of the window asked alone.
  const ofTheTheme = windows.find((w) => w.title.includes('Contraste élevé'));
  expect(ofTheTheme, 'the update nobody reviewed had no window of its own').toBeTruthy();
  expect(ofTheTheme!.text).toContain(WARNING);
  // And nothing was installed without its own gesture.
  expect(installed).toBe('1.0.0');
});
