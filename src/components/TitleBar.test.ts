import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, gitInfo, project, resetApp } from '../test/ipc';
import { getByTip } from '../test/tip';
import TitleBar from './TitleBar.svelte';

// The title bar of the 2.0 design (ESC-158): no tabs (the projects are on the rail, ProjectRail.test.ts); the logo, the
// active project's name and its menu, the search, the editor and the window's buttons.

describe('TitleBar', () => {
  beforeEach(() => {
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [agent({ status: 'waiting' }), agent({ id: 'b1', projectId: 'p2', status: 'running' })],
    });
    menu.close();
  });

  it('shows the app’s logo, and no tab of a project', () => {
    fakeBackend();
    render(TitleBar);
    expect(screen.getByRole('img', { name: 'Escouade' })).toHaveAttribute('src', '/logo.svg');
    expect(screen.queryByRole('tab')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /studio-web/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Stats|Vue d’ensemble/ })).not.toBeInTheDocument();
  });

  it('names the active project with its changes, and opens its menu from its name', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ modified: 2, added: 1, total: 3 }) };
    render(TitleBar);
    const name = screen.getByRole('button', { name: 'Menu du projet demo-api' });
    expect(name).toHaveTextContent('demo-api');
    expect(getByTip(name, 'Modifications git non commitées')).toHaveTextContent('Δ 3');
    expect(name.querySelector('[data-icon="down"]')).not.toBeNull();
    expect(name).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(name);
    expect(name).toHaveAttribute('aria-expanded', 'true');
    const labels = menu.open!.items.filter((i) => !i.separator).map((i) => i.label);
    expect(labels).toContain('Renommer…');
    expect(labels).toContain('Monter');
    expect(labels).toContain('Fermer le projet…');
    menu.close();
    await Promise.resolve();
    expect(name).toHaveAttribute('aria-expanded', 'false');
  });

  it('says no Δ when nothing changed, and follows the project chosen', async () => {
    fakeBackend();
    render(TitleBar);
    expect(screen.getByRole('button', { name: 'Menu du projet demo-api' })).not.toHaveTextContent('Δ');
    app.selectProject('p2');
    expect(await screen.findByRole('button', { name: 'Menu du projet studio-web' })).toBeInTheDocument();
  });

  it('marks a private project with a lock beside its name (choice 28)', () => {
    fakeBackend();
    app.projects[0] = { ...app.projects[0], private: true };
    render(TitleBar);
    const lock = screen.getByRole('img', { name: 'Projet privé : invisible pour les agents des autres projets' });
    expect(screen.getByRole('button', { name: 'Menu du projet demo-api' })).toContainElement(lock);
  });

  it('names no project behind « Vue d’ensemble » or the statistics, and offers no editor there', () => {
    fakeBackend();
    app.openStats();
    render(TitleBar);
    expect(screen.queryByRole('button', { name: /Menu du projet/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Ouvrir l’éditeur du projet' })).not.toBeInTheDocument();
  });

  it('opens the search of the conversations from its field, Ctrl K', async () => {
    fakeBackend();
    render(TitleBar);
    const search = screen.getByRole('button', { name: 'Rechercher dans les conversations (Ctrl+K)' });
    expect(search).toHaveTextContent('Rechercher');
    expect(search).toHaveTextContent('Ctrl K');
    await userEvent.click(search);
    expect(app.modal).toEqual({ kind: 'convSearch' });
  });

  it('opens the editor on the active project', async () => {
    fakeBackend();
    app.ui.activeProject = 'p2';
    render(TitleBar);
    const editor = screen.getByRole('button', { name: 'Ouvrir l’éditeur du projet' });
    expect(editor).toHaveTextContent('Éditeur');
    expect(editor.querySelector('[data-icon="code"]')).not.toBeNull();
    await userEvent.click(editor);
    expect(app.editor.p2).toMatchObject({ on: true, source: 'project' });
  });

  it('draws the window’s buttons as icons, no character', () => {
    fakeBackend();
    render(TitleBar);
    for (const [name, icon] of [
      ['Réduire', 'min'],
      ['Agrandir', 'max'],
      ['Fermer', 'close'],
    ])
      expect(screen.getByRole('button', { name }).querySelector(`[data-icon="${icon}"]`)).not.toBeNull();
  });

  it('keeps the logo and the window’s buttons alone at the first start (choice 49)', () => {
    resetApp({ projects: [] });
    fakeBackend();
    render(TitleBar);
    expect(screen.getByRole('img', { name: 'Escouade' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Rechercher|Menu du projet|éditeur/ })).not.toBeInTheDocument();
    expect(screen.getAllByRole('button').map((b) => b.getAttribute('aria-label'))).toEqual(['Réduire', 'Agrandir', 'Fermer']);
  });
});

describe('TitleBar in English', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'studio-web' })] });
    setLang('en');
  });

  it('names its buttons and their tooltips in English', () => {
    fakeBackend();
    app.git = { p1: gitInfo({ total: 3 }) };
    render(TitleBar);
    expect(screen.getByRole('button', { name: 'Menu of the project demo-api' })).toBeInTheDocument();
    expect(getByTip(document.body, 'Uncommitted git changes')).toHaveTextContent('Δ 3');
    expect(screen.getByRole('button', { name: 'Search the conversations (Ctrl+K)' })).toHaveTextContent('Search');
    expect(screen.getByRole('button', { name: 'Open the project’s editor' })).toHaveAttribute(
      'data-tip',
      'Browse and edit the project’s files',
    );
    expect(screen.getByRole('button', { name: 'Minimize' })).toHaveAttribute('data-tip', 'Minimize');
    expect(screen.getByRole('button', { name: 'Close' })).toHaveAttribute('data-tip', 'Close (the app stays in the notification area)');
  });
});

// The machine (ESC-537, docs/SERVER-UX.md § 4): said once this computer knows a server, amber while cut.
describe('TitleBar, the machine', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project()], agents: [] });
  });

  it('says nothing of a machine while no server is known', async () => {
    const { connection } = await import('../lib/connection.svelte');
    const { servers } = await import('../lib/servers.svelte');
    servers.view = { servers: [], active: null, tokensInFile: false, thisDevice: 'D' };
    connection.server = null;
    fakeBackend();
    render(TitleBar);
    expect(screen.queryByTestId('title-machine')).not.toBeInTheDocument();
  });

  it('says « Ce poste » once a server is known, and the server with its host when on it, amber while cut', async () => {
    const { connection } = await import('../lib/connection.svelte');
    const { servers } = await import('../lib/servers.svelte');
    const travail = { id: 's1', name: 'Travail', ssh: 'acme-escouade', host: 'h', remotePort: 1, device: 'd', active: false };
    servers.view = { servers: [travail], active: null, tokensInFile: false, thisDevice: 'D' };
    connection.server = null;
    fakeBackend();
    const { unmount } = render(TitleBar);
    expect(screen.getByTestId('title-machine')).toHaveTextContent('Ce poste');
    expect(screen.getByRole('status', { name: 'Machine : Ce poste' })).toBeInTheDocument();
    unmount();
    connection.server = { ...travail, active: true };
    connection.state = 'open';
    render(TitleBar);
    const machine = screen.getByTestId('title-machine');
    expect(machine).toHaveTextContent('Travail');
    expect(machine).toHaveTextContent('acme-escouade');
    expect(machine).not.toHaveClass('cut');
    connection.state = 'lost';
    await Promise.resolve();
    await new Promise((r) => setTimeout(r, 0));
    expect(screen.getByTestId('title-machine')).toHaveClass('cut');
    expect(screen.getByTestId('title-machine')).toHaveTextContent('reconnexion');
    connection.server = null;
    connection.state = 'local';
  });
});
