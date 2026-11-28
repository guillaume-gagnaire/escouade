// What waits, and answering it from the phone (MO-5, docs/MOBILE.md § 3.3 to § 3.6 and § 5), at 360 × 780 and 430 × 932,
// against an `escouade-server` of the test's and its fake `claude`: an authorization read to its last line then allowed
// under a simulated lock (M3, M4), the next one opening; a question answered on the spot; a request of the box with its
// renders (the viewer) and its PDF; a message sent with a photo and the system's dictation (M9); the cut, then the
// return (M13). The screens of B, in both themes, for the captures of design/mobile-2-3/app/.

import { expect, test as base, type Locator, type Page } from '@playwright/test';
import { agentTold, demo, projectOf, startSpace, stoppedAgent, type TestSpace } from './harness';
import { open, shot, tab, travail } from './page';

const test = base.extend<{ space: TestSpace }>({
  space: async ({}, use) => {
    const space = await startSpace();
    await use(space);
    await space.stop();
  },
});

/** A line of the thread, by what it shows. */
const line = (page: Page, text: string | RegExp) => page.getByTestId('line').filter({ hasText: text });
/** Unfolds a line (touches its head). */
const unfold = async (l: Locator) => {
  const head = l.getByTestId('line-head');
  if ((await head.getAttribute('aria-expanded')) !== 'true') await head.click();
  await expect(head).toHaveAttribute('aria-expanded', 'true');
};

interface Lock {
  __ESCOUADE_LOCK__?: 'ok' | 'cancel' | 'none';
  __ESCOUADE_UNLOCKS__?: string[];
  __ESCOUADE_DICTATE__?: string;
}

/** What the core says of an agent, as its glance gives it. */
async function statusOf(space: TestSpace, id: string): Promise<string | null> {
  const g = await space.call<{ agents: { id: string; status: string }[] }>('space_glance', { counts: false });
  return g.agents.find((a) => a.id === id)?.status ?? null;
}

test('an authorization read to its last line, then allowed under the lock; the next one opens', async ({ page, space }, info) => {
  const p = await projectOf(space);
  const long = await agentTold(space, p, 'Nettoie tout, permission trente');
  const short = await agentTold(space, p, 'Lance les tests, permission');
  await page.addInitScript(() => ((window as Lock).__ESCOUADE_LOCK__ = 'cancel'));
  await open(page, travail(space));
  const first = line(page, 'étape 1 sur 30');
  const second = line(page, 'rm -rf build');
  await expect(first).toBeVisible({ timeout: 45_000 });
  await expect(second).toBeVisible({ timeout: 45_000 });
  // The dot says each is new; « Tes agents se sont arrêtés » counts them.
  await expect(page.getByRole('heading', { name: /Tes agents se sont arrêtés\s*2/ })).toBeVisible();
  await expect(first.getByLabel('Nouveau')).toBeVisible();

  // Thirty lines: the fold shows twelve; « Autoriser » waits for the last one, read in full screen (M4).
  await unfold(first);
  await expect(first.getByLabel('Nouveau')).toHaveCount(0);
  const allow = first.getByRole('button', { name: 'Autoriser' });
  await expect(allow).toBeDisabled();
  await expect(first.getByTestId('read-to-end')).toHaveText('Lis la commande jusqu’au bout pour autoriser.');
  // A shorter screen (the keyboard's, a phone held sideways): the last line below the view until it is scrolled to.
  const size = page.viewportSize()!;
  await page.setViewportSize({ width: size.width, height: 480 });
  await first.getByRole('button', { name: 'Lire la commande entière · 30 lignes' }).click();
  const sheet = page.getByRole('dialog', { name: 'La commande' });
  await expect(sheet.getByTestId('command')).toContainText('rm -rf build # la dernière ligne');
  const sheetAllow = sheet.getByRole('button', { name: 'Autoriser' });
  await page.waitForTimeout(500);
  await expect(sheetAllow).toBeDisabled();
  await sheet.getByTestId('command-end').scrollIntoViewIfNeeded();
  await expect(sheetAllow).toBeEnabled();
  await page.setViewportSize(size);

  // The lock cancelled: nothing goes, the agent still waits.
  await sheetAllow.click();
  await expect(sheet.getByRole('status')).toHaveText('Verrou annulé : rien n’est parti.');
  expect(await statusOf(space, long.id)).toBe('waiting');
  // The lock passed: « Autorisé » goes to that agent, saying what and where.
  await page.evaluate(() => ((window as Lock).__ESCOUADE_LOCK__ = 'ok'));
  await sheetAllow.click();
  await expect(sheet).toBeHidden();
  expect(await page.evaluate(() => (window as Lock).__ESCOUADE_UNLOCKS__)).toEqual([
    'Autoriser la commande de nettoie-tout-fake ? · Escouade · Travail',
    'Autoriser la commande de nettoie-tout-fake ? · Escouade · Travail',
  ]);
  await expect.poll(() => statusOf(space, long.id), { timeout: 20_000 }).not.toBe('waiting');
  await expect(first).toHaveCount(0);

  // The next request opens in its place: a short command, read at once, armed.
  await expect(second.getByTestId('line-head')).toHaveAttribute('aria-expanded', 'true');
  await expect(second.getByTestId('command')).toHaveText('$ rm -rf build');
  await expect(second.getByRole('button', { name: 'Autoriser' })).toBeEnabled();
  expect(await statusOf(space, short.id)).toBe('waiting');

  // « Refuser » takes a word, without the lock.
  await second.getByRole('button', { name: 'Refuser' }).click();
  await second.getByRole('textbox', { name: 'Un mot pour lance-les-fake' }).fill('pas sur ce dossier');
  await second.getByRole('button', { name: 'Refuser' }).last().click();
  await expect.poll(() => statusOf(space, short.id), { timeout: 20_000 }).not.toBe('waiting');
  expect(await page.evaluate(() => (window as Lock).__ESCOUADE_UNLOCKS__)).toHaveLength(2);
  await expect(page.getByRole('heading', { name: 'Rien ne t’attend' })).toBeVisible();
});

test('a question answered on the spot, without the lock', async ({ page, space }) => {
  const p = await projectOf(space);
  const agent = await agentTold(space, p, 'Quelle base ? question');
  await open(page, travail(space));
  const q = line(page, 'Quelle base de données ?');
  await expect(q).toBeVisible({ timeout: 45_000 });
  await unfold(q);
  await q.getByRole('radio', { name: 'PostgreSQL' }).click();
  await expect(q.getByRole('radio', { name: 'PostgreSQL' })).toHaveAttribute('aria-checked', 'true');
  await q.getByRole('button', { name: 'Répondre' }).click();
  await expect(q).toHaveCount(0, { timeout: 20_000 });
  expect(await page.evaluate(() => (window as Lock).__ESCOUADE_UNLOCKS__ ?? [])).toEqual([]);
  const items = await space.call<{ kind: string; answers?: Record<string, string> | null }[]>('get_conversation', { id: agent.id });
  expect(items.find((i) => i.kind === 'question')?.answers).toEqual({ 'Quelle base de données ?': 'PostgreSQL' });
});

test('a request of the box: its renders in the viewer, its PDF, « Plus tard », « Un mot », an answer taken back', async ({
  page,
  space,
}) => {
  const p = await projectOf(space, 'portail-adherents');
  await agentTold(space, p, '[demande] rends les trois directions du tableau de bord');
  await open(page, travail(space));
  const ask = line(page, 'Choisis la direction du tableau de bord');
  await expect(ask).toBeVisible({ timeout: 45_000 });
  await expect(page.getByRole('heading', { name: /Aujourd’hui\s*1/ })).toBeVisible();
  // Folded, its renders: real images, read from the core.
  await expect(ask.getByLabel('Les rendus de ses options').locator('img')).toHaveCount(3);
  await expect(ask.locator('img').first()).toHaveAttribute('src', /^data:image\/png;base64,/);

  await unfold(ask);
  await expect(ask.getByText('Recommandé : B · Cartes empilées.')).toBeVisible();
  await expect(ask.getByRole('button', { name: 'Choisir B' })).toBeVisible();

  // The viewer: one render at a time, the other theme of the same option, the system's back closes it.
  await ask.getByRole('button', { name: 'Comparer B · Cartes empilées en grand' }).click();
  const viewer = page.getByRole('dialog', { name: 'Les rendus' });
  await expect(viewer).toContainText('B · Cartes empilées');
  await expect(viewer.getByRole('img', { name: 'B · Cartes empilées' }).locator('img')).toHaveAttribute('src', /^data:image\/png/);
  await expect(viewer).toContainText('3 sur 6');
  await viewer.getByRole('button', { name: 'Clair' }).click();
  await expect(viewer).toContainText('4 sur 6');
  await expect(viewer.getByRole('button', { name: 'Clair' })).toHaveAttribute('aria-pressed', 'true');
  await page.goBack();
  await expect(viewer).toBeHidden();

  // The PDF, page by page, drawn by pdf.js.
  await ask.getByRole('button', { name: /etude\.pdf/ }).click();
  const pdf = page.getByRole('dialog', { name: 'Lecture de etude.pdf' });
  await expect(pdf).toContainText('3 pages');
  await expect(pdf.locator('canvas')).toHaveCount(3);
  await expect
    .poll(() =>
      pdf
        .locator('canvas')
        .first()
        .evaluate((c: HTMLCanvasElement) => c.width),
    )
    .toBeGreaterThan(0);
  await page.goBack();
  await expect(pdf).toBeHidden();

  // « Plus tard » offers its times; « Un mot » a word that goes with the answer.
  await ask.getByRole('button', { name: 'Plus tard' }).click();
  await expect(ask.getByRole('group', { name: 'Reporter jusqu’à…' }).getByRole('button', { name: 'Dans 1 heure' })).toBeVisible();
  await ask.getByRole('group', { name: 'Reporter jusqu’à…' }).getByRole('button', { name: 'Annuler' }).click();
  await ask.getByRole('button', { name: 'Un mot' }).click();
  await ask.getByRole('textbox', { name: 'Un mot pour l’agent (facultatif)' }).fill('la courbe reste lisible');
  await ask.getByRole('button', { name: 'Choisir B' }).click();

  // Answered from the phone: the line goes to « Répondu récemment »; « Annuler » takes it back within its seconds.
  const toast = page.getByRole('status').filter({ hasText: 'Ta réponse part dans' });
  await expect(toast).toBeVisible();
  await expect(page.getByRole('heading', { name: /Répondu récemment/ })).toBeVisible();
  const answered = await space.call<{ asks: { state: string; answer: { options: string[]; note: string; via: string } | null }[] }>(
    'inbox_list',
    { projectId: null },
  );
  expect(answered.asks[0].answer).toMatchObject({ options: ['b'], note: 'la courbe reste lisible', via: 'phone' });
  await toast.getByRole('button', { name: 'Annuler' }).click();
  await expect
    .poll(async () => (await space.call<{ asks: { state: string }[] }>('inbox_list', { projectId: null })).asks[0].state)
    .toBe('open');
  await expect(page.getByRole('heading', { name: /Aujourd’hui\s*1/ })).toBeVisible();
});

test('a message sent from the conversation, with a photo and the system’s dictation', async ({ page, space }) => {
  const p = await projectOf(space);
  const agent = await agentTold(space, p, 'Lance les tests, permission');
  await page.addInitScript(() => ((window as Lock).__ESCOUADE_DICTATE__ = 'si ça casse'));
  await open(page, travail(space));
  await expect(line(page, 'rm -rf build')).toBeVisible({ timeout: 45_000 });
  // From « Agents »: its conversation, its request at its end.
  await tab(page, 'Agents').click();
  await page.getByRole('button', { name: 'lance-les-fake' }).click();
  const talk = page.getByRole('dialog', { name: 'lance-les-fake' });
  await expect(talk.getByTestId('conversation')).toContainText('Lance les tests, permission');
  await expect(talk.getByTestId('command')).toHaveText('$ rm -rf build');
  // The request answered from there: the agent goes on, then waits for a message.
  await talk.getByRole('button', { name: 'Autoriser' }).click();
  await expect.poll(() => statusOf(space, agent.id), { timeout: 20_000 }).not.toBe('waiting');

  const field = talk.getByRole('textbox', { name: 'Écris à lance-les-fake…' });
  await field.fill('relance seulement les avoirs');
  await talk.getByRole('button', { name: 'Dicter' }).click();
  await expect(field).toHaveValue('relance seulement les avoirs si ça casse');
  await talk.getByTestId('photo-input').setInputFiles({
    name: 'essai.png',
    mimeType: 'image/png',
    buffer: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==', 'base64'),
  });
  await expect(talk.getByRole('img', { name: 'essai.jpg' })).toBeVisible();
  await talk.getByRole('button', { name: 'Envoyer' }).click();
  await expect(talk.getByRole('status').filter({ hasText: 'Envoyé' })).toBeVisible();
  await expect(field).toHaveValue('');
  await expect
    .poll(async () => {
      const items = await space.call<{ kind: string; text?: string; images?: number }[]>('get_conversation', { id: agent.id });
      return items.find((i) => i.kind === 'user' && i.text?.startsWith('relance seulement'));
    })
    .toMatchObject({ text: 'relance seulement les avoirs si ça casse', images: 1 });
  await page.goBack();
  await expect(talk).toBeHidden();
});

test('the cut, then the return: what the phone knew stays, dated; nothing answers; then everything does again', async ({
  page,
  space,
}, info) => {
  await stoppedAgent(space);
  await open(page, travail(space));
  const l = line(page, 'Autorisation');
  await expect(l).toBeVisible({ timeout: 45_000 });
  await unfold(l);
  await expect(l.getByRole('button', { name: 'Autoriser' })).toBeEnabled();

  space.cut();
  await expect(page.getByRole('radio', { name: 'Travail, injoignable' })).toBeVisible();
  await expect(l.getByText('Répondre attend la connexion : une réponse ne part jamais sur un écran périmé.')).toBeVisible();
  await expect(l.getByRole('button', { name: 'Autoriser' })).toBeDisabled();
  await expect(l.getByRole('button', { name: 'Refuser' })).toBeDisabled();
  await expect(page.getByTestId('stale')).toContainText(/Lu à \d\d:\d\d : Travail ne répond plus/);
  await shot(page, info, 'horsligne');

  // Opened again while cut (M13): the last glance, kept without the command.
  await page.reload();
  await expect(line(page, 'Autorisation')).toContainText('facturation');
  await expect(page.getByTestId('stale')).toBeVisible();
  const kept = await page.evaluate(() => JSON.stringify(localStorage));
  expect(kept).toContain('facturation');
  expect(kept).not.toContain('rm -rf build');

  // The return: the space read again, the answer possible again.
  await space.restore();
  await expect(page.getByRole('radio', { name: 'Travail, 1 en attente' })).toBeVisible({ timeout: 30_000 });
  await expect(page.getByTestId('stale')).toHaveCount(0);
  await unfold(line(page, 'Autorisation'));
  await expect(line(page, 'Autorisation').getByRole('button', { name: 'Autoriser' })).toBeEnabled();
});

test('the screens of B, in both themes', async ({ page, space }, info) => {
  test.setTimeout(240_000);
  await demo(space);
  for (const theme of ['dark', 'light'] as const) {
    await open(page, travail(space), theme);
    const ask = line(page, 'Choisis la direction du tableau de bord');
    await expect(ask).toBeVisible({ timeout: 45_000 });
    await expect(page.getByRole('heading', { name: /Tes agents se sont arrêtés\s*4/ })).toBeVisible({ timeout: 45_000 });
    await expect(ask.locator('img')).toHaveCount(3);
    await shot(page, info, 'fil');

    const perm = line(page, 'rm -rf build');
    await unfold(perm);
    await expect(perm.getByRole('button', { name: 'Autoriser' })).toBeEnabled();
    await shot(page, info, 'autorisation');

    const long = line(page, 'étape 1 sur 30');
    await unfold(long);
    await long.getByRole('button', { name: /Lire la commande entière/ }).click();
    await page.getByRole('dialog', { name: 'La commande' }).getByTestId('command-end').scrollIntoViewIfNeeded();
    await shot(page, info, 'commande');
    await page.goBack();

    await unfold(line(page, 'Quelle base de données ?'));
    await shot(page, info, 'question');
    const plan = line(page, 'Approuver le plan');
    await unfold(plan);
    await expect(plan.getByRole('heading', { name: 'Plan' })).toBeVisible();
    await shot(page, info, 'plan');

    await unfold(ask);
    await ask.getByRole('button', { name: 'Choisir B' }).scrollIntoViewIfNeeded();
    await shot(page, info, 'boite');
    await ask.getByRole('button', { name: 'Comparer B · Cartes empilées en grand' }).click();
    await expect(page.getByRole('dialog', { name: 'Les rendus' }).locator('img')).toBeVisible();
    await shot(page, info, 'visionneuse');
    await page.goBack();
    await ask.getByRole('button', { name: /etude\.pdf/ }).click();
    const pdf = page.getByRole('dialog', { name: 'Lecture de etude.pdf' });
    await expect
      .poll(() =>
        pdf
          .locator('canvas')
          .first()
          .evaluate((c: HTMLCanvasElement) => c.width),
      )
      .toBeGreaterThan(0);
    await page.waitForTimeout(500);
    await shot(page, info, 'pdf');
    await page.goBack();

    await unfold(perm);
    await perm.getByRole('button', { name: /^Conversation/ }).click();
    const talk = page.getByRole('dialog', { name: 'lance-les-fake' });
    await expect(talk.getByTestId('command')).toBeVisible();
    await talk.getByRole('textbox').fill('et relance seulement les avoirs si ça casse');
    await shot(page, info, 'conversation');
    await page.goBack();
    await expect(talk).toBeHidden();
  }
});
