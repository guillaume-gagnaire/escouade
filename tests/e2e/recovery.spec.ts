import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { expect, test } from './fixture';

// ESC-469: on 2026-11-15 a power cut left `state.json` its size and nothing but zeros, and the app started empty without a
// word. Now the start finds the file unreadable, sets it aside under a dated name, takes back its newest copy of
// `backups/`, and a notice says so until it is closed. Here the test's data folder holds such a state and one copy.

const COPIED_AT = '20261115T092000Z';

test.use({
  preStart: {
    run: (root: string) => {
      // The project's repository, which the fixture keeps.
      const repo = path.join(root, 'demo-api');
      fs.mkdirSync(repo);
      const git = (...args: string[]) => execFileSync('git', args, { cwd: repo, stdio: 'pipe' });
      git('init', '-q', '-b', 'main');
      git('config', 'user.email', 'e2e@test');
      git('config', 'user.name', 'e2e');
      fs.writeFileSync(path.join(repo, 'README.md'), '# demo-api\n');
      git('add', '-A');
      git('commit', '-qm', 'init');
      const data = path.join(root, 'data');
      const state = {
        projects: [{ id: 'p469', name: 'atelier', path: repo, color: 'oklch(0.72 0.12 48)' }],
        tickets: [
          { id: 't1', projectId: 'p469', key: 'ATL-1', title: 'Relances par e-mail', column: 'todo', rank: 1 },
          { id: 't2', projectId: 'p469', key: 'ATL-2', title: 'Facture en PDF', column: 'todo', rank: 2 },
        ],
        ui: { activeProject: 'p469', view: 'project', selectedAgent: {} },
      };
      const copies = path.join(data, 'backups', 'state');
      fs.mkdirSync(copies, { recursive: true });
      fs.writeFileSync(path.join(copies, `state-${COPIED_AT}.json.gz`), zlib.gzipSync(JSON.stringify(state)));
      // The state as the cut left it: its size, zeros.
      fs.writeFileSync(path.join(data, 'state.json'), Buffer.alloc(16 * 1024));
    },
  },
});

test('a state of zeros comes back from its copy, and a notice that stays says so', async ({ app }) => {
  let page = app.page;
  const band = page.getByTestId('recovery-band');
  await expect(band).toBeVisible({ timeout: 30_000 });
  await expect(band).toContainText('Des données d’Escouade étaient illisibles au démarrage');
  await expect(band).toContainText(
    'Le fichier state.json (tes projets, tickets et agents) ne contenait que des zéros, comme après une coupure du poste : Escouade a repris sa copie du',
  );
  await expect(band).toContainText(/Ce qui a changé (entre|après) le .+ est perdu\./);
  await expect(band).toContainText('Le fichier illisible est gardé sous');
  await expect(band).toContainText(path.join(app.data, 'backups'));

  // The project and its tickets are there.
  await expect(page.locator('header.bar .proj .name')).toHaveText('atelier');
  await page
    .getByRole('button', { name: /^Kanban/ })
    .first()
    .click();
  await expect(page.getByText('Relances par e-mail')).toBeVisible();
  await expect(page.getByText('Facture en PDF')).toBeVisible();

  // The file of zeros kept, under a dated name; the log says it.
  const kept = fs.readdirSync(app.data).filter((n) => n.startsWith('state.unreadable-'));
  expect(kept).toHaveLength(1);
  expect(fs.readFileSync(path.join(app.data, kept[0])).every((b) => b === 0)).toBe(true);
  expect(fs.readFileSync(path.join(app.data, 'app.log'), 'utf8')).toMatch(/state\.json did not read \(Zeros\)/);

  // It stays across a restart, until it is closed.
  page = await app.restart();
  await expect(page.getByTestId('recovery-band')).toBeVisible({ timeout: 30_000 });
  await page.getByTestId('recovery-close').click();
  await expect(page.getByTestId('recovery-band')).toHaveCount(0);
  page = await app.restart();
  await expect(page.locator('header.bar .proj .name')).toHaveText('atelier');
  await expect(page.getByTestId('recovery-band')).toHaveCount(0);
  // The copy from before the cut stays among the copies.
  expect(fs.readdirSync(path.join(app.data, 'backups', 'state'))).toContain(`state-${COPIED_AT}.json.gz`);
});
