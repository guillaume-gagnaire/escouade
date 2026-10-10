import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, send, test } from './fixture';

// Two Claude accounts, both simulated: Principal is the app's own Claude Code folder (the app is started with a
// CLAUDE_CONFIG_DIR of the test's, which Principal inherits: it is never given one), the second has a folder of its own
// under the test's. The fake `claude` keeps its sessions in the folder of the account it runs for, and stops every turn at
// the usage limit while that folder holds a `fake-limit` file.

/** A sign-in that is not out of date. The usage endpoint is a closed port: nothing is ever read of it, the accounts stay signed in. */
const SIGNED_IN = JSON.stringify({ claudeAiOauth: { accessToken: 'tok', refreshToken: 'ref', expiresAt: 4_102_444_800_000 } });

const principalDir = (root: string) => path.join(root, 'claude');
const proDir = (root: string) => path.join(root, 'claude-pro');

/** Both accounts signed in, Principal out of quota for good, and the second account in the settings the app starts with. */
function twoAccounts(name: string) {
  return (root: string) => {
    for (const dir of [principalDir(root), proDir(root)]) {
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, '.credentials.json'), SIGNED_IN);
    }
    fs.writeFileSync(path.join(principalDir(root), 'fake-limit'), '');
    return { accounts: [{ id: 'pro', name, configDir: proDir(root), claudePath: '', active: true }] };
  };
}

/** The `CLAUDE_CONFIG_DIR` of each agent's process so far, in order (the one-shot questions, `claude -p`, left out). */
const processes = (launches: { argv: string[]; configDir: string | null }[]) =>
  launches.filter((l) => !l.argv.includes('-p')).map((l) => l.configDir);

test.describe('two accounts, the first out of quota', () => {
  const NAME = 'Pro de l’équipe plateforme (partagé)';
  test.use({ preStart: { run: twoAccounts(NAME) } });

  test('the first ticket stops at the usage limit and goes on on the second account, the next one starts there', async ({ app }) => {
    const { page } = app;
    const [principal, pro] = [principalDir(app.root), proDir(app.root)];
    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: /^Kanban/ }).click();
    const review = page.getByRole('region', { name: 'À tester' });
    const add = async (title: string) => {
      await page.getByRole('button', { name: 'Nouveau ticket' }).click();
      await page.getByRole('textbox', { name: 'Titre du ticket' }).fill(title);
      await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
    };

    // The first ticket goes to Principal, the first account, which is out of quota: its agent meets the limit there, and
    // goes on on the second account with its session, rather than wait for Principal to reset.
    await add('Premier [ok]');
    await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
    expect(processes(app.launches())).toEqual([principal, pro]);
    // The session went along: the second account resumed it.
    const resumed = app.launches().filter((l) => !l.argv.includes('-p'))[1];
    expect(resumed.argv.some((a) => a.startsWith('--resume='))).toBe(true);
    // The conversation says so.
    await review.getByRole('button', { name: /DEM-1/ }).click();
    await expect(page.getByText(`L’agent reprend sur le compte ${NAME} (il était sur Principal).`)).toBeVisible();

    // The status bar shows the second account as the current one; its long name is cut short, whole in its tooltip.
    const group = page.getByRole('button', { name: `Quotas par compte (compte en cours : ${NAME})` });
    await expect(group).toBeVisible();
    const label = group.locator('.acct');
    await expect(label).toHaveAttribute('title', NAME);
    expect(await label.evaluate((e) => e.scrollWidth > e.clientWidth)).toBe(true);
    // The bars are the button's to describe: both windows, said to a screen reader.
    await expect(group).toHaveAccessibleDescription(/^Quota sur 5 heures : .*\. Quota sur 7 jours : /);

    // The next ticket starts on the second account at once: Principal is past the threshold.
    await page.getByRole('button', { name: /^Kanban/ }).click();
    await add('Deuxième [ok]');
    await expect(review.getByText('DEM-2', { exact: true })).toBeVisible({ timeout: 60_000 });
    expect(processes(app.launches())).toEqual([principal, pro, pro]);
  });
});

test.describe('two accounts, a conversation stopped by the usage limit', () => {
  test.use({ preStart: { run: twoAccounts('Pro') } });

  test('goes on, with its session, on the other account from the card of its turn', async ({ app }) => {
    const { page } = app;
    const [principal, pro] = [principalDir(app.root), proDir(app.root)];
    await addProject(page, app.repo);
    await send(page, 'Bonjour');
    // Principal is out of quota: the turn stops at the limit, and the card offers the other account.
    await expect(page.getByText("Le tour s'est terminé en erreur")).toBeVisible();
    await expect(page.getByText(/^Reprise automatique/)).toBeVisible();
    await page.getByRole('button', { name: 'Reprendre sur Pro' }).click();
    // "continue" is sent, as the automatic resume sends it, and answered on the second account.
    await expect(page.getByText('Bonjour, tu as dit : continue')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Reprendre sur Pro' })).toBeHidden();
    await expect(page.getByText(/^Reprise automatique/)).toBeHidden();
    expect(processes(app.launches())).toEqual([principal, pro]);
    // Its session is in both folders (copied, not moved), the second's with the turn that followed.
    const session = (dir: string) => {
      const projects = path.join(dir, 'projects');
      return fs
        .readdirSync(projects)
        .flatMap((f) => fs.readdirSync(path.join(projects, f)).map((n) => path.join(projects, f, n)))
        .filter((f) => f.endsWith('.jsonl'));
    };
    expect(session(principal)).toHaveLength(1);
    expect(session(pro).map((f) => path.basename(f))).toEqual(session(principal).map((f) => path.basename(f)));
    expect(fs.readFileSync(session(pro)[0], 'utf8')).toContain('Bonjour, tu as dit : continue');
  });
});
