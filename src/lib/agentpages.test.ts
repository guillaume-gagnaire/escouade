// The agents' pages in the window (AP-3; docs/AGENT-PAGES.md § 5): what the backend says becomes a row of
// « Navigateurs » (`page:p-12`), opened beside its agent's conversation when the user looks at it, waiting with its point
// otherwise or when the agent asked so (`quiet`); a new version reloads where it is; its row closes (its agent, its ×)
// and the page stays kept; erased, its cards say so.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { agentpages, onAgentPageEvent, openFromCard, showVersion } from './agentpages.svelte';
import { closePage, devpages } from './devpage.svelte';
import { leaves, itemKey } from './panes';
import { app } from './state.svelte';
import type { AgentPageRow } from './types';
import { agent, fakeBackend, resetApp } from '../test/ipc';

function row(over: Partial<AgentPageRow> = {}): AgentPageRow {
  return {
    page: 'p-12',
    projectId: 'p1',
    agentId: 'a7',
    agentName: 'pagination',
    ticketId: null,
    title: 'Revue avant fusion · POR-41',
    template: 'review',
    versions: [{ n: 1, at: 0 }],
    shown: 1,
    open: true,
    quiet: false,
    loaded: false,
    ...over,
  };
}

/** What the work area shows: the panes' contents, or what is alone. */
const onScreen = () => (app.panes && app.panesOn ? leaves(app.panes.root).map(itemKey) : [app.singleItem && itemKey(app.singleItem)]);

beforeEach(() => {
  resetApp({ agents: [agent({ id: 'a7', name: 'pagination' }), agent({ id: 'a8', name: 'export' })] });
  app.ui.panes = {};
  agentpages.info = {};
  devpages.pages = {};
  app.ui.view = 'project';
  app.ui.activeProject = 'p1';
});

describe('a page an agent shows', () => {
  it('opens beside the conversation the user looks at, which stays active', async () => {
    fakeBackend({ agentpage_info: () => row() });
    app.selectAgent('a7');
    await onAgentPageEvent({ page: 'p-12', phase: 'shown' });
    const p = devpages.pages['page:p-12'];
    expect(p).toMatchObject({ title: 'Revue avant fusion · POR-41', projectId: 'p1', by: { kind: 'page', id: 'a7', page: 'p-12' } });
    expect(p.unseen).toBeFalsy();
    expect(onScreen()).toEqual(['agent:a7', 'page:page:p-12']);
    expect(app.panes?.active).toBe('agent:a7');
  });

  it('waits with its point when the user looks at another agent, or when the agent asked it to (quiet)', async () => {
    fakeBackend({ agentpage_info: () => row() });
    app.selectAgent('a8');
    await onAgentPageEvent({ page: 'p-12', phase: 'shown' });
    expect(devpages.pages['page:p-12'].unseen).toBe(true);
    expect(onScreen()).toEqual(['agent:a8']);

    devpages.pages = {};
    fakeBackend({ agentpage_info: () => row({ quiet: true }) });
    app.selectAgent('a7');
    await onAgentPageEvent({ page: 'p-12', phase: 'shown' });
    expect(devpages.pages['page:p-12'].unseen).toBe(true);
    expect(onScreen()).toEqual(['agent:a7']);
  });

  it('a new version stays where it is; its loads say its state', async () => {
    fakeBackend({ agentpage_info: () => row() });
    app.selectAgent('a7');
    await onAgentPageEvent({ page: 'p-12', phase: 'shown' });
    fakeBackend({
      agentpage_info: () =>
        row({
          versions: [
            { n: 1, at: 0 },
            { n: 2, at: 1 },
          ],
          shown: 2,
        }),
    });
    await onAgentPageEvent({ page: 'p-12', phase: 'shown' });
    expect(onScreen()).toEqual(['agent:a7', 'page:page:p-12']);
    expect(agentpages.info['p-12']?.shown).toBe(2);
    await onAgentPageEvent({ page: 'p-12', phase: 'finished' });
    expect(devpages.pages['page:p-12'].phase).toBe('ready');
    await onAgentPageEvent({ page: 'p-12', phase: 'wants', url: 'https://example.com/' });
    expect(devpages.pages['page:p-12'].wants).toBe('https://example.com/');
  });

  it('its agent shows another of its versions (drive_browser): its bar says which, where it is (ESC-679)', async () => {
    const versions = [
      { n: 1, at: 0 },
      { n: 2, at: 1 },
    ];
    fakeBackend({ agentpage_info: () => row({ versions, shown: 2 }) });
    app.selectAgent('a7');
    await onAgentPageEvent({ page: 'p-12', phase: 'shown' });
    fakeBackend({ agentpage_info: () => row({ versions, shown: 1 }) });
    await onAgentPageEvent({ page: 'p-12', phase: 'version' });
    expect(agentpages.info['p-12']?.shown).toBe(1);
    expect(onScreen()).toEqual(['agent:a7', 'page:page:p-12']);
  });

  it('its row closes by its agent (nothing asked back) or by its × (asked), the page kept; erased, it is no longer known', async () => {
    const b = fakeBackend({ agentpage_info: () => row({ open: false }) });
    devpages.pages['page:p-12'] = {
      url: '',
      title: 't',
      phase: 'ready',
      projectId: 'p1',
      by: { kind: 'page', id: 'a7', page: 'p-12' },
      opened: 1,
    };
    await onAgentPageEvent({ page: 'p-12', phase: 'closed' });
    expect(devpages.pages['page:p-12']).toBeUndefined();
    expect(b.called('agentpage_close_row')).toEqual([]);
    await vi.waitFor(() => expect(agentpages.info['p-12']?.open).toBe(false));

    devpages.pages['page:p-12'] = {
      url: '',
      title: 't',
      phase: 'ready',
      projectId: 'p1',
      by: { kind: 'page', id: 'a7', page: 'p-12' },
      opened: 1,
    };
    closePage('page:p-12');
    expect(b.called('agentpage_close_row')).toEqual([{ cmd: 'agentpage_close_row', args: { page: 'p-12' } }]);
    expect(b.called('devpage_close')).toEqual([]);

    await onAgentPageEvent({ page: 'p-12', phase: 'erased' });
    expect(agentpages.info['p-12']).toBeNull();
  });

  it('its card opens it again, beside its agent; « v1 ▾ » asks for that version', async () => {
    const b = fakeBackend({
      agentpage_info: () => row({ open: false }),
      agentpage_reopen: () =>
        row({
          versions: [
            { n: 1, at: 0 },
            { n: 2, at: 1 },
          ],
          shown: 2,
        }),
      agentpage_version: (a: { n: number }) =>
        row({
          versions: [
            { n: 1, at: 0 },
            { n: 2, at: 1 },
          ],
          shown: a.n,
        }),
    });
    app.selectAgent('a7');
    await openFromCard('p-12');
    expect(b.called('agentpage_reopen')).toHaveLength(1);
    expect(onScreen()).toEqual(['agent:a7', 'page:page:p-12']);
    await showVersion('page:p-12', 1);
    expect(b.called('agentpage_version')).toEqual([{ cmd: 'agentpage_version', args: { page: 'p-12', n: 1 } }]);
    expect(agentpages.info['p-12']?.shown).toBe(1);
  });
});

describe('one pane per agent (ESC-678)', () => {
  it('its page takes the pane of the tab its agent drove beside the conversation', async () => {
    fakeBackend({ agentpage_info: () => row() });
    app.selectAgent('a7');
    devpages.pages['a7tab'] = {
      url: 'http://localhost:4111/',
      title: 'web',
      phase: 'ready',
      projectId: 'p1',
      by: { kind: 'user' },
      opened: 1,
      driver: 'a7',
    };
    app.dropInPane({ kind: 'agent', id: 'a7' }, 'right', { kind: 'page', id: 'a7tab' });
    expect(onScreen()).toEqual(['agent:a7', 'page:a7tab']);
    await onAgentPageEvent({ page: 'p-12', phase: 'shown' });
    expect(onScreen()).toEqual(['agent:a7', 'page:page:p-12']);
    expect(devpages.pages['page:p-12'].driver).toBe('a7');
  });
});
