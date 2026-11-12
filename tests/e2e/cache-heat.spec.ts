import { addProject, closeSettings, expect, openAdvanced, openSettingsPage, send, test } from './fixture';

// The temperature of an agent's cache (ESC-398): on its card beside its model, in its conversation's header with the
// detail of its last turn on hover, what the cache saved today in the status bar; kept across a restart of the app.

test('an agent shows the temperature of its cache, its detail on hover, and keeps it after a restart', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  await expect(page.locator('footer')).toContainText('1 terminé');

  // The card: degrees near 100 just after the turn, hot, said in words.
  const card = page.locator('.card').first();
  const onCard = card.locator('.a-meta [data-heat]');
  await expect(onCard).toHaveText(/^(100|9\d) °C$/);
  await expect(onCard).toHaveAttribute('data-tone', 'hot');
  await expect(onCard).toHaveAttribute('aria-label', /^Cache : (100|9\d) °C, expire dans 4 min \d\d$/);
  // In the line of its model, not pushed out of a narrow card.
  const [heatBox, cardBox] = [(await onCard.boundingBox())!, (await card.boundingBox())!];
  expect(heatBox.x + heatBox.width).toBeLessThanOrEqual(cardBox.x + cardBox.width);
  const colour = await onCard.evaluate((el) => getComputedStyle(el).color);
  expect(colour).not.toBe(await card.locator('.a-meta').evaluate((el) => getComputedStyle(el).color));

  // The header: the same figure, and on hover the detail of the last turn.
  const fig = page.locator('header.head .fig-heat');
  await expect(fig).toBeVisible();
  await expect(fig.locator('dd')).toHaveText(/^(100|9\d) °C$/);
  await fig.hover();
  const pop = page.getByRole('tooltip').filter({ hasText: 'Cache :' });
  await expect(pop).toBeVisible();
  await expect(pop).toContainText('expire dans 4 min');
  await expect(pop).toContainText('vit 5 min sans activité');
  await expect(pop).toContainText('Lus depuis le cache (chauds)');
  await expect(pop).toContainText('Mis en cache (froids)');
  await expect(pop).toContainText('Nouveaux, plein tarif');
  await expect(pop).toContainText('Économisé sur ce tour');
  await expect(pop).toContainText('Si le prochain message part à froid');
  // Whole in the window.
  const box = (await pop.boundingBox())!;
  // The window's own size: `viewportSize()` is null when no size is emulated (the CI's real window).
  const view = await page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(view.width);
  await page.mouse.move(5, view.height / 2);
  await expect(pop).toBeHidden();

  // The status bar: what the cache saved today, hidden until « Économies du cache dans la barre d’état » is on (ESC-421).
  await expect(page.locator('footer')).toContainText('Aujourd');
  await expect(page.locator('footer [data-heat-today]')).toHaveCount(0);
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Comptes et harness');
  await openAdvanced(dialog);
  const savings = dialog.getByRole('switch', { name: 'Économies du cache dans la barre d’état' });
  await expect(savings).toHaveAttribute('aria-checked', 'false');
  await savings.click();
  await expect(savings).toHaveAttribute('aria-checked', 'true');
  await closeSettings(dialog);
  await expect(page.locator('footer [data-heat-today]')).toBeVisible();
  await expect(page.locator('footer [data-heat-today]')).toContainText(/≈\s*0,0\d+\s*\$/);

  // Restarted: the cache lives at its vendor, the figure is still warm.
  const page2 = await app.restart();
  const again = page2.locator('.card').first().locator('.a-meta [data-heat]');
  await expect(again).toHaveText(/^(100|9\d|8\d) °C$/);
  await expect(page2.locator('footer [data-heat-today]')).toBeVisible();
});
