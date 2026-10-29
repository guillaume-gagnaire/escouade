import { expect, test, type Page } from '@playwright/test';

// The store's frame and home page, in both languages and both themes, measured against the validated preview
// (design/store/index.html, #home): the same tokens, the same header, the same title.
const PREVIEW = new URL('../../../design/store/index.html', import.meta.url).href;

/** What the eye compares between the store and the preview. */
async function measure(page: Page) {
  return page.evaluate(() => {
    const css = (sel: string, prop: string) => {
      const el = document.querySelector(sel);
      return el ? getComputedStyle(el).getPropertyValue(prop) : null;
    };
    return {
      bg: getComputedStyle(document.body).backgroundColor || getComputedStyle(document.documentElement).backgroundColor,
      html: getComputedStyle(document.documentElement).backgroundColor,
      text: getComputedStyle(document.body).color,
      font: getComputedStyle(document.body).fontSize,
      header: (document.querySelector('header.top') as HTMLElement | null)?.getBoundingClientRect().height ?? null,
      title: css('.hero h1', 'font-size'),
      titleWeight: css('.hero h1', 'font-weight'),
      more: css('.hero h1 .more', 'color'),
      seal: css('.seal', 'color'),
      sealBorder: css('.seal', 'border-top-color'),
    };
  });
}

for (const scheme of ['dark', 'light'] as const) {
  test(`the home page looks like the preview (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`${PREVIEW}${scheme === 'light' ? '?theme=light' : ''}#home`);
    await expect(page.locator('.hero h1')).toBeVisible();
    const preview = await measure(page);

    await page.goto('/');
    await expect(page.locator('.hero h1')).toBeVisible();
    const store = await measure(page);

    expect(store.html).toBe(preview.html);
    expect(store.text).toBe(preview.text);
    expect(store.font).toBe(preview.font);
    expect(store.header).toBe(preview.header);
    expect(store.title).toBe(preview.title);
    expect(store.titleWeight).toBe(preview.titleWeight);
    expect(store.more).toBe(preview.more);
    expect(store.seal).toBe(preview.seal);
    expect(store.sealBorder).toBe(preview.sealBorder);
  });
}

test('the French home page links to the legal pages and to English', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('lang', 'fr');
  await expect(page.locator('h1')).toContainText('Des plugins pour Escouade.');
  const foot = page.locator('footer');
  await expect(foot.getByRole('link', { name: 'Confidentialité' })).toHaveAttribute('href', 'https://escouade.dev/privacy/');
  await expect(foot.getByRole('link', { name: 'Mentions légales' })).toHaveAttribute('href', 'https://escouade.dev/legal/');
  await foot.getByRole('link', { name: 'English' }).click();
  await expect(page).toHaveURL(/\/en$/);
  await expect(page.locator('h1')).toContainText('Plugins for Escouade.');
});

test('the English home page links to the English legal pages and back to French', async ({ page }) => {
  await page.goto('/en');
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  const foot = page.locator('footer');
  await expect(foot.getByRole('link', { name: 'Privacy' })).toHaveAttribute('href', 'https://escouade.dev/en/privacy/');
  await expect(foot.getByRole('link', { name: 'Legal notice' })).toHaveAttribute('href', 'https://escouade.dev/en/legal/');
  await expect(page.locator('header').getByRole('link', { name: 'Documentation' })).toHaveAttribute(
    'href',
    'https://escouade.dev/en/docs/',
  );
  await foot.getByRole('link', { name: 'Français' }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'fr');
});

test('a missing page says so in its language, and answers 404', async ({ page }) => {
  const res = await page.goto('/en/nowhere');
  expect(res?.status()).toBe(404);
  await expect(page.locator('h1')).toHaveText('This page does not exist.');
  await expect(page.getByRole('link', { name: 'Back to the home page' })).toBeVisible();
});

test('a phone shows the page without scrolling sideways', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  const wide = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(wide).toBeLessThanOrEqual(390);
  await expect(page.locator('header .bar')).toHaveCSS('height', '56px');
});

test('/api/health says how the service is, without a secret', async ({ request }) => {
  const res = await request.get('/api/health');
  const body = await res.json();
  expect(Object.keys(body.checks).sort()).toEqual(['database', 'disk', 'queue']);
  expect(res.headers()['cache-control']).toBe('no-store');
  if (process.env.DATABASE_URL) {
    // The migrations are played and the queue's first beat sent at start; give them the time to land.
    await expect
      .poll(async () => (await (await request.get('/api/health')).json()).checks, { timeout: 15_000 })
      .toMatchObject({ database: 'ok', queue: 'ok' });
    // The disk is the machine's: 200 when it has room, 503 when it is full, as Coolify must see it.
    const now = await request.get('/api/health');
    const { checks } = await now.json();
    expect(now.status()).toBe(checks.disk.startsWith('ok') ? 200 : 503);
  } else {
    // Without a database the pages are served, and the health says what is missing.
    expect(res.status()).toBe(503);
    expect(body.checks.database).toBe('missing');
  }
});
