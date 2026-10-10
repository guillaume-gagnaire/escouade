import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import { agent, branchInfo, fakeBackend, gitInfo, resetApp } from '../test/ipc';
import StatusBar from './StatusBar.svelte';

describe('StatusBar', () => {
  beforeEach(() => {
    resetApp({
      agents: [
        agent({ id: 'a1', status: 'running' }),
        agent({ id: 'a2', status: 'waiting' }),
        agent({ id: 'a3', status: 'waiting', archived: true }),
        agent({ id: 'a4', status: 'done' }),
      ],
    });
    app.now = Date.UTC(2026, 8, 27, 20, 0, 0);
  });

  it('shows the running Claude processes with their memory and CPU, each agent in the tooltip', () => {
    fakeBackend();
    const GB = 1024 ** 3;
    app.agents.a4 = { ...app.agents.a4, name: 'tests-e2e' };
    app.resources = {
      instances: 2,
      memory: 1.5 * GB,
      cpu: 12.4,
      agents: [
        { id: 'a1', memory: 0.5 * GB, cpu: 2.4 },
        { id: 'a4', memory: GB, cpu: 10 },
      ],
    };
    render(StatusBar);
    const item = screen.getByText(/2 Claude/).closest('.it')!;
    expect(item).toHaveTextContent('2 Claude · 1,5 Go · 12 % CPU');
    const title = item.getAttribute('title')!;
    // The biggest first.
    expect(title.split('\n').slice(1)).toEqual(['tests-e2e : 1 Go · 10 %', 'refacto-auth : 512 Mo · 2 %']);
  });

  it('says nothing about processes when none runs', () => {
    fakeBackend();
    render(StatusBar);
    expect(screen.queryByText(/Claude ·/)).not.toBeInTheDocument();
  });

  it('shows when the weekly quota resets too', () => {
    fakeBackend();
    app.usage = {
      fiveHour: { pct: 10, resetsAt: null },
      sevenDay: { pct: 38.4, resetsAt: app.now + (2 * 86400 + 5 * 3600) * 1000 },
      todayCost: 0,
      updatedAt: 1,
      accounts: [],
      current: 'principal',
    };
    render(StatusBar);
    const week = screen.getByText(/Hebdo/).closest('.it')!;
    expect(week).toHaveTextContent('38 %');
    expect(week).toHaveTextContent('reset 2j 5h');
    // The date of the reset, in full, without seconds: the day depends on the time zone of the machine.
    expect(week.getAttribute('title')).toMatch(/^Réinitialisation : (29|30) septembre 2026 à \d{2}:\d{2}$/);
  });

  it('counts active, waiting and finished agents (archived excluded)', () => {
    fakeBackend();
    render(StatusBar);
    expect(screen.getByText('1 actif')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /1 en attente/ })).toBeInTheDocument();
    expect(screen.getByText('1 terminé')).toBeInTheDocument();
  });

  it('shows quotas with the time left before the session reset, and the day cost', () => {
    fakeBackend();
    app.usage = {
      fiveHour: { pct: 62, resetsAt: app.now + (1 * 3600 + 48 * 60) * 1000 },
      sevenDay: { pct: 38.4, resetsAt: null },
      todayCost: 4.12,
      updatedAt: 1,
      accounts: [],
      current: 'principal',
    };
    render(StatusBar);
    expect(screen.getByText('62 %')).toBeInTheDocument();
    expect(screen.getByText('reset 1h48')).toBeInTheDocument();
    expect(screen.getByText('38 %')).toBeInTheDocument();
    expect(screen.getByText(/4,12/)).toBeInTheDocument();
  });

  it('shows dashes when quotas are unknown', () => {
    fakeBackend();
    render(StatusBar);
    expect(screen.getAllByText('—')).toHaveLength(2);
  });

  it('toggles the sound and saves the setting', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(StatusBar);
    await userEvent.click(screen.getByRole('button', { name: '♪ On' }));
    expect(screen.getByRole('button', { name: '♪ Off' })).toBeInTheDocument();
    expect(backend.called('save_settings')[0].args.settings.sound).toBe(false);
  });

  it('jumps to the waiting agent', async () => {
    fakeBackend();
    render(StatusBar);
    await userEvent.click(screen.getByRole('button', { name: /1 en attente/ }));
    expect(app.agent?.id).toBe('a2');
  });
});

describe('StatusBar branch and sync with the remote', () => {
  const tracked = { upstream: 'origin/main', hasRemote: true };
  const syncButton = () => screen.getByRole('button', { name: /⎇ main/ });
  const picker = () => screen.findByRole('dialog', { name: 'Branches' });
  const LIST = [branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api' }), branchInfo({ name: 'feat/login' })];

  beforeEach(() => {
    resetApp();
    menu.close();
    app.now = Date.UTC(2026, 8, 27, 20, 0, 0);
  });

  it('shows the branch with the commits to push and to pull, and when it was last fetched', () => {
    fakeBackend();
    app.git = { p1: gitInfo({ ...tracked, behind: 3, ahead: 1, lastFetch: app.now - 3 * 60_000 }) };
    render(StatusBar);
    expect(syncButton()).toHaveTextContent('↑1');
    expect(syncButton()).toHaveTextContent('↓3');
    // The way they are in the picker: ahead first.
    expect(syncButton().textContent!.replace(/\s+/g, ' ')).toContain('⎇ main ↑1 ↓3');
    expect(syncButton().title).toContain('origin/main');
    expect(syncButton().title).toContain('Dernier fetch : il y a 3 min');
  });

  it('shows the branch of any repository, with no sync to say when it has no remote, and a detached HEAD', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ hasRemote: false }) };
    render(StatusBar);
    expect(syncButton()).toHaveTextContent(/^⎇ main$/);
    expect(syncButton().title).toBe('Changer de branche');
    app.git = { p1: gitInfo({ ...tracked, branch: '(detached)', upstream: null }) };
    await tick();
    expect(screen.getByRole('button', { name: /⎇ HEAD détachée/ })).toHaveTextContent(/^⎇ HEAD détachée$/);
    app.git = { p1: gitInfo({ isRepo: false, branch: '' }) };
    await tick();
    expect(screen.queryByText(/⎇/)).toBeNull();
    app.git = { p1: gitInfo(tracked) };
    await tick();
    expect(syncButton()).toBeInTheDocument();
    app.ui.view = 'stats';
    await tick();
    expect(screen.queryByText(/⎇/)).toBeNull();
  });

  it('opens the branch picker over the bar, and puts the focus back on the button when it closes', async () => {
    const backend = fakeBackend({ branch_list: () => LIST });
    app.git = { p1: gitInfo(tracked) };
    render(StatusBar);
    expect(syncButton()).toHaveAttribute('aria-expanded', 'false');
    expect(syncButton()).toHaveAttribute('aria-haspopup', 'dialog');
    await userEvent.click(syncButton());
    await picker();
    expect(syncButton()).toHaveAttribute('aria-expanded', 'true');
    expect(backend.called('branch_list')[0].args).toEqual({ projectId: 'p1' });
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Branches' })).toBeNull());
    expect(syncButton()).toHaveAttribute('aria-expanded', 'false');
    expect(syncButton()).toHaveFocus();
  });

  it('closes the picker when the project has no branch to show any more', async () => {
    fakeBackend({ branch_list: () => LIST });
    app.git = { p1: gitInfo(tracked) };
    render(StatusBar);
    await userEvent.click(syncButton());
    await picker();
    app.ui.view = 'stats';
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Branches' })).toBeNull());
  });

  it('says a branch is not published, or that its remote branch is gone', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ hasRemote: true, branch: 'feat/x' }) };
    render(StatusBar);
    expect(screen.getByRole('button', { name: /⎇ feat\/x/ })).toHaveTextContent('non publiée');
    app.git = { p1: gitInfo({ hasRemote: true, branch: 'feat/x', upstream: 'origin/feat/x', upstreamGone: true }) };
    await tick();
    const button = screen.getByRole('button', { name: /⎇ feat\/x/ });
    expect(button).toHaveTextContent('distante supprimée');
    expect(button.title).toContain("origin/feat/x n'existe plus");
  });

  it('pulls from the picker, then says what came in', async () => {
    const backend = fakeBackend({ branch_list: () => LIST, git_pull: () => '3 commits tirés' });
    app.git = { p1: gitInfo({ ...tracked, behind: 3 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    await picker();
    await userEvent.click(screen.getByRole('button', { name: /Récupérer/ }));
    await waitFor(() => expect(app.toasts).toEqual([expect.objectContaining({ text: '3 commits tirés', kind: 'ok' })]));
    expect(backend.called('git_pull')[0].args).toEqual({ projectId: 'p1' });
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Branches' })).toBeNull());
  });

  it('shows the push running and allows nothing else meanwhile', async () => {
    let finish!: (summary: string) => void;
    const backend = fakeBackend({ branch_list: () => LIST, git_push: () => new Promise((r) => (finish = r)) });
    app.git = { p1: gitInfo({ ...tracked, ahead: 1 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    await picker();
    await userEvent.click(screen.getByRole('button', { name: /Pousser/ }));
    await waitFor(() => expect(syncButton()).toBeDisabled());
    expect(syncButton()).toHaveTextContent('Envoi…');
    expect(backend.called('git_push')[0].args).toEqual({ projectId: 'p1' });
    finish('1 commit poussé');
    await waitFor(() => expect(syncButton()).toBeEnabled());
    expect(app.toasts.map((t) => [t.text, t.kind])).toEqual([['1 commit poussé', 'ok']]);
  });

  it('fetches on demand', async () => {
    const backend = fakeBackend({ branch_list: () => LIST, git_fetch: () => 'Fetch terminé : déjà à jour' });
    app.git = { p1: gitInfo(tracked) };
    render(StatusBar);
    await userEvent.click(syncButton());
    await picker();
    await userEvent.click(screen.getByRole('button', { name: /Fetch/ }));
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['Fetch terminé : déjà à jour']));
    expect(backend.called('git_fetch')[0].args).toEqual({ projectId: 'p1' });
  });

  it('reports a failed pull as an error', async () => {
    const refusal = 'La branche locale et origin/main ont divergé : pull impossible en avance rapide.';
    fakeBackend({
      branch_list: () => LIST,
      git_pull: () => {
        throw refusal;
      },
    });
    app.git = { p1: gitInfo({ ...tracked, ahead: 1, behind: 1 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    await picker();
    await userEvent.click(screen.getByRole('button', { name: /Récupérer/ }));
    await waitFor(() => expect(app.toasts).toEqual([expect.objectContaining({ text: refusal, kind: 'error' })]));
    await waitFor(() => expect(syncButton()).toBeEnabled());
  });
});

describe('StatusBar update', () => {
  beforeEach(() => {
    resetApp();
    app.now = Date.UTC(2026, 9, 10, 9, 0, 0);
  });

  it('shows the update downloading, then offers the restart that installs it', async () => {
    fakeBackend();
    app.update = { version: '1.6.0', notes: '- Nouveautés', ready: false };
    render(StatusBar);
    expect(screen.getByText('Mise à jour 1.6.0…')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Mise à jour/ })).toBeNull();

    app.update = { ...app.update, ready: true };
    await tick();
    await userEvent.click(screen.getByRole('button', { name: 'Mise à jour 1.6.0 prête · Redémarrer' }));
    expect(app.modal).toEqual({ kind: 'update' });
  });

  it('counts down to the automatic restart, which « Plus tard » calls off', async () => {
    const backend = fakeBackend();
    app.update = { version: '1.6.0', notes: '', ready: true };
    app.restartAt = app.now + 25_000;
    render(StatusBar);
    expect(screen.getByText('Redémarrage dans 25 s')).toBeInTheDocument();
    app.now += 1_000;
    await tick();
    expect(screen.getByText('Redémarrage dans 24 s')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Plus tard' }));
    expect(backend.called('update_postpone')).toHaveLength(1);
    expect(screen.queryByText(/Redémarrage dans/)).toBeNull();
    expect(screen.getByRole('button', { name: 'Mise à jour 1.6.0 prête · Redémarrer' })).toBeInTheDocument();
  });
});

describe('StatusBar day cost', () => {
  it('includes what running turns cost so far, marked as an estimate', () => {
    resetApp({ agents: [agent({ id: 'a1', status: 'running', liveCost: 0.5 }), agent({ id: 'a2', liveCost: 0.25 })] });
    app.usage = { fiveHour: null, sevenDay: null, todayCost: 1, updatedAt: 1, accounts: [], current: 'principal' };
    fakeBackend();
    render(StatusBar);
    expect(screen.getByText('≈ 1,75 $')).toBeInTheDocument();
  });
});

describe('StatusBar in English', () => {
  beforeEach(() => {
    resetApp({
      agents: [
        agent({ id: 'a1', status: 'running' }),
        agent({ id: 'a2', status: 'waiting' }),
        agent({ id: 'a3', status: 'waiting', archived: true }),
        agent({ id: 'a4', status: 'done' }),
      ],
    });
    app.now = Date.UTC(2026, 8, 27, 20, 0, 0);
    menu.close();
    setLang('en');
  });

  it('counts the agents, and writes the quotas, their reset and the day cost in English', () => {
    fakeBackend();
    app.usage = {
      fiveHour: { pct: 62, resetsAt: app.now + (1 * 3600 + 48 * 60) * 1000 },
      sevenDay: { pct: 38.4, resetsAt: app.now + (2 * 86400 + 5 * 3600) * 1000 },
      todayCost: 4.12,
      updatedAt: 1,
      accounts: [],
      current: 'principal',
    };
    render(StatusBar);
    expect(screen.getByText('1 active')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /1 waiting/ })).toHaveAttribute(
      'title',
      'Go to the next agent that is waiting or needs a look (Ctrl+J)',
    );
    expect(screen.getByText('1 done')).toBeInTheDocument();
    const session = screen.getByText(/5-hour session/).closest('.it')!;
    expect(session).toHaveTextContent('62%');
    expect(session).toHaveTextContent('resets in 1h48');
    // A date, in the language: the day depends on the time zone of the machine.
    expect(session.getAttribute('title')).toMatch(/^Resets: September 2[78], 2026 at \d{1,2}:\d{2}\s[AP]M$/);
    const week = screen.getByText(/Weekly/).closest('.it')!;
    expect(week).toHaveTextContent('38%');
    expect(week).toHaveTextContent('resets in 2d 5h');
    expect(screen.getByText(/Today/)).toHaveTextContent('Today $4.12');
  });

  it('says when a quota is unknown', () => {
    fakeBackend();
    render(StatusBar);
    expect(screen.getByText(/5-hour session/).closest('.it')).toHaveAttribute('title', 'Session quota unavailable');
    expect(screen.getByText(/Weekly/).closest('.it')).toHaveAttribute('title', 'Weekly quota unavailable');
  });

  it('writes the Claude processes with their sizes in English, each agent in the tooltip', () => {
    fakeBackend();
    const GB = 1024 ** 3;
    app.resources = {
      instances: 2,
      memory: 1.5 * GB,
      cpu: 12.4,
      agents: [{ id: 'a1', memory: 0.5 * GB, cpu: 2.4 }],
    };
    render(StatusBar);
    const item = screen.getByText(/2 Claude/).closest('.it')!;
    expect(item).toHaveTextContent('2 Claude · 1.5 GB · 12% CPU');
    expect(item.getAttribute('title')!.split('\n')[0]).toBe('Running Claude processes (with the tools and MCP servers they start)');
    expect(item.getAttribute('title')!.split('\n')[1]).toBe('refacto-auth: 512 MB · 2%');
  });

  it('marks the day cost as an estimate while a turn runs, and says why', () => {
    app.agents.a1 = { ...app.agents.a1, liveCost: 0.5 };
    fakeBackend();
    render(StatusBar);
    expect(screen.getByText(/Today/)).toHaveAttribute(
      'title',
      'Estimate (public prices) while Claude works; exact cost at the end of the turn',
    );
    expect(screen.getByText('≈ $0.50')).toBeInTheDocument();
  });

  it('writes the sync with the remote, the picker’s foot and the toast in English', async () => {
    const backend = fakeBackend({ branch_list: () => [branchInfo({ name: 'main', current: true })], git_pull: () => '3 commits pulled' });
    app.git = { p1: gitInfo({ upstream: 'origin/main', hasRemote: true, behind: 3, ahead: 1, lastFetch: app.now - 3 * 60_000 }) };
    render(StatusBar);
    const button = screen.getByRole('button', { name: /⎇ main/ });
    expect(button.title).toBe('Tracking origin/main: 3 to pull, 1 to push\nLast fetch: 3 min ago');
    await userEvent.click(button);
    await screen.findByRole('dialog', { name: 'Branches' });
    expect(screen.getByRole('button', { name: /Push/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Fetch/ })).toHaveTextContent('now');
    await userEvent.click(screen.getByRole('button', { name: /Pull/ }));
    await waitFor(() => expect(app.toasts).toEqual([expect.objectContaining({ text: '3 commits pulled', kind: 'ok' })]));
    expect(backend.called('git_pull')).toHaveLength(1);
  });

  it('says a branch is not published, or was deleted from the remote, that it was never fetched, and a detached HEAD', async () => {
    fakeBackend({ branch_list: () => [] });
    app.git = { p1: gitInfo({ hasRemote: true, branch: 'feat/x' }) };
    render(StatusBar);
    const button = screen.getByRole('button', { name: /⎇ feat\/x/ });
    expect(button).toHaveTextContent('not published');
    expect(button.title).toBe('Branch not published to the remote repository yet\nLast fetch: never');
    await userEvent.click(button);
    await screen.findByRole('dialog', { name: 'Branches' });
    expect(screen.getByRole('button', { name: 'Publish' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    app.git = { p1: gitInfo({ hasRemote: true, branch: 'feat/x', upstream: 'origin/feat/x', upstreamGone: true }) };
    await tick();
    expect(screen.getByRole('button', { name: /⎇ feat\/x/ })).toHaveTextContent('remote deleted');
    expect(screen.getByRole('button', { name: /⎇ feat\/x/ }).title).toContain(
      'The tracked branch origin/feat/x no longer exists on the remote repository',
    );
    app.git = { p1: gitInfo({ hasRemote: false, branch: '(detached)' }) };
    await tick();
    expect(screen.getByRole('button', { name: /⎇ Detached HEAD/ }).title).toBe('Switch branch');
  });

  it('writes the update, its countdown and the sound in English', async () => {
    fakeBackend();
    app.update = { version: '1.6.0', notes: '', ready: true };
    app.restartAt = app.now + 25_000;
    render(StatusBar);
    expect(screen.getByText('Restarting in 25 s')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Later' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '♪ On' })).toHaveAttribute('title', 'Notification sound');
    expect(screen.getByRole('button', { name: '⚙' })).toHaveAttribute('title', 'Settings (Ctrl+,)');
    app.restartAt = null;
    await tick();
    expect(screen.getByRole('button', { name: 'Update 1.6.0 ready · Restart' })).toBeInTheDocument();
    app.update = { version: '1.6.0', notes: '', ready: false };
    await tick();
    expect(screen.getByText('Update 1.6.0…')).toBeInTheDocument();
  });
});
