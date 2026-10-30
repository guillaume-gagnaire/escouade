// The register of the decisions (ESC-259) on a temporary repository whose `.escouade/` already holds a spec and four
// decisions: the register numbered and dated with each one's state, origin and what it changed, its filter by state, the
// detail of one, « Accepter » in one click (the file says who and when), « Proposer une décision » by hand, and an accepted
// decision replaced by a new one, which supersedes it once accepted (the old one says by which, its text untouched). The
// captures go to ESCOUADE_E2E_SHOTS, dark and light.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `decisions-${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

/** A day `days` before today, as the decisions write it. */
function day(days: number): string {
  const d = new Date();
  d.setDate(d.getDate() - days);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

const q = (v: unknown) => JSON.stringify(v);

/** A decision's file, as Escouade writes it (`docs::decision_write`). */
function decisionFile(o: {
  number: number;
  title: string;
  date: string;
  state: string;
  origin: string;
  originRef: string;
  pages?: string[];
  tickets?: string[];
  supersedes?: number[];
  supersededBy?: number | null;
  changes?: { page: string; added: number; removed: number }[];
  acceptedBy?: string;
  context: string;
  decision: string;
  consequences: string;
}) {
  const head = [
    ['number', String(o.number)],
    ['title', q(o.title)],
    ['date', q(o.date)],
    ['state', q(o.state)],
    ['origin', q(o.origin)],
    ['origin_ref', q(o.originRef)],
    ['pages', q(o.pages ?? [])],
    ['tickets', q(o.tickets ?? [])],
    ['documents', q([])],
    ['supersedes', q(o.supersedes ?? [])],
    ['superseded_by', o.supersededBy == null ? 'null' : String(o.supersededBy)],
    ['changes', q(o.changes ?? [])],
    ['accepted_at', q(o.acceptedBy ? `${o.date}T10:00:00+02:00` : '')],
    ['accepted_by', q(o.acceptedBy ?? '')],
  ];
  return `---\n${head.map(([k, v]) => `${k}: ${v}`).join('\n')}\n---\n\n## Contexte\n\n${o.context}\n\n## Décision\n\n${o.decision}\n\n## Conséquences\n\n${o.consequences}\n`;
}

const page_ = (title: string, subject: string, body: string) => `---\ntitle: ${q(title)}\nsubject: ${q(subject)}\n---\n\n${body}\n`;

test('the register of the decisions: numbered and dated, filtered, accepted in one click, proposed by hand and replaced', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  await page.setViewportSize({ width: 1440, height: 900 });
  const git = (...args: string[]) => execFileSync('git', args, { cwd: app.repo, encoding: 'utf8' });
  const esc = path.join(app.repo, '.escouade');
  const write = (rel: string, text: string) => {
    fs.mkdirSync(path.dirname(path.join(esc, rel)), { recursive: true });
    fs.writeFileSync(path.join(esc, rel), text);
  };
  write('spec/adhesion/en-ligne.md', page_('Adhésion en ligne', 'Adhésion en ligne', 'Trois étapes.'));
  write('spec/paiement/echeances.md', page_('Échéances', 'Paiement', 'Quatre échéances au plus.'));
  write('spec/paiement/stripe.md', page_('Prestataire', 'Paiement', 'Stripe.'));
  write('spec/espace/notifications.md', page_('Notifications', 'Espace adhérent', 'Par e-mail.'));
  write(
    'decisions/0004-paiement-par-stripe.md',
    decisionFile({
      number: 4,
      title: 'Paiement par Stripe',
      date: day(40),
      state: 'accepted',
      origin: 'discussion',
      originRef: 'Prestataire de paiement',
      changes: [{ page: 'paiement/stripe', added: 6, removed: 0 }],
      acceptedBy: 'Claire',
      context: 'Il faut un prestataire de paiement.',
      decision: 'Stripe, pour la carte et le prélèvement.',
      consequences: 'Un compte Stripe à ouvrir.',
    }),
  );
  write(
    'decisions/0009-paiement-en-3-fois-au-plus.md',
    decisionFile({
      number: 9,
      title: 'Paiement en 3 fois au plus',
      date: day(26),
      state: 'superseded',
      origin: 'document',
      originRef: 'Cahier des charges v3',
      supersededBy: 13,
      pages: ['paiement/echeances'],
      acceptedBy: 'Claire',
      context: 'Le cahier des charges demande le paiement en plusieurs fois.',
      decision: 'Trois échéances au plus.',
      consequences: 'Un échéancier.',
    }),
  );
  write(
    'decisions/0013-paiement-en-4-fois.md',
    decisionFile({
      number: 13,
      title: 'Paiement en 4 fois',
      date: day(4),
      state: 'accepted',
      origin: 'ticket',
      originRef: 'DEM-31',
      supersedes: [9],
      changes: [{ page: 'paiement/echeances', added: 2, removed: 1 }],
      acceptedBy: 'Claire',
      context: 'Les familles demandent plus d’échéances.',
      decision: 'Quatre échéances au plus.',
      consequences: 'L’échéancier passe à quatre lignes.',
    }),
  );
  write(
    'decisions/0014-tarif-famille-a-partir-de-3-membres.md',
    decisionFile({
      number: 14,
      title: 'Tarif famille à partir de 3 membres',
      date: day(1),
      state: 'proposed',
      origin: 'brainstorm',
      originRef: 'Tarif famille',
      tickets: ['DEM-38'],
      changes: [
        { page: 'adhesion/en-ligne', added: 4, removed: 0 },
        { page: 'paiement/echeances', added: 2, removed: 1 },
      ],
      context: 'Les familles adhèrent membre par membre et paient plein tarif ; le CR du 22/10 demande un geste.',
      decision: '−20 % sur chaque membre dès le troisième, dans le même foyer.',
      consequences: 'Un foyer à déclarer à l’adhésion ; reçus fiscaux par membre inchangés.',
    }),
  );
  write(
    'decisions/0015-relances-par-e-mail-seulement.md',
    decisionFile({
      number: 15,
      title: 'Relances par e-mail seulement',
      date: day(0),
      state: 'proposed',
      origin: 'brainstorm',
      originRef: 'Relance des adhésions',
      changes: [{ page: 'espace/notifications', added: 3, removed: 0 }],
      context: 'Les adhésions à renouveler se perdent.',
      decision: 'Pas de SMS : consentement et coût.',
      consequences: 'À revoir si le taux de renouvellement baisse.',
    }),
  );
  git('add', '.');
  git('commit', '-qm', 'spec et décisions');
  const committed = git('rev-parse', 'HEAD');

  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('group', { name: 'Vue du projet' }).getByRole('button', { name: 'Docs' }).click();
  const spaces = page.getByRole('region', { name: 'Espaces des docs' });
  await expect(spaces.getByRole('button', { name: /Décisions/ })).toContainText('5');
  await spaces.getByRole('button', { name: /Décisions/ }).click();

  // The register: numbered, dated, the latest first, each one's state, origin and what it changed.
  await expect(page.locator('[data-counts]')).toHaveText('5 décisions · 2 acceptées · 2 proposées · 1 remplacée');
  const table = page.getByRole('table', { name: 'Registre des décisions' });
  // By its number, or a proposal by its id (ESC-303: it has no number until accepted).
  const row = (n: number | string) => table.locator(`tr[data-decision="${n}"]`);
  await expect(table.locator('tbody tr')).toHaveCount(5);
  expect(await table.locator('tbody tr').evaluateAll((rs) => rs.map((r) => r.getAttribute('data-decision')))).toEqual([
    '15',
    '14',
    '13',
    '9',
    '4',
  ]);
  await expect(row(15)).toContainText('Auj.');
  await expect(row(15)).toContainText('Brainstorm « Relance des adhésions »');
  await expect(row(15)).toContainText('Espace adhérent');
  await expect(row(14)).toContainText('Hier');
  await expect(row(14)).toContainText('Adhésion en ligne, Paiement');
  await expect(row(13)).toContainText('Ticket DEM-31');
  await expect(row(9)).toContainText('Remplacée');
  await expect(row(9)).toContainText('Paiement · par 013');
  await expect(row(9)).toContainText('Cahier des charges v3');

  // The detail of one: its parts, what it changed (its pages, with their lines), its links.
  await row(14).getByRole('button', { name: 'Tarif famille à partir de 3 membres' }).click();
  const detail = page.getByRole('complementary', { name: 'Tarif famille à partir de 3 membres' });
  await expect(detail).toContainText('ADR-014 · proposée hier');
  await expect(detail).toContainText('−20 % sur chaque membre dès le troisième');
  await expect(detail.getByRole('button', { name: 'Ouvrir la page Adhésion en ligne' })).toHaveText('Adhésion en ligne +4 −0');
  await expect(detail.getByRole('button', { name: 'Ouvrir la page Échéances' })).toHaveText('Échéances +2 −1');
  await expect(detail.locator('[data-links]')).toContainText('DEM-38');
  await expect(detail.locator('[data-links]')).toContainText('Brainstorm « Tarif famille »');
  await shoot(page, 'register');

  // The filter, by state.
  await page.getByRole('button', { name: 'Filtrer les décisions par état' }).click();
  await shoot(page, 'filter');
  await page.getByRole('menuitemradio', { name: /Proposées/ }).click();
  await expect(table.locator('tbody tr')).toHaveCount(2);
  await page.getByRole('button', { name: 'Filtrer les décisions par état' }).click();
  await page.getByRole('menuitemradio', { name: /Toutes/ }).click();
  await expect(table.locator('tbody tr')).toHaveCount(5);

  // « Accepter », a click in the register: the file says so, and who.
  const file14 = path.join(esc, 'decisions', '0014-tarif-famille-a-partir-de-3-membres.md');
  await row(14).getByRole('button', { name: 'Accepter la décision 014' }).click();
  await expect(row(14)).toContainText('Acceptée');
  await expect(row(14).getByRole('button', { name: /Accepter/ })).toHaveCount(0);
  await expect.poll(() => fs.readFileSync(file14, 'utf8')).toContain('state: "accepted"');
  expect(fs.readFileSync(file14, 'utf8')).toMatch(/accepted_by: "[^"]+"/);
  await expect(detail).toContainText(/ADR-014 · acceptée aujourd’hui/);
  await expect(page.locator('[data-docs-share]')).toContainText('1 changement à partager');

  // « Proposer une décision », by hand.
  await page.getByRole('button', { name: 'Proposer une décision' }).click();
  const form = page.getByRole('dialog', { name: 'Proposer une décision' });
  await form.getByRole('textbox', { name: 'Titre' }).fill('Reçu fiscal par e-mail');
  await form.getByRole('textbox', { name: 'Contexte' }).fill('Les reçus partent par la poste.');
  await form.getByRole('textbox', { name: 'Décision' }).fill('Le reçu fiscal part par e-mail, en PDF.');
  await shoot(page, 'propose');
  await form.getByRole('button', { name: 'Proposer' }).click();
  await expect(form).toHaveCount(0);
  // No number until it is accepted (ESC-303): its id, and a file without a number.
  await expect(row('recu-fiscal-par-e-mail')).toContainText('Proposée');
  await expect(page.getByRole('complementary', { name: 'Reçu fiscal par e-mail' })).toContainText(
    'recu-fiscal-par-e-mail · proposée aujourd’hui, numérotée à son acceptation',
  );
  await expect.poll(() => fs.existsSync(path.join(esc, 'decisions', 'recu-fiscal-par-e-mail.md'))).toBe(true);

  // An accepted decision is not rewritten: « La remplacer » proposes a new one, which supersedes it once accepted.
  const file4 = path.join(esc, 'decisions', '0004-paiement-par-stripe.md');
  const before = fs.readFileSync(file4, 'utf8');
  await row(4).getByRole('button', { name: 'Paiement par Stripe' }).click();
  await page.getByRole('complementary', { name: 'Paiement par Stripe' }).getByRole('button', { name: 'La remplacer' }).click();
  const again = page.getByRole('dialog', { name: 'Remplacer la décision 004' });
  await again.getByRole('textbox', { name: 'Titre' }).fill('Paiement par Mollie');
  await again.getByRole('textbox', { name: 'Décision' }).fill('Mollie, moins cher pour une association.');
  await again.getByRole('button', { name: 'Proposer' }).click();
  await expect(again).toHaveCount(0);
  const mollie = page.getByRole('complementary', { name: 'Paiement par Mollie' });
  await expect(mollie.getByRole('button', { name: 'Voir la décision 004' })).toHaveText('Remplace 004');
  // Until it is accepted, the old one stands.
  await expect(row(4)).toContainText('Acceptée');
  expect(fs.readFileSync(file4, 'utf8')).toBe(before);
  // Accepted: the next number of the register, 16 (the one proposed by hand still has none).
  await mollie.getByRole('button', { name: 'Accepter' }).click();
  await expect(row(4)).toContainText('Remplacée');
  await expect(row(4)).toContainText('par 016');
  await expect.poll(() => fs.existsSync(path.join(esc, 'decisions', '0016-paiement-par-mollie.md'))).toBe(true);
  expect(fs.existsSync(path.join(esc, 'decisions', 'paiement-par-mollie.md'))).toBe(false);
  await row(4).getByRole('button', { name: 'Paiement par Stripe' }).click();
  const old = page.getByRole('complementary', { name: 'Paiement par Stripe' });
  await expect(old).toContainText('ADR-004 · remplacée par 016');
  await expect(old.getByRole('button', { name: 'La remplacer' })).toHaveCount(0);
  const after = fs.readFileSync(file4, 'utf8');
  expect(after).toContain('state: "superseded"');
  expect(after).toContain('superseded_by: 16');
  // Its text untouched.
  expect(after.split('---\n')[2]).toBe(before.split('---\n')[2]);
  await shoot(page, 'superseded');
  // Nothing committed by Escouade.
  expect(git('rev-parse', 'HEAD')).toBe(committed);
});
