import { expect, test } from '@playwright/test';

// The security headers on every answer, the pages' strict policy (Nuxt's scripts run by their nonce, nothing else
// inline), and the authors' terms linked from every page.

test('every answer carries the security headers', async ({ request }) => {
  for (const path of ['/', '/en/account', '/terms', '/api/health', '/api/me', '/nowhere']) {
    const h = (await request.get(path)).headers();
    expect(h['strict-transport-security'], path).toBe('max-age=31536000; includeSubDomains');
    expect(h['x-content-type-options'], path).toBe('nosniff');
    expect(h['referrer-policy'], path).toBe('strict-origin-when-cross-origin');
    expect(h['permissions-policy'], path).toContain('camera=()');
    expect(h['x-frame-options'], path).toBe('DENY');
    expect(h['content-security-policy'], path).toContain("frame-ancestors 'none'");
  }
  expect((await request.get('/api/health')).headers()['content-security-policy']).toContain("default-src 'none'");
});

test('a page’s policy allows its own scripts by a fresh nonce, and the page runs without a violation', async ({ page }) => {
  const violations: string[] = [];
  page.on('console', (m) => {
    if (/Content Security Policy|Refused to/i.test(m.text())) violations.push(m.text());
  });
  const first = await page.goto('/');
  const csp = first!.headers()['content-security-policy']!;
  const nonce = csp.match(/'nonce-([^']+)'/)?.[1];
  expect(nonce).toBeTruthy();
  expect(csp).not.toContain('unsafe-inline');
  const html = await first!.text();
  // Every inline script carries the nonce of this answer.
  for (const tag of html.match(/<script(?![^>]*\bsrc=)[^>]*>/g) ?? []) expect(tag).toContain(`nonce="${nonce}"`);
  // Vue mounted: the scripts ran.
  await expect
    .poll(() => page.evaluate(() => !!(document.querySelector('#__nuxt') as { __vue_app__?: unknown } | null)?.__vue_app__))
    .toBe(true);
  const second = await page.goto('/en/terms');
  expect(second!.headers()['content-security-policy']).not.toContain(nonce!);
  await page.goto('/account');
  expect(violations).toEqual([]);
});

test('the authors’ terms are linked from the footer, versioned, in both languages', async ({ page }) => {
  await page.goto('/');
  await page.locator('footer').getByRole('link', { name: 'Conditions des auteurs' }).click();
  await expect(page).toHaveURL(/\/terms$/);
  await expect(page.locator('h1')).toHaveText('Les conditions des auteurs');
  await expect(page.getByText('Version 1, du 29 octobre 2026')).toBeVisible();
  await expect(page.getByText('« Vérifié par Escouade » veut dire qu’une version précise')).toBeVisible();
  await page.locator('footer').getByRole('link', { name: 'English' }).click();
  await expect(page).toHaveURL(/\/en\/terms$/);
  await expect(page.locator('h1')).toHaveText('The authors’ terms');
  await expect(page.locator('footer').getByRole('link', { name: 'Authors’ terms' })).toHaveAttribute('href', '/en/terms');
});
