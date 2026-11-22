// The link escouade://plugin/<id> (STORE-11) on the real app, launched with it as the system launches it (`"escouade.exe"
// "%1"`), against the fake store of tests/fixtures/store/fake-store.mjs: the plugin's page opened in the view Plugins, nothing
// installed nor downloaded; a link the app cannot read said; an id the store does not have said on the page. (Handing a
// link over to the instance open goes through `single-instance`, which a sandboxed run leaves out: its arguments are read
// by the same `deeplink::links_in`, tested in Rust.)

import { expect, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';

let fake: { url: string; key: string; close: () => Promise<void> };

test.beforeAll(async () => {
  fake = await startFakeStore(0);
  process.env.ESCOUADE_STORE_URL = fake.url;
  process.env.ESCOUADE_STORE_MIRROR_URL = `${fake.url}/mirror`;
  process.env.ESCOUADE_STORE_TEST_KEY = fake.key;
});

test.afterAll(async () => {
  await fake?.close();
  delete process.env.ESCOUADE_STORE_URL;
  delete process.env.ESCOUADE_STORE_MIRROR_URL;
  delete process.env.ESCOUADE_STORE_TEST_KEY;
});

const state = async () => (await fetch(`${fake.url}/_fake/state`)).json();

test.describe('a link to a plugin of the store, and a bad one', () => {
  test.use({ appArgs: ['escouade://plugin/nope', 'escouade://plugin/dev.exemple.team-rules'] });

  test('opens the page of the plugin, installs nothing, and says the link it cannot read', async ({ app }) => {
    const { page } = app;
    await expect(page.locator('.plugins-work')).toBeVisible();
    await expect(page.locator('.phead')).toContainText("Règles de l'équipe");
    await expect(page.getByRole('button', { name: /^Installer la v/ })).toBeVisible();
    await expect(page.locator('.toast', { hasText: 'Lien Escouade illisible, rien n’a été fait : escouade://plugin/nope' })).toBeVisible();
    // Nothing installed, no install window, no archive asked of the store.
    await expect(page.getByRole('dialog')).toHaveCount(0);
    const asked: string[] = (await state()).requests;
    expect(asked.filter((r) => !/^GET \/(api\/v1\/(catalog|revocations)\.json|_fake\/state)/.test(r))).toEqual([]);
    expect((await state()).installs).toEqual([]);
    const installed = (await page.evaluate(() =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string): Promise<unknown> } }).__TAURI_INTERNALS__.invoke('plugins_list'),
    )) as { id: string; origin: { kind: string } }[];
    // The workspace's of a build of development are there from the start: nothing else is installed.
    expect(installed.filter((p) => p.origin.kind !== 'workspace')).toEqual([]);
  });
});

test.describe('a link to a plugin the store does not have', () => {
  test.use({ appArgs: ['escouade://plugin/dev.exemple.nowhere'] });

  test('says it on the page, nothing else done', async ({ app }) => {
    const { page } = app;
    await expect(page.locator('.plugins-work')).toContainText('« dev.exemple.nowhere » n’est pas au catalogue du store.');
    await expect(page.getByRole('dialog')).toHaveCount(0);
  });
});

test.describe('an invitation to a team, opened from elsewhere (SR2-08)', () => {
  const LINK =
    'escouade://team/join/team.acme.fr/00112233445566778899aabbccddeeff#s=AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI&t=AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM';
  test.use({ appArgs: [LINK] });

  test('opens « Rejoindre une équipe » with its server and six words, and joins nothing without the user', async ({ app }) => {
    const { page } = app;
    const dialog = page.getByRole('dialog', { name: 'Rejoindre une équipe' });
    await expect(dialog.getByLabel('Lien d’invitation')).toHaveValue(LINK);
    await expect(dialog.getByText('team.acme.fr', { exact: true })).toBeVisible();
    await expect(dialog.getByRole('group', { name: 'Empreinte de l’équipe' }).getByRole('listitem')).toHaveCount(6);
    await expect(dialog.getByRole('button', { name: 'Rejoindre' })).toBeEnabled();
    // Nothing joined, nothing asked of the server: joining stays the user's gesture.
    const teams = await page.evaluate(() =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string): Promise<unknown> } }).__TAURI_INTERNALS__.invoke('team_list'),
    );
    expect(teams).toEqual([]);
    await dialog.getByRole('button', { name: 'Retour' }).click();
    await expect(page.getByText('Cet appareil n’est dans aucune équipe.')).toBeVisible();
  });
});
