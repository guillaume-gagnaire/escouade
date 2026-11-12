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

  test('asks the mode of new agents first, the recommended one chosen, then the statistics', async ({ app }) => {
    const { page } = app;
    const dialog = page.getByRole('dialog', { name: 'Comment tes agents demandent-ils ?' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole('radio', { name: /Édits auto/ })).toHaveAttribute('aria-checked', 'true');
    await expect(dialog.getByRole('radio', { name: /Bypass/ })).toHaveCount(0);
    await dialog.getByRole('radio', { name: /^Demander/ }).click();
    await dialog.getByRole('button', { name: 'Choisir' }).click();
    await expect(page.getByRole('dialog', { name: 'Aider à améliorer Escouade ?' })).toBeVisible();
    const settings = () => JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
    await expect.poll(() => settings().agentMode).toBe('default');
    expect(settings().allowBypass).toBe(false);
  });
});
