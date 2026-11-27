// The way back of the agents' pages (ESC-676, AP-4; docs/AGENT-PAGES.md § 6 and § 8), by a fake agent that calls
// Escouade's real MCP server (tests/fixtures/fake-claude.mjs, « [page demande] », « [page piege] »): the agent shows a
// review that carries its verdict (show_page with ask); the request is in « À toi » with its page; the user chooses A in
// the page, which only proposes: the band of Escouade under the page says it in the request's words, and « Envoyer »
// sends it; the agent reads the answer, and the page shows it, its options frozen. A trap page that tries to answer by
// itself (an option it does not have, two for one, a field of its own, another channel, its own « Envoyer », its option
// again and again) sends nothing: the request stays open, the band says the request's words and not the page's. « Ouvrir
// la page » of « À toi » opens it beside its agent. At 1440 × 900 and 1024 × 728.
import type { Page } from '@playwright/test';
import { addProject, expect, send, test, unfoldSections } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;

async function invoke<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
}

interface Ask {
  id: string;
  state: string;
  answer: { options: string[] } | null;
  page?: { page: string; template: string } | null;
}

/** What page `p` holds, read in Escouade's world (its text, or the HTML of `selector`), once it says `wanted`. */
async function pageHas(page: Page, p: string, wanted: string, selector: string | null = null) {
  let said = '';
  await expect
    .poll(
      async () => {
        said = await invoke<string>(page, 'agentpage_read', { page: p, selector, text: !selector }).catch((e) => String(e));
        return said.includes(wanted);
      },
      { timeout: 30_000 },
    )
    .toBe(true);
  return said;
}

const card = (page: Page, p: string) => page.locator(`[data-page-card="${p}"]`).first();

test.describe('the way back of an agent’s page', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`the user chooses in the page, the band of Escouade sends it, a trap page sends nothing (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the page is a WebView2 here (WebKit by the probe)');
      test.setTimeout(240_000);
      const { page } = app;
      await page.setViewportSize(size);
      await addProject(page, app.repo);
      await unfoldSections(page);

      // The review with its verdict, beside the conversation; the band waits for a choice in the page.
      await send(page, 'Montre-moi la revue avant fusion [page demande]');
      await expect(card(page, 'p-1')).toHaveAttribute('data-state', 'beside', { timeout: 30_000 });
      const view = page.locator('[data-agent-page="p-1"]');
      const band = view.getByTestId('choice-band');
      await expect(band).toHaveAttribute('data-state', 'idle');
      await expect(band).toContainText('rien ne part sans ton geste ici');
      // The same request in the box, with its page.
      const left = await invoke<Ask>(page, 'inbox_get', { id: 'a-1' });
      expect(left.state).toBe('open');
      expect(left.page).toMatchObject({ page: 'p-1', template: 'review' });
      await expect.poll(async () => (await invoke<{ loaded: boolean } | null>(page, 'agentpage_view', { page: 'p-1' }))?.loaded).toBe(true);
      await pageHas(page, 'p-1', 'Fusionner dans main');

      // A in the page: a proposal, said in the request's words; nothing answered yet.
      await invoke(page, 'agentpage_click', { page: 'p-1', selector: '[data-option="a"]' });
      await expect(band).toHaveAttribute('data-state', 'proposed', { timeout: 15_000 });
      await expect(band.getByTestId('choice-chip')).toHaveText('A · Fusionner dans main');
      expect((await invoke<Ask>(page, 'inbox_get', { id: 'a-1' })).state).toBe('open');
      // « Envoyer », the user's gesture in the window: the answer goes, and the agent reads it.
      await band.getByTestId('choice-send').click();
      await expect(band).toHaveAttribute('data-state', /sending|answered/);
      await expect.poll(async () => (await invoke<Ask>(page, 'inbox_get', { id: 'a-1' })).state, { timeout: 40_000 }).toBe('read');
      const read = await invoke<Ask>(page, 'inbox_get', { id: 'a-1' });
      expect(read.answer?.options).toEqual(['a']);
      await expect(band).toHaveAttribute('data-state', 'answered');
      // The page shows it: A checked, every option frozen.
      const verdict = await pageHas(page, 'p-1', 'data-answered', '#verdict');
      expect(verdict).toMatch(/id="option-a"[^>]*aria-pressed="true"/);
      expect(verdict).toMatch(/id="option-b"[^>]*aria-pressed="false"/);
      expect(verdict.match(/disabled/g)?.length).toBe(3);

      // The trap: everything it tries by itself, then its option again and again, and its own « Envoyer ».
      await send(page, 'Une autre revue [page piege]');
      await expect(card(page, 'p-2')).toHaveAttribute('data-state', 'beside', { timeout: 30_000 });
      const trap = page.locator('[data-agent-page="p-2"]');
      const said = await pageHas(page, 'p-2', 'fini');
      for (const refused of ['_/choose 404', '_/answer 404']) expect(said).toContain(refused);
      expect(said).toContain('_/choose 204');
      // It lit the band, in the request's words, never the page's; and nothing was sent.
      const trapBand = trap.getByTestId('choice-band');
      await expect(trapBand).toHaveAttribute('data-state', 'proposed');
      await expect(trapBand.getByTestId('choice-chip')).toHaveText('A · Fusionner dans main');
      await expect(trapBand).not.toContainText('supprimer');
      await page.waitForTimeout(2_000);
      const kept = await invoke<Ask>(page, 'inbox_get', { id: 'a-2' });
      expect(kept.state).toBe('open');
      expect(kept.answer).toBeNull();
      // « Annuler »: the proposal goes, the request still waits.
      await trapBand.getByTestId('choice-cancel').click();
      await expect(trapBand).toHaveAttribute('data-state', 'idle');
      expect((await invoke<Ask>(page, 'inbox_get', { id: 'a-2' })).state).toBe('open');

      // « À toi »: the request with its page; « Ouvrir la page » opens it beside its agent.
      await page.getByTestId('inbox-tile').click();
      const pane = page.getByTestId('ask-pane');
      await page.getByTestId('inbox-row').filter({ hasText: 'Fusionner POR-41 ?' }).first().click();
      const thumb = pane.getByTestId('ask-page');
      await expect(thumb).toHaveAttribute('data-page', 'p-2');
      await expect(thumb).toContainText('Une revue');
      await thumb.getByTestId('ask-open-page').click();
      await expect(trap).toBeVisible();
      await expect(trap.getByTestId('choice-band')).toHaveAttribute('data-state', 'idle');
    });
  }
});
