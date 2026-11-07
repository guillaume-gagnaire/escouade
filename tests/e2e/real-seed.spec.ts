// The real seed, with no network (ESC-354, docs/PLUGINS.md §8.7): a release build of the app with the seed
// `store/scripts/seed.ts fetch` downloaded (the store's bytes and signatures, checked with the keys the app embeds), on data
// folders of the test's, every request of the app sent to a closed port (its proxy and the system's). A blank folder installs
// nothing, and a harness chosen at the start installs its plugin from the seed; a configuration of 2.1 installs its plugins
// and nothing else, origin `store`, marked `seed`; a harness starts with no download; the view Plugins shows what is
// installed, uninstallable, and what the seed has besides; a plugin uninstalled is not put back at the next start.
//
// Skipped unless ESCOUADE_E2E_REAL_SEED is set, with ESCOUADE_E2E_EXE naming the release build (`npx tauri build
// --no-bundle` after `seed.ts fetch`; a build of development reads the fake store's keys and the seed of
// `scripts/seed-dev.sh`). ESCOUADE_E2E_REAL_OUT: a folder kept for the report (the times, from the app's log).

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_CODEX, send, test } from './fixture';

const OUT = process.env.ESCOUADE_E2E_REAL_OUT ?? path.join(os.tmpdir(), 'escouade-real-seed');
/** Where every request of the app goes: nothing answers there. */
const NOWHERE = 'http://127.0.0.1:9';
const OFFLINE = { HTTPS_PROXY: NOWHERE, HTTP_PROXY: NOWHERE, ALL_PROXY: NOWHERE, NO_PROXY: '' };
/** Réglages › Réseau's proxy, the one the plugins' requests take. */
const PROXY = { proxyUrl: NOWHERE, noProxy: '' };

const CLAUDE = 'dev.escouade.claude-code';
const CODEX = 'dev.escouade.codex';
const TRELLO = 'dev.escouade.trello';

test.skip(!process.env.ESCOUADE_E2E_REAL_SEED, 'ESCOUADE_E2E_REAL_SEED: a release build with the real seed');

function note(line: string) {
  fs.mkdirSync(OUT, { recursive: true });
  fs.appendFileSync(path.join(OUT, 'report.md'), `${line}\n`);
}

interface Entry {
  id: string;
  origin: { kind: string; version?: string; seed?: boolean; verified?: boolean };
}

/** What `plugins.json` lists. */
function listed(data: string): Entry[] {
  const file = path.join(data, 'plugins.json');
  if (!fs.existsSync(file)) return [];
  return JSON.parse(fs.readFileSync(file, 'utf8')).plugins as Entry[];
}

/** What is installed (from the store or its seed): the plugins the app still gives (origin `builtin`, until their version is signed) left out. */
const installed = (data: string) => listed(data).filter((e) => e.origin.kind !== 'builtin');

/** The ids the app still gives. */
const given = (data: string) =>
  listed(data)
    .filter((e) => e.origin.kind === 'builtin')
    .map((e) => e.id);

/** The lines of the app's logs of this test that say `what`. */
function logged(data: string, what: RegExp): string[] {
  const dir = path.join(data, 'logs');
  const files = [path.join(data, 'app.log'), ...(fs.existsSync(dir) ? fs.readdirSync(dir).map((f) => path.join(dir, f)) : [])];
  return files
    .filter((f) => fs.existsSync(f) && fs.statSync(f).isFile())
    .flatMap((f) => fs.readFileSync(f, 'utf8').split('\n'))
    .filter((l) => what.test(l));
}

async function openInstalled(page: Page) {
  if (!(await page.locator('.plugins-side').isVisible())) await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
}

test.describe('a first installation, offline', () => {
  test.use({ appEnv: OFFLINE, preStart: { run: () => ({ noAccounts: true, ...PROXY }) } });

  test('installs nothing; Claude Code chosen at the start installs its plugin from the seed', async ({ app }) => {
    const { page } = app;
    const welcome = page.getByRole('region', { name: 'Aucun compte' });
    const claude = welcome.getByRole('listitem').filter({ hasText: 'Claude Code' });
    await expect(claude).toBeVisible();
    expect(installed(app.data)).toEqual([]);

    const clicked = Date.now();
    await claude.getByRole('button', { name: 'Ajouter un compte…' }).click();
    await expect(page.locator('.toast').filter({ hasText: 'Plugin Claude Code installé' }).first()).toBeVisible();
    const seen = Date.now() - clicked;
    await expect.poll(() => installed(app.data).map((e) => e.id)).toEqual([CLAUDE]);
    const [entry] = installed(app.data);
    expect(entry!.origin).toMatchObject({ kind: 'store', seed: true, verified: true });
    note(`- Vierge : « Ajouter un compte… » sur Claude Code → la ligne « Plugin Claude Code installé » en ${seen} ms`);
    for (const l of logged(app.data, /installed from the seed/)) note(`  - ${l.trim()}`);
    note(`  - toujours donnés par l'app (builtin) : ${given(app.data).join(', ') || 'aucun'}`);
  });
});

test.describe('a configuration of 2.1, offline', () => {
  // Claude Code and Codex accounts, a Trello account connected; what 2.1 kept: no plugins.json, the plugins given by the app.
  test.use({
    appEnv: { ...OFFLINE, ESCOUADE_CODEX: FAKE_CODEX },
    preStart: {
      run: (root: string) => {
        fs.writeFileSync(path.join(root, 'data', 'integrations.json'), JSON.stringify({ trello: {} }));
        return PROXY;
      },
    },
  });

  test('installs its plugins and nothing else; a harness starts with no download; uninstalled stays so', async ({ app }) => {
    test.setTimeout(240_000);
    let { page } = app;
    await expect
      .poll(() =>
        installed(app.data)
          .map((e) => e.id)
          .sort(),
      )
      .toEqual([CLAUDE, CODEX, TRELLO].sort());
    for (const e of installed(app.data)) expect(e.origin, e.id).toMatchObject({ kind: 'store', seed: true, verified: true });
    note('- 2.1 configurée (Claude Code, Codex, Trello) : installés au démarrage, origine store, marque seed :');
    for (const l of logged(app.data, /installed from the seed/)) note(`  - ${l.trim()}`);

    // A Claude Code agent, its plugin precompiled at its first activation.
    await addProject(page, app.repo);
    await send(page, 'Bonjour');
    await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible({ timeout: 60_000 });
    await expect.poll(() => logged(app.data, new RegExp(`plugin ${CLAUDE}: .* in \\d+ ms`)).length).toBeGreaterThan(0);
    note('- Premier agent Claude Code, sans téléchargement :');
    for (const l of logged(app.data, /plugin dev\.escouade\.[a-z-]+: .*(precompiled|cache|compiled).* in \d+ ms/)) note(`  - ${l.trim()}`);

    // The view: what is installed, uninstallable; what the seed has besides, in the app.
    await openInstalled(page);
    const added = page.getByRole('table', { name: 'Ajoutés' });
    for (const name of ['Claude Code', 'Codex', 'Trello']) await expect(added).toContainText(name);
    const proposed = page.getByRole('region', { name: 'Proposés par Escouade' });
    for (const name of ['Copilot CLI', 'GitHub', 'Linear', 'Deepgram', 'Discord', 'Goose']) await expect(proposed).toContainText(name);
    await expect(proposed).toContainText('Dans l’app, sans téléchargement');
    await expect(proposed).not.toContainText('Non installé');

    await added.getByRole('button', { name: /Trello/ }).click();
    await expect(page.locator('.plugins-work')).toContainText('l’amorce d’Escouade, sans réseau');
    await page.getByRole('button', { name: 'Désinstaller' }).click();
    await page.getByRole('dialog', { name: 'Désinstaller Trello ?' }).getByRole('button', { name: 'Désinstaller' }).click();
    await expect(proposed).toContainText('Tu l’as désinstallé (v2.2.2)');

    // Started anew: the seed does not put it back.
    page = await app.restart();
    await openInstalled(page);
    await expect(page.getByRole('region', { name: 'Proposés par Escouade' })).toContainText('Tu l’as désinstallé (v2.2.2)');
    expect(installed(app.data).map((e) => e.id)).not.toContain(TRELLO);
    // The store was never reached: no catalogue kept. (The app's own update check goes around a proxy that fails, by
    // design: what it tried is noted.)
    expect(fs.existsSync(path.join(app.data, 'store', 'catalog.json'))).toBe(false);
    note('- Réseau : aucun catalogue du store lu (store/catalog.json absent)');
    for (const l of logged(app.data, /through the proxy failed|failed to check for updates/)) note(`  - ${l.trim().slice(0, 160)}`);
  });
});
