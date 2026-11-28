// The flagship case of the agents' pages, mode 1 (ESC-683, AP-9; docs/AGENT-PAGES.md § 1, docs/AGENT-BROWSERS-TICKETS.md
// ticket 18), on Windows, end to end: a fake agent that calls Escouade's real MCP server (tests/fixtures/fake-claude.mjs,
// « [page demande] », « [pilote p-1] », « [page piege] ») shows its review with its verdict beside its conversation
// (show_page with ask); it drives it under the user's eyes, scrolls to the criteria and highlights the third, and looks
// at what it shows (drive_browser look: a real capture of the view, highlight included); the user takes the hand by a
// real wheel of Windows over the page, and the agent's next gesture is refused; the user chooses A in the page, the band
// of Escouade says it in the request's words, « Envoyer » sends it, and the answer reaches the agent, which says so in
// its conversation. A trap page that tries to answer alone (an option it does not have, another channel, a file to
// join, a go-ahead, its own « Envoyer ») joins nothing: no answer, no file, no go-ahead, nothing said to the agent.
// At 1440 × 900 and 1024 × 728 (and at the CI's 1028 × 779 by ESCOUADE_E2E_VIEWPORT).
import fs from 'node:fs';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test, unfoldSections, type App } from './fixture';
import { nativeWheel } from './native-wheel';

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
  answer: { options: string[]; yes?: boolean | null; files?: unknown[] } | null;
  files?: unknown[];
  page?: { page: string; template: string } | null;
}

type Drive = { hand: string; agent: string | null; step: number };
const state = (page: Page, key: string) => invoke<Drive>(page, 'browser_drive_state', { key });

const streams = (app: App) =>
  app.launches().filter((l) => l.argv.includes('--input-format') && !l.argv.includes('--no-session-persistence'));

/** The MCP server of the agent's process, with its own token: the same agent, called directly. */
function agentServer(argv: string[]) {
  const entry = JSON.parse(fs.readFileSync(argv[argv.indexOf('--mcp-config') + 1], 'utf8')).mcpServers.escouade;
  return { url: entry.url as string, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
}

type Block = { type: string; text?: string; data?: string; mimeType?: string };

/** One call of a tool of Escouade, as Claude Code makes it. */
async function callTool(server: { url: string; token: string }, name: string, args: object) {
  let id = 0;
  const rpc = async (method: string, params: object, notify = false) => {
    // A reset socket (closed by the 2.3's server after 2 s idle, SEC-17, while a synchronous step held the event loop)
    // was never read: the call goes again, once, on a new one (ESC-701).
    const post = () =>
      fetch(server.url, {
        method: 'POST',
        headers: {
          authorization: `Bearer ${server.token}`,
          'content-type': 'application/json',
          accept: 'application/json, text/event-stream',
        },
        body: JSON.stringify({ jsonrpc: '2.0', ...(notify ? {} : { id: ++id }), method, params }),
      });
    const res = await post().catch((e: { cause?: { code?: string } }) =>
      ['ECONNRESET', 'UND_ERR_SOCKET', 'EPIPE'].includes(e?.cause?.code ?? '') ? post() : Promise.reject(e),
    );
    const text = await res.text();
    return text ? JSON.parse(text) : null;
  };
  await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
  await rpc('notifications/initialized', {}, true);
  const r = (await rpc('tools/call', { name, arguments: args })).result;
  const blocks = r.content as Block[];
  return { blocks, text: blocks.map((c) => c.text ?? '').join('\n'), error: !!r.isError };
}

/** What two images of the view show: their size, the colours of the first, and how many of their pixels differ. */
function compared(page: Page, one: Block, other: Block) {
  return page.evaluate(
    async ([a, b]) => {
      const pixels = async (src: string) => {
        const img = new Image();
        img.src = src;
        await img.decode();
        const c = document.createElement('canvas');
        c.width = img.naturalWidth;
        c.height = img.naturalHeight;
        const ctx = c.getContext('2d')!;
        ctx.drawImage(img, 0, 0);
        return { width: c.width, height: c.height, px: ctx.getImageData(0, 0, c.width, c.height).data };
      };
      const [x, y] = [await pixels(a), await pixels(b)];
      const colours = new Set<number>();
      let differ = 0;
      for (let i = 0; i < x.px.length; i += 4) {
        colours.add((x.px[i] << 16) | (x.px[i + 1] << 8) | x.px[i + 2]);
        const d = Math.abs(x.px[i] - y.px[i]) + Math.abs(x.px[i + 1] - y.px[i + 1]) + Math.abs(x.px[i + 2] - y.px[i + 2]);
        if (d > 30) differ++;
      }
      return { width: x.width, height: x.height, same: x.width === y.width && x.height === y.height, colours: colours.size, differ };
    },
    [`data:${one.mimeType};base64,${one.data}`, `data:${other.mimeType};base64,${other.data}`] as const,
  );
}

const card = (page: Page, p: string) => page.locator(`[data-page-card="${p}"]`).first();
const conv = (page: Page) => page.locator('main.conv');

test.describe('the flagship case of an agent’s page, mode 1', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`the agent shows its review, drives it, the user takes the hand and chooses A, the agent reads it; a trap joins nothing (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'mode 1 is exercised on WebView2 here (its real inputs, its overlay)');
      test.setTimeout(300_000);
      const { page } = app;
      await page.setViewportSize(size);
      await addProject(page, app.repo);
      await unfoldSections(page);

      // show_page(review, ask): the review beside the conversation, its request in « À toi », the band waits.
      await send(page, 'Montre-moi la revue avant fusion [page demande]');
      await expect(card(page, 'p-1')).toHaveAttribute('data-state', 'beside', { timeout: 30_000 });
      const view = page.locator('[data-agent-page="p-1"]');
      await expect(view).toBeVisible();
      await expect(conv(page)).toBeVisible();
      const band = view.getByTestId('choice-band');
      await expect(band).toHaveAttribute('data-state', 'idle');
      const asked = await invoke<Ask>(page, 'inbox_get', { id: 'a-1' });
      expect(asked.state).toBe('open');
      expect(asked.page).toMatchObject({ page: 'p-1', template: 'review' });
      await expect
        .poll(async () => (await invoke<{ loaded: boolean } | null>(page, 'agentpage_view', { page: 'p-1' }))?.loaded, { timeout: 30_000 })
        .toBe(true);

      // drive_browser: it scrolls to the criteria, highlights the third, scrolls to the changes, under the user's eyes.
      await send(page, 'Montre-moi le critère qui coince [pilote p-1]');
      const rows = page.getByTestId('drive-row');
      await expect(rows).toHaveCount(3, { timeout: 30_000 });
      await expect(rows.nth(2)).toHaveAttribute('data-step', '3', { timeout: 30_000 });
      await expect(rows.nth(1)).toContainText('Met en évidence');
      const driveBand = view.getByTestId('drive-band');
      await expect(driveBand.getByTestId('drive-hand')).toContainText('pilote');
      expect((await state(page, 'page:p-1')).hand).toBe('agent');

      // The same agent highlights the third criterion again and looks: a real capture of the view, the highlight in it.
      expect(streams(app).length).toBeGreaterThanOrEqual(1);
      const server = agentServer(streams(app)[0].argv);
      const look = await callTool(server, 'drive_browser', {
        browser: 'page:p-1',
        action: 'highlight',
        target: 'criteria.2',
        label: 'Critère 3',
        look: true,
      });
      expect(look.error, look.text).toBe(false);
      const image = look.blocks.find((b) => b.type === 'image');
      expect(image, look.text).toBeTruthy();
      expect(image!.mimeType).toMatch(/^image\/(png|jpeg|webp)$/);
      // The same view without the highlight: what differs between the two is the highlight, drawn in the capture.
      const bare = await callTool(server, 'drive_browser', { browser: 'page:p-1', action: 'clear', look: true });
      expect(bare.error, bare.text).toBe(false);
      const plain = bare.blocks.find((b) => b.type === 'image');
      expect(plain, bare.text).toBeTruthy();
      const seen = await compared(page, image!, plain!);
      // A real view, not a blank: wide, many colours (text, the page's own), the highlight in it.
      expect(seen.width, JSON.stringify(seen)).toBeGreaterThan(200);
      expect(seen.height, JSON.stringify(seen)).toBeGreaterThan(150);
      expect(seen.same, JSON.stringify(seen)).toBe(true);
      expect(seen.colours, JSON.stringify(seen)).toBeGreaterThan(50);
      expect(seen.differ, JSON.stringify(seen)).toBeGreaterThan(500);
      // Nothing of the highlight in the page's DOM: the browser's overlay draws it.
      const html = await invoke<string>(page, 'agentpage_read', { page: 'p-1', selector: null, text: false });
      expect(html).not.toContain('escouade-drive');

      // The user takes the hand: a real wheel of Windows over the page. The agent's next gesture is refused.
      const room = view.locator('.room').first();
      const box = (await room.boundingBox())!;
      const dpr = await page.evaluate(() => devicePixelRatio);
      const aimed: string[] = [];
      for (let i = 0; i < 3 && (await state(page, 'page:p-1')).hand !== 'user'; i++) {
        aimed.push(nativeWheel(box, dpr));
        // The core polls the user's gestures every 500 ms while the agent has the hand.
        await expect
          .poll(async () => (await state(page, 'page:p-1')).hand, { timeout: 4_000 })
          .toBe('user')
          .catch(() => {});
      }
      // A page wider than its window (the CI's window of 1028 × 779 at 1440): the window shows a strip of the webview,
      // where Chromium does not always take a posted wheel (ESC-694). There alone, the band's « Reprendre la main »,
      // the user's other gesture, takes the hand; the real wheel stays required wherever the window shows the page.
      const [, left, width, winLeft, winWidth] = (
        (aimed[0] ?? '').match(/webview (-?\d+),-?\d+ (\d+)x\d+ in a window (-?\d+),-?\d+ (\d+)x/) ?? []
      ).map(Number);
      const cut = left + width > winLeft + winWidth + 3;
      if (cut && (await state(page, 'page:p-1')).hand !== 'user') {
        test.info().annotations.push({ type: 'hand', description: `taken by the band, the window cuts the page (${aimed[0]})` });
        await driveBand.getByRole('button', { name: 'Reprendre la main' }).click();
      }
      await expect
        .poll(async () => (await state(page, 'page:p-1')).hand, {
          timeout: 15_000,
          message: `the wheel did not take the hand (${aimed.join('; ')})`,
        })
        .toBe('user');
      await expect(driveBand.getByTestId('drive-hand')).toContainText('Tu as la main');
      const refused = await callTool(server, 'drive_browser', { browser: 'page:p-1', action: 'scroll', by: 200 });
      expect(refused.error).toBe(true);
      expect(refused.text).toContain('L’utilisateur a repris la main');

      // A in the page: a proposal, said by the band in the request's words; nothing answered yet. The page proposes
      // nothing within 2 s of the agent's last gesture (CHOICE_QUIET: it would be the agent's doing): a user's pace.
      await page.waitForTimeout(2_500);
      await invoke(page, 'agentpage_click', { page: 'p-1', selector: '[data-option="a"]' });
      await expect(band).toHaveAttribute('data-state', 'proposed', { timeout: 15_000 });
      await expect(band.getByTestId('choice-chip')).toHaveText('A · Fusionner dans main');
      expect((await invoke<Ask>(page, 'inbox_get', { id: 'a-1' })).state).toBe('open');
      // « Envoyer »: the answer goes, and reaches the agent, which says it in its conversation.
      await band.getByTestId('choice-send').click();
      await expect(conv(page).getByText('Réponse reçue pour a-1 : A').first()).toBeVisible({ timeout: 40_000 });
      const read = await invoke<Ask>(page, 'inbox_get', { id: 'a-1' });
      expect(read.state).toBe('read');
      expect(read.answer?.options).toEqual(['a']);
      await expect(band).toHaveAttribute('data-state', 'answered');

      // The trap page: everything it tries by itself joins nothing.
      await send(page, 'Une autre revue [page piege]');
      await expect(card(page, 'p-2')).toHaveAttribute('data-state', 'beside', { timeout: 30_000 });
      let said = '';
      await expect
        .poll(
          async () => {
            said = await invoke<string>(page, 'agentpage_read', { page: 'p-2', selector: null, text: true }).catch((e) => String(e));
            return said.includes('fini');
          },
          { timeout: 40_000 },
        )
        .toBe(true);
      // Another channel, a file, a go-ahead: no such way; a body with a file or a go-ahead: refused.
      for (const refusedRoute of ['_/answer 404', '_/file 404', '_/approve 404']) expect(said).toContain(refusedRoute);
      expect(said).not.toMatch(/_\/(file|approve|answer) 2\d\d/);
      expect(said.match(/_\/choose 4\d\d/g)?.length ?? 0).toBeGreaterThanOrEqual(5);
      // Its own option lights the band, in the request's words, never the page's; then nothing goes, past the delay.
      const trapBand = page.locator('[data-agent-page="p-2"]').getByTestId('choice-band');
      await expect(trapBand).toHaveAttribute('data-state', 'proposed');
      await expect(trapBand.getByTestId('choice-chip')).toHaveText('A · Fusionner dans main');
      await page.waitForTimeout(12_000);
      const kept = await invoke<Ask>(page, 'inbox_get', { id: 'a-2' });
      expect(kept.state).toBe('open');
      expect(kept.answer).toBeNull();
      expect(kept.files ?? []).toHaveLength(0);
      await expect(conv(page).getByText(/Réponse reçue pour a-2/)).toHaveCount(0);
    });
  }
});
