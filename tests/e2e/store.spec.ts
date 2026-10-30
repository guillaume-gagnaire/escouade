// The store in the app (STORE-9) on the real app, against the fake store of tests/fixtures/store/fake-store.mjs (its key
// made for the run, trusted by the debug build through ESCOUADE_STORE_TEST_KEY): the catalogue read and searched, a
// verified version installed with its seal checked here, an unverified one after its band, a changed archive refused,
// « N'installer que des versions vérifiées », a version withdrawn by the revocation list stopped and said in red, nothing
// counted without the user's consent to the statistics; and the store page compared to the preview (#b1 of
// design/plugins/maquettes-b.html) in both themes, nothing overflowing at 1440 × 900 and 1028 × 779.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const PREVIEW = path.join(ROOT, 'design', 'plugins', 'maquettes-b.html');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

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
const post = (p: string, body: unknown) =>
  fetch(`${fake.url}${p}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });

async function openStore(page: Page) {
  // A second click of the rail's icon goes back to the project: only when the view is closed.
  if (!(await page.locator('.plugins-side').isVisible())) await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Store/ })
    .click();
  await expect(page.locator('.plugins-work .sub')).toContainText('catalogue lu à');
}

/** The install window: installs, and waits for the plugin's page. */
async function install(page: Page, button: string) {
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: button, exact: true }).click();
  await expect(dialog).toBeHidden();
}

test('the store in the app: searched here, a seal checked here, an unverified version, a version withdrawn', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await addProject(page, app.repo);
  await openStore(page);
  const work = page.locator('.plugins-work');
  await expect(work.locator('.sec').first()).toContainText("Règles de l'équipe");

  // The search runs on this machine: nothing asked of the store while typing.
  const before = (await state()).requests.length;
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill('regles equipe');
  await expect(work.locator('.rowp')).toHaveCount(1);
  // What the store saw meanwhile is its catalogue and its list (the reading of the start), never the words searched.
  const seen: string[] = (await state()).requests.slice(before);
  expect(seen.filter((r) => !/^GET \/(api\/v1\/(catalog|revocations)\.json|_fake\/state)/.test(r))).toEqual([]);
  expect(seen.join(' ')).not.toContain('regles');
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill('');

  // A verified version: its seal checked here, then on its line and its page.
  const rules = work.locator('.rowp', { hasText: "Règles de l'équipe" }).first();
  await rules.getByRole('button', { name: 'Installer' }).click();
  const dialog = page.getByRole('dialog', { name: "Installer Règles de l'équipe ?" });
  await expect(dialog).toContainText('Vérifié par Escouade');
  await expect(dialog).toContainText('vérifiée ici, par l’app');
  await expect(dialog).toContainText('archive reçue identique');
  await install(page, 'Installer');
  await expect(page.locator('.phead')).toContainText('Vérifié par Escouade');
  await expect(page.locator('.pp-aside')).toContainText('Relu par Escouade le');

  // An unverified version: the band, « Installer quand même ».
  await openStore(page);
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill('slack');
  await work.locator('.rowp').first().getByRole('button', { name: 'Installer' }).click();
  await expect(page.getByRole('dialog')).toContainText('Personne n’a relu cette version.');
  await install(page, 'Installer quand même');
  await expect(page.locator('.phead')).toContainText('Non vérifié');

  // A changed archive: refused, said, nothing installed.
  await openStore(page);
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill('piégé');
  await work.locator('.rowp').first().getByRole('button', { name: 'Installer' }).click();
  await expect(work.locator('.err')).toContainText("L'archive reçue ne correspond pas");
  await expect(page.getByRole('dialog')).toHaveCount(0);

  // « N'installer que des versions vérifiées »: an unverified version is refused.
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
  await page.getByRole('switch', { name: 'N’installer que des versions vérifiées' }).click();
  await openStore(page);
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill('plus tard');
  await work.locator('.rowp').first().getByRole('button', { name: 'Installer' }).click();
  await expect(work.locator('.err')).toContainText("n'est pas vérifiée");

  // Nothing counted: the user did not accept the statistics.
  expect((await state()).installs).toEqual([]);

  // The revocation list withdraws the rules: stopped, said in red at the head of the installed ones.
  await post('/_fake/revocations', {
    entries: [
      {
        id: 'dev.exemple.team-rules',
        versions: '*',
        action: 'disable',
        reason: { fr: 'Envoyait les commandes ailleurs.', en: 'Sent the commands elsewhere.' },
        date: '2026-10-30',
      },
    ],
  });
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
  await page.getByRole('button', { name: 'Relire maintenant' }).click();
  const band = page.locator('.band');
  await expect(band).toContainText("Règles de l'équipe v1.0.0 a été retiré par Escouade le 2026-10-30.");
  await expect(band).toContainText('Envoyait les commandes ailleurs.');
  await expect(page.getByRole('table', { name: 'Ajoutés' })).toContainText('Désactivé par Escouade');
  await expect(work).toContainText(/Liste de révocation lue à \d\d:\d\d · nº 1/);
});

/** Where an element is and how big, rounded: what the app and the preview must share. */
const box = (el: Locator) =>
  el.evaluate((node) => {
    const r = node.getBoundingClientRect();
    const s = getComputedStyle(node);
    return { w: Math.round(r.width), h: Math.round(r.height), font: `${s.fontWeight} ${s.fontSize}`, padding: s.padding };
  });

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

test('the store looks as the preview (#b1), in both themes and at both sizes', async ({ app }) => {
  test.setTimeout(120_000);
  test.skip(!fs.existsSync(chromium.executablePath()), 'the preview needs Playwright’s Chromium');
  const { page } = app;
  await addProject(page, app.repo);
  await page.setViewportSize({ width: 1440, height: 900 });
  await openStore(page);
  const browser = await chromium.launch();
  const preview = await browser.newPage({ viewport: { width: 1500, height: 900 * 4 + 200 } });
  await preview.goto(pathToFileURL(PREVIEW).href);
  await preview.evaluate(() => document.fonts.ready);
  const b1 = preview.locator('#b1');
  const work = page.locator('.plugins-work');

  expect((await box(work.locator('h2'))).font).toBe((await box(b1.locator('.head h2'))).font);
  const appRow = await box(work.locator('.rowp').first());
  const preRow = await box(b1.locator('.rowp').first());
  expect(appRow.padding).toBe(preRow.padding);
  expect(Math.abs(appRow.h - preRow.h)).toBeLessThanOrEqual(6);
  expect((await box(work.locator('.rowp .tile').first())).w).toBe((await box(b1.locator('.rowp .ptile').first())).w);
  expect((await box(work.locator('.cats button').first())).padding).toBe((await box(b1.locator('.cats span').first())).padding);
  expect((await box(work.locator('.sec h3').first())).font).toBe((await box(b1.locator('.sec h3').first())).font);
  const seal = await box(work.locator('.seal').first());
  const preSeal = await box(b1.locator('.seal').first());
  expect(seal.padding).toBe(preSeal.padding);
  expect(seal.font).toBe(preSeal.font);

  for (const size of [
    { width: 1440, height: 900 },
    { width: 1028, height: 779 },
  ]) {
    await page.setViewportSize(size);
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      const over = await page.evaluate(() =>
        [...document.querySelectorAll('.plugins-work *, .plugins-side *')]
          .filter((n) => {
            const r = n.getBoundingClientRect();
            return r.width > 0 && r.right > window.innerWidth + 1;
          })
          .map((n) => `${n.tagName.toLowerCase()}.${[...n.classList].join('.')}`),
      );
      expect(over, `store at ${size.width}, ${t}`).toEqual([]);
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `app-store-${size.width}-${t}.png`) });
      }
    }
  }
  if (SHOTS) await b1.screenshot({ path: path.join(SHOTS, 'preview-store.png') });
  await browser.close();
});
