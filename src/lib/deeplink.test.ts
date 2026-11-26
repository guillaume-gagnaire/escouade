import { beforeEach, describe, expect, it } from 'vitest';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import { setLang } from './i18n';
import { app } from './state.svelte';

// The link `escouade://agent/<id>` (ESC-2): the agent's conversation shown, its composer focused, nothing else done.
describe('the link escouade://agent/<id>', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'landing', path: 'C:\\code\\landing' })],
      agents: [agent(), agent({ id: 'b2', projectId: 'p2', name: 'hero' })],
    });
    app.toasts = [];
    app.ui.activeProject = 'p1';
    app.ui.view = 'stats';
    app.editor.p2 = { on: true, source: 'project', places: {} } as any;
  });

  it('shows the agent in its project, the editor closed, the composer focused, the view Plugins not opened', async () => {
    const backend = fakeBackend({ deep_links_take: () => [{ kind: 'agent', id: 'b2' }] });
    const focus = app.focusComposer;
    await app.takeLinks();
    expect(app.ui.view).toBe('project');
    expect(app.ui.activeProject).toBe('p2');
    expect(app.ui.selectedAgent.p2).toBe('b2');
    expect(app.editor.p2.on).toBe(false);
    expect(app.focusComposer).toBeGreaterThan(focus);
    // Navigation only: nothing sent, answered, started nor stopped.
    expect(backend.calls.map((c) => c.cmd).filter((c) => c !== 'deep_links_take' && c !== 'save_ui')).toEqual([]);
  });

  it('focuses the composer again for the agent already shown', async () => {
    fakeBackend({ deep_links_take: () => [{ kind: 'agent', id: 'a1' }] });
    app.selectAgent('a1');
    const focus = app.focusComposer;
    await app.takeLinks();
    expect(app.focusComposer).toBe(focus + 1);
  });

  it('says an agent it does not know, the view unchanged', async () => {
    fakeBackend({ deep_links_take: () => [{ kind: 'agent', id: 'ffff' }] });
    await app.takeLinks();
    expect(app.ui.view).toBe('stats');
    expect(app.ui.activeProject).toBe('p1');
    expect(app.toasts.map((t) => [t.kind, t.text])).toEqual([['error', 'Agent introuvable : ffff']]);
  });

  it('follows the last of two links', async () => {
    fakeBackend({
      deep_links_take: () => [
        { kind: 'agent', id: 'b2' },
        { kind: 'agent', id: 'a1' },
      ],
    });
    await app.takeLinks();
    expect(app.ui.activeProject).toBe('p1');
    expect(app.ui.selectedAgent.p1).toBe('a1');
  });

  it('shows an archived agent, as the sidebar does', async () => {
    app.agents.b2 = { ...app.agents.b2, archived: true };
    fakeBackend({ deep_links_take: () => [{ kind: 'agent', id: 'b2' }] });
    await app.takeLinks();
    expect(app.ui.selectedAgent.p2).toBe('b2');
  });

  it('is taken when the backend says a link came', () => {
    const before = app.deepLinks;
    (app as any).onEvent({ type: 'deepLink' });
    expect(app.deepLinks).toBe(before + 1);
  });
});
