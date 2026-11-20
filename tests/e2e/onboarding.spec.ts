import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, openSettingsPage, test } from './fixture';

// ESC-504 (ON-1): the first launch, in a data folder as a new installation leaves it (no account, no project, no mode for
// the new agents, no answer to the statistics), with the fake `claude` as Claude Code. Run at both sizes, the CI's
// runner's (ESCOUADE_E2E_VIEWPORT=1024x728) and the default one.

/** A sign-in that is not out of date (as accounts.spec.ts has it): the machine's Claude Code is signed in. */
const SIGNED_IN = JSON.stringify({ claudeAiOauth: { accessToken: 'tok', refreshToken: 'ref', expiresAt: 4_102_444_800_000 } });

/** What a new installation's settings say: nothing chosen yet. The fixture writes its own beside (sound, language…). */
const NEW_INSTALLATION = { noAccounts: true, agentMode: '', allowBypass: false, telemetry: null };

const read = (file: string) => JSON.parse(fs.readFileSync(file, 'utf8'));

/** Nothing of the window goes past its right edge (docs/ONBOARDING.md § 8: no scroll across). */
async function fitsAcross(page: Page) {
  const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(over).toBeLessThanOrEqual(0);
}

test.describe('a first launch with Claude Code signed in on the machine', () => {
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

  test('goes through the harness, its account, the starting settings and the statistics, is kept, and is done again from Aide', async ({
    app,
  }) => {
    let page = app.page;
    const settings = () => read(path.join(app.root, 'data', 'settings.json'));
    const state = () => read(path.join(app.root, 'data', 'state.json'));
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await expect(journey).toBeVisible();
    // The rail and the work are not there: the journey takes the window, the title bar and the status bar apart.
    await expect(page.getByRole('navigation', { name: 'Les stations du premier lancement' })).toBeVisible();
    await expect(page.getByRole('heading', { level: 1, name: 'Avec quel harness tes agents travaillent-ils ?' })).toBeFocused();
    const claude = journey.getByRole('radio', { name: /Claude Code/ });
    await expect(claude).toHaveAttribute('aria-checked', 'true');
    await expect(claude).toContainText('Connecté · ada@atlas.dev');
    await expect(claude).toContainText('Recommandé');
    await expect(journey).toContainText('Ses agents partent sur le compte déjà connecté');
    await fitsAcross(page);
    // Entrée from the question: the harness first in « Ordre des harness », its account added (the account's station
    // folded in), as Réglages › Comptes et harness writes them.
    await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { level: 1, name: 'Quand tes agents doivent-ils te demander ?' })).toBeFocused();
    await expect.poll(() => settings().accounts.map((a: { id: string }) => a.id)).toEqual(['principal']);
    expect(settings().engineOrder?.[0] ?? 'claude').toBe('claude');
    // The modes explained, the recommended one chosen, Bypass off with why; the model and the effort said.
    await expect(journey).toContainText(/Modèle Sonnet[^,]*, effort moyen : ceux de ton compte/);
    const modes = journey.getByRole('radiogroup', { name: 'Quand tes agents doivent-ils te demander ?' }).getByRole('radio');
    await expect(modes).toHaveCount(5);
    await expect(modes.nth(0)).toHaveAttribute('aria-checked', 'true');
    await expect(modes.nth(4)).toHaveAttribute('aria-disabled', 'true');
    await expect(modes.nth(4)).toContainText('Éteint : l’agent ferait tout sans rien te montrer');
    await fitsAcross(page);
    // ↓ to the choices, ↓ to « Demander », Entrée: « Mode par défaut des agents », as Réglages › Agents writes it.
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await expect(modes.nth(1)).toHaveAttribute('aria-checked', 'true');
    await expect(modes.nth(1)).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { level: 1, name: 'Que veux-tu brancher ?' })).toBeFocused();
    await expect.poll(() => settings().agentMode).toBe('default');
    // « Ce que tu veux brancher » (ESC-512): nothing ticked, Entrée goes on, each card passed.
    await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { level: 1, name: 'Où travaille ton premier agent ?' })).toBeVisible();
    // Échap is « Plus tard »: the project passed, « à reprendre » on the line; the first agent, with no project, says so
    // and is passed too (ESC-505).
    await page.keyboard.press('Escape');
    await expect(page.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeVisible();
    await expect(journey).toContainText('Ton premier agent a besoin d’un projet');
    await page.keyboard.press('Escape');
    await expect(page.getByRole('heading', { level: 1, name: 'Aider à améliorer Escouade ?' })).toBeVisible();
    const line = page.getByRole('navigation', { name: 'Les stations du premier lancement' });
    await expect(line.getByRole('listitem').nth(3)).toContainText('rien de branché');
    await expect(line.getByRole('listitem').nth(4)).toContainText('passé · à reprendre');
    await expect(line.getByRole('listitem').nth(5)).toContainText('passé · à reprendre');
    await expect(line.getByRole('listitem').nth(1)).toContainText('Principal');
    await expect(line.getByRole('listitem').nth(2)).toContainText('Demander');
    await journey.getByRole('button', { name: 'Non merci' }).click();
    await expect.poll(() => settings().telemetry).toBe(false);
    await expect(journey.getByRole('heading', { name: 'Escouade est prêt' })).toBeVisible();
    await expect(journey).toContainText('2 étapes passées : elles t’attendent dans Aide › Premier lancement.');
    await journey.getByRole('button', { name: /Commencer/ }).click();
    await expect(journey).toBeHidden();
    // The app: no project yet, the way to the first one.
    await expect(page.getByRole('heading', { name: 'Un premier projet' })).toBeVisible();
    await expect.poll(() => state().onboarding?.finished ?? null).not.toBeNull();
    expect(state().onboarding.steps).toEqual({
      harness: 'done',
      account: 'done',
      defaults: 'done',
      hookups: 'done',
      project: 'skipped',
      agent: 'skipped',
      telemetry: 'done',
    });
    expect(state().onboarding.hookups).toEqual({ sources: 'skipped', voice: 'skipped', forge: 'skipped' });

    // Started again: never opened by itself once it ended, nor the questions of before (the mode, the statistics).
    page = await app.restart();
    await expect(page.getByRole('heading', { name: 'Un premier projet' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Premier lancement' })).toHaveCount(0);
    await expect(page.getByRole('dialog')).toHaveCount(0);

    // Aide › Premier lancement: every station from the first, what is set in place; « Fermer » undoes nothing.
    await page.getByRole('button', { name: 'Aide', exact: true }).click();
    await page.getByRole('menuitem', { name: /Premier lancement/ }).click();
    const again = page.getByRole('region', { name: 'Premier lancement' });
    await expect(again).toBeVisible();
    await expect(page.getByRole('heading', { level: 1, name: 'Avec quel harness tes agents travaillent-ils ?' })).toBeVisible();
    const stops = page.getByRole('navigation', { name: 'Les stations du premier lancement' }).getByRole('listitem');
    await expect(stops.nth(0)).toContainText('maintenant');
    await expect(stops.nth(2)).toContainText('Demander');
    await stops.nth(2).getByRole('button').click();
    await expect(again.getByRole('radio', { name: /Demander/ })).toHaveAttribute('aria-checked', 'true');
    await again.getByRole('button', { name: 'Fermer' }).click();
    await expect(again).toBeHidden();
    expect(settings().accounts.map((a: { id: string }) => a.id)).toEqual(['principal']);
    expect(settings().agentMode).toBe('default');

    // And from Réglages › Général.
    const dialog = await openSettingsPage(page, 'Général');
    await dialog.getByRole('button', { name: 'Refaire le premier lancement' }).click();
    await expect(page.getByRole('region', { name: 'Premier lancement' })).toBeVisible();
    await page.getByRole('region', { name: 'Premier lancement' }).getByRole('button', { name: 'Fermer' }).click();
  });
});

test.describe('a first launch with no harness on the machine', () => {
  test.use({
    preStart: { run: (root: string) => ({ ...NEW_INSTALLATION, claudePath: path.join(root, 'nowhere', 'claude.cmd') }) },
  });

  test('offers the command that installs each harness, copied in one click, and is skipped whole', async ({ app }) => {
    let page = app.page;
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await expect(journey).toBeVisible();
    await expect(journey).toContainText('Aucun harness n’est installé.');
    const claude = journey.getByRole('radio', { name: /Claude Code/ });
    await expect(claude).toContainText('Introuvable');
    await expect(claude).toHaveAttribute('aria-checked', 'true');
    await expect(journey.getByRole('button', { name: /^Continuer/ })).toBeDisabled();
    await journey.getByRole('button', { name: 'Copier la commande de Claude Code' }).click();
    await expect(page.locator('.toast').filter({ hasText: 'Commande copiée' }).first()).toBeVisible();
    await expect(journey.getByRole('button', { name: 'Chercher à nouveau' })).toBeEnabled();
    await fitsAcross(page);
    // « Tout passer »: the journey ends, every station passed; the screen with no harness.
    await journey.getByRole('button', { name: 'Tout passer' }).click();
    await expect(journey).toBeHidden();
    await expect(page.getByRole('region', { name: 'Aucun harness trouvé' })).toBeVisible();
    const state = () => read(path.join(app.root, 'data', 'state.json'));
    await expect.poll(() => state().onboarding?.finished ?? null).not.toBeNull();
    expect(Object.values(state().onboarding.steps)).toEqual(['skipped', 'skipped', 'skipped', 'skipped', 'skipped', 'skipped', 'skipped']);
    // The statistics passed: asked once more at the next start, as the window of before, then never again.
    page = await app.restart();
    const ask = page.getByRole('dialog', { name: 'Aider à améliorer Escouade ?' });
    await expect(ask).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(ask).toBeHidden();
    await expect.poll(() => state().onboarding?.telemetryAskedAgain).toBe(true);
    page = await app.restart();
    await expect(page.getByRole('region', { name: 'Aucun harness trouvé' })).toBeVisible();
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(page.getByRole('region', { name: 'Premier lancement' })).toHaveCount(0);
  });
});

// Both themes, at the size of the run: every station whole in the window (its foot and main button in view, nothing
// across), drawn to the test's folder for the review.
for (const theme of ['dark', 'light'] as const) {
  test.describe(`the first launch in the ${theme} theme`, () => {
    test.use({
      preStart: {
        run: (root: string) => {
          const dir = path.join(root, 'claude');
          fs.mkdirSync(dir, { recursive: true });
          fs.writeFileSync(path.join(dir, '.credentials.json'), SIGNED_IN);
          fs.writeFileSync(path.join(dir, '.claude.json'), JSON.stringify({ oauthAccount: { emailAddress: 'ada@atlas.dev' } }));
          return { ...NEW_INSTALLATION, theme };
        },
      },
    });

    test('holds each station whole in the window', async ({ app }, testInfo) => {
      const page = app.page;
      const journey = page.getByRole('region', { name: 'Premier lancement' });
      await expect(journey).toBeVisible();
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      const size = page.viewportSize();
      const tag = `${theme}-${size?.width ?? 0}`;
      const whole = async (name: string) => {
        await fitsAcross(page);
        // Nor across the line of the stations.
        const line = page.getByRole('navigation', { name: 'Les stations du premier lancement' });
        expect(await line.evaluate((e) => e.scrollWidth - e.clientWidth)).toBeLessThanOrEqual(0);
        const button = journey.locator('.foot .btn.primary');
        if (await button.count()) {
          const box = await button.boundingBox();
          expect(box && box.y + box.height).toBeLessThanOrEqual(size?.height ?? 10_000);
        }
        await page.screenshot({ path: testInfo.outputPath(`onboarding-${name}-${tag}.png`) });
      };
      await expect(journey.getByRole('radio', { name: /Claude Code/ })).toContainText('Connecté · ada@atlas.dev');
      await whole('harness');
      await page.keyboard.press('Enter');
      await expect(page.getByRole('heading', { level: 1, name: 'Quand tes agents doivent-ils te demander ?' })).toBeVisible();
      await whole('modes');
      await page.keyboard.press('Enter');
      await expect(page.getByRole('heading', { level: 1, name: 'Que veux-tu brancher ?' })).toBeVisible();
      await whole('brancher');
      // A card ticked, its choices open: still whole in the window (the station scrolls inside, never across).
      await journey.getByRole('checkbox', { name: 'La voix' }).click();
      await journey.getByRole('radio', { name: /Whisper, sur ta machine/ }).click();
      await expect(journey.getByRole('button', { name: /^Télécharger/ })).toBeVisible();
      await whole('brancher-voix');
      await journey.getByRole('checkbox', { name: 'Une source de tickets' }).click();
      await whole('brancher-sources');
      await journey.getByRole('button', { name: /^Continuer/ }).click();
      await expect(page.getByRole('heading', { level: 1, name: 'Où travaille ton premier agent ?' })).toBeVisible();
      await whole('projet');
      await page.keyboard.press('Escape');
      await expect(page.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeVisible();
      await whole('agent');
      await page.keyboard.press('Escape');
      await expect(page.getByRole('heading', { level: 1, name: 'Aider à améliorer Escouade ?' })).toBeVisible();
      await whole('statistiques');
    });
  });
}
