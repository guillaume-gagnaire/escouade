import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { expect, test } from './fixture';

// ESC-507 (ON-4): the first launch from one end to the other, in a data folder as a new installation leaves it, Claude
// Code (the fake `claude`) signed in on the machine: the harness, its account, the starting settings, nothing plugged in,
// the demonstration project, a first agent that answers, the app started again half-way and taken up where it was, the
// statistics, a discovery done, then « Refaire le premier lancement ». The time to the first agent's answer is measured
// and said (an annotation of the test, and its output). Then its variants: a device set up by a version of before (no
// journey imposed, the dot on « Aide » alone), and a managed device (one harness allowed, Bypass forbidden). The one with
// no harness is onboarding.spec.ts'. Run at both sizes, the CI's runner's (ESCOUADE_E2E_VIEWPORT=1024x728) and the
// default one.

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const TURNS = path.join(ROOT, 'tests', 'fixtures', 'onboarding-turns.json');
const SIGNED_IN = JSON.stringify({ claudeAiOauth: { accessToken: 'tok', refreshToken: 'ref', expiresAt: 4_102_444_800_000 } });
const NEW_INSTALLATION = { noAccounts: true, agentMode: '', allowBypass: false, telemetry: null };

const read = (file: string) => JSON.parse(fs.readFileSync(file, 'utf8'));

/** The machine's Claude Code signed in, as a new installation finds it. */
function signIn(root: string) {
  const dir = path.join(root, 'claude');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, '.credentials.json'), SIGNED_IN);
  fs.writeFileSync(path.join(dir, '.claude.json'), JSON.stringify({ oauthAccount: { emailAddress: 'ada@atlas.dev' } }));
}

async function fitsAcross(page: Page) {
  const over = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(over).toBeLessThanOrEqual(0);
}

const heading = (page: Page, name: string) => page.getByRole('heading', { level: 1, name });

test.describe('a first launch from one end to the other', () => {
  test.use({
    preStart: {
      run: (root: string) => {
        signIn(root);
        return NEW_INSTALLATION;
      },
    },
    appEnv: { FAKE_CLAUDE_DEMO: TURNS },
  });

  test('reaches a first agent that answers, is taken up after a restart, keeps its funnel, leads to a discovery, and is done again', async ({
    app,
  }, testInfo) => {
    test.setTimeout(300_000);
    let page = app.page;
    const state = () => read(path.join(app.data, 'state.json'));
    const journey = () => page.getByRole('region', { name: 'Premier lancement' });
    // The clock starts with the first station on screen: what the user sees first.
    await expect(heading(page, 'Avec quel harness tes agents travaillent-ils ?')).toBeFocused();
    const began = Date.now();
    const claude = journey().getByRole('radio', { name: /Claude Code/ });
    await expect(claude).toContainText('Connecté · ada@atlas.dev');
    await expect(claude).toHaveAttribute('aria-checked', 'true');
    await fitsAcross(page);
    // Each station by Entrée on the recommended choice: the harness (its account folded in), the mode, nothing plugged in.
    await page.keyboard.press('Enter');
    await expect(heading(page, 'Quand tes agents doivent-ils te demander ?')).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(heading(page, 'Que veux-tu brancher ?')).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(heading(page, 'Où travaille ton premier agent ?')).toBeVisible();
    // The demonstration: one choice and its button, no folder to type.
    await journey()
      .getByRole('radio', { name: /Un projet de démonstration/ })
      .click();
    await journey()
      .getByRole('button', { name: /Créer la démonstration/ })
      .click();
    await expect(heading(page, 'Ton premier message')).toBeVisible();
    const frame = journey().getByTestId('onboarding-frame');
    await expect(frame.getByRole('textbox')).toHaveValue('Lance les tests et dis-moi ce qui casse.');
    await frame.getByRole('button', { name: 'Envoyer' }).click();
    const sentAt = Date.now();
    await expect(heading(page, 'Ton agent te demande la permission')).toBeVisible({ timeout: 60_000 });
    await frame
      .getByRole('button', { name: /^Autoriser/ })
      .first()
      .click();
    await expect(heading(page, 'Ton agent a répondu')).toBeVisible({ timeout: 60_000 });
    const replied = Date.now();
    const total = Math.round((replied - began) / 100) / 10;
    const turn = Math.round((replied - sentAt) / 100) / 10;
    testInfo.annotations.push({ type: 'time to the first agent', description: `${total} s (its turn: ${turn} s)` });
    console.log(
      `[ESC-507] ${await page.evaluate(() => window.innerWidth)}px: first agent answered ${total} s after the first station (its turn: ${turn} s)`,
    );
    // The design's goal for who has a harness signed in: under two minutes (docs/ONBOARDING.md § 3, P1).
    expect(total).toBeLessThan(120);
    // The first reply kept for the funnel, in the demonstration.
    await expect.poll(() => state().onboarding?.firstReplyAt ?? null).not.toBeNull();
    expect(state().onboarding.firstReplyDemo).toBe(true);

    // The app closed before the last station: started again, it takes up the journey there.
    await journey()
      .getByRole('button', { name: /^Continuer/ })
      .click();
    // The features (F-2, ESC-559): passed, what is used kept.
    await expect(heading(page, 'Que veux-tu voir autour de tes agents ?')).toBeVisible();
    await journey().getByRole('button', { name: 'Passer : garder l’essentiel' }).click();
    await expect(heading(page, 'Aider à améliorer Escouade ?')).toBeVisible();
    page = await app.restart();
    await expect(page.getByRole('region', { name: 'Premier lancement' })).toBeVisible();
    await expect(heading(page, 'Aider à améliorer Escouade ?')).toBeVisible();
    const line = page.getByRole('navigation', { name: 'Les stations du premier lancement' }).getByRole('listitem');
    await expect(line.nth(4)).toContainText('bac-a-sable');
    await expect(line.nth(7)).toContainText('maintenant');
    await journey().getByRole('button', { name: 'Non merci' }).click();
    await expect(journey()).toContainText('Escouade est prêt');

    // What the funnel keeps, and nothing else: what was found, the choices kept or not, never a name or a path. With a no,
    // it is dropped (told, never sent).
    await expect.poll(() => state().onboarding?.told ?? false).toBe(true);
    const o = state().onboarding;
    expect(o.profile).toBe('ready');
    expect(o.choices).toEqual({ harness: 'recommended', defaults: 'recommended', project: 'other' });
    expect(o.steps).toEqual({
      harness: 'done',
      account: 'done',
      defaults: 'done',
      // Gone through by Entrée, nothing ticked: the station done, its cards passed.
      hookups: 'done',
      project: 'done',
      agent: 'done',
      features: 'skipped',
      telemetry: 'done',
    });
    expect(JSON.stringify({ p: o.profile, c: o.choices })).not.toMatch(/bac-a-sable|ada@|[\\/]/);

    // On to the map: a discovery done for real (a terminal), ticked by itself.
    await journey().getByRole('button', { name: 'Voir ce qu’il y a à découvrir' }).click();
    await expect(heading(page, 'À découvrir')).toBeVisible();
    await expect(page.getByText(/^0 sur \d+ · /)).toBeVisible();
    await fitsAcross(page);
    await page.locator('[data-stop="terminal"]').click();
    await page.getByTestId('discover-detail').getByRole('button', { name: 'Essayer' }).click();
    const tip = page.locator('[data-testid="discover-tip"][data-tip="terminal"]');
    await expect(tip).toBeVisible();
    await tip.getByRole('button', { name: 'Compris' }).click();
    await page.getByRole('button', { name: 'Nouveau terminal' }).click();
    await page.getByRole('menuitem').first().click();
    await expect(page.locator('.xterm')).toBeVisible();
    await expect.poll(() => state().onboarding?.discover?.terminal ?? null).toBe('done');

    // « Refaire le premier lancement », from Aide: every station from the first, a funnel of its own; nothing undone.
    await page.getByRole('button', { name: 'Aide', exact: true }).click();
    await page.getByRole('menuitem', { name: /Premier lancement/ }).click();
    await expect(page.getByRole('region', { name: 'Premier lancement' })).toBeVisible();
    await expect(heading(page, 'Avec quel harness tes agents travaillent-ils ?')).toBeVisible();
    await expect.poll(() => state().onboarding?.told ?? true).toBe(false);
    await journey().getByRole('button', { name: 'Fermer' }).click();
    await expect(page.getByRole('region', { name: 'Premier lancement' })).toBeHidden();
    await expect.poll(() => state().onboarding?.told ?? false).toBe(true);
    expect(state().onboarding.runStarted).toBeGreaterThan(o.runStarted);
    expect(read(path.join(app.data, 'settings.json')).accounts.map((a: { id: string }) => a.id)).toEqual(['principal']);
  });
});

test.describe('a device set up by a version of before', () => {
  test.use({
    // An account, a mode and the statistics answered, as a 2.2 leaves them; its project is added below.
    preStart: { run: () => ({ agentMode: 'acceptEdits', telemetry: false }) },
  });

  test('imposes no journey nor question, and invites to discover by the dot on « Aide » alone', async ({ app }) => {
    const { page } = app;
    // Started on a device with an account and no project: not a new installation, the journey never opens by itself.
    await expect(page.getByRole('heading', { name: 'Un premier projet' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Premier lancement' })).toHaveCount(0);
    await expect(page.getByRole('dialog')).toHaveCount(0);
    // The invitation is discreet: a dot on « Aide », said to a screen reader; nothing opens by itself. (The band
    // « Nouveau dans la 2.3 » comes with a 2.3 build, docs/ONBOARDING.md § 6: this one is not.)
    await expect(page.getByTestId('help-dot')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Aide', exact: true })).toHaveAccessibleDescription(/Il reste des choses à découvrir/);
    await expect(page.getByTestId('discover-tip')).toHaveCount(0);
    await expect(page.getByRole('heading', { level: 1, name: 'À découvrir' })).toHaveCount(0);
    // Never started on this device: nothing of the journey kept (state.json may not even be written yet).
    const file = path.join(app.data, 'state.json');
    expect(fs.existsSync(file) ? (read(file).onboarding?.started ?? null) : null).toBeNull();
    await fitsAcross(page);
  });
});

test.describe('a managed device: one harness allowed, Bypass forbidden', () => {
  test.use({
    appEnv: async ({}, use) => {
      const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-managed-'));
      const file = path.join(dir, 'policies.json');
      fs.writeFileSync(file, JSON.stringify({ 'harnesses.allowed': ['claude'], 'permissions.allowBypass': false }));
      await use({ ESCOUADE_TEST_POLICIES: file });
      fs.rmSync(dir, { recursive: true, force: true });
    },
    preStart: {
      run: (root: string) => {
        signIn(root);
        return NEW_INSTALLATION;
      },
    },
  });

  test('offers the harness allowed alone, says Bypass is forbidden, and tells a managed profile', async ({ app }) => {
    const { page } = app;
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await expect(heading(page, 'Avec quel harness tes agents travaillent-ils ?')).toBeFocused();
    await expect(journey).toContainText('Claude Code est le seul harness permis sur ce poste.');
    // The others greyed, with why, never recommended.
    const denied = journey.getByRole('radio', { name: /non permis par la stratégie de ce poste/ });
    expect(await denied.count()).toBeGreaterThan(0);
    for (const r of await denied.all()) {
      await expect(r).toHaveAttribute('aria-disabled', 'true');
      await expect(r).not.toContainText('Recommandé');
    }
    await expect(journey.getByRole('radio', { name: /Claude Code/ })).toHaveAttribute('aria-checked', 'true');
    await fitsAcross(page);
    await page.keyboard.press('Enter');
    await expect(heading(page, 'Quand tes agents doivent-ils te demander ?')).toBeFocused();
    const bypass = journey.getByRole('radio', { name: /Bypass/ });
    await expect(bypass).toHaveAttribute('aria-disabled', 'true');
    await expect(bypass).toContainText('Imposé par la stratégie de ce poste');
    await fitsAcross(page);
    // ↓ from the question never lands on Bypass: it is not takeable.
    await page.keyboard.press('ArrowDown');
    for (let i = 0; i < 5; i++) await page.keyboard.press('ArrowDown');
    await expect(bypass).toHaveAttribute('aria-checked', 'false');
    await journey.getByRole('button', { name: 'Tout passer' }).click();
    await expect(journey).toBeHidden();
    await expect.poll(() => read(path.join(app.data, 'state.json')).onboarding?.profile ?? null).toBe('managed');
    expect(read(path.join(app.data, 'settings.json')).allowBypass ?? false).toBe(false);
  });
});
