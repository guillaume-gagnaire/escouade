import { expect, test, type Page } from '@playwright/test';
import { seedCatalog } from '../fake/catalog.mjs';
import { FAKE_GITHUB } from '../fake/env';

// The public pages (STORE-4) on the built server, with plugins published straight into the base: the home page, the
// search, a plugin's page (its README rendered and cut down, its image copied, its permissions in words, its versions,
// where it comes from), « Signaler » without an account, an author's page and Escouade's own; compared to the
// validated preview (design/store/, #home, #search, #plugin) in both themes, at 1440, 1028 and 390 px.
const PREVIEW = new URL('../../../design/store/index.html', import.meta.url).href;
const SIZES = [
  [1440, 900],
  [1028, 779],
  [390, 844],
] as const;

test.skip(!process.env.DATABASE_URL, 'the public pages read the base of DATABASE_URL');
// One seed for the file: « Nouveaux » shows the newest of the base, which another worker's seed would be.
test.describe.configure({ mode: 'serial' });

let seeded: Awaited<ReturnType<typeof seedCatalog>>;
test.beforeAll(async () => {
  seeded = await seedCatalog();
});

/** What the eye compares between the store and the preview, by selector: sizes, weights, colors. */
async function measure(page: Page, sels: Record<string, string>) {
  return page.evaluate((sels) => {
    const out: Record<string, Record<string, string | number> | null> = {};
    for (const [k, sel] of Object.entries(sels)) {
      const el = document.querySelector(sel);
      if (!el) {
        out[k] = null;
        continue;
      }
      const s = getComputedStyle(el);
      out[k] = {
        size: s.fontSize,
        weight: s.fontWeight,
        color: s.color,
        bg: s.backgroundColor,
        border: s.borderTopColor,
        radius: s.borderTopLeftRadius,
        height: Math.round(el.getBoundingClientRect().height),
      };
    }
    return out;
  }, sels);
}

const LOOKS = {
  home: {
    path: '/',
    sels: {
      find: '.bigfind',
      chip: '.cats .chip',
      head: '.section-head .h2',
      rowName: '.row .name b',
      rowDesc: '.row .desc',
      rowMeta: '.row .meta',
      tile: '.row .tile',
      publish: '.publish',
    },
  },
  search: {
    path: '/search?cat=notifications&q=attend',
    sels: {
      opt: '.filters .opt',
      on: '.filters .opt.on',
      h3: '.filters h3',
      q: '.results-head .q',
      sort: '.sort',
      count: '.count',
      rowName: '.results .row .name b',
      side: '.results .row .side',
      mark: '.row mark',
    },
  },
  plugin: {
    path: '',
    sels: {
      title: '.plug-head h1',
      line: '.plug-head .line',
      desc: '.plug-head .desc',
      open: '.plug-head .act .btn.primary',
      tab: '.tabs a.on',
      perm: '.perm',
      strong: '.perm.strong',
      h3: '.aside h3',
      chain: '.chain .v',
      facts: '.facts dt',
      report: '.report',
    },
  },
} as const;

for (const scheme of ['dark', 'light'] as const)
  for (const [name, look] of Object.entries(LOOKS))
    test(`${name} looks like the preview (${scheme})`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await page.goto(`${PREVIEW}${scheme === 'light' ? '?theme=light' : ''}#${name}`);
      const preview = await measure(page, look.sels);
      const path = name === 'plugin' ? `/plugins/${seeded.ids.slack}` : look.path;
      await page.goto(path);
      const store = await measure(page, look.sels);
      for (const k of Object.keys(look.sels)) {
        expect(store[k], `${k} is on the page`).not.toBeNull();
        expect(preview[k], `${k} is in the preview`).not.toBeNull();
        // A tile's hue is drawn from its plugin's id: its colors are not the preview's plugin's.
        for (const p of ['size', 'weight', 'color', 'bg', 'border', 'radius'] as const)
          if (!(k === 'tile' && p !== 'size' && p !== 'radius' && p !== 'weight')) expect(store[k]![p], `${k} ${p}`).toBe(preview[k]![p]);
      }
      if (name === 'home') {
        // The preview's row of the verified holds signed plugins; the store's, until STORE-8 signs any, Escouade's
        // own: « Livré avec Escouade » in the seal's colors and size.
        const own = await measure(page, { t: '.row .own' });
        await page.goto(`${PREVIEW}${scheme === 'light' ? '?theme=light' : ''}#home`);
        const seal = await measure(page, { t: '.row .seal' });
        for (const p of ['size', 'weight', 'color', 'bg', 'border', 'radius', 'height'] as const)
          expect(own.t![p], `seal ${p}`).toBe(seal.t![p]);
      }
    });

for (const [w, h] of SIZES)
  test(`the public pages hold in ${w} px, without scrolling sideways`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    for (const path of [
      '/',
      '/search?q=slack',
      `/plugins/${seeded.ids.slack}`,
      `/plugins/${seeded.ids.slack}?tab=versions`,
      `/authors/${seeded.login}`,
      '/plugins/dev.escouade.github',
      `/plugins/${seeded.ids.slack}/report`,
    ]) {
      await page.goto(path);
      const wide = await page.evaluate(() => document.documentElement.scrollWidth);
      expect(wide, path).toBeLessThanOrEqual(w);
    }
  });

test('the home page lists the categories, Escouade’s own among the verified, and the new plugins', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.cats .chip').first()).toContainText('Harness');
  await expect(page.locator('.cats')).toContainText('Sources du Kanban');
  await expect(page.locator('.cats')).toContainText('Hébergeurs de code');
  await expect(page.locator('.rows .row').first()).toContainText('Livré avec Escouade');
  // The new ones, newest first: the seeded plugins are dated a few minutes back, so a plugin another test publishes
  // meanwhile comes before them; the page is compared with what the store says is the newest.
  const fresh = ((await (await page.request.get('/api/catalog/home')).json()) as { fresh: { name: { fr: string } }[] }).fresh;
  await expect(page.locator('.mini').first().locator('b').first()).toHaveText(fresh[0]!.name.fr);
  // The search of the hero leads to the page of the search.
  await page.locator('.bigfind input').fill('téléphone');
  await page.locator('.bigfind input').press('Enter');
  await expect(page).toHaveURL(/\/search\?q=t%C3%A9l%C3%A9phone/);
  await expect(page.locator('.results')).toContainText(`ntfy ${seeded.run}`);
});

test('the search ignores accents and case, marks what it found, and filters by links', async ({ page }) => {
  await page.goto(`/search?q=TELEPHONE+${seeded.run}`);
  await expect(page.locator('.results .row')).toHaveCount(1);
  await expect(page.locator('.results .row mark', { hasText: 'téléphone' })).toHaveCount(1);
  await page.goto(`/search?q=${seeded.run}`);
  await expect(page.locator('.results .row')).toHaveCount(3);
  await page.locator('.filters .opt', { hasText: 'Sans programme' }).click();
  await expect(page).toHaveURL(/noprog=1/);
  await expect(page.locator('.results .row')).toHaveCount(2);
  await page.locator('.filters .opt', { hasText: 'Thèmes' }).click();
  await expect(page.locator('.results .row')).toHaveCount(1);
  await expect(page.locator('.results .row')).toContainText(`Nord ${seeded.run}`);
  await expect(page.locator('.filters .opt[aria-checked="true"]')).toHaveText('Sans programme');
  await page.locator('.filters .opt', { hasText: 'Vérifiés seulement' }).click();
  await expect(page.locator('.none')).toContainText('Aucun plugin ne répond');
  // « / » puts the cursor in the page's search.
  await page.goto('/search');
  await page.keyboard.press('/');
  await expect(page.locator('.results-head input[name=q]')).toBeFocused();
});

test('a plugin’s page shows its README cut down, its copied image, its permissions in words and where it comes from', async ({
  page,
  request,
}) => {
  const violations: string[] = [];
  page.on('console', (m) => /Content Security Policy|Refused to/i.test(m.text()) && violations.push(m.text()));
  await page.goto(`/plugins/${seeded.ids.slack}`);
  await expect(page.locator('.plug-head h1')).toContainText(`Slack ${seeded.run}`);
  await expect(page.locator('.plug-head')).toContainText('En revue');
  await expect(page.locator('.plug-head .line')).toContainText(seeded.login);
  await expect(page.getByRole('link', { name: 'Ouvrir dans Escouade' })).toHaveAttribute('href', `escouade://plugin/${seeded.ids.slack}`);
  const md = page.locator('.readme .md');
  await expect(md).toContainText('Quand un ticket passe À tester');
  // The traps: nothing that runs, styles or tracks survived; the page is whole and nothing ran.
  expect(await md.locator('script, style, iframe, [onerror], [style]').count()).toBe(0);
  expect(await page.evaluate(() => (window as unknown as { __pwned?: unknown }).__pwned)).toBeUndefined();
  await expect(page.locator('body')).toBeVisible();
  expect(await md.locator('a[href^="javascript"]').count()).toBe(0);
  await expect(md.locator('a', { hasText: 'la doc' })).toHaveAttribute(
    'href',
    /^https:\/\/github\.com\/.+\/blob\/[0-9a-f]{40}\/docs\/setup\.md$/,
  );
  // The image of the repository, copied on the store; the tracker's dropped.
  const img = md.locator('img');
  await expect(img).toHaveCount(1);
  const src = (await img.getAttribute('src'))!;
  expect(src).toMatch(/^\/files\/[0-9a-f]{64}\.png$/);
  const file = await request.get(src);
  expect(file.status()).toBe(200);
  expect(file.headers()['content-type']).toBe('image/png');
  expect(file.headers()['cache-control']).toContain('immutable');
  expect((await request.get('/files/..%2f..%2fpackage.json')).status()).toBe(404);
  expect(await page.locator('img[src*="tracker"]').count()).toBe(0);
  // Its permissions in words, the strongest first; where it comes from.
  await expect(page.locator('.perm').first()).toContainText('Lance un programme sur ton ordinateur, avec tes droits');
  await expect(page.locator('.aside')).toContainText('Se connecte à hooks.slack.com');
  await expect(page.locator('.aside')).toContainText('Garde un secret : Adresse du webhook');
  await expect(page.locator('.chain')).toContainText(`github.com/${seeded.login}/escouade-slack`);
  await expect(page.locator('.chain')).toContainText('v1.2.0 →');
  await expect(page.locator('.chain')).toContainText('Pas encore signée');
  await expect(page.locator('.facts')).toContainText(seeded.ids.slack);
  // The tabs: its documentation pages, its versions.
  await page.locator('.tabs a', { hasText: 'Documentation' }).click();
  await expect(page.locator('.readme .md')).toContainText('Le salon par projet.');
  await page.locator('.tabs a', { hasText: 'Versions' }).click();
  await expect(page.locator('.versions tbody tr')).toHaveCount(2);
  await expect(page.locator('.versions tbody tr').first()).toContainText('v1.2.0');
  expect(violations).toEqual([]);
});

test('the nonce marks Nuxt’s scripts only, never a tag in the page’s body', async ({ request }) => {
  const res = await request.get(`/plugins/${seeded.ids.slack}`);
  const html = await res.text();
  const nonce = /'nonce-([^']+)'/.exec(res.headers()['content-security-policy']!)![1]!;
  const app = html.slice(
    html.indexOf('<div id="__nuxt">'),
    html.indexOf('<div id="teleports">') > 0 ? html.indexOf('<div id="teleports">') : html.lastIndexOf('</div>'),
  );
  expect(app).toContain('Quand un ticket passe');
  expect(app).not.toContain('nonce=');
  expect(app).not.toMatch(/<script|<style/i);
  // Nuxt's own scripts carry it.
  expect(html).toContain(`<script nonce="${nonce}"`);
});

test('a README in the visitor’s language, the other one otherwise', async ({ page }) => {
  await page.goto(`/plugins/${seeded.ids.ntfy}`);
  await expect(page.locator('.readme .md')).toContainText('En français.');
  await page.goto(`/en/plugins/${seeded.ids.ntfy}`);
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await expect(page.locator('.readme .md')).toContainText('In English.');
  await expect(page.locator('.aside')).toContainText('Connects to ntfy.sh');
  await page.goto(`/en/plugins/${seeded.ids.nord}`);
  await expect(page.locator('.aside')).toContainText('Asks for nothing');
});

test('« Signaler » takes a report without an account, and acknowledges it by e-mail', async ({ page, request }) => {
  await page.goto(`/plugins/${seeded.ids.slack}`);
  await page.getByRole('link', { name: 'Signaler ce plugin' }).click();
  await expect(page).toHaveURL(new RegExp(`/plugins/${seeded.ids.slack.replace(/\./g, '\\.')}/report$`));
  await page.getByLabel('Permissions trompeuses').check();
  await page.getByLabel('Version').selectOption('1.1.0');
  await page.getByLabel('Explique ce que tu as vu').fill('Le programme lit ~/.ssh alors que rien ne le dit dans ses permissions.');
  const email = `rapport-${seeded.run}@exemple.dev`;
  await page.getByLabel('Ton adresse e-mail').fill(email);
  await page.getByLabel(/bonne foi/).check();
  await page.getByRole('button', { name: 'Envoyer le signalement' }).click();
  await expect(page.locator('.notice.ok')).toContainText('reçu');
  await expect
    .poll(
      async () =>
        ((await (await fetch(`${FAKE_GITHUB}/_fake/state`)).json()) as { mails: { to: string[]; raw: string }[] }).mails.find((m) =>
          m.to.includes(email),
        )?.raw ?? '',
    )
    .toContain('Permissions trompeuses');
  // The page says that a report is open, not who sent it.
  await page.goto(`/plugins/${seeded.ids.slack}`);
  await expect(page.locator('.aside')).toContainText('1 signalement ouvert');
  await expect(page.locator('body')).not.toContainText(email);
  // A report without the statement of good faith, or from another site, is refused.
  const form = { plugin: seeded.ids.slack, lang: 'fr', reason: 'other', details: 'x'.repeat(40), email };
  const noFaith = await request.post('/api/reports', { form, headers: { origin: new URL(page.url()).origin }, maxRedirects: 0 });
  expect(noFaith.headers().location).toContain('notice=faith');
  const elsewhere = await request.post('/api/reports', {
    form: { ...form, faith: 'yes' },
    headers: { origin: 'https://evil.example' },
    maxRedirects: 0,
  });
  expect(elsewhere.status()).toBe(403);
});

test('an author’s page lists their plugins; Escouade’s own have theirs, with the mark « Escouade »', async ({ page }) => {
  await page.goto(`/authors/${seeded.login}`);
  await expect(page.locator('.page-title')).toContainText(seeded.login);
  await expect(page.locator('.rows .row')).toHaveCount(3);
  await expect(page.getByRole('link', { name: 'Son profil GitHub' })).toHaveAttribute('href', `https://github.com/${seeded.login}`);
  // Initials, never an image from GitHub: the only images are the plugins' icons, from the store itself.
  for (const src of await page.locator('img').evaluateAll((all) => all.map((i) => i.getAttribute('src'))))
    expect(src).toMatch(/^\/files\/[0-9a-f]{64}\.png$/);
  await page.goto('/plugins/dev.escouade.github');
  await expect(page.locator('.plug-head')).toContainText('Livré avec Escouade');
  await expect(page.locator('.aside')).toContainText('Lance un programme d’Escouade');
  await expect(page.getByRole('link', { name: 'Ouvrir dans Escouade' })).toHaveAttribute('href', 'escouade://plugin/dev.escouade.github');
  await page.locator('.plug-head .line a.me').click();
  await expect(page).toHaveURL(/\/authors\/escouade$/);
  await expect(page.locator('.rows .row').first()).toContainText('Livré avec Escouade');
  expect((await page.goto('/plugins/dev.nobody.nothing'))?.status()).toBe(404);
  expect((await page.goto('/en/authors/nobody-at-all-here'))?.status()).toBe(404);
});

for (const scheme of ['dark', 'light'] as const)
  test(`a plugin’s icon takes its letters’ place on every page, from the store itself; without one, the letters (${scheme})`, async ({
    page,
  }) => {
    const violations: string[] = [];
    page.on('console', (m) => /Content Security Policy|Refused to/i.test(m.text()) && violations.push(m.text()));
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`${PREVIEW}${scheme === 'light' ? '?theme=light' : ''}#search`);
    const preview = await measure(page, { tile: '.row .tile' });
    const nord = `Nord ${seeded.run}`;
    const ntfy = `ntfy ${seeded.run}`;
    for (const path of [`/search?q=${seeded.run}`, `/authors/${seeded.login}`, `/en/search?q=${seeded.run}`]) {
      await page.goto(path);
      const row = page.locator('.row', { hasText: nord }).first();
      const img = row.locator('.tile img');
      await expect(img, path).toHaveAttribute('src', /^\/files\/[0-9a-f]{64}\.png$/);
      await expect.poll(() => img.evaluate((i: HTMLImageElement) => i.complete && i.naturalWidth), path).toBe(256);
      expect(await row.locator('.tile').textContent()).toBe('');
      // The same size and corners as a tile of letters, as the preview draws it.
      const tile = await row.locator('.tile').evaluate((el) => ({
        radius: getComputedStyle(el).borderTopLeftRadius,
        height: Math.round(el.getBoundingClientRect().height),
      }));
      expect(tile).toEqual({ radius: preview.tile!.radius, height: preview.tile!.height });
      // Without an icon, its two letters on its hue.
      const plain = page.locator('.row', { hasText: ntfy }).first().locator('.tile');
      await expect(plain).toHaveText('nt');
      expect(await plain.locator('img').count()).toBe(0);
    }
    // The plugin's page, its large tile at 256 px; the home page's small tiles at 64 px.
    await page.goto(`/plugins/${seeded.ids.nord}`);
    await expect(page.locator('.plug-head .tile img')).toHaveAttribute('src', /^\/files\/[0-9a-f]{64}\.png$/);
    await expect.poll(() => page.locator('.plug-head .tile img').evaluate((i: HTMLImageElement) => i.naturalWidth)).toBe(256);
    await page.goto(`/plugins/${seeded.ids.slack}`);
    expect(await page.locator('.plug-head .tile img').count()).toBe(0);
    await page.goto('/');
    const small = page.locator('a', { hasText: nord }).locator('.tile.sm img').first();
    if (await small.count()) await expect.poll(() => small.evaluate((i: HTMLImageElement) => i.naturalWidth)).toBe(64);
    expect(violations).toEqual([]);
  });

test('an icon is served by the store under its SHA-256, immutable, as an image and nothing else', async ({ page, request }) => {
  await page.goto(`/plugins/${seeded.ids.nord}`);
  const src = (await page.locator('.plug-head .tile img').getAttribute('src'))!;
  const r = await request.get(src);
  expect(r.status()).toBe(200);
  expect(r.headers()['content-type']).toBe('image/png');
  expect(r.headers()['cache-control']).toBe('public, max-age=31536000, immutable');
  expect(r.headers()['content-security-policy']).toBe("default-src 'none'; sandbox");
  const { createHash } = await import('node:crypto');
  expect(
    `/files/${createHash('sha256')
      .update(await r.body())
      .digest('hex')}.png`,
  ).toBe(src);
});

for (const scheme of ['dark', 'light'] as const)
  test(`Escouade’s own show their brand where its rules allow it, else their category’s glyph (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto('/plugins/dev.escouade.github');
    const github = page.locator('.plug-head .tile.brand.github');
    await expect(github.locator('svg path')).toHaveCount(1);
    // GitHub's mark in white or in black only, never recoloured.
    expect(await github.evaluate((el) => getComputedStyle(el).color)).toBe(scheme === 'dark' ? 'rgb(255, 255, 255)' : 'rgb(0, 0, 0)');
    await page.goto('/plugins/dev.escouade.jira');
    await expect(page.locator('.plug-head .tile.glyph.source svg path')).toHaveCount(2);
    await page.goto('/plugins/dev.escouade.claude-code');
    await expect(page.locator('.plug-head .tile.glyph.harness')).toBeVisible();
    await page.goto('/authors/escouade');
    await expect(page.locator('.rows .row .tile.brand, .rows .row .tile.glyph')).toHaveCount(17);
    expect(await page.locator('.rows .row img').count()).toBe(0);
  });
