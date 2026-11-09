// The artifacts of Claude Code from an agent of Escouade (ESC-400). In stream-json, Claude Code is an Agent SDK session whose
// artifacts are off unless CLAUDE_CODE_ARTIFACT says on: without it, « Unknown skill: artifact-design » and no Artifact
// tool (tests/fixtures/claude-artifacts, a real turn of Claude Code 2.1.319). The fake `claude` plays both.
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

const URL = 'https://claude.ai/artifact/AbCdEfGhIjKlMnOpQrStUv';

/** What the app asks the opener plugin to open, answered here: no browser opens (as links.spec.ts does). */
async function watchOpened(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as { esc400Opened: string[] };
    w.esc400Opened = [];
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.includes('open_url')) {
        w.esc400Opened.push(JSON.parse(String(init?.body ?? '{}')).url);
        return Promise.resolve(new Response('null', { headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } }));
      }
      return fetch(input, init);
    };
  });
  return () => page.evaluate(() => (window as unknown as { esc400Opened: string[] }).esc400Opened);
}

test('an agent publishes an artifact as in a terminal, its link opened in the browser at the click', async ({ app }) => {
  const { page } = app;
  const opened = await watchOpened(page);
  await addProject(page, app.repo);
  await send(page, 'Fais-moi une page [artefact]');

  const conv = page;
  // Its skill is known, its tool named in words.
  const call = conv.locator('.tool', { has: page.locator('.badge', { hasText: /^Artefact$/ }) });
  await expect(call).toBeVisible();
  await expect(call.locator('.arg')).toHaveText('esc400-trial.html');
  await expect(conv.getByText('Unknown skill')).toHaveCount(0);
  // The agent was launched with its artifacts on (its plugin's launch: not the core's reading of the models, with no session).
  const agents = app.launches().filter((l) => l.argv.includes('stream-json') && !l.argv.includes('--no-session-persistence'));
  expect(agents.length).toBeGreaterThan(0);
  expect(agents.map((l) => l.artifact)).toEqual(agents.map(() => '1'));

  // Its link: nothing opens until the user clicks it, then the browser does.
  const link = call.getByRole('button', { name: `Ouvrir ${URL} dans le navigateur`, exact: true });
  await expect(link).toHaveText('Ouvrir l’artefact');
  expect(await opened()).toEqual([]);
  await link.click();
  await expect.poll(opened).toEqual([URL]);
  // The click opened the link, not the row's detail.
  await expect(call.locator('.row')).toHaveAttribute('aria-expanded', 'false');

  // The link the agent writes in its reply opens too.
  await conv.locator('.md a', { hasText: URL }).click();
  await expect.poll(opened).toEqual([URL, URL]);
});
