import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { expect, openSettingsPage, test } from './fixture';

// The assistant of the features (F-2, ESC-559): a station of the first launch's line, after the first agent, on a new
// installation with Claude Code (the fake `claude`) signed in: every feature off before it (`NEW_INSTALL`), the assistant
// gone through screen by screen, the Kanban switched on then off again, the Docs space on; after it, the Docs tab is there
// and the Kanban's is not. Then the same at the update of an installation that has tickets: the assistant opens by itself,
// the Kanban « il reste allumé », switched off there, gone after. « Revoir la présentation » opens it again. With
// ESCOUADE_E2E_SHOTS, the station and the window are shot in both themes at the window's size (run once more with
// ESCOUADE_E2E_VIEWPORT=1024x728 for the small one).

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIGNED_IN = JSON.stringify({ claudeAiOauth: { accessToken: 'tok', refreshToken: 'ref', expiresAt: 4_102_444_800_000 } });
/** A data folder as a new installation leaves it: settings with no mode, no switches (`features: null`), no answer. */
const NEW_INSTALLATION = { noAccounts: true, agentMode: '', allowBypass: false, telemetry: null, features: null };
const FEATURES_Q = 'Que veux-tu voir autour de tes agents ?';

const read = (file: string) => JSON.parse(fs.readFileSync(file, 'utf8'));
const heading = (page: Page, name: string, level = 1) => page.getByRole('heading', { level, name });

/** `where` shot in both themes, at the window's size. */
async function shoot(page: Page, where: Locator, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const width = await page.evaluate(() => window.innerWidth);
  await page.mouse.move(0, 0);
  for (const t of ['light', 'dark'] as const) {
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
      document.documentElement.style.colorScheme = t;
    }, t);
    await expect(where).toBeVisible();
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${t === 'light' ? 'clair' : 'sombre'}-${width}.png`) });
  }
  await page.evaluate(() => {
    document.documentElement.dataset.theme = 'dark';
    document.documentElement.style.colorScheme = 'dark';
  });
}

/** Nothing wider than the window. */
async function fitsAcross(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
}

test.describe('the assistant at the first launch', () => {
  test.use({
    preStart: {
      run: (root: string) => {
        const dir = path.join(root, 'claude');
        fs.mkdirSync(dir, { recursive: true });
        fs.writeFileSync(path.join(dir, '.credentials.json'), SIGNED_IN);
        fs.writeFileSync(path.join(dir, '.claude.json'), JSON.stringify({ oauthAccount: { emailAddress: 'ada@atlas.dev' } }));
        return NEW_INSTALLATION;
      },
    },
  });

  test('is a station after the first agent: a feature switched off has no tab after it, one switched on has', async ({ app }) => {
    test.setTimeout(240_000);
    const page = app.page;
    const settings = () => read(path.join(app.data, 'settings.json'));
    const state = () => read(path.join(app.data, 'state.json'));
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    // A new installation: every feature off until the assistant.
    await expect.poll(() => settings().features?.kanban).toBe(false);
    expect(Object.values(settings().features).every((v) => v === false)).toBe(true);
    await expect(heading(page, 'Avec quel harness tes agents travaillent-ils ?')).toBeFocused();
    await expect(journey.getByRole('radio', { name: /Claude Code/ })).toContainText('Connecté · ada@atlas.dev');
    await page.keyboard.press('Enter');
    await expect(heading(page, 'Quand tes agents doivent-ils te demander ?')).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(heading(page, 'Que veux-tu brancher ?')).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(heading(page, 'Où travaille ton premier agent ?')).toBeVisible();
    await journey.getByRole('textbox', { name: 'Le dossier du projet' }).fill(app.repo);
    await journey.getByRole('button', { name: /Ouvrir le projet/ }).click();
    await expect(heading(page, 'Ton premier message')).toBeVisible();
    const frame = journey.getByTestId('onboarding-frame');
    await frame.getByRole('button', { name: 'Envoyer' }).click();
    await expect(heading(page, 'Ton agent a répondu')).toBeVisible({ timeout: 60_000 });
    await journey.getByRole('button', { name: /^Continuer/ }).click();

    // The station of the features: the line says where it is, one feature per screen.
    await expect(heading(page, FEATURES_Q)).toBeFocused();
    const line = page.getByRole('navigation', { name: 'Les stations du premier lancement' }).getByRole('listitem');
    await expect(line).toHaveCount(8);
    await expect(line.nth(6)).toContainText('Fonctionnalités');
    await expect(line.nth(6)).toContainText('maintenant');
    await expect(heading(page, 'Le Kanban', 2)).toBeVisible();
    const tour = journey.getByTestId('features-tour');
    await expect(tour.getByRole('img', { name: 'Capture d’Escouade : Kanban' })).toBeVisible();
    await expect(tour.getByRole('region', { name: 'Le Kanban' }).getByRole('listitem')).toHaveCount(3);
    const kanban = tour.getByRole('switch', { name: 'Activer le Kanban' });
    await expect(kanban).toHaveAttribute('aria-checked', 'false');
    await fitsAcross(page);
    await shoot(page, tour, 'assistant-station');
    // ↓ to the switch, Espace switches it on, as Réglages › Fonctionnalités saves it.
    await heading(page, FEATURES_Q).focus();
    await page.keyboard.press('ArrowDown');
    await expect(kanban).toBeFocused();
    await page.keyboard.press(' ');
    await expect(kanban).toHaveAttribute('aria-checked', 'true');
    await expect.poll(() => settings().features.kanban).toBe(true);
    // « Suivant » screen by screen: the editor, then the Docs space, switched on.
    await journey.getByRole('button', { name: /^Suivant/ }).click();
    await expect(heading(page, 'L’éditeur', 2)).toBeVisible();
    await journey.getByRole('button', { name: /^Suivant/ }).click();
    await expect(heading(page, 'L’espace Docs', 2)).toBeVisible();
    await tour.getByRole('switch', { name: 'Activer l’espace Docs' }).click();
    await expect.poll(() => settings().features.docs).toBe(true);
    // Back to the Kanban by its chip: switched off again.
    await tour.getByRole('navigation', { name: 'Les fonctionnalités' }).getByRole('button', { name: 'Kanban' }).click();
    await kanban.click();
    await expect.poll(() => settings().features.kanban).toBe(false);
    // To the last screen, then « Continuer ».
    await tour.getByRole('navigation', { name: 'Les fonctionnalités' }).getByRole('button', { name: 'Voix' }).click();
    await expect(heading(page, 'La voix', 2)).toBeVisible();
    await journey.getByRole('button', { name: /^Continuer/ }).click();
    await expect(heading(page, 'Aider à améliorer Escouade ?')).toBeVisible();
    await expect(line.nth(6)).toContainText('1 allumée');
    await journey.getByRole('button', { name: 'Non merci' }).click();
    await journey.getByRole('button', { name: /Commencer/ }).click();
    await expect(journey).toBeHidden();
    await expect.poll(() => state().onboarding?.steps?.features ?? null).toBe('done');

    // The app: the Docs tab is there, the Kanban's is not.
    const tabs = page.getByRole('group', { name: 'Vue du projet' });
    await expect(tabs.getByRole('button')).toHaveText([/^Agents/, /^Docs/]);
    expect(settings().features).toMatchObject({ kanban: false, docs: true, editor: false, voice: false });
    // The assistant never opens by itself after the first launch.
    await expect(page.getByTestId('features-tour-window')).toHaveCount(0);

    // « Revoir la présentation » (Réglages › Fonctionnalités): the assistant again, the settings closed; « Fermer la
    // présentation » changes nothing.
    const dialog = await openSettingsPage(page, 'Fonctionnalités');
    await dialog.getByRole('button', { name: 'Revoir la présentation' }).click();
    const again = page.getByRole('dialog', { name: 'Les fonctionnalités d’Escouade' });
    await expect(again).toBeVisible();
    await expect(page.getByRole('dialog', { name: 'Réglages' })).toHaveCount(0);
    await expect(again.getByRole('switch', { name: 'Activer le Kanban' })).toHaveAttribute('aria-checked', 'false');
    await again.getByRole('button', { name: 'Suivant' }).click();
    await again.getByRole('button', { name: 'Suivant' }).click();
    await expect(again.getByRole('switch', { name: 'Activer l’espace Docs' })).toHaveAttribute('aria-checked', 'true');
    await again.getByRole('button', { name: 'Fermer la présentation' }).click();
    await expect(again).toBeHidden();
    expect(settings().features).toMatchObject({ kanban: false, docs: true });
  });
});

test.describe('the assistant at the update', () => {
  // An installation of before the version that brings the switches: its settings have none, a project of it has a ticket
  // (done: the autopilot starts nothing).
  test.use({
    preStart: {
      run: (root: string) => {
        const project = { id: 'p1', name: 'demo-api', path: path.join(root, 'demo-api'), color: '#5aa9e6' };
        const ticket = { id: 't1', projectId: 'p1', key: 'DEM-1', title: 'Une tâche d’avant', column: 'done' };
        fs.writeFileSync(path.join(root, 'data', 'state.json'), JSON.stringify({ projects: [project], tickets: [ticket] }));
        return { features: null };
      },
    },
  });

  test('opens by itself on an installation with tickets: the Kanban stays on, switched off there it goes', async ({ app }) => {
    test.setTimeout(240_000);
    let page = app.page;
    const file = path.join(app.data, 'settings.json');
    // The first window of the version: the assistant opens by itself, the Kanban kept on, the rest off.
    const tour = page.getByRole('dialog', { name: 'Choisis tes fonctionnalités' });
    await expect(tour).toBeVisible();
    await expect(tour).toContainText('Ce que tu utilises reste allumé');
    await expect(heading(page, 'Le Kanban', 2)).toBeVisible();
    await expect(tour).toContainText('Tu l’utilises déjà sur 1 projet : il reste allumé.');
    const kanban = tour.getByRole('switch', { name: 'Activer le Kanban' });
    await expect(kanban).toHaveAttribute('aria-checked', 'true');
    expect(read(file).features).toMatchObject({ kanban: true, docs: false, editor: false, cost: false });
    expect(read(file).featuresUsed).toEqual([{ feature: 'kanban', projects: 1 }]);
    await fitsAcross(page);
    await shoot(page, tour, 'assistant-mise-a-jour');
    // Gone through: the Kanban switched off there, the editor on; « Terminer » at the last screen.
    await kanban.click();
    await expect.poll(() => read(file).features.kanban).toBe(false);
    await tour.getByRole('button', { name: 'Suivant' }).click();
    await tour.getByRole('switch', { name: 'Activer l’éditeur' }).click();
    await expect.poll(() => read(file).features.editor).toBe(true);
    for (let i = 2; i < 9; i++) await tour.getByRole('button', { name: 'Suivant' }).click();
    await tour.getByRole('button', { name: 'Terminer' }).click();
    await expect(tour).toBeHidden();
    // The Kanban's tab is gone, its ticket kept; started again, the assistant does not come back.
    await page
      .getByRole('button', { name: /demo-api/ })
      .first()
      .click();
    const tabs = () => page.getByRole('group', { name: 'Vue du projet' });
    await expect(tabs().getByRole('button')).toHaveText([/^Agents/]);
    page = await app.restart();
    await expect(page.getByRole('dialog', { name: 'Choisis tes fonctionnalités' })).toHaveCount(0);
    await expect(tabs().getByRole('button')).toHaveText([/^Agents/]);
    expect(read(path.join(app.data, 'state.json')).tickets.length).toBe(1);
  });
});
