import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, send, test } from './fixture';

// ESC-290: under Linux, the page's process of the AppImage aborted each time one project was opened, the window frozen.
// The process that ends is written in app.log, the page loaded again without that project, which it offers to open again.
// Here WebView2's renderer, crashed through the DevTools protocol (`Page.crash`), stands for it.

/** A git repository with one commit, named `name`, in the test's folder. */
function repoNamed(root: string, name: string): string {
  const dir = path.join(root, name);
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(dir, 'README.md'), `# ${name}\n`);
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

// What a tool that read a binary file and a minified one gives (the fake claude's « sorties piégées »): drawn in the real
// page cleaned and a slice at a time, the page answering on.
test('the output of a tool that read binary files is shown cleaned and bounded', async ({ app }) => {
  const page = app.page;
  await addProject(page, app.repo);
  await send(page, 'sorties piégées');
  await expect(page.getByText('Lu.', { exact: true })).toBeVisible({ timeout: 30_000 });
  await page.locator('.row[role="button"]', { hasText: 'bank 1.bnk' }).first().click();
  const out = page.locator('.detail pre.out').last();
  await expect(out).toContainText('RIFF␀␀␀␀WAVEfmt');
  const text = (await out.textContent()) ?? '';
  expect(text).not.toMatch(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/);
  expect(text).not.toContain('[31m');
  // The line of 5 MB: cut by the core already (and by `LongText` past 20 000 characters).
  expect(text.length).toBeLessThan(25_000);
  expect(text).toContain('xxxx');
  // The page still answers.
  expect(await page.evaluate(() => 1 + 1)).toBe(2);
});

test('a page that crashes on a project comes back without it and offers it again', async ({ app }) => {
  let page = app.page;
  await addProject(page, repoNamed(app.root, 'calme'), { firstAgent: false });
  await addProject(page, repoNamed(app.root, 'remedy'), { firstAgent: false });
  const shown = () => page.locator('header.bar .proj .name');
  await expect(shown()).toHaveText('remedy');

  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Page.crash').catch(() => {});
  page = await app.reconnect();

  const toast = page.getByText('L’interface s’est arrêtée en affichant « remedy » : elle a été rechargée sans ce projet.');
  await expect(toast).toBeVisible({ timeout: 30_000 });
  // Not opened by itself: the other project is.
  await expect(shown()).toHaveText('calme');
  const log = fs.readFileSync(path.join(app.data, 'app.log'), 'utf8');
  expect(log).toMatch(/page process ended \(crashed\), showing project \w+ agent -: loaded again on no project/);
  expect(log).not.toContain(app.root);

  await page.getByRole('button', { name: 'Rouvrir' }).click();
  await expect(toast).toBeHidden();
  await expect(shown()).toHaveText('remedy');
});
