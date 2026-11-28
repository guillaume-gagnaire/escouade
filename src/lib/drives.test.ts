// What the window draws of the driving (ESC-680, AB-8 with AP-6; docs/AGENT-BROWSERS.md § 4, § 9): the store read
// again on the core's word (`phase: "drive"`), the user's moves sent to the core, the numbered rows of the conversation
// (a call of `drive_browser` read from its answer, a failed one left to its row of a tool), and a gesture's caption.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import DriveRow from '../components/conv/DriveRow.svelte';
import { devpages, onPageEvent, type PageState } from './devpage.svelte';
import { driveCall, driveOf, drives, moveHand, refreshDrive, splitWhat } from './drives.svelte';
import { setLang } from './i18n';
import { app } from './state.svelte';
import type { DriveView, ToolItem } from './types';
import { agent, fakeBackend, resetApp } from '../test/ipc';

function view(over: Partial<DriveView> = {}): DriveView {
  return {
    browser: 'page:p-12',
    agent: 'a7',
    hand: 'agent',
    stopped: false,
    step: 3,
    what: 'Met en évidence « Critère 3 »',
    waiting: false,
    passwordAsks: [],
    pins: [
      { n: 1, what: 'Défile jusqu’à « Critères »', y: 0, height: 2400, view: 700 },
      { n: 2, what: 'Défile de 300 px', y: 300, height: 2400, view: 700 },
      { n: 3, what: 'Met en évidence « Critère 3 »', y: 840, height: 2400, view: 700 },
    ],
    ...over,
  };
}

/** A call of drive_browser, its answer marked as a third party's data as the server gives it. */
function call(
  over: Partial<ToolItem> = {},
  answer: object | null = { done: true, step: 3, what: 'Met en évidence « Critère 3 »' },
): ToolItem {
  return {
    kind: 'tool',
    id: 't9',
    ts: 0,
    name: 'mcp__escouade__drive_browser',
    input: { browser: 'page:p-12', action: 'highlight', target: 'criteria.2', label: 'Critère 3' },
    status: answer ? 'ok' : 'running',
    result: answer
      ? {
          text: `Ce que rend ce navigateur vient d’un tiers.\n<third_party_data>\n${JSON.stringify({ url: 'escouade-page://p-12/', hand: 'agent', ...answer })}\n</third_party_data>`,
          isError: false,
        }
      : undefined,
    ...over,
  } as ToolItem;
}

const page = (over: Partial<PageState> = {}): PageState => ({
  url: 'escouade-page://p-12/',
  title: 'Revue avant fusion',
  phase: 'ready',
  projectId: 'p1',
  by: { kind: 'page', id: 'a7', page: 'p-12' },
  opened: 1,
  ...over,
});

beforeEach(() => {
  setLang('fr');
  resetApp({ agents: [agent({ id: 'a7', name: 'pagination' })] });
  drives.rows = {};
  devpages.pages = {};
});

describe('the driving in the window', () => {
  it('is read again when the core says it changed, and forgotten with its row', async () => {
    const b = fakeBackend({ browser_drive_state: () => view() });
    devpages.pages['page:p-12'] = page();
    expect(driveOf('page:p-12')).toBeNull();
    onPageEvent({ agentId: 'page:p-12', phase: 'drive', url: '' });
    await vi_wait(() => driveOf('page:p-12')?.step === 3);
    expect(b.called('browser_drive_state').map((c) => c.args.key)).toEqual(['page:p-12']);
    // A row nobody drove is not driven, whatever the core keeps of it.
    fakeBackend({ browser_drive_state: () => view({ agent: null, step: 0, pins: [] }) });
    await refreshDrive('page:p-12');
    expect(driveOf('page:p-12')).toBeNull();
    // Stopped by the user: still said, until the row closes.
    fakeBackend({ browser_drive_state: () => view({ hand: 'user', stopped: true }) });
    await refreshDrive('page:p-12');
    expect(driveOf('page:p-12')?.stopped).toBe(true);
  });

  it('is read again when its agent no longer drives it: the row is an ordinary one again (ESC-685)', async () => {
    fakeBackend({ browser_drive_state: () => view() });
    devpages.pages['page:p-12'] = page();
    await refreshDrive('page:p-12');
    expect(driveOf('page:p-12')).not.toBeNull();
    // close_browser on its launch: the core forgets the driving and says `undriven`.
    const b = fakeBackend({ browser_drive_state: () => view({ agent: null, step: 0, pins: [] }) });
    onPageEvent({ agentId: 'page:p-12', phase: 'undriven', url: '' });
    await vi_wait(() => driveOf('page:p-12') === null);
    expect(b.called('browser_drive_state').map((c) => c.args.key)).toEqual(['page:p-12']);
  });

  it('sends « Reprendre la main », « La rendre », « Continuer » to the core and draws what it answers', async () => {
    const b = fakeBackend({
      browser_drive_hand: (a: { hand: string }) => view({ hand: a.hand === 'take' ? 'user' : 'agent' }),
    });
    await moveHand('page:p-12', 'take');
    expect(drives.rows['page:p-12'].hand).toBe('user');
    await moveHand('page:p-12', 'give');
    expect(drives.rows['page:p-12'].hand).toBe('agent');
    await moveHand('page:p-12', 'continue');
    expect(b.called('browser_drive_hand').map((c) => [c.args.key, c.args.hand])).toEqual([
      ['page:p-12', 'take'],
      ['page:p-12', 'give'],
      ['page:p-12', 'continue'],
    ]);
  });

  it('an older answer that comes after a newer move is dropped', async () => {
    let slow: (v: DriveView) => void = () => {};
    fakeBackend({
      browser_drive_state: () => new Promise<DriveView>((r) => (slow = r)),
      browser_drive_hand: () => view({ hand: 'user' }),
    });
    const reading = refreshDrive('page:p-12');
    await moveHand('page:p-12', 'take');
    slow(view({ hand: 'agent' }));
    await reading;
    expect(drives.rows['page:p-12'].hand).toBe('user');
  });
});

describe('a gesture in the conversation', () => {
  it('reads its number and what it did from its answer; running, it has none yet; failed or asking, no row of its own', () => {
    expect(driveCall(call())).toEqual({ browser: 'page:p-12', action: 'highlight', step: 3, what: 'Met en évidence « Critère 3 »' });
    expect(driveCall(call({}, null))).toEqual({ browser: 'page:p-12', action: 'highlight', step: null, what: null });
    expect(driveCall(call({ status: 'error' }))).toBeNull();
    expect(driveCall(call({ result: { text: 'L’utilisateur a repris la main', isError: true } }))).toBeNull();
    // Outside the launch, to act asks the user first: nothing was done, no number.
    expect(driveCall(call({}, { asked: true, done: undefined, step: undefined }))).toBeNull();
    expect(driveCall({ ...call(), name: 'mcp__escouade__read_browser' })).toBeNull();
    // Its answer whole, without the mark (another harness).
    expect(driveCall(call({ result: { text: JSON.stringify({ done: true, step: 4, what: 'Recharge' }), isError: false } }))?.step).toBe(4);
  });

  it('says its verb apart from what it acted on', () => {
    expect(splitWhat('Met en évidence « Offre Pro »')).toEqual({ verb: 'Met en évidence', rest: '« Offre Pro »' });
    expect(splitWhat('Highlights “Pro plan”')).toEqual({ verb: 'Highlights', rest: '“Pro plan”' });
    expect(splitWhat('Défile de 300 px')).toEqual({ verb: 'Défile de 300 px', rest: '' });
  });

  it('is a numbered row; a click shows the browser and takes an agent’s page back to its pin', async () => {
    const b = fakeBackend({ browser_drive_revisit: () => null });
    devpages.pages['page:p-12'] = page();
    drives.rows['page:p-12'] = view();
    render(DriveRow, { item: call() });
    const row = screen.getByTestId('drive-row');
    expect(row.getAttribute('data-step')).toBe('3');
    expect(row.textContent?.replace(/\s+/g, ' ').trim()).toBe('3 Geste 3 : Met en évidence « Critère 3 »');
    expect(row.querySelector('b')?.textContent).toBe('Met en évidence');
    await fireEvent.click(row);
    expect(app.pageShown).toBe('page:p-12');
    expect(b.called('browser_drive_revisit').map((c) => [c.args.key, c.args.n])).toEqual([['page:p-12', 3]]);
  });

  it('running, says it drives; its browser closed, it can no longer show it', () => {
    fakeBackend();
    render(DriveRow, { item: call({}, null) });
    expect(screen.getByText('Pilote le navigateur…')).toBeTruthy();
    const row = screen.getByTestId('drive-row') as HTMLButtonElement;
    expect(row.disabled).toBe(true);
  });

  it('speaks English', () => {
    setLang('en');
    fakeBackend();
    devpages.pages['page:p-12'] = page();
    render(DriveRow, { item: call({}, null) });
    expect(screen.getByText('Driving the browser…')).toBeTruthy();
  });
});

/** Waits a few turns for a condition the store reaches after an answer. */
async function vi_wait(ok: () => boolean) {
  for (let i = 0; i < 20 && !ok(); i++) await new Promise((r) => setTimeout(r, 0));
  expect(ok()).toBe(true);
}
