// The window on the remote API (ESC-495, docs/REMOTE.md « La fondation 2b »), with `ESCOUADE_E2E_REMOTE=1` alone (the
// CI's part « remote API »): its WebSocket closed by force while a terminal writes, the band says the connection to the
// core is lost and tried again, then the terminal goes on from where it was, no line missing, none twice.

import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, REMOTE, test } from './fixture';

test.skip(!REMOTE, 'the window on the remote API alone (ESCOUADE_E2E_REMOTE=1)');

/** The lines the terminal is to print, one every few milliseconds. */
const N = 400;
const line = (i: number) => `L${String(i).padStart(4, '0')}.`;

test('a cut while a terminal writes: the band, then the rest of the terminal with no line missing or twice', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  await expect(page.locator('.xterm')).toBeVisible();
  await page.waitForTimeout(2500); // PowerShell start-up
  await page.locator('.xterm').click();
  await page.keyboard.type(`1..${N} | % { 'L{0:d4}.' -f $_; Start-Sleep -Milliseconds 12 }`);
  await page.keyboard.press('Enter');
  /** What the terminal shows, as its user reads it. */
  const shown = () =>
    page.evaluate(() => {
      const terms = (window as unknown as { __escouadeTerminals: () => { text: string }[] }).__escouadeTerminals();
      return terms.map((t) => t.text).join('\n');
    });
  await expect.poll(shown, { timeout: 30_000 }).toContain(line(40));
  // The socket closed by force, the next try held off 2 s: the band says it.
  await page.evaluate(() => (window as unknown as { __escouadeRemote: { drop(ms: number): void } }).__escouadeRemote.drop(2000));
  const band = page.getByTestId('connection-band');
  await expect(band).toHaveText('Connexion au cœur perdue · nouvel essai…');
  // Back: the band gone, and the terminal goes on to its last line.
  await expect(band).toBeHidden({ timeout: 20_000 });
  await expect.poll(shown, { timeout: 60_000 }).toContain(line(N));
  const text = await shown();
  let at = -1;
  for (let i = 1; i <= N; i++) {
    const l = line(i);
    const first = text.indexOf(l);
    expect(first, `${l} shown`).toBeGreaterThan(at);
    expect(text.indexOf(l, first + 1), `${l} shown once`).toBe(-1);
    at = first;
  }
  expect(text).not.toContain('perdus pendant la coupure');
  // The window speaks to the core again: a command answers.
  await page.keyboard.type('Set-Content after-the-cut.txt ok');
  await page.keyboard.press('Enter');
  await expect.poll(() => fs.existsSync(path.join(app.repo, 'after-the-cut.txt')), { timeout: 20_000 }).toBe(true);
});
