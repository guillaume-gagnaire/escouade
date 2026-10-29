import { expect, test, type Page } from '@playwright/test';
import { FAKE_GITHUB } from '../fake/env';

// The author's space through the fake GitHub (tests/fake/github.mjs and repos.mjs), on /account (docs/STORE.md §2.2):
// a version found on a new tag waits in the banner with what it asks more, and is published from there; a refused
// first version says why below its row; a published version is taken back, the plugin taken off the list and put
// back, deleted and restored, and the app's catalogue follows each step; a plugin is offered to another account, who
// accepts it and finds its publications frozen; the account cannot be deleted while it has plugins; each action is
// told by e-mail. The page is measured against the preview (design/store/index.html, #account). They need a database
// (DATABASE_URL), as the accounts do.
test.skip(!process.env.DATABASE_URL, 'the author’s space needs DATABASE_URL');

const PREVIEW = new URL('../../../design/store/index.html', import.meta.url).href;
// Random, not the clock: the plugins of an earlier run on a base kept between runs would otherwise be a letter or two
// away from this run's, and held for a person as names too close (docs/STORE.md §3.3).
const RUN = Math.random().toString(36).slice(2, 10).padEnd(8, '0');
const who = (name: string) => `${name}-${RUN}`;

const mails = async () => ((await (await fetch(`${FAKE_GITHUB}/_fake/state`)).json()) as { mails: { to: string[]; raw: string }[] }).mails;
const mailText = (raw: string) =>
  Buffer.from(
    raw.replace(/=\r?\n/g, '').replace(/=([0-9A-F]{2})/g, (_, h) => String.fromCharCode(parseInt(h, 16))),
    'latin1',
  ).toString('utf8');

/** Signs `login` in from /account, then installs the App on `repos` from the page's button. */
async function author(page: Page, login: string, repos: string) {
  await page.goto('/account');
  await page.locator('main').getByRole('link', { name: 'Se connecter avec GitHub' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByRole('button', { name: 'Authorize Escouade Store' }).click();
  await expect(page).toHaveURL(/\/account$/);
  if (!repos) return;
  await page.getByRole('link', { name: 'Installer l’App sur un dépôt' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByLabel('Repositories').fill(repos);
  await page.getByRole('button', { name: 'Install & Authorize' }).click();
  await expect(page).toHaveURL(/notice=installed$/);
}

/** Submits the newest tag of `repo` from /submit and waits for its checks; publishes it if `publish`. */
async function submit(page: Page, repo: string, publish = true) {
  await page.goto('/submit');
  await page.locator(`.choices a[href="/submit?repo=${encodeURIComponent(repo)}"]`).click();
  await page.getByRole('button', { name: 'Lancer les contrôles' }).click();
  await expect(page).toHaveURL(/\/submit\?version=\d+$/);
  await expect(page.locator('.check.run')).toHaveCount(0, { timeout: 20_000 });
  if (!publish) return;
  await page.getByRole('link', { name: 'Continuer' }).click();
  await page.getByRole('button', { name: 'Publier, non vérifié' }).click();
  await expect(page).toHaveURL(/notice=published$/);
}

/** Keeps `email` as the account's address, through the link of its e-mail. */
async function keepEmail(page: Page, email: string) {
  await page.goto('/account');
  await page.locator('#email').fill(email);
  await page.getByRole('button', { name: 'Envoyer le lien' }).click();
  await expect(page).toHaveURL(/notice=email-sent$/);
  const mail = (await mails()).findLast((m) => m.to.includes(email))!;
  const link = /https?:\/\/\S+\/api\/account\/email\/verify\?token=[\w-]+(?:&lang=\w+)?/.exec(mailText(mail.raw))![0];
  await page.goto(link.replace(/^https?:\/\/[^/]+/, ''));
  await expect(page).toHaveURL(/notice=email-verified$/);
}

const catalog = async (page: Page) =>
  (await (await page.request.get('/api/v1/catalog.json')).json()) as {
    plugins: {
      id: string;
      latest: string | null;
      author: string;
      transferred_at?: string;
      versions: { version: string; yanked: boolean }[];
    }[];
  };

/** Waits until the catalogue (rebuilt within a minute, at once after a change by the store) says what `test` wants. */
const inCatalog = (page: Page, id: string) => async () => (await catalog(page)).plugins.find((p) => p.id === id) ?? null;

test('an author publishes a new tag from the banner, takes a version back, takes the plugin off the list and deletes it', async ({
  page,
}) => {
  const login = who('lea-space');
  const ntfy = `ntfy-${RUN}`;
  const trap = `chat-${RUN}-trap`;
  const id = `io.github.${login}.${ntfy}`.toLowerCase();
  await author(page, login, `${ntfy}, ${trap}`);
  const email = `${login}@example.dev`;
  await keepEmail(page, email);
  await submit(page, `${login}/${ntfy}`);
  await submit(page, `${login}/${trap}`, false);

  // The table: the plugin published, the refused first version with its reason below its row.
  await page.goto('/account');
  const row = page.locator('table.mine tbody tr', { hasText: id });
  await expect(row.locator('.tag')).toHaveText('En revue');
  await expect(row).toContainText('v0.3.0');
  const refused = page.locator('table.mine tbody tr', { hasText: trap });
  await expect(refused.locator('.tag')).toHaveText('Rejetée');
  await expect(page.locator('.why-row .why').first()).toContainText('est refusée par le contrôle');
  await expect(page.locator('.why-row .why a')).toHaveAttribute('href', 'https://escouade.dev/docs/plugins/publishing');

  // A new tag that asks one more permission waits in the banner, then is published from it.
  await fetch(`${FAKE_GITHUB}/_fake/tag`, {
    method: 'POST',
    body: JSON.stringify({ repo: `${login}/${ntfy}`, tag: 'v0.4.0', manifest: { permissions: { network: ['ntfy.sh'], decide: true } } }),
  });
  await expect
    .poll(
      async () => {
        await page.goto('/account');
        return page.locator('.banner').count();
      },
      { timeout: 20_000 },
    )
    .toBe(1);
  const banner = page.locator('.banner');
  await expect(banner).toContainText(`${ntfy} v0.4.0 trouvée sur GitHub, contrôles passés`);
  await expect(banner).toContainText(/Étiquette poussée il y a \d+ min · [0-9a-f]{7} → [0-9a-f]{7}/);
  await expect(banner).toContainText('elle demande une permission de plus : decide');
  await expect
    .poll(async () => (await mails()).filter((m) => m.to.includes(email) && mailText(m.raw).includes('la v0.4.0 attend ton clic')).length)
    .toBe(1);
  await banner.getByRole('button', { name: 'Publier la v0.4.0' }).click();
  await expect(page).toHaveURL(/notice=version-publish$/);
  await expect(page.locator('.banner')).toHaveCount(0);
  await expect.poll(async () => (await inCatalog(page, id)())?.latest).toBe('0.4.0');

  // The plugin's management: its versions, and taking one back.
  await row.getByRole('link', { name: `Gérer ${ntfy}` }).click();
  await expect(page).toHaveURL(new RegExp(`plugin=${id.replace(/\./g, '\\.')}#manage$`));
  const versions = page.locator('table.versions tbody tr');
  await expect(versions).toHaveCount(2);
  await expect(page.locator('.stats h3')).toHaveText(`${ntfy} · installations`);
  await expect(page.locator('.stats .bars rect')).toHaveCount(12);
  await versions.filter({ hasText: 'v0.4.0' }).getByText('Retirer', { exact: true }).click();
  await versions.filter({ hasText: 'v0.4.0' }).getByRole('button', { name: 'Retirer la v0.4.0' }).click();
  await expect(page).toHaveURL(/notice=version-yank$/);
  await expect(versions.filter({ hasText: 'v0.4.0' }).locator('.tag')).toHaveText('Retirée');
  await expect.poll(async () => (await inCatalog(page, id)())?.latest).toBe('0.3.0');
  expect((await inCatalog(page, id)())!.versions.find((v) => v.version === '0.4.0')!.yanked).toBe(true);
  await expect
    .poll(
      async () =>
        (await mails()).filter(
          (m) => m.to.includes(email) && mailText(m.raw).includes('La v0.4.0 de ') && mailText(m.raw).includes('plus propos'),
        ).length,
    )
    .toBe(1);

  // Off the list, out of the catalogue; back on it.
  await page.getByRole('button', { name: 'Retirer de la liste' }).click();
  await expect(page).toHaveURL(/notice=plugin-unlist$/);
  await expect(row.locator('.tag')).toHaveText('Retiré de la liste');
  await expect.poll(inCatalog(page, id)).toBeNull();
  // Its page stays readable, and says so; the search no longer finds it.
  await page.goto(`/plugins/${id}`);
  await expect(page.locator('.band')).toContainText('Retiré de la liste par son auteur');
  await page.goto(`/search?q=${ntfy}`);
  await expect(page.locator('.results .row')).toHaveCount(0);
  await page.goto(`/account?plugin=${id}`);
  await page.getByRole('button', { name: 'Remettre dans la liste' }).click();
  await expect(page).toHaveURL(/notice=plugin-relist$/);
  await expect.poll(async () => (await inCatalog(page, id)())?.latest).toBe('0.3.0');

  // The account cannot go while it has plugins.
  await expect(page.locator('.side-card p.warn')).toContainText(`Transfère ou supprime d’abord tes plugins (${id})`);
  const refusedDelete = await page.request.post('/api/account/delete', {
    form: { confirm: login },
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
  expect(refusedDelete.headers().location).toBe('/account?notice=delete-plugins');

  // Deleted once its id is typed; restored within its 30 days.
  await page.locator('.row .delete summary').click();
  await page.locator('#confirm-plugin').fill('io.github.nope');
  await page.getByRole('button', { name: 'Supprimer', exact: true }).click();
  await expect(page).toHaveURL(/notice=plugin-unconfirmed$/);
  await page.locator('.row .delete summary').click();
  await page.locator('#confirm-plugin').fill(id);
  await page.getByRole('button', { name: 'Supprimer', exact: true }).click();
  await expect(page).toHaveURL(/notice=plugin-delete$/);
  await expect(row.locator('.tag')).toHaveText('Supprimé');
  await expect(page.locator('.notice.deleted')).toContainText('sa page et ses archives partent le');
  await expect.poll(inCatalog(page, id)).toBeNull();
  await page.goto(`/plugins/${id}?tab=versions`);
  await expect(page.locator('.band.bad')).toContainText('Retiré du store par son auteur');
  await expect(page.locator('.versions tr', { hasText: 'v0.4.0' })).toContainText('Retirée par l’auteur');
  await page.goto(`/account?plugin=${id}`);
  await page.getByRole('button', { name: 'Annuler la suppression' }).click();
  await expect(page).toHaveURL(/notice=plugin-restore$/);
  await expect.poll(async () => (await inCatalog(page, id)())?.latest).toBe('0.3.0');
});

test('a plugin offered to another account is theirs once they accept, its publications frozen 7 days', async ({ page, browser }) => {
  const lea = who('lea-give');
  const sam = who('sam-take');
  const repo = `relay-${RUN}`;
  const id = `io.github.${lea}.${repo}`.toLowerCase();
  const other = await browser.newPage();
  await author(other, sam, '');
  await author(page, lea, repo);
  await submit(page, `${lea}/${repo}`);
  await page.goto(`/account?plugin=${id}`);
  await page.locator('#transfer-login').fill('nobody-at-all');
  await page.getByRole('button', { name: 'Proposer le transfert' }).click();
  await expect(page).toHaveURL(/notice=plugin-no-account$/);
  await page.locator('#transfer-login').fill(sam);
  await page.getByRole('button', { name: 'Proposer le transfert' }).click();
  await expect(page).toHaveURL(/notice=plugin-transfer$/);
  await expect(page.locator('.row', { hasText: 'Transférer' })).toContainText(`Proposé à ${sam}, en attente de sa réponse.`);

  await other.goto('/account');
  const offer = other.locator('.banner', { hasText: `${lea} te transfère ${id}` });
  await expect(offer).toBeVisible();
  await offer.getByRole('button', { name: 'Accepter' }).click();
  await expect(other).toHaveURL(/notice=plugin-accept$/);
  await expect(other.locator('table.mine tbody tr', { hasText: id })).toBeVisible();
  await expect(other.locator('.manage')).toContainText(`Transféré par ${lea} le`);
  await expect(other.locator('.manage .notice.warn')).toContainText('Publications gelées jusqu’au');
  await expect.poll(async () => (await inCatalog(page, id)())?.author).toBe(sam);
  expect((await inCatalog(page, id)())!.transferred_at).toMatch(/^\d{4}-/);

  await page.goto('/account');
  await expect(page.locator('table.mine tbody tr', { hasText: id })).toHaveCount(0);
  await other.close();
});

test('another account cannot act on a plugin, and a form from another site is refused', async ({ page, browser }) => {
  const lea = who('lea-mine');
  const repo = `bell-${RUN}`;
  const id = `io.github.${lea}.${repo}`.toLowerCase();
  await author(page, lea, repo);
  await submit(page, `${lea}/${repo}`);
  const other = await browser.newPage();
  await author(other, who('mal-other'), '');
  const origin = new URL(page.url()).origin;
  const res = await other.request.post(`/api/account/plugins/${id}/unlist`, { headers: { origin }, maxRedirects: 0 });
  expect(res.headers().location).toContain('notice=plugin-not-found');
  const forged = await page.request.post(`/api/account/plugins/${id}/unlist`, {
    headers: { origin: 'https://evil.example' },
    maxRedirects: 0,
  });
  expect(forged.status()).toBe(403);
  expect((await catalog(page)).plugins.some((p) => p.id === id)).toBe(true);
  await other.close();
});

/** What the eye compares between the account and the preview: the banner, the rows, the refusal, the stats. */
async function measure(page: Page) {
  return page.evaluate(() => {
    const css = (sel: string, ...props: string[]) => {
      const el = document.querySelector(sel);
      return el ? props.map((p) => getComputedStyle(el).getPropertyValue(p)).join(' | ') : null;
    };
    return {
      banner: css('.banner', 'padding-top', 'border-top-color', 'border-top-left-radius', 'background-color', 'font-size', 'column-gap'),
      bannerSub: css('.banner .sub', 'font-size', 'color'),
      tile: css('.banner .tile', 'width', 'border-top-left-radius', 'font-size', 'font-weight'),
      primary: css('.banner .btn.primary', 'height', 'background-color', 'color'),
      pl: css('table.t .pl', 'column-gap'),
      plName: css('table.t .pl b', 'font-weight'),
      plId: css('table.t .pl small', 'font-size', 'color'),
      td: css('table.t tbody td', 'padding-top', 'border-bottom-color', 'font-size'),
      bad: css('.tag.bad', 'height', 'color', 'background-color', 'border-top-left-radius'),
      review: css('.tag.review', 'border-top-style'),
      why: css('.why', 'padding-top', 'border-top-color', 'background-color', 'font-size', 'border-top-left-radius'),
      whyBold: css('.why b', 'color', 'font-weight'),
      fix: css('.why .fix', 'color', 'margin-top'),
      statN: css('.stat-n', 'font-size', 'font-weight', 'letter-spacing'),
      axis: css('.axis', 'font-size', 'color'),
    };
  });
}

for (const scheme of ['dark', 'light'] as const)
  test(`the author’s space looks like the preview (${scheme})`, async ({ page }) => {
    const login = who(`ada-space-${scheme}`);
    const repo = `look-${scheme}-${RUN}`;
    const trap = `look-${scheme}-${RUN}-trap`;
    await author(page, login, `${repo}, ${trap}`);
    await submit(page, `${login}/${repo}`);
    await submit(page, `${login}/${trap}`, false);
    await fetch(`${FAKE_GITHUB}/_fake/tag`, { method: 'POST', body: JSON.stringify({ repo: `${login}/${repo}`, tag: 'v0.4.0' }) });
    await expect
      .poll(
        async () => {
          await page.goto('/account');
          return page.locator('.banner').count();
        },
        { timeout: 20_000 },
      )
      .toBe(1);
    const store = page.url();
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`${PREVIEW}${scheme === 'light' ? '?theme=light' : ''}#account`);
    await expect(page.locator('.banner')).toBeVisible();
    const preview = await measure(page);
    await page.goto(store);
    await expect(page.locator('.banner')).toBeVisible();
    expect(await measure(page)).toEqual(preview);
  });
