import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DictationTarget } from '../lib/dictation.svelte';
import { FEATURES, featureOn } from '../lib/features';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { Feature, Features } from '../lib/types';
import { heat } from '../test/heat';
import { agent, fakeBackend, project, resetApp, SETTINGS, ticket } from '../test/ipc';
import ConversationButton from './ConversationButton.svelte';
import HeatFigure from './HeatFigure.svelte';
import MicButton from './MicButton.svelte';
import ProjectRail from './ProjectRail.svelte';
import SidePanel from './SidePanel.svelte';
import Sidebar from './Sidebar.svelte';
import StatusBar from './StatusBar.svelte';
import TitleBar from './TitleBar.svelte';

// Each view without what a feature switched off takes with it (F-1, ESC-550): its tab, its buttons, its figures; on again,
// all of it comes back. One `describe` per feature; its MCP tools and its background are the core's (features_tests.rs).

vi.mock('../lib/terminals', async (original) => ({
  ...(await original<typeof import('../lib/terminals')>()),
  launchLog: () => ({}),
  disposeLog() {},
}));

const ALL_ON = Object.fromEntries(FEATURES.map((f) => [f, true])) as Features;

/** The switches with `off` off. */
function only(...off: Feature[]) {
  app.settings = { ...SETTINGS, features: { ...ALL_ON, ...Object.fromEntries(off.map((f) => [f, false])) } };
}

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [project()], agents: [agent({ status: 'running' })], tickets: [ticket()] });
  fakeBackend();
});

describe('the register of the window', () => {
  it('reads the core’s switches, on until they are settled, the scheduled tasks gone with the Kanban', () => {
    expect(featureOn(null, 'kanban')).toBe(true);
    expect(featureOn({ features: null }, 'docs')).toBe(true);
    expect(featureOn({ features: { ...ALL_ON, kanban: false } }, 'scheduled')).toBe(false);
    expect(featureOn({ features: { ...ALL_ON, scheduled: false } }, 'kanban')).toBe(true);
  });
});

describe('Kanban off', () => {
  it('takes its tab away, and its board cannot open; on again, it is back', async () => {
    only('kanban');
    const { unmount } = render(Sidebar, { project: project() });
    expect(screen.queryByRole('button', { name: /^Kanban/ })).toBeNull();
    app.openBoard('p1');
    expect(app.boardOn).toBe(false);
    unmount();
    only();
    render(Sidebar, { project: project() });
    expect(screen.getByRole('button', { name: /^Kanban/ })).toBeInTheDocument();
  });
});

describe('Espace Docs off', () => {
  it('takes its tab away, and the Docs cannot open', () => {
    only('docs');
    render(Sidebar, { project: project() });
    expect(screen.queryByRole('button', { name: 'Docs' })).toBeNull();
    app.openDocs('p1');
    expect(app.docsOn).toBe(false);
  });
});

describe('Éditeur off', () => {
  it('takes the editor’s button away from the title bar, and the editor cannot open', async () => {
    only('editor');
    render(TitleBar);
    expect(screen.queryByRole('button', { name: 'Ouvrir l’éditeur du projet' })).toBeNull();
    await app.openEditor({ projectId: 'p1', source: 'project' });
    expect(app.editorOn).toBe(false);
  });
});

describe('Coût off', () => {
  it('takes the day’s cost from the status bar and the temperature from the cards', () => {
    only('cost');
    render(StatusBar);
    expect(screen.queryByText("Aujourd'hui")).toBeNull();
    render(HeatFigure, { agent: { id: 'a1', engine: 'claude', heat: heat() }, now: heat().at });
    expect(screen.queryByText('100 °C')).toBeNull();
  });

  it('shows them again once on', () => {
    only();
    render(StatusBar);
    expect(screen.getByText("Aujourd'hui", { exact: false })).toBeInTheDocument();
    render(HeatFigure, { agent: { id: 'a1', engine: 'claude', heat: heat() }, now: heat().at });
    expect(screen.getByText('100 °C')).toBeInTheDocument();
  });
});

describe('Statistiques off', () => {
  it('takes their tile from the rail, and their view leaves for the project', () => {
    only('stats');
    render(ProjectRail);
    expect(screen.queryByRole('button', { name: 'Stats' })).toBeNull();
    // The plugins' tile, in the same group, stays.
    expect(screen.getByRole('button', { name: 'Plugins' })).toBeInTheDocument();
    app.openStats();
    expect(app.ui.view).not.toBe('stats');
  });
});

describe('Workflows off', () => {
  it('takes their tab from the side panel, the files in its place', () => {
    only('workflows');
    app.panelTab = 'workflow';
    render(SidePanel, { project: project(), agent: app.agents.a1 });
    expect(screen.queryByRole('tab', { name: /Workflow/ })).toBeNull();
    expect(screen.getByRole('tab', { selected: true })).toHaveAttribute('id', 'side-tab-files');
  });
});

describe('Tâches programmées off', () => {
  it('takes their section from the sidebar, with the Kanban too', () => {
    only();
    const first = render(Sidebar, { project: project() });
    expect(document.querySelector('[data-scheduled-section]')).not.toBeNull();
    first.unmount();
    only('scheduled');
    const { unmount } = render(Sidebar, { project: project() });
    expect(document.querySelector('[data-scheduled-section]')).toBeNull();
    unmount();
    only('kanban');
    render(Sidebar, { project: project() });
    expect(document.querySelector('[data-scheduled-section]')).toBeNull();
  });
});

describe('Overseer off', () => {
  it('takes its button from the title bar, though it is turned on', () => {
    app.settings = { ...SETTINGS, overseer: { ...SETTINGS.overseer!, enabled: true }, features: { ...ALL_ON, overseer: false } };
    render(TitleBar);
    expect(document.querySelector('[data-overseer-button]')).toBeNull();
  });
});

describe('Voix off', () => {
  it('takes the microphone and the spoken conversation away', () => {
    const target: DictationTarget = { kind: 'composer', id: 'composer', el: () => null, insert: vi.fn() };
    only();
    const on = render(MicButton, { props: { target } });
    expect(document.querySelector('.mic-wrap')).not.toBeNull();
    on.unmount();
    only('voice');
    render(MicButton, { props: { target } });
    render(ConversationButton, { agent: app.agents.a1 });
    expect(document.querySelector('.mic-wrap')).toBeNull();
    expect(document.querySelector('.conv-wrap')).toBeNull();
  });
});
