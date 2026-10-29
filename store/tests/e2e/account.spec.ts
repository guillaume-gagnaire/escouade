import { createHmac, randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { expect, test, type Page } from '@playwright/test';
import { E2E_APP, FAKE_GITHUB } from '../fake/env';

// The author's account through the fake GitHub (tests/fake/github.mjs): signing in, installing the App with or
// without the store's state, the session's cookie, the e-mail checked by its link, the export, the sign-out and the
// deletion with their tokens revoked, the signed webhooks and the forms' Origin. Each test signs in its own login:
// they run side by side on one base. They need a database (DATABASE_URL), as the account does.
test.skip(!process.env.DATABASE_URL, 'the accounts need DATABASE_URL');

// The logins of this run: a base kept between runs (a developer's) holds the accounts of the earlier ones.
const RUN = Date.now().toString(36).slice(-5);
const who = (name: string) => `${name}-${RUN}`;

const fake = async () =>
  (await (await fetch(`${FAKE_GITHUB}/_fake/state`)).json()) as {
    revoked: string[];
    mails: { to: string[]; raw: string }[];
    exchanged: string[];
  };

/** Signs `login` in from the header's button, through the fake GitHub's authorization page. */
async function signIn(page: Page, login: string, from = '/') {
  await page.goto(from);
  await page
    .locator('header')
    .getByRole('link', { name: /Se connecter avec GitHub|Sign in with GitHub/ })
    .click();
  await expect(page.getByRole('heading', { name: 'Authorize Escouade Store' })).toBeVisible();
  await page.getByLabel('Login').fill(login);
  await page.getByRole('button', { name: 'Authorize Escouade Store' }).click();
}

/** The text of a mail as the fake SMTP received it, its quoted-printable undone. */
const mailText = (raw: string) =>
  Buffer.from(
    raw.replace(/=\r?\n/g, '').replace(/=([0-9A-F]{2})/g, (_, h) => String.fromCharCode(parseInt(h, 16))),
    'latin1',
  ).toString('utf8');

test('a visitor is offered to sign in, and the account page says what the store keeps', async ({ page }) => {
  await page.goto('/account');
  await expect(page.locator('h1')).toHaveText('Ton compte d’auteur');
  await expect(page.locator('main').getByRole('link', { name: 'Se connecter avec GitHub' })).toHaveAttribute(
    'href',
    '/api/auth/github?to=%2Faccount',
  );
  await expect(page.getByText('Un seul cookie, celui de ta session')).toBeVisible();
  expect(await page.context().cookies()).toEqual([]);
});

test('signing in with GitHub opens the account, with a session cookie that scripts cannot read', async ({ page }) => {
  await signIn(page, who('lea-signin'));
  await expect(page).toHaveURL(/\/account$/);
  await expect(page.locator('h1')).toHaveText(`Mes plugins · ${who('lea-signin')}`);
  await expect(page.locator('header').getByRole('link', { name: 'Mes plugins' })).toBeVisible();
  await expect(page.locator('header .me')).toContainText('LS');
  await expect(page.getByText('Tu n’as encore publié aucun plugin.')).toBeVisible();
  await expect(page.getByText('n’est installée sur aucun de tes dépôts')).toBeVisible();

  const cookies = await page.context().cookies();
  expect(cookies.map((c) => c.name)).toEqual(['__Host-store_session']);
  expect(cookies[0]).toMatchObject({ httpOnly: true, secure: true, sameSite: 'Lax', path: '/' });
  expect(cookies[0]!.domain).toBe('localhost');
  expect(await page.evaluate(() => document.cookie)).toBe('');

  // The page in English, the same session.
  await page.goto('/en/account');
  await expect(page.locator('h1')).toHaveText(`My plugins · ${who('lea-signin')}`);
});

test('a callback with a state the browser did not leave with signs nobody in', async ({ page }) => {
  await page.goto('/api/auth/github/callback?code=stolen&state=forged');
  await expect(page).toHaveURL(/\/account\?notice=signin-failed$/);
  await expect(page.getByRole('status')).toHaveText('La connexion n’a pas abouti. Réessaie depuis ce bouton.');
  expect(await page.context().cookies()).toEqual([]);
  expect((await fake()).exchanged).not.toContain('stolen');
});

test('cancelling on GitHub comes back to the page, signed out', async ({ page }) => {
  await page.goto('/en/account');
  await page.locator('main').getByRole('link', { name: 'Sign in with GitHub' }).click();
  await page.getByRole('link', { name: 'Cancel' }).click();
  await expect(page).toHaveURL(/\/en\/account\?notice=signin-cancelled$/);
  await expect(page.getByRole('status')).toHaveText('Sign-in cancelled on GitHub.');
});

test('installing the App from the account lists its repositories', async ({ page }) => {
  await signIn(page, who('kim-install'));
  await page.getByRole('link', { name: 'Installer l’App sur un dépôt' }).click();
  await expect(page.getByRole('heading', { name: 'Install Escouade Store' })).toBeVisible();
  await page.getByLabel('Login').fill(who('kim-install'));
  await page.getByLabel('Repositories').fill('escouade-ntfy, escouade-teams');
  await page.getByRole('button', { name: 'Install & Authorize' }).click();
  await expect(page).toHaveURL(/\/account\?notice=installed$/);
  await expect(page.getByRole('status')).toContainText('L’App est installée.');
  const install = page.locator('.installs > li');
  await expect(install).toHaveCount(1);
  await expect(install).toContainText(who('kim-install'));
  await expect(install).toContainText('2 dépôts');
  await expect(install.locator('.repos li')).toHaveText([`${who('kim-install')}/escouade-ntfy`, `${who('kim-install')}/escouade-teams`]);
  await expect(page.getByRole('link', { name: 'Changer les dépôts' })).toBeVisible();
});

test('an installation started on GitHub comes back without state: the store asks GitHub again, with its own', async ({ page }) => {
  const before = (await fake()).exchanged.length;
  await page.goto(`${FAKE_GITHUB}/apps/escouade-store/installations/new`);
  await page.getByLabel('Login').fill(who('noa-direct'));
  await page.getByRole('button', { name: 'Install & Authorize' }).click();
  // The code GitHub sent without the state is not used: the authorization page comes back.
  await expect(page.getByRole('heading', { name: 'Authorize Escouade Store' })).toBeVisible();
  expect((await fake()).exchanged.length).toBe(before);
  expect(await page.context().cookies('http://localhost')).toEqual([expect.objectContaining({ name: '__Host-store_state' })]);
  await page.getByLabel('Login').fill(who('noa-direct'));
  await page.getByRole('button', { name: 'Authorize Escouade Store' }).click();
  await expect(page).toHaveURL(/\/account\?notice=installed$/);
  await expect(page.locator('h1')).toHaveText(`Mes plugins · ${who('noa-direct')}`);
  await expect(page.locator('.installs .repos li')).toHaveText([`${who('noa-direct')}/escouade-slack`]);
});

test('an address is kept once the link of its e-mail is followed, and can be removed', async ({ page }) => {
  await signIn(page, who('eve-mail'));
  const form = page.locator('aside form').first();
  await form.getByRole('textbox').fill('eve@exemple.dev');
  await form.getByRole('button', { name: 'Envoyer le lien' }).click();
  await expect(page).toHaveURL(/\/account\?notice=email-sent$/);
  await expect(page.getByText('Lien envoyé à eve@exemple.dev, à suivre dans les 24 heures.')).toBeVisible();

  const mail = (await fake()).mails.find((m) => m.to.includes('eve@exemple.dev'));
  expect(mail).toBeTruthy();
  const text = mailText(mail!.raw);
  expect(text).toContain('From: Escouade Store <store@escouade.dev>');
  expect(text).toMatch(/^Subject: =\?UTF-8\?Q\?V/m);
  expect(text).toContain('suis ce lien dans les 24 heures');
  expect(text).toContain('suis ce lien dans les 24 heures');
  const link = text.match(/http:\/\/localhost:\d+\/api\/account\/email\/verify\?token=[\w-]+&lang=fr/)?.[0];
  expect(link).toBeTruthy();

  await page.goto(link!);
  await expect(page).toHaveURL(/\/account\?notice=email-verified$/);
  await expect(page.locator('aside .input').first()).toContainText('eve@exemple.dev');
  await expect(page.locator('aside .input .ok')).toHaveText('vérifiée');
  // A link works once.
  await page.goto(link!);
  await expect(page).toHaveURL(/notice=email-expired$/);

  await page.getByRole('button', { name: 'Retirer l’adresse' }).click();
  await expect(page).toHaveURL(/notice=email-removed$/);
  await expect(page.locator('aside .input .ok')).toHaveCount(0);
});

test('a link the mail server refuses is said not sent, and a wrong address refused', async ({ page }) => {
  await signIn(page, who('ugo-refused'));
  const form = page.locator('aside form').first();
  await form.getByRole('textbox').fill('refuse@exemple.dev');
  await form.getByRole('button', { name: 'Envoyer le lien' }).click();
  await expect(page).toHaveURL(/notice=email-not-sent$/);
  await expect(page.getByRole('status')).toContainText('Le lien n’est pas parti');
  // The browser's own check aside, the server refuses what would not reach anyone.
  const res = await page.request.post('/api/account/email', {
    form: { email: 'nope@', lang: 'fr' },
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
  expect(res.status()).toBe(303);
  expect(res.headers().location).toBe('/account?notice=email-invalid');
});

test('the export holds the account and its journal, and no token', async ({ page }) => {
  await signIn(page, who('zoe-export'));
  const [download] = await Promise.all([page.waitForEvent('download'), page.getByRole('link', { name: 'Exporter' }).click()]);
  expect(download.suggestedFilename()).toBe(`escouade-store-${who('zoe-export')}.json`);
  const raw = await readFile((await download.path())!, 'utf8');
  const data = JSON.parse(raw);
  expect(data.account).toMatchObject({ login: who('zoe-export'), provider: 'github' });
  expect(data.journal.map((e: { action: string }) => e.action)).toContain('account.signed_in');
  expect(raw).not.toMatch(/ghu_|v1\.[\w-]+\.[\w-]+\./);
  const session = (await page.context().cookies())[0]!.value;
  expect(raw).not.toContain(session);
});

test('signing out ends the session and revokes its GitHub token', async ({ page }) => {
  await signIn(page, who('sam-out'));
  await page.getByRole('button', { name: 'Se déconnecter' }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('header').getByRole('link', { name: 'Se connecter avec GitHub' })).toBeVisible();
  expect(await page.context().cookies()).toEqual([]);
  expect((await fake()).revoked).toContain(who('sam-out'));
  await page.goto('/account');
  await expect(page.locator('h1')).toHaveText('Ton compte d’auteur');
});

test('deleting the account needs its login typed back, then signs out and revokes the token', async ({ page }) => {
  await signIn(page, who('max-delete'));
  await page.getByText('Supprimer le compte').click();
  await page.getByPlaceholder('Ton login').fill('max');
  await page.getByRole('button', { name: 'Supprimer définitivement' }).click();
  await expect(page).toHaveURL(/notice=delete-unconfirmed$/);
  await expect(page.locator('h1')).toHaveText(`Mes plugins · ${who('max-delete')}`);

  await page.getByText('Supprimer le compte').click();
  await page.getByPlaceholder('Ton login').fill(who('max-delete'));
  await page.getByRole('button', { name: 'Supprimer définitivement' }).click();
  await expect(page).toHaveURL(/\/account\?notice=deleted$/);
  await expect(page.getByRole('status')).toHaveText('Ton compte est supprimé. Merci d’être passé.');
  await expect(page.locator('h1')).toHaveText('Ton compte d’auteur');
  expect(await page.context().cookies()).toEqual([]);
  expect((await fake()).revoked).toContain(who('max-delete'));
});

test('a form posted from another site is refused, even with the session', async ({ page }) => {
  await signIn(page, who('ian-origin'));
  for (const path of ['/api/auth/logout', '/api/account/delete', '/api/account/email']) {
    const res = await page.request.post(path, {
      form: { confirm: who('ian-origin') },
      headers: { origin: 'https://evil.example' },
      maxRedirects: 0,
    });
    expect(res.status(), path).toBe(403);
  }
  await page.goto('/account');
  await expect(page.locator('h1')).toHaveText(`Mes plugins · ${who('ian-origin')}`);
});

test('the webhooks are taken only signed by the App, once per delivery', async ({ request }) => {
  const body = JSON.stringify({ action: 'created', sender: { id: 1 } });
  const sign = (raw: string, secret = E2E_APP.GITHUB_WEBHOOK_SECRET) => `sha256=${createHmac('sha256', secret).update(raw).digest('hex')}`;
  const send = (headers: Record<string, string>, data = body) =>
    request.post('/api/github/webhook', { data, headers: { 'content-type': 'application/json', 'x-github-event': 'star', ...headers } });
  const delivery = randomUUID();

  expect((await send({ 'x-github-delivery': delivery })).status()).toBe(401);
  expect((await send({ 'x-github-delivery': delivery, 'x-hub-signature-256': sign(body, 'not-the-secret') })).status()).toBe(401);
  // Signed over other bytes than those sent.
  expect((await send({ 'x-github-delivery': delivery, 'x-hub-signature-256': sign(body) }, `${body} `)).status()).toBe(401);

  const ok = await send({ 'x-github-delivery': delivery, 'x-hub-signature-256': sign(body) });
  expect(ok.status()).toBe(202);
  expect(await ok.json()).toEqual({ ok: true, handled: 'ignored' });
  const again = await send({ 'x-github-delivery': delivery, 'x-hub-signature-256': sign(body) });
  expect(await again.json()).toEqual({ ok: true, handled: 'duplicate' });
});

// The account's page measured against the validated preview (design/store/index.html, #account), in both themes.
const PREVIEW = new URL('../../../design/store/index.html', import.meta.url).href;

async function measureAccount(page: Page) {
  return page.evaluate(() => {
    const css = (sel: string, ...props: string[]) => {
      const el = document.querySelector(sel);
      return el ? props.map((p) => getComputedStyle(el).getPropertyValue(p)).join(' | ') : null;
    };
    const box = (sel: string) => {
      const r = document.querySelector(sel)?.getBoundingClientRect();
      return r ? { x: Math.round(r.x), w: Math.round(r.width) } : null;
    };
    return {
      title: css('.page-title', 'font-size', 'font-weight', 'letter-spacing', 'color'),
      more: css('.page-title .more', 'color'),
      th: css('table.t th', 'font-size', 'font-weight', 'color', 'border-bottom-color'),
      card: css('.side-card', 'padding-top', 'border-top-left-radius', 'border-top-color', 'font-size'),
      cardTitle: css('.side-card .h2', 'font-size', 'font-weight'),
      cardText: css('.side-card p', 'font-size', 'color'),
      label: css('.side-card .field label', 'font-size', 'font-weight', 'color'),
      input: css('.side-card .input', 'height', 'border-top-color', 'background-color', 'border-top-left-radius'),
      button: css('.side-card .btn.sm', 'height', 'font-size', 'border-top-left-radius'),
      danger: css('.side-card .btn.danger', 'color'),
      me: css('header .me', 'font-size', 'font-weight'),
      avatar: css('header .avatar', 'width', 'height', 'font-size', 'color'),
      section: box('.acct-layout > section'),
      aside: box('.acct-layout > aside'),
    };
  });
}

for (const scheme of ['dark', 'light'] as const) {
  test(`the account looks like the preview (${scheme})`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`${PREVIEW}${scheme === 'light' ? '?theme=light' : ''}#account`);
    await expect(page.locator('.page-title')).toBeVisible();
    const preview = await measureAccount(page);

    await signIn(page, who(`ada-look-${scheme}`), '/account');
    await expect(page.locator('.page-title')).toBeVisible();
    const store = await measureAccount(page);
    expect(store).toEqual(preview);
  });
}
