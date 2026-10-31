import { expect, test, type Page } from '@playwright/test';
import { FAKE_GITHUB } from '../fake/env';

// The submission through the fake GitHub (tests/fake/github.mjs and repos.mjs): an author installs the App on their
// repositories, picks one and a tag, follows the ten checks as they run, and publishes; a booby-trapped plugin is
// rejected with its reasons; a program's binaries are taken with their attestations; a new tag comes by webhook and
// a moved tag is said; the page is measured against the preview (design/store/index.html, #submit). They need a
// database (DATABASE_URL), as the accounts do.
test.skip(!process.env.DATABASE_URL, 'the submission needs DATABASE_URL');

const PREVIEW = new URL('../../../design/store/index.html', import.meta.url).href;
const RUN = Date.now().toString(36).slice(-5);
const who = (name: string) => `${name}-${RUN}`;
// The repositories too: a plugin of an earlier run, of the same name, would hold this one for a person (a name too
// close to another author's, docs/STORE.md §3.3).
const repo = (name: string, kind = '') => `${name}-${RUN}${kind}`;

/** Signs `login` in from /submit, then installs the App on `repos` from the page's button. */
async function author(page: Page, login: string, repos: string) {
  await page.goto('/submit');
  await page.locator('main').getByRole('link', { name: 'Se connecter avec GitHub' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByRole('button', { name: 'Authorize Escouade Store' }).click();
  await expect(page).toHaveURL(/\/submit$/);
  await page.getByRole('link', { name: 'Installer l’App sur un dépôt' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByLabel('Repositories').fill(repos);
  await page.getByRole('button', { name: 'Install & Authorize' }).click();
  await expect(page).toHaveURL(/\/submit\?notice=installed$/);
}

/** Picks `repo` and its newest tag, runs the checks and waits until they end. */
async function submit(page: Page, repo: string) {
  await page.locator('.choices li', { hasText: repo }).getByRole('link', { name: 'Choisir' }).click();
  await expect(page.locator('h1')).toContainText(`Le dossier et la version de ${repo}`);
  await page.getByRole('button', { name: 'Lancer les contrôles' }).click();
  await expect(page).toHaveURL(/\/submit\?version=\d+$/);
  await expect(page.locator('.check.run')).toHaveCount(0, { timeout: 20_000 });
}

const check = (page: Page, name: string) => page.locator('.check', { has: page.locator('b.name', { hasText: new RegExp(`^${name}$`) }) });

test('a visitor is asked to sign in before submitting', async ({ page }) => {
  await page.goto('/submit');
  await expect(page.locator('h1')).toHaveText('Publier un plugin');
  await expect(page.locator('main').getByRole('link', { name: 'Se connecter avec GitHub' })).toHaveAttribute(
    'href',
    '/api/auth/github?to=%2Fsubmit',
  );
  await expect(page.locator('header').getByRole('link', { name: 'Publier un plugin' })).toHaveAttribute('href', '/submit');
  await page.goto('/en/submit');
  await expect(page.locator('h1')).toHaveText('Publish a plugin');
});

test('an author submits a tag: the commit resolved, the ten checks passed, then published unverified', async ({ page }) => {
  const login = who('sam-submit');
  const ntfy = repo('ntfy');
  await author(page, login, `${ntfy}, ${repo('vault', '-private')}`);
  await expect(page.locator('.choices li', { hasText: `${login}/${repo('vault', '-private')}` })).toContainText('Privé : refusé');
  await expect(page.locator('.choices li', { hasText: `${login}/${repo('vault', '-private')}` }).getByRole('link')).toHaveCount(0);

  await page
    .locator('.choices li', { hasText: `${login}/${ntfy}` })
    .getByRole('link', { name: 'Choisir' })
    .click();
  const tags = page.locator('.choices.tags li');
  await expect(tags).toHaveCount(2);
  await expect(tags.nth(0)).toContainText(/v0\.3\.0\s*→ [0-9a-f]{7}/);
  await expect(tags.nth(1)).toContainText('v0.2.4');
  await page.getByRole('button', { name: 'Lancer les contrôles' }).click();
  await expect(page).toHaveURL(/\/submit\?version=\d+$/);
  await expect(page.locator('.check.run')).toHaveCount(0, { timeout: 20_000 });

  await expect(page.locator('.check.pass')).toHaveCount(10);
  await expect(page.locator('.stepper li.on')).toContainText('10 sur 10');
  await expect(page.locator('h1')).toHaveText(`Les contrôles de ${ntfy} v0.3.0`);
  await expect(page.locator('.pick').nth(1)).toContainText(/archive figée : sha256:[0-9a-f]{4}…[0-9a-f]{4}/);
  await expect(page.locator('.pick').nth(1)).toContainText('Étiquette du 25 oct. 2026');
  await expect(check(page, 'Licence')).toContainText('MIT, comme le fichier LICENSE');
  await expect(check(page, 'Captures')).toContainText('1 sur 1, réencodées par le store');

  await page.getByRole('link', { name: 'Continuer' }).click();
  await expect(page.locator('.stepper li.on')).toContainText('Publier');
  await expect(page.getByLabel('Notifications')).toBeChecked();
  await expect(page.getByLabel('Demander la revue « Vérifié par Escouade »')).toBeChecked();
  await page.getByRole('button', { name: 'Publier, non vérifié' }).click();
  await expect(page).toHaveURL(/notice=published$/);
  await expect(page.locator('.out .notice')).toContainText('Publiée, non vérifiée.');
  await expect(page.locator('.out')).toContainText(/Empreinte de la version : sha256:[0-9a-f]{64}/);

  // A new tag by webhook waits for its author; the published tag moved is said, the version unchanged.
  await fetch(`${FAKE_GITHUB}/_fake/tag`, { method: 'POST', body: JSON.stringify({ repo: `${login}/${ntfy}`, tag: 'v0.4.0' }) });
  await expect
    .poll(
      async () => {
        await page.goto('/submit');
        return (await page.locator('.recent li').allTextContents()).find((t) => t.includes('v0.4.0')) ?? '';
      },
      { timeout: 20_000 },
    )
    .toMatch(/à publier/);
  await page.locator('.recent li', { hasText: 'v0.3.0' }).getByRole('link').click();
  const versionPage = page.url();
  await fetch(`${FAKE_GITHUB}/_fake/tag`, {
    method: 'POST',
    body: JSON.stringify({ repo: `${login}/${ntfy}`, tag: 'v0.3.0', extra: { 'evil.js': 'steal()' } }),
  });
  await expect
    .poll(
      async () => {
        await page.goto(versionPage);
        return page.locator('.notice.warn').count();
      },
      { timeout: 20_000 },
    )
    .toBe(1);
  await expect(page.locator('.notice.warn')).toContainText('Étiquette déplacée sur le dépôt');
  await expect(page.locator('.out .notice')).toContainText('Publiée, non vérifiée.');
});

test('a booby-trapped plugin is rejected, each check saying what, where and how to fix it', async ({ page }) => {
  const login = who('q7x-trap');
  await author(page, login, repo('gate', '-trap'));
  await submit(page, `${login}/${repo('gate', '-trap')}`);
  await expect(check(page, 'Licence').locator('.why')).toContainText('Le manifeste dit MIT, le fichier LICENSE est sous Apache-2.0.');
  await expect(check(page, 'Licence').locator('.why .fix')).toContainText('mets les deux d’accord, puis pousse une nouvelle étiquette.');
  await expect(check(page, 'Licence').locator('.why code')).toHaveText('license');
  await expect(check(page, 'Archive').locator('.why')).toContainText('docs/key est un lien symbolique.');
  await expect(page.locator('.check.fail')).toHaveCount(2);
  await expect(page.getByRole('button', { name: 'Continuer' })).toBeDisabled();
  await expect(page.locator('.foot-acts')).toContainText('Un contrôle a échoué');
  await expect(page.getByRole('link', { name: 'Comment publier un plugin' }).last()).toHaveAttribute(
    'href',
    'https://escouade.dev/docs/plugins/publishing',
  );

  // In English, the same checks in its words.
  await page.goto(page.url().replace('/submit', '/en/submit'));
  await expect(check(page, 'License').locator('.why')).toContainText('The manifest says MIT, the LICENSE file is under Apache-2.0.');
});

test('a program’s binaries are taken from the release, with their attestations', async ({ page }) => {
  const login = who('ada-bin');
  await author(page, login, repo('pager', '-bin'));
  await submit(page, `${login}/${repo('pager', '-bin')}`);
  await expect(page.locator('.check.pass')).toHaveCount(10);
  await expect(check(page, 'Binaires')).toContainText(
    'windows-x64, macos-arm64, linux-x64 · sans attestation de provenance pour 1 : publiable non vérifié, ne pourra pas être vérifié',
  );
  await expect(check(page, 'Permissions')).toContainText('Lance un programme');
});

test('a version is its author’s alone', async ({ page, browser }) => {
  const login = who('lea-own');
  await author(page, login, repo('chat'));
  await submit(page, `${login}/${repo('chat')}`);
  const id = /version=(\d+)/.exec(page.url())![1];
  const other = await browser.newPage();
  const res = await other.request.get(`/api/submissions/${id}`);
  expect(res.status()).toBe(401);
  const forged = await other.request.post(`/api/submissions/${id}/publish`, {
    form: { categories: 'tools' },
    headers: { origin: 'https://evil.example' },
    maxRedirects: 0,
  });
  expect(forged.status()).toBe(403);
  await other.close();
});

/** What the eye compares between the submission and the preview. */
async function measure(page: Page) {
  return page.evaluate(() => {
    const css = (sel: string, prop: string) => {
      const el = document.querySelector(sel);
      return el ? getComputedStyle(el).getPropertyValue(prop) : null;
    };
    return {
      layout: css('.submit-layout', 'grid-template-columns'),
      stepOn: css('.stepper li.on', 'background-color'),
      stepDot: css('.stepper li .d', 'width'),
      title: css('.page-title', 'font-size'),
      pick: css('.pick', 'border-radius'),
      checkPad: css('.check', 'padding'),
      pass: css('.check.pass .st', 'color'),
      passBg: css('.check.pass .st', 'background-color'),
      fail: css('.check.fail .st', 'color'),
      whyBorder: css('.why', 'border-top-color'),
      whyBold: css('.why b', 'color'),
    };
  });
}

for (const scheme of ['dark', 'light'] as const)
  test(`the checks look like the preview (${scheme})`, async ({ page }) => {
    const login = who(`mo-look-${scheme}`);
    await author(page, login, repo(`look-${scheme}`, '-trap'));
    await submit(page, `${login}/${repo(`look-${scheme}`, '-trap')}`);
    const store = page.url();
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto(`${PREVIEW}${scheme === 'light' ? '?theme=light' : ''}#submit`);
    await expect(page.locator('.check').first()).toBeVisible();
    const preview = await measure(page);
    await page.goto(store);
    await expect(page.locator('.check.fail').first()).toBeVisible();
    expect(await measure(page)).toEqual(preview);
  });

test('a plugin in a folder of a repository with several: the folders listed, a folder typed judged, its tags alone (ESC-313)', async ({
  page,
}) => {
  const login = who('sam-mono');
  const mono = repo('plugins', '-mono');
  await author(page, login, mono);
  await page
    .locator('.choices li', { hasText: `${login}/${mono}` })
    .getByRole('link', { name: 'Choisir' })
    .click();
  await expect(page.locator('h1')).toContainText(`Le dossier et la version de ${login}/${mono}`);
  // The root holds no plugin: the folders that hold a manifest are offered, the links and the folder without are not.
  const folders = page.locator('.choices.folders li');
  await expect(folders).toHaveText([/plugins\/ntfy/, /plugins\/slack/]);
  await expect(page.locator('main')).toContainText('La racine du dépôt ne porte pas de plugin');
  await expect(page.getByRole('button', { name: 'Lancer les contrôles' })).toHaveCount(0);
  const shots = process.env.SHOTS_DIR;
  if (shots) await page.screenshot({ path: `${shots}/submit-folder-fr.png`, fullPage: true });

  // A folder typed: absolute, climbing, through a link, without a manifest; each refused, and why.
  const typed = async (path: string) => {
    await page.getByLabel('Ou un chemin dans le dépôt').fill(path);
    await page.getByRole('button', { name: 'Lire ce dossier' }).click();
    return page.locator('.notice.bad');
  };
  await expect(await typed('/etc')).toHaveText('Ce chemin est absolu : donne un dossier relatif à la racine du dépôt (plugins/slack).');
  await expect(await typed('plugins/../..')).toHaveText('Ce chemin sort du dépôt par « .. » : le store le refuse.');
  await expect(await typed('linked')).toHaveText(
    'Ce dossier passe par un lien symbolique : le store ne suit pas de lien. Donne le dossier réel.',
  );
  await expect(await typed('docs')).toHaveText('Ce dossier ne porte pas d’escouade-plugin.json à cette étiquette.');
  await expect(page.getByRole('button', { name: 'Lancer les contrôles' })).toHaveCount(0);

  // A folder picked: its tags alone, then the ten checks on that folder.
  await folders.filter({ hasText: 'plugins/slack' }).getByRole('link', { name: 'Choisir' }).click();
  await expect(page).toHaveURL(/path=plugins(%2F|\/)slack/);
  await expect(folders.filter({ hasText: 'plugins/slack' })).toHaveClass(/on/);
  const tags = page.locator('.choices.tags li');
  await expect(tags).toHaveCount(1);
  await expect(tags.nth(0)).toContainText('slack@0.2.0');
  await expect(page.locator('main')).toContainText('Une autre étiquette du dépôt, pour un autre dossier, n’est pas montrée.');
  if (shots) {
    await page.screenshot({ path: `${shots}/submit-folder-picked-fr.png`, fullPage: true });
    await page.goto(`/en/submit?repo=${encodeURIComponent(`${login}/${mono}`)}&path=linked`);
    await expect(page.locator('.notice.bad')).toHaveText(
      'This folder goes through a symbolic link: the store follows no link. Give the real folder.',
    );
    await page.goto(`/en/submit?repo=${encodeURIComponent(`${login}/${mono}`)}&path=plugins%2Fslack`);
    await expect(page.locator('h1')).toContainText('The folder and the version');
    await page.screenshot({ path: `${shots}/submit-folder-en.png`, fullPage: true });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(`/submit?repo=${encodeURIComponent(`${login}/${mono}`)}&path=plugins%2Fslack`);
    await expect(page.locator('.choices.tags li')).toHaveCount(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
    await page.screenshot({ path: `${shots}/submit-folder-narrow-fr.png`, fullPage: true });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(`/submit?repo=${encodeURIComponent(`${login}/${mono}`)}&path=plugins%2Fslack`);
  }
  await page.getByRole('button', { name: 'Lancer les contrôles' }).click();
  await expect(page).toHaveURL(/\/submit\?version=\d+$/);
  await expect(page.locator('.check.run')).toHaveCount(0, { timeout: 20_000 });
  await expect(page.locator('.check.pass')).toHaveCount(10);
  await expect(page.locator('.pick').nth(0)).toContainText('dossier plugins/slack');
  await expect(check(page, 'Licence')).toContainText('MIT, comme le fichier LICENSE du dossier');
  await expect(check(page, 'README')).toContainText('README.md de la racine du dépôt (le dossier n’en a pas)');
  if (shots) await page.screenshot({ path: `${shots}/submit-folder-checks-fr.png`, fullPage: true });
});
