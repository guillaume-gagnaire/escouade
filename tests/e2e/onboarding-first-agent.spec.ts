import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { expect, test } from './fixture';

// ESC-505 (ON-2): the first project and the first agent of the first launch, in a data folder as a new installation leaves
// it, Claude Code (the fake `claude`) signed in on the machine. The demonstration project, written by the app without the
// network; its first agent, a message proposed and sent by the user's click alone; its answer live, its first permission
// explained beside the real card and granted, its diff shown; the demonstration deleted from its band. Then the same with
// a folder of the machine, after a clone that fails; and an account at its usage limit. Run at both sizes, the CI's
// runner's (ESCOUADE_E2E_VIEWPORT=1024x728) and the default one.

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
/** The demonstration's turn, as the fake `claude` plays it: the tests run after a permission, a file fixed. */
const TURNS = path.join(ROOT, 'tests', 'fixtures', 'onboarding-turns.json');

const SIGNED_IN = JSON.stringify({ claudeAiOauth: { accessToken: 'tok', refreshToken: 'ref', expiresAt: 4_102_444_800_000 } });
const NEW_INSTALLATION = { noAccounts: true, agentMode: '', allowBypass: false, telemetry: null };

/** The machine's Claude Code signed in, as a new installation finds it; `limit`: its account at its usage limit. */
function signedIn(limit = false, theme: 'dark' | 'light' = 'dark') {
  return {
    run: (root: string) => {
      const dir = path.join(root, 'claude');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, '.credentials.json'), SIGNED_IN);
      fs.writeFileSync(path.join(dir, '.claude.json'), JSON.stringify({ oauthAccount: { emailAddress: 'ada@atlas.dev' } }));
      if (limit) fs.writeFileSync(path.join(dir, 'fake-limit'), '');
      return { ...NEW_INSTALLATION, theme };
    },
  };
}

/** The messages the fake `claude` read on its stdin: none before the user's click. */
function sent(root: string): string[] {
  const file = path.join(root, 'fake-claude.stdin.jsonl');
  if (!fs.existsSync(file)) return [];
  return fs
    .readFileSync(file, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l))
    .filter((m) => m.type === 'user')
    .map((m) => {
      const c = m.message?.content;
      return typeof c === 'string' ? c : (c ?? []).map((p: { text?: string }) => p.text ?? '').join('');
    });
}

async function fitsAcross(page: Page) {
  const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(over).toBeLessThanOrEqual(0);
}

/**
 * The harness and the starting settings, each by Entrée on the recommended choice, then « Ce que tu veux brancher »
 * (ESC-512) gone on with nothing ticked: the station of the first project.
 */
async function toProject(page: Page) {
  await expect(page.getByRole('heading', { level: 1, name: 'Avec quel harness tes agents travaillent-ils ?' })).toBeFocused();
  // The machine's sign-in read: until then the harness is not chosen yet, and Entrée has nothing to go on with.
  const claude = page.getByRole('region', { name: 'Premier lancement' }).getByRole('radio', { name: /Claude Code/ });
  await expect(claude).toContainText('Connecté · ada@atlas.dev');
  await expect(claude).toHaveAttribute('aria-checked', 'true');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { level: 1, name: 'Quand tes agents doivent-ils te demander ?' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { level: 1, name: 'Que veux-tu brancher ?' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { level: 1, name: 'Où travaille ton premier agent ?' })).toBeVisible();
}

// In both themes: the stations drawn to the test's folder for the review.
for (const theme of ['dark', 'light'] as const)
  test.describe(`the first agent on the demonstration project, ${theme} theme`, () => {
    test.use({ preStart: signedIn(false, theme), appEnv: { FAKE_CLAUDE_DEMO: TURNS } });

    test('makes the demo offline, proposes the first message, sends it at the click, explains the permission and shows the diff', async ({
      app,
    }, testInfo) => {
      const page = app.page;
      const tag = `${theme}-${page.viewportSize()?.width ?? 0}`;
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      const journey = page.getByRole('region', { name: 'Premier lancement' });
      await toProject(page);
      // What a project, an agent, a worktree and a branch are, in a line each.
      await expect(journey).toContainText('Un projet : un dossier, avec ses agents, son Kanban et ses docs.');
      await expect(journey).toContainText('Un agent : une conversation avec ton harness, qui lit et modifie le code.');
      await expect(journey).toContainText('Un worktree : une copie de travail à l’agent seul : deux agents ne se gênent pas.');
      await expect(journey).toContainText('Une branche par agent : son travail reste à part jusqu’à ce que tu l’intègres.');
      await expect(journey.getByRole('radio', { name: /Un dossier de ta machine/ })).toHaveAttribute('aria-checked', 'true');
      await fitsAcross(page);
      await page.screenshot({ path: testInfo.outputPath(`onboarding-projet-${tag}.png`) });
      // The demonstration: where it is written, then made by the app, a repository of its own.
      await journey.getByRole('radio', { name: /Un projet de démonstration/ }).click();
      await expect(journey).toContainText('Rien ne sort de ta machine.');
      await journey.getByRole('button', { name: /Créer la démonstration/ }).click();
      const demo = path.join(app.root, 'data', 'demo', 'bac-a-sable');
      await expect(page.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeVisible();
      expect(fs.existsSync(path.join(demo, '.git', 'HEAD'))).toBe(true);
      expect(fs.readFileSync(path.join(demo, 'src', 'todo.ts'), 'utf8')).toContain('export class TodoList');
      // The first agent, its message proposed in its real composer: its tests can run (Node is there), so they come first.
      await expect(journey).toContainText('Il part sur ton compte Claude Code et compte dans son quota.');
      const frame = journey.getByTestId('onboarding-frame');
      const composer = frame.getByRole('textbox');
      await expect(composer).toHaveValue('Lance les tests et dis-moi ce qui casse.');
      const starters = journey.getByRole('radiogroup', { name: 'Les messages proposés' }).getByRole('radio');
      await expect(starters).toHaveCount(3);
      await starters.nth(1).click();
      await expect(composer).toHaveValue('Explique-moi ce dépôt en cinq lignes.');
      await starters.nth(0).click();
      await expect(composer).toHaveValue('Lance les tests et dis-moi ce qui casse.');
      await expect(journey.getByRole('button', { name: /^Continuer/ })).toBeDisabled();
      // The microphone of the framed composer hovered, then a key typed: no bubble of the discovery (ESC-506) over the
      // journey, nor while the first message is being typed.
      await frame.getByRole('button', { name: /^(Dicter|Finir la dictée)$/ }).hover();
      await page.waitForTimeout(1200);
      await expect(page.getByTestId('discover-tip')).toHaveCount(0);
      await composer.press('End');
      await composer.press('Space');
      await composer.press('Backspace');
      await expect(composer).toHaveValue('Lance les tests et dis-moi ce qui casse.');
      await expect(page.getByTestId('discover-tip')).toHaveCount(0);
      await fitsAcross(page);
      await page.screenshot({ path: testInfo.outputPath(`onboarding-agent-message-${tag}.png`) });
      // Nothing sent before the click.
      await page.waitForTimeout(500);
      expect(sent(app.root)).toEqual([]);
      await frame.getByRole('button', { name: 'Envoyer' }).click();
      // Gone from the composer at once; then it waits for the agent's worktree (made at the first message) before the
      // harness gets it: the state waited for is its arrival at the fake `claude`, however long the preparation takes.
      await expect(page.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeHidden();
      await expect.poll(() => sent(app.root).length, { timeout: 60_000 }).toBe(1);
      // Its answer live, then its first permission, explained beside the real card.
      await expect(page.getByRole('heading', { level: 1, name: 'Ton agent te demande la permission' })).toBeVisible();
      await expect(frame).toContainText('Je lance les tests pour voir ce qui casse');
      await expect(journey).toContainText(
        'En Édits auto, chaque commande te demande : Autoriser cette fois, Toujours pour cette commande dans ce projet, ou Refuser',
      );
      await expect(journey).toContainText('Au clavier : Ctrl+Entrée autorise, Ctrl+Maj+Entrée autorise toujours.');
      await fitsAcross(page);
      await page.screenshot({ path: testInfo.outputPath(`onboarding-agent-permission-${tag}.png`) });
      await frame
        .getByRole('button', { name: /^Autoriser/ })
        .first()
        .click();
      // Done: in its worktree, on its branch; the demo's own checkout untouched; its diff.
      await expect(page.getByRole('heading', { level: 1, name: 'Ton agent a répondu' })).toBeVisible({ timeout: 30_000 });
      await expect(frame).toContainText('Corrigé dans');
      await expect(journey).toContainText('ton dépôt n’a pas bougé');
      await expect(journey).toContainText('1 fichier changé');
      expect(fs.readFileSync(path.join(demo, 'src', 'todo.ts'), 'utf8')).toContain('filter((t) => t.done)');
      await page.screenshot({ path: testInfo.outputPath(`onboarding-agent-fini-${tag}.png`) });
      await journey.getByTestId('onboarding-diff').click();
      const diff = page.getByRole('dialog', { name: /Ce que .* a changé/ });
      await expect(diff).toBeVisible();
      await expect(diff).toContainText('src/todo.ts');
      await expect(diff).toContainText('filter((t) => !t.done)');
      await page.keyboard.press('Escape');
      await expect(diff).toBeHidden();
      // On: the statistics, the end, the app on the demonstration, its band; deleted from it, its folder gone.
      await journey.getByRole('button', { name: /^Continuer/ }).click();
      await expect(page.getByRole('heading', { level: 1, name: 'Aider à améliorer Escouade ?' })).toBeVisible();
      const line = page.getByRole('navigation', { name: 'Les stations du premier lancement' }).getByRole('listitem');
      await expect(line.nth(4)).toContainText('bac-a-sable');
      await journey.getByRole('button', { name: 'Non merci' }).click();
      // The end leads on to the map « À découvrir » (ESC-506): nothing of the first launch, the demo nor its agent ticked
      // a discovery by itself, and no bubble comes over it.
      await expect(journey).toContainText('La carte t’attend dans Aide › À découvrir.');
      await journey.getByRole('button', { name: 'Voir ce qu’il y a à découvrir' }).click();
      await expect(journey).toBeHidden();
      await expect(page.getByRole('heading', { level: 1, name: 'À découvrir' })).toBeVisible();
      await expect(page.getByText(/^0 sur \d+ · /)).toBeVisible();
      await expect(page.getByTestId('discover-tip')).toHaveCount(0);
      await fitsAcross(page);
      await page.screenshot({ path: testInfo.outputPath(`onboarding-a-decouvrir-${tag}.png`) });
      await page.getByRole('button', { name: 'Retour au projet' }).click();
      const band = page.getByTestId('demo-band');
      await expect(band).toContainText('Bac à sable · un dépôt de démonstration sur ta machine');
      await expect(band).toContainText('rien ne sort d’ici');
      await fitsAcross(page);
      await page.screenshot({ path: testInfo.outputPath(`onboarding-bac-a-sable-${tag}.png`) });
      await band.getByRole('button', { name: 'Supprimer' }).click();
      const confirm = page.getByRole('dialog', { name: 'Supprimer le projet de démonstration ?' });
      await confirm.getByRole('button', { name: 'Supprimer' }).click();
      await expect(band).toBeHidden();
      await expect.poll(() => fs.existsSync(demo), { timeout: 20_000 }).toBe(false);
      await expect(page.getByRole('heading', { name: 'Un premier projet' })).toBeVisible();
    });
  });

test.describe('the first agent in a folder of the machine', () => {
  test.use({ preStart: signedIn() });

  test('says a clone that fails and lets pick a folder, then the agent answers at the click and changes nothing', async ({ app }) => {
    const page = app.page;
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await toProject(page);
    // A clone that fails: why, what to do, and the way to try again; nothing left behind.
    await journey.getByRole('radio', { name: /Cloner un dépôt/ }).click();
    await journey.getByRole('combobox', { name: 'Adresse du dépôt' }).fill('https://127.0.0.1:9/acme/api.git');
    await journey.getByRole('textbox', { name: 'Dans le dossier' }).fill(app.root);
    await expect(journey.getByRole('textbox', { name: 'Nom du dossier' })).toHaveValue('api');
    await journey.getByRole('button', { name: /Cloner et ouvrir/ }).click();
    await expect(journey.getByTestId('onboarding-clone-failed')).toBeVisible({ timeout: 60_000 });
    await expect(journey.getByTestId('onboarding-clone-failed')).toContainText(
      /Vérifie l’adresse et ton accès, puis réessaie|Cloner demande le réseau/,
    );
    await expect(journey.getByRole('button', { name: /Réessayer/ })).toBeEnabled();
    expect(fs.existsSync(path.join(app.root, 'api'))).toBe(false);
    await fitsAcross(page);
    // A folder of the machine: the repository of the test.
    await journey.getByRole('radio', { name: /Un dossier de ta machine/ }).click();
    await journey.getByRole('textbox', { name: 'Le dossier du projet' }).fill(app.repo);
    await expect(journey.getByRole('button', { name: /Ouvrir le projet/ })).toBeEnabled();
    await journey.getByRole('button', { name: /Ouvrir le projet/ }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeVisible();
    const frame = journey.getByTestId('onboarding-frame');
    const composer = frame.getByRole('textbox');
    // No test command there: « Lance les tests » is not proposed.
    const starters = journey.getByRole('radiogroup', { name: 'Les messages proposés' }).getByRole('radio');
    await expect(starters).toHaveCount(2);
    await expect(composer).toHaveValue('Explique-moi ce dépôt en cinq lignes.');
    // Edited before it goes, then sent by the click.
    await composer.fill('Explique-moi ce dépôt en trois lignes.');
    await page.waitForTimeout(500);
    expect(sent(app.root)).toEqual([]);
    await frame.getByRole('button', { name: 'Envoyer' }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeHidden();
    await expect.poll(() => sent(app.root).length, { timeout: 60_000 }).toBe(1);
    await expect(page.getByRole('heading', { level: 1, name: 'Ton agent a répondu' })).toBeVisible({ timeout: 30_000 });
    expect(sent(app.root)[0]).toContain('Explique-moi ce dépôt en trois lignes.');
    await expect(frame).toContainText('Bonjour, tu as dit');
    await expect(journey).toContainText('Il n’a rien changé : il n’y a pas de diff à voir.');
    await fitsAcross(page);
    await journey.getByRole('button', { name: /^Continuer/ }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Aider à améliorer Escouade ?' })).toBeVisible();
    const state = JSON.parse(fs.readFileSync(path.join(app.root, 'data', 'state.json'), 'utf8'));
    await expect
      .poll(() => JSON.parse(fs.readFileSync(path.join(app.root, 'data', 'state.json'), 'utf8')).onboarding?.steps?.agent ?? null)
      .toBe('done');
    expect(state.onboarding.steps.project).toBe('done');
  });
});

test.describe('the first agent on an account at its usage limit', () => {
  test.use({ preStart: signedIn(true) });

  test('says the limit, offers another account, and lets go on later', async ({ app }) => {
    const page = app.page;
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await toProject(page);
    await journey.getByRole('textbox', { name: 'Le dossier du projet' }).fill(app.repo);
    await journey.getByRole('button', { name: /Ouvrir le projet/ }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeVisible();
    const frame = journey.getByTestId('onboarding-frame');
    await expect(frame.getByRole('textbox')).toHaveValue('Explique-moi ce dépôt en cinq lignes.');
    await frame.getByRole('button', { name: 'Envoyer' }).click();
    await expect.poll(() => sent(app.root).length, { timeout: 60_000 }).toBe(1);
    await expect(page.getByRole('heading', { level: 1, name: 'Ce compte est à sa limite' })).toBeVisible({ timeout: 30_000 });
    await expect(journey.getByRole('alert')).toContainText('Ajoute un autre compte, ou reviens après.');
    await expect(journey.getByRole('button', { name: 'Ajouter un compte…' })).toBeVisible();
    await expect(journey.getByRole('button', { name: /^Continuer/ })).toBeDisabled();
    await fitsAcross(page);
    // « Plus tard »: the station waits in Aide › Premier lancement, the journey goes on.
    await journey.getByRole('button', { name: 'Plus tard' }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Aider à améliorer Escouade ?' })).toBeVisible();
    const line = page.getByRole('navigation', { name: 'Les stations du premier lancement' }).getByRole('listitem');
    await expect(line.nth(5)).toContainText('passé · à reprendre');
  });
});
