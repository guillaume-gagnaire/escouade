// ESC-555: an image pasted in an agent's field, seen full size before it is sent. Its thumbnail opens the conversation's
// viewer (ESC-171) on it, the other image of the message next; Échap closes it and gives the focus back to the thumbnail,
// the text of the message as it was. ESCOUADE_E2E_SHOTS names a folder for the screenshots, in both themes, at the size of
// the window (ESCOUADE_E2E_VIEWPORT=1024x728 for the runner's).
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** The page, light then dark, into the folder of the screenshots. */
async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const size = await page.evaluate(() => `${window.innerWidth}x${window.innerHeight}`);
  for (const theme of ['light', 'dark']) {
    await page.evaluate((th) => (document.documentElement.dataset.theme = th), theme);
    await page.waitForTimeout(200);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${size}-${theme}.png`), animations: 'disabled' });
  }
  await page.evaluate(() => delete document.documentElement.dataset.theme);
}

test('an image pasted in the field opens the conversation’s viewer by its thumbnail, Échap closing it', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const field = page.getByRole('textbox').last();
  await expect(field).toBeVisible({ timeout: 20_000 });
  await field.fill('Que vois-tu sur ces captures ?');

  // Two screenshots, as WebView2 gives a paste: PNG files drawn on a canvas.
  await field.evaluate(async (el) => {
    const draw = (w: number, h: number, a: string, b: string) =>
      new Promise<Blob>((ok) => {
        const c = document.createElement('canvas');
        c.width = w;
        c.height = h;
        const g = c.getContext('2d')!;
        const grad = g.createLinearGradient(0, 0, w, h);
        grad.addColorStop(0, a);
        grad.addColorStop(1, b);
        g.fillStyle = grad;
        g.fillRect(0, 0, w, h);
        g.fillStyle = 'rgba(255,255,255,0.85)';
        g.fillRect(w * 0.1, h * 0.12, w * 0.5, h * 0.1);
        g.fillRect(w * 0.1, h * 0.3, w * 0.8, h * 0.5);
        c.toBlob((blob) => ok(blob!), 'image/png');
      });
    const data = new DataTransfer();
    data.items.add(new File([await draw(1280, 800, '#3b5bdb', '#9c36b5')], 'maquette.png', { type: 'image/png' }));
    data.items.add(new File([await draw(800, 600, '#e8590c', '#f59f00')], 'erreur.png', { type: 'image/png' }));
    el.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
  });

  const thumb = page.getByRole('button', { name: 'Agrandir maquette.png' });
  await expect(thumb).toBeVisible({ timeout: 10_000 });
  await expect(page.getByRole('button', { name: 'Agrandir erreur.png' })).toBeVisible();
  await shoot(page, 'composer-thumbs');

  await thumb.click();
  const viewer = page.getByRole('dialog', { name: 'Visionneuse d’images' });
  await expect(viewer).toBeVisible();
  await expect(viewer.locator('.stage img')).toHaveAttribute('alt', 'maquette.png');
  await expect(viewer).toContainText('1 sur 2');
  await expect.poll(() => viewer.locator('.stage img').evaluate((i) => (i as HTMLImageElement).naturalWidth)).toBe(1280);
  // Not sent yet: nothing to open nor to show in its folder.
  await expect(viewer.getByRole('button', { name: /dossier/ })).toHaveCount(0);
  await shoot(page, 'composer-viewer');
  await viewer.getByRole('button', { name: 'Image suivante' }).click();
  await expect(viewer.locator('.stage img')).toHaveAttribute('alt', 'erreur.png');

  await page.keyboard.press('Escape');
  await expect(viewer).toBeHidden();
  await expect(thumb).toBeFocused();
  await expect(field).toHaveValue('Que vois-tu sur ces captures ?');

  // « × » takes the image away without opening the viewer.
  await page.getByRole('button', { name: 'Retirer erreur.png' }).click();
  await expect(page.getByRole('button', { name: 'Agrandir erreur.png' })).toHaveCount(0);
  await expect(viewer).toBeHidden();
  // By the keyboard: Entrée on the thumbnail.
  await thumb.focus();
  await page.keyboard.press('Enter');
  await expect(viewer).toBeVisible();
  await expect(viewer).not.toContainText('sur');
  await page.keyboard.press('Escape');
  await expect(viewer).toBeHidden();
  await expect(thumb).toBeFocused();
});
