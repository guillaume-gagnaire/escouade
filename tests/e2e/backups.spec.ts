// The screen of the copies (ESC-487), in Réglages › Avancé › Copies des données: the test's data folder holds a state
// that lost a project and two tickets, and two copies of it, the older one with everything. The copies are listed with
// what they hold, the older one compared to today (nothing written), taken back (confirmed, then put in place at the
// restart: the projects and tickets back, the file of before set aside under a dated name); a closed project's history
// gone from the disk is found again from its copy. The captures go to ESCOUADE_E2E_SHOTS, dark and light.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import type { Page } from '@playwright/test';
import { expect, openSettingsPage, test } from './fixture';

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
    await page.screenshot({ path: path.join(SHOTS, `backups-${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

/** A copy's stamp, `hours` before now: `20261116T093600Z`. */
function stamp(hours: number): string {
  return new Date(Date.now() - hours * 3_600_000)
    .toISOString()
    .replace(/[-:]/g, '')
    .replace(/\.\d+Z$/, 'Z');
}

const OLDER = stamp(20);
const NEWER = stamp(2);

function repo(root: string, name: string): string {
  const dir = path.join(root, name);
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(dir, 'README.md'), `# ${name}\n`);
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

const quiet = { autopilot: false };
const ticket = (id: string, projectId: string, key: string, title: string, rank: number) => ({
  id,
  projectId,
  key,
  title,
  column: 'todo',
  rank,
});

test.use({
  preStart: {
    run: (root: string) => {
      const atelier = repo(root, 'atelier');
      const boutique = repo(root, 'boutique');
      const archive = repo(root, 'archive');
      const data = path.join(root, 'data');
      const p1 = { id: 'p487a', name: 'atelier', path: atelier, color: 'oklch(0.72 0.12 48)', board: quiet };
      const p2 = { id: 'p487b', name: 'boutique', path: boutique, color: 'oklch(0.7 0.12 160)', board: quiet };
      const ui = { activeProject: 'p487a', view: 'project', selectedAgent: {} };
      // Everything, the day before.
      const older = {
        projects: [p1, p2],
        tickets: [
          ticket('t1', 'p487a', 'ATL-1', 'Relances par e-mail', 1),
          ticket('t2', 'p487a', 'ATL-2', 'Facture en PDF', 2),
          ticket('t3', 'p487b', 'BTQ-1', 'Panier partagé', 1),
        ],
        ui,
      };
      // Then the project boutique and two tickets gone (as a mistake, or a cut, would leave them).
      const today = { projects: [p1], tickets: [ticket('t1', 'p487a', 'ATL-1', 'Relances par e-mail', 1)], ui };
      const copies = path.join(data, 'backups', 'state');
      fs.mkdirSync(copies, { recursive: true });
      fs.writeFileSync(path.join(copies, `state-${OLDER}.json.gz`), zlib.gzipSync(JSON.stringify(older)));
      fs.writeFileSync(path.join(copies, `state-${NEWER}.json.gz`), zlib.gzipSync(JSON.stringify(today)));
      fs.writeFileSync(path.join(data, 'state.json'), JSON.stringify(today));
      // A closed project's history gone from the disk, its copy left.
      const history = {
        project: { id: 'c487', name: 'archive', path: archive, color: 'oklch(0.7 0.1 300)', board: quiet },
        agents: [],
        tickets: [ticket('t9', 'c487', 'ARC-1', 'Vieux rapport', 1)],
        closedAt: Date.now() - 86_400_000,
      };
      const closed = path.join(data, 'backups', 'closed-projects', 'c487');
      fs.mkdirSync(closed, { recursive: true });
      fs.writeFileSync(path.join(closed, `c487-${OLDER}.json.gz`), zlib.gzipSync(JSON.stringify(history)));
    },
  },
});

async function copiesOf(page: Page) {
  const settings = await openSettingsPage(page, 'Avancé');
  const group = settings.getByTestId('backups');
  await group.scrollIntoViewIfNeeded();
  await expect(group.getByTestId('backups-total')).toBeVisible({ timeout: 30_000 });
  return { settings, group };
}

test('the copies are listed, compared to today, taken back at the restart, and a lost closed project found again', async ({ app }) => {
  let page = app.page;
  await expect(page.locator('header.bar .proj .name')).toHaveText('atelier', { timeout: 30_000 });
  const { group } = await copiesOf(page);
  // The state, what it holds today and its copies (the start may make one more of today's).
  const state = group.locator('[data-testid="backup-file"][data-file="state.json"]');
  await expect(state).toContainText('Projets, tickets et agents');
  await expect(state).toContainText(/Aujourd’hui : 1 projet, 0 agent, 1 ticket · [23] copies/);
  await state.getByRole('button', { name: 'Voir les copies' }).click();
  const older = group.getByTestId('backup-copy').filter({ hasText: '2 projets, 0 agent, 3 tickets' });
  await expect(older).toHaveCount(1);

  // Compared: by name, nothing written.
  // Today's state as the app keeps it (it may write it again by itself, the same projects and tickets): never the copy's.
  const today = () => JSON.parse(fs.readFileSync(path.join(app.data, 'state.json'), 'utf8'));
  const projectsOf = (s: { projects: { name: string }[] }) => s.projects.map((p) => p.name);
  expect(projectsOf(today())).toEqual(['atelier']);
  await older.getByRole('button', { name: 'Comparer' }).click();
  const diff = group.getByTestId('backup-diff');
  await expect(diff).toContainText('Reviendraient : boutique.');
  await expect(diff).toContainText('Reviendraient : ATL-2 Facture en PDF, BTQ-1 Panier partagé.');
  expect(projectsOf(today())).toEqual(['atelier']);
  await diff.scrollIntoViewIfNeeded();
  await shoot(page, 'compare');

  // The lost closed project, offered to be found again.
  const lost = group.getByTestId('backup-lost');
  await expect(lost).toContainText('archive');
  await expect(lost).toContainText('0 agent, 1 ticket');

  // Taken back: confirmed first, saying what it replaces and that the file of now is set aside.
  await older.getByRole('button', { name: 'Reprendre' }).click();
  const confirm = page.getByRole('dialog').filter({ hasText: /Reprendre la copie du/ });
  await expect(confirm).toContainText('Le fichier state.json d’aujourd’hui (1 projet, 0 agent, 1 ticket) sera remplacé par cette copie');
  await expect(confirm).toContainText('mis de côté sous un nom daté');
  await shoot(page, 'confirm');
  await confirm.getByRole('button', { name: 'Reprendre la copie' }).click();
  // Back on the copies: the copy waits for the next start, today's file untouched until then.
  await expect(confirm).toHaveCount(0);
  await expect(page.getByRole('dialog', { name: 'Réglages' })).toBeVisible();
  const after = await copiesOf(page);
  const pending = after.group.getByTestId('backups-pending');
  await expect(pending).toContainText('sera reprise au prochain démarrage');
  await expect(pending.getByRole('button', { name: 'Redémarrer maintenant' })).toBeEnabled();
  expect(projectsOf(today())).toEqual(['atelier']);
  await pending.scrollIntoViewIfNeeded();
  await shoot(page, 'pending');

  // The lost history found again now (no restart for it): among the closed projects, « Rouvrir » offered.
  await after.group.getByTestId('backup-lost').getByRole('button', { name: 'Retrouver' }).click();
  await expect(after.group.getByTestId('backup-reopen')).toBeVisible();
  expect(fs.existsSync(path.join(app.data, 'closed-projects', 'c487.json'))).toBe(true);

  // The restart: the copy read again and put in place, the projects and tickets back, the band says so.
  page = await app.restart();
  const band = page.getByTestId('recovery-band');
  await expect(band).toBeVisible({ timeout: 30_000 });
  await expect(band.getByTestId('restored-item')).toContainText('Escouade a repris la copie du');
  await expect(band.getByTestId('restored-item')).toContainText('Le fichier d’avant est gardé sous');
  await shoot(page, 'restored');
  await page
    .getByRole('button', { name: /^Kanban/ })
    .first()
    .click();
  await expect(page.getByText('Facture en PDF')).toBeVisible();
  const kept = fs.readdirSync(app.data).filter((n) => n.startsWith('state.before-restore-'));
  expect(kept).toHaveLength(1);
  expect(projectsOf(JSON.parse(fs.readFileSync(path.join(app.data, kept[0]), 'utf8')))).toEqual(['atelier']);
  expect(projectsOf(today())).toEqual(['atelier', 'boutique']);
  // The project boutique back on the rail.
  await expect(page.getByRole('tab', { name: /boutique/ }).first()).toBeVisible();
  await page.getByTestId('recovery-close').click();
  await expect(band).toHaveCount(0);

  // The screen lists the file set aside, with its place on the disk; nothing waits any more.
  const last = await copiesOf(page);
  await expect(last.group.getByTestId('backups-pending')).toHaveCount(0);
  await expect(last.group.getByTestId('backups-set-aside')).toContainText('state.json, avant une reprise');
  await expect(last.group.getByTestId('backups-set-aside')).toContainText(path.join(app.data, kept[0]));
  await last.group.getByTestId('backups-set-aside').scrollIntoViewIfNeeded();
  await shoot(page, 'set-aside');

  // The journal names no folder of the disk.
  expect(fs.readFileSync(path.join(app.data, 'app.log'), 'utf8')).not.toContain(app.root);
});
