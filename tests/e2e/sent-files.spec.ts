// ESC-171: the files an agent sends (Claude Code's SendUserFile). The fake claude writes two PNG images and a PDF in a
// temporary folder of its own folder (`.impeccable/tmp/`, as a real agent did) and sends them, their paths with `/`. The
// conversation shows the caption, the images as thumbnails that open the viewer, the PDF with « Ouvrir » and « Afficher
// dans le dossier »; the images, copied when they were sent, show still once that folder is gone and after a restart.

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

/** The images of the card, drawn: their bytes read (a `data:` URL of the core) and decoded. */
async function drawn(page: Page): Promise<number[]> {
  return page.locator('[data-sent] .thumbs img').evaluateAll((imgs) => imgs.map((i) => (i as HTMLImageElement).naturalWidth));
}

test('the images and the file an agent sends show in its conversation, and the images outlive their folder', async ({ app }) => {
  let page = app.page;
  await addProject(page, app.repo);
  await send(page, 'envoie des fichiers');

  const card = page.locator('[data-sent]');
  await expect(card).toBeVisible({ timeout: 20_000 });
  await expect(card).toContainText('Envoyé par Claude');
  await expect(card).toContainText('2 images et 1 fichier');
  await expect(card.getByText('Les deux vues et le contrat.')).toBeVisible();
  // Not a row of the thread of tool calls.
  await expect(page.getByText('SendUserFile')).toHaveCount(0);

  const thumbs = card.getByRole('button', { name: /^Voir .* en grand$/ });
  await expect(thumbs).toHaveCount(2);
  await expect.poll(() => drawn(page)).toEqual([160, 120]);
  const pdf = card.locator('.row').filter({ hasText: 'contrat.pdf' });
  await expect(pdf).toContainText(/1\s+Ko/);
  await expect(pdf.getByRole('button', { name: 'Ouvrir contrat.pdf' })).toBeVisible();
  await expect(pdf.getByRole('button', { name: 'Afficher contrat.pdf dans le dossier' })).toBeVisible();

  // The card stays in the conversation's column, the window as small as the runner's.
  const fits = await page.evaluate(() => {
    const sent = document.querySelector('[data-sent]')!.getBoundingClientRect();
    const scroll = document.querySelector('.scroll')!.getBoundingClientRect();
    return sent.left >= scroll.left - 1 && sent.right <= scroll.right + 1;
  });
  expect(fits).toBe(true);

  // The viewer: the image clicked, the next one, Échap.
  await thumbs.nth(1).click();
  const viewer = page.getByRole('dialog', { name: 'Visionneuse d’images' });
  await expect(viewer).toBeVisible();
  await expect(viewer.getByRole('img')).toHaveAttribute('alt', 'vue-dessus.png');
  await expect(viewer).toContainText('2 sur 2');
  // Over the whole window, not held in the conversation's column.
  const covers = await viewer.evaluate((v) => {
    const r = v.getBoundingClientRect();
    return r.left === 0 && r.top === 0 && r.width === window.innerWidth && r.height === window.innerHeight;
  });
  expect(covers).toBe(true);
  await viewer.getByRole('button', { name: 'Image suivante' }).click();
  await expect(viewer.getByRole('img')).toHaveAttribute('alt', 'vue-face.png');
  await page.keyboard.press('Escape');
  await expect(viewer).toBeHidden();

  // An image of a message by a local path, relative to the agent's folder (the fake claude says back what it is told): read
  // by the core; one by http is its text alone.
  await send(page, 'regarde ![rendu](.impeccable/tmp/vue-face.png) et ![web](http://example.com/x.png)');
  const said = page.locator('.assistant .md').filter({ hasText: 'regarde' });
  await expect(said.locator('img[alt="rendu"]')).toBeVisible({ timeout: 20_000 });
  await expect.poll(() => said.locator('img[alt="rendu"]').evaluate((i) => (i as HTMLImageElement).naturalWidth)).toBe(160);
  await expect(said.locator('[data-img-off]')).toHaveText('web');
  await said.locator('img[alt="rendu"]').click();
  await expect(page.getByRole('dialog', { name: 'Visionneuse d’images' }).getByRole('img')).toHaveAttribute('alt', 'rendu');
  await page.keyboard.press('Escape');
  // One of the web is never loaded by itself: its label, and the core downloads it on « Afficher l’image » alone.
  await send(page, 'aussi ![plan](https://example.com/plan.png?d=secret)');
  const web = page.locator('.assistant .md').filter({ hasText: 'aussi' }).locator('[data-remote-src]');
  await expect(web).toContainText('plan · image de example.com');
  await expect(web.getByRole('button', { name: 'Afficher l’image' })).toBeVisible();
  await expect(page.locator('.assistant .md img[src^="https:"]')).toHaveCount(0);

  // The temporary folder goes, as it does with the agent's worktree: the images were copied, the PDF is said gone.
  const tmp = path.join(app.repo, '.impeccable', 'tmp');
  expect(fs.existsSync(path.join(tmp, 'vue-face.png'))).toBe(true);
  fs.rmSync(tmp, { recursive: true, force: true });
  page = await app.restart();
  const again = page.locator('[data-sent]');
  await expect(again).toBeVisible({ timeout: 20_000 });
  await expect.poll(() => drawn(page)).toEqual([160, 120]);
  await expect(again.locator('.row').filter({ hasText: 'contrat.pdf' })).toContainText('Fichier introuvable');
  await expect(again.getByRole('button', { name: 'Ouvrir contrat.pdf' })).toHaveCount(0);
});
