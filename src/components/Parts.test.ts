import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FEATURES } from '../lib/features';
import { setLang } from '../lib/i18n';
import { setLocks } from '../lib/locks.svelte';
import { partOn, partState } from '../lib/parts';
import { pageOfPartShown, rowOfPartShown, SETTINGS_ROWS } from '../lib/settings-pages';
import { searchSettings } from '../lib/settings-search';
import { app } from '../lib/state.svelte';
import { canPrepare, canTest } from '../lib/test-launch.svelte';
import type { Features, Part, Project } from '../lib/types';
import { agent, fakeBackend, project, resetApp, SETTINGS, ticket } from '../test/ipc';
import SettingsModal from './modals/SettingsModal.svelte';
import Sidebar from './Sidebar.svelte';

// The parts of a project switched on or off (ESC-658): the order of who says it (this device, the repository's
// .escouade.yml, the team, else on), the page of the settings, and each view without what a part off takes with it in
// its project alone; on again, all of it comes back. Its MCP tools, its agents' instructions and its background are the
// core's (parts_tests.rs, mcp/parts_tests.rs).

vi.mock('../lib/terminals', async (original) => ({
  ...(await original<typeof import('../lib/terminals')>()),
  launchLog: () => ({}),
  disposeLog() {},
}));

const ALL_ON = Object.fromEntries(FEATURES.map((f) => [f, true])) as Features;

/** `p` with the repository proposing `repo` and this device choosing `device`. */
function withParts(p: Project, device: Partial<Record<Part, boolean>> = {}, repo: Partial<Record<Part, boolean>> = {}): Project {
  const values = Object.fromEntries(Object.entries(repo).map(([k, v]) => [`parts.${k}`, v]));
  return {
    ...p,
    parts: device,
    shared: {
      ...p.shared,
      local: [],
      trusted: '',
      file: Object.keys(values).length ? { keys: Object.keys(values), values, warnings: [], error: null, version: 1, pending: null } : null,
    },
  };
}

/** The project of the tests put back with its parts as given. */
function put(device: Partial<Record<Part, boolean>> = {}, repo: Partial<Record<Part, boolean>> = {}) {
  const p = withParts(project(), device, repo);
  app.replaceProject(p);
  return p;
}

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [project()], agents: [agent({ status: 'running' })], tickets: [ticket()] });
  app.settings = { ...SETTINGS, features: { ...ALL_ON } };
  setLocks({ locks: {}, rows: {}, warnings: [] });
  fakeBackend();
});

describe('who says whether a part is on', () => {
  it('the device before the repository, the repository before everything on', () => {
    const p = project();
    for (const part of ['docs', 'memory', 'kanban', 'launch'] as Part[])
      expect(partState(p, part)).toEqual({ part, on: true, by: 'default' });
    expect(partState(withParts(p, {}, { docs: false }), 'docs')).toEqual({ part: 'docs', on: false, by: 'repo' });
    expect(partState(withParts(p, { docs: true }, { docs: false }), 'docs')).toEqual({ part: 'docs', on: true, by: 'device' });
    expect(partState(withParts(p, { kanban: false }, { kanban: true }), 'kanban').on).toBe(false);
    // A value that is not a boolean proposes nothing.
    const odd = withParts(p);
    odd.shared!.file = { keys: ['parts.docs'], values: { 'parts.docs': 'false' }, warnings: [], error: null, version: 1, pending: null };
    expect(partState(odd, 'docs').by).toBe('default');
    expect(partOn(null, 'docs')).toBe(true);
  });

  it('the team imposes over the device and recommends under the repository', () => {
    const p = withParts(project(), { kanban: true }, { docs: true });
    p.shared!.teamParts = { team: 'Acme', defaults: { 'parts.docs': false, 'parts.launch': false }, enforced: { 'parts.kanban': false } };
    expect(partState(p, 'kanban')).toEqual({ part: 'kanban', on: false, by: 'teamEnforced', team: 'Acme' });
    expect(partState(p, 'docs').by).toBe('repo');
    expect(partState(p, 'launch')).toEqual({ part: 'launch', on: false, by: 'teamDefault', team: 'Acme' });
  });

  it('the feature of the app off takes the part with it in every project', () => {
    put({ docs: true });
    app.settings = { ...SETTINGS, features: { ...ALL_ON, docs: false } };
    expect(app.partOn('p1', 'docs')).toBe(false);
    expect(app.partOn('p1', 'memory')).toBe(true);
  });
});

describe('Réglages › Projets › Parties du projet', () => {
  const panel = () => within(document.getElementById('settings-panel')!);
  const rowOf = (part: string) => document.querySelector(`[data-part="${part}"]`) as HTMLElement;

  it('lists each part with what it does and what turning it off changes, all on by default', () => {
    render(SettingsModal, { page: 'parts', projectId: 'p1' });
    expect(screen.getByRole('heading', { name: 'Parties du projet' })).toBeInTheDocument();
    const names = panel()
      .getAllByRole('switch')
      .map((s) => s.getAttribute('aria-label'));
    expect(names).toEqual(['Docs', 'Mémoire partagée', 'Kanban', 'Lancement et navigateurs']);
    for (const s of panel().getAllByRole('switch')) expect(s).toHaveAttribute('aria-checked', 'true');
    expect(rowOf('kanban')).toHaveTextContent('le projet n’a plus que des agents');
    expect(rowOf('memory')).toHaveTextContent('Les souvenirs restent dans .escouade/memory/.');
    const docs = panel().getByRole('switch', { name: 'Docs' });
    expect(document.getElementById(docs.getAttribute('aria-describedby')!)).toHaveTextContent('Les pages, décisions et documents restent.');
    expect(screen.getByText(/rien n’est effacé, et la rallumer retrouve tout/)).toBeInTheDocument();
  });

  it('a switch saves this device’s choice by its own command, and « Reprendre le défaut » forgets it', async () => {
    const sent: unknown[] = [];
    fakeBackend({
      project_set_part: (a: { projectId: string; part: Part; on: boolean | null }) => {
        sent.push(a);
        const now = app.projects.find((p) => p.id === a.projectId)!;
        const parts = { ...now.parts };
        if (a.on === null) delete parts[a.part];
        else parts[a.part] = a.on;
        return { ...now, parts };
      },
    });
    put({}, { docs: false });
    render(SettingsModal, { page: 'parts', projectId: 'p1' });
    // Proposed by the repository: off, and said so.
    expect(panel().getByRole('switch', { name: 'Docs' })).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByTestId('part-docs-by')).toHaveTextContent('Proposé par le dépôt (.escouade.yml) : ton choix ici passe devant.');
    await userEvent.click(panel().getByRole('switch', { name: 'Docs' }));
    expect(sent).toEqual([{ projectId: 'p1', part: 'docs', on: true }]);
    await waitFor(() => expect(screen.getByTestId('part-docs-by')).toHaveTextContent('Choisi sur ce poste : il passe devant le défaut'));
    expect(panel().getByRole('switch', { name: 'Docs' })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(panel().getByRole('button', { name: 'Reprendre le défaut de « Docs »' }));
    expect(sent.at(-1)).toEqual({ projectId: 'p1', part: 'docs', on: null });
    await waitFor(() => expect(screen.getByTestId('part-docs-by')).toHaveTextContent('Proposé par le dépôt'));
  });

  it('a part the team imposes is greyed with who; one whose feature is off says where to turn it on', () => {
    const p = withParts(project());
    p.shared!.teamParts = { team: 'Acme', defaults: {}, enforced: { 'parts.memory': false } };
    app.replaceProject(p);
    app.settings = { ...SETTINGS, features: { ...ALL_ON, kanban: false } };
    render(SettingsModal, { page: 'parts', projectId: 'p1' });
    expect(panel().getByRole('switch', { name: 'Mémoire partagée' })).toBeDisabled();
    expect(screen.getByTestId('part-memory-by')).toHaveTextContent('Imposé par l’équipe Acme');
    expect(panel().getByRole('switch', { name: 'Kanban' })).toBeDisabled();
    expect(rowOf('kanban')).toHaveTextContent('Coupé pour tout Escouade, dans Réglages › Fonctionnalités');
  });

  it('the pages and rows of a part off leave its project’s bar and search, not another project’s', () => {
    const off = withParts(project(), { kanban: false, launch: false, docs: false, memory: false });
    expect(pageOfPartShown('board', off)).toBe(false);
    expect(pageOfPartShown('tickets', off)).toBe(false);
    expect(pageOfPartShown('launch', off)).toBe(false);
    expect(pageOfPartShown('git', off)).toBe(true);
    const row = (id: string) => SETTINGS_ROWS.find((r) => r.id === id)!;
    expect(rowOfPartShown(row('docsBranch'), off)).toBe(false);
    expect(rowOfPartShown(row('memory'), off)).toBe(false);
    expect(rowOfPartShown(row('docsBranch'), project())).toBe(true);
    app.replaceProject(off);
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    const bar = screen.getByRole('navigation');
    expect(within(bar).queryByText('Lancement')).toBeNull();
    expect(within(bar).getByText('Parties du projet')).toBeInTheDocument();
    expect(searchSettings('pilote auto', app.projects, 'p1').filter((h) => h.projectId === 'p1')).toEqual([]);
  });

  it('a page of a part turned off while it is open leaves for the parts', async () => {
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    put({ kanban: false });
    await waitFor(() => expect(screen.getByRole('heading', { name: 'Parties du projet' })).toBeInTheDocument());
  });
});

describe('a part off in its project', () => {
  it('the Kanban: its tab goes and the board cannot open; the scheduled tasks go with it; back when on again', () => {
    put({ kanban: false });
    const { unmount } = render(Sidebar, { project: app.projects[0] });
    expect(screen.queryByRole('button', { name: /^Kanban/ })).toBeNull();
    app.openBoard('p1');
    expect(app.boardOn).toBe(false);
    unmount();
    put({ kanban: true });
    render(Sidebar, { project: app.projects[0] });
    expect(screen.getByRole('button', { name: /^Kanban/ })).toBeInTheDocument();
    app.openBoard('p1');
    expect(app.boardOn).toBe(true);
  });

  it('the Docs: their tab goes and they cannot open', () => {
    put({}, { docs: false });
    render(Sidebar, { project: app.projects[0] });
    expect(screen.queryByRole('button', { name: 'Docs' })).toBeNull();
    app.openDocs('p1');
    expect(app.docsOn).toBe(false);
  });

  it('the launch: its section, its test launches and their preparation go', () => {
    const a = agent({ worktree: { path: 'C:/wt', branch: 'agent-1' } as never, recipe: { prep: [], run: [] } as never });
    expect(canTest(a) || canPrepare(a)).toBe(true);
    const { unmount } = render(Sidebar, { project: app.projects[0] });
    expect(screen.getByRole('region', { name: 'Lancement' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Navigateurs' })).toBeInTheDocument();
    unmount();
    put({ launch: false });
    expect(canTest(a)).toBe(false);
    expect(canPrepare(a)).toBe(false);
    render(Sidebar, { project: app.projects[0] });
    expect(screen.queryByRole('region', { name: 'Lancement' })).toBeNull();
    // The browsers (ESC-657) go with it.
    expect(screen.queryByRole('region', { name: 'Navigateurs' })).toBeNull();
  });

  it('another project keeps every part', () => {
    put({ kanban: false, docs: false, launch: false, memory: false });
    expect(app.partOn('p2', 'kanban')).toBe(true);
    expect(app.partOn('p1', 'kanban')).toBe(false);
  });
});
