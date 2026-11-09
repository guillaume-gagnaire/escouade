import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { BARE_ENGINE, CLAUDE_ENGINE, branchInfo, fakeBackend, gitInfo, project, resetApp, usableEngines } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import { afterGesture } from '../../test/settings';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

/** The page, not the bar of the window (which has its « Avancé » too). */
const panel = () => document.getElementById('settings-panel')!;

describe('ProjectPage', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.modal = { kind: 'settings' };
  });

  it('holds who the project is, its harness and account, the assistant, then « Avancé », and nothing of the other pages', () => {
    fakeBackend();
    app.settings.accounts = [
      { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true },
      { id: 'pro', name: 'Pro', configDir: 'C:/acc/pro', claudePath: '', active: true },
    ];
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    expect(screen.getByRole('heading', { level: 1, name: 'demo-api' })).toBeInTheDocument();
    const rows = [...panel().querySelectorAll('.sr .l, .fold .ft')].map((x) => x.textContent?.trim());
    const at = (text: string) => rows.findIndex((r) => r?.startsWith(text));
    for (const [a, b] of [
      ['Nom du projet', 'Dossier'],
      ['Dossier', 'Couleur'],
      ['Couleur', 'Lettres de la tuile'],
      ['Libellé sous la tuile', 'Compte préféré'],
      ['Compte préféré', 'Assistant de configuration'],
      ['Assistant de configuration', 'Avancé'],
    ]) {
      expect(at(a), a).toBeGreaterThanOrEqual(0);
      expect(at(a), `${a} before ${b}`).toBeLessThan(at(b));
    }
    for (const elsewhere of [
      'Un worktree par agent',
      'Projet privé',
      'Remote control pour les agents de ce projet',
      'Partager chaque nouveau ticket',
    ]) {
      expect(screen.queryByRole('switch', { name: elsewhere }), elsewhere).toBeNull();
    }
    expect(screen.queryByRole('heading', { name: 'Workflow' })).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Lancement' })).toBeNull();
  });

  it('opens the setup assistant again on the project (ESC-94)', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'project', projectId: 'p2' });
    expect(screen.getByText('Revoir les réglages du projet étape par étape, ou y copier ceux d’un autre projet.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir l’assistant…' }));
    expect(app.modal).toEqual({ kind: 'newProject', projectId: 'p2' });
  });

  it('closes the project from « Avancé », says it keeps its history, and offers to erase it with the question', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    // « Zone sensible » of the 2.0 window: folded at the foot of « Projet », it names what it hides.
    const advanced = within(panel()).getByRole('button', { name: /^Avancé/ });
    expect(advanced).toHaveTextContent('AvancéFermer le projet');
    expect(within(panel()).queryByRole('heading', { name: 'Zone sensible' })).toBeNull();
    await userEvent.click(advanced);
    expect(
      screen.getByText(
        "Le projet quitte l'application : ses agents, ses terminaux et ses lancements s'arrêtent. Son historique (conversations, tickets, réglages) est gardé : rouvrir ce dossier le retrouve. Les fichiers et les worktrees sur le disque ne sont pas touchés.",
      ),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet…' }));
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      option: { label: 'Effacer aussi son historique (conversations, tickets, réglages)', value: false },
    });
  });

  it('says it in English', async () => {
    setLang('en');
    fakeBackend();
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    expect(screen.getByRole('heading', { name: 'Identity' })).toBeInTheDocument();
    await userEvent.click(within(panel()).getByRole('button', { name: /^Advanced/ }));
    expect(
      screen.getByText(
        'The project leaves the app: its agents, terminals and launches stop. Its history (conversations, tickets, settings) is kept: adding this folder again brings it back. The files and worktrees on disk are not touched.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'MCP server' })).not.toBeInTheDocument();
    expect(screen.queryByRole('switch', { name: 'Agents can use Escouade' })).not.toBeInTheDocument();
  });
});

describe('ProjectPage: its default harness (ESC-29)', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.modal = { kind: 'settings' };
  });
  const LABEL = 'Harness par défaut';

  it('is offered only from two harnesses found on', () => {
    fakeBackend();
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    expect(screen.queryByRole('group', { name: LABEL })).toBeNull();
  });

  it('is Claude Code until another is chosen, and saved with the project', async () => {
    usableEngines('claude', 'codex');
    const backend = fakeBackend();
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    const group = screen.getByRole('group', { name: LABEL });
    expect(screen.getByText('Celui des nouveaux agents et des tickets du projet, sauf s’ils en choisissent un autre.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Claude Code', pressed: true })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Harness nu' }));
    expect(group).toBeInTheDocument();
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.engine).toBe('codex');
  });

  it('offers only the harnesses that can be used, and keeps the one chosen saying why it cannot any more (ESC-391)', () => {
    usableEngines('claude', 'codex');
    app.enginesOff = ['codex'];
    fakeBackend();
    const { unmount } = render(SettingsModal, { page: 'project', projectId: 'p1' });
    // Codex found with an account, its plugin off: Claude Code alone, no menu.
    expect(screen.queryByRole('group', { name: LABEL })).toBeNull();
    unmount();
    resetApp({ projects: [project({ engine: 'codex' })] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    usableEngines('claude', 'codex');
    app.enginesOff = ['codex'];
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    screen.getByRole('group', { name: LABEL });
    expect(screen.getByRole('button', { name: 'Harness nu (plugin coupé)', pressed: true })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Claude Code', pressed: false })).toBeInTheDocument();
  });
});
