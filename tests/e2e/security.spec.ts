import fs from 'node:fs';
import path from 'node:path';
import { expect, test } from './fixture';

// The audit's SEC-07 and SEC-09 in the real app: the band while « Ignorer la vérification des certificats TLS » is on, and
// the first start's question of the mode of new agents.

test.describe('certificates not checked through the proxy (SEC-07)', () => {
  test.use({ preStart: { run: () => ({ insecureTls: true, proxyUrl: 'http://ada:secret@127.0.0.1:9000' }) } });

  test('a band says so in every view, without the proxy’s password, and leads to the file of authorities', async ({ app }) => {
    const { page } = app;
    const band = page.getByTestId('tls-band');
    await expect(band).toHaveText(/Certificats TLS non vérifiés par le proxy 127\.0\.0\.1:9000/);
    await expect(band).not.toContainText('secret');
    await band.getByRole('button', { name: 'Choisir le certificat racine' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await expect(settings.getByRole('textbox', { name: 'Autorités de certification supplémentaires' })).toBeVisible();
    await expect(settings.getByText("Avec Zscaler ou un proxy d'entreprise : choisis son certificat racine")).toBeVisible();
  });
});

test.describe('the first start of a new installation (SEC-09)', () => {
  // Settings of this version not answered yet: as a new installation has them after its first start.
  test.use({ preStart: { run: () => ({ agentMode: '', allowBypass: false, telemetry: null }) } });

  test('asks the mode of new agents in the first launch, the recommended one chosen and Bypass off, then the statistics', async ({
    app,
  }) => {
    const { page } = app;
    // An account is there already: the first launch goes on at its starting settings (ESC-504).
    const journey = page.getByRole('region', { name: 'Premier lancement' });
    await expect(page.getByRole('heading', { level: 1, name: 'Quand tes agents doivent-ils te demander ?' })).toBeVisible();
    await expect(journey.getByRole('radio', { name: /Édits auto/ })).toHaveAttribute('aria-checked', 'true');
    // Bypass shown, never offered.
    await expect(journey.getByRole('radio', { name: /Bypass/ })).toHaveAttribute('aria-disabled', 'true');
    await journey.getByRole('radio', { name: /^Demander/ }).click();
    await journey.getByRole('button', { name: /Garder Demander/ }).click();
    const settings = () => JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
    await expect.poll(() => settings().agentMode).toBe('default');
    expect(settings().allowBypass).toBe(false);
    // « Ce que tu veux brancher » (ESC-512): nothing ticked, gone on with nothing plugged in, the statistics still unasked.
    await expect(page.getByRole('heading', { level: 1, name: 'Que veux-tu brancher ?' })).toBeVisible();
    for (const card of await journey.getByRole('group', { name: 'Les branchements' }).getByRole('checkbox').all())
      await expect(card).toHaveAttribute('aria-checked', 'false');
    await journey.getByRole('button', { name: /^Continuer/ }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Où travaille ton premier agent ?' })).toBeVisible();
    expect(settings().telemetry ?? null).toBeNull();
    // The first project passed, then the first agent (ESC-505), which has no project to work in: the statistics come next,
    // whatever was passed, still unanswered, so nothing was sent before the question.
    await journey.getByRole('button', { name: 'Plus tard' }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Ton premier message' })).toBeVisible();
    await expect(journey).toContainText('Ton premier agent a besoin d’un projet');
    await journey.getByRole('button', { name: 'Plus tard' }).click();
    // The features (F-2, ESC-559), passed: no switch changed.
    await expect(page.getByRole('heading', { level: 1, name: 'Que veux-tu voir autour de tes agents ?' })).toBeVisible();
    await journey.getByRole('button', { name: 'Passer : garder l’essentiel' }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'Aider à améliorer Escouade ?' })).toBeVisible();
    expect(settings().telemetry ?? null).toBeNull();
  });
});
