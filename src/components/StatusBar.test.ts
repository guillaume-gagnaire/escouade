import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync, tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import type { Account, AccountUsage, EngineDecl, RateWindow, Settings } from '../lib/types';
import { agent, BARE_ENGINE, branchInfo, CLAUDE_ENGINE, fakeBackend, gitInfo, resetApp, usageOf, usageWith } from '../test/ipc';
import StatusBar from './StatusBar.svelte';
import { getByTip, tabTo } from '../test/tip';

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

  it('shows the running agent processes with their memory and CPU, each agent in the tooltip', () => {
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
    const item = screen.getByText(/2 agents/).closest('.it')!;
    expect(item).toHaveTextContent('2 agents · 1,5 Go · 12 % CPU');
    const title = item.getAttribute('data-tip')!;
    // The biggest first.
    expect(title.split('\n').slice(1)).toEqual(['tests-e2e : 1 Go · 10 %', 'refacto-auth : 512 Mo · 2 %']);
  });

  it('says nothing about processes when none runs', () => {
    fakeBackend();
    render(StatusBar);
    expect(screen.queryByText(/agents? ·/)).not.toBeInTheDocument();
  });

  it('shows when the weekly quota resets too', () => {
    fakeBackend();
    app.usage = usageWith({
      fiveHour: { pct: 10, resetsAt: null },
      sevenDay: { pct: 38.4, resetsAt: app.now + (2 * 86400 + 5 * 3600) * 1000 },
      todayCost: 0,
      updatedAt: 1,
      accounts: [],
      current: 'principal',
    });
    render(StatusBar);
    expect(screen.getByText('7 j')).toBeInTheDocument();
    // The reset is not written in the bar (the panel writes it): the value of the bar, and its tooltip, say when. The
    // day depends on the time zone of the machine.
    expect(screen.queryByText(/^reset/)).not.toBeInTheDocument();
    const week = screen.getByRole('meter', { name: 'Quota sur 7 jours' });
    expect(week).toHaveAttribute('aria-valuenow', '38');
    expect(week.getAttribute('aria-valuetext')).toMatch(
      /^38 % · remise à zéro le (29|30)\/09 à \d{2}:\d{2} · projection à la remise à zéro : 56 %$/,
    );
  });

  it('counts active, waiting and finished agents (archived excluded)', () => {
    fakeBackend();
    render(StatusBar);
    expect(screen.getByText('actif').closest('.it')).toHaveTextContent('1 actif');
    expect(screen.getByRole('button', { name: /1 en attente/ })).toHaveTextContent('1 en attente Ctrl J');
    expect(screen.getByText('terminé').closest('.it')).toHaveTextContent('1 terminé');
    // An icon, not a character, says « terminé ».
    expect(screen.getByText('terminé').closest('.it')!.querySelector('[data-icon="check"]')).not.toBeNull();
  });

  it('shows the quotas with their percentages, the reset in their tooltips, and the day cost', () => {
    fakeBackend();
    app.usage = usageWith({
      fiveHour: { pct: 62, resetsAt: app.now + (1 * 3600 + 48 * 60) * 1000 },
      sevenDay: { pct: 38.4, resetsAt: null },
      todayCost: 4.12,
      updatedAt: 1,
      accounts: [],
      current: 'principal',
    });
    render(StatusBar);
    expect(screen.getByText('5 h')).toBeInTheDocument();
    expect(screen.queryByText('reset 1h48')).not.toBeInTheDocument();
    expect(screen.getByText('5 h').closest('.q')).toHaveAttribute('data-tip', expect.stringMatching(/^62 % · remise à zéro le /));
    expect(screen.getByRole('meter', { name: 'Quota sur 5 heures' })).toHaveAttribute('aria-valuenow', '62');
    expect(screen.getByRole('meter', { name: 'Quota sur 7 jours' })).toHaveAttribute('aria-valuenow', '38');
    // The percentages are written after the bars (lot 0 of the 2.0 design), hidden under 1240 pixels.
    expect(screen.getByText('62 %')).toHaveClass('pct');
    expect(screen.getByText('38 %')).toHaveClass('pct');
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
    const sound = screen.getByRole('button', { name: 'Son des notifications' });
    expect(sound).toHaveAttribute('aria-pressed', 'true');
    expect(sound.querySelector('[data-icon="sound"]')).not.toBeNull();
    await userEvent.click(sound);
    expect(sound).toHaveAttribute('aria-pressed', 'false');
    expect(sound.querySelector('[data-icon="mute"]')).not.toBeNull();
    expect(backend.called('save_settings')[0].args.settings.sound).toBe(false);
  });

  it('gives the focus to the card of the waiting agent, as Ctrl+J does, the conversation open staying', async () => {
    fakeBackend();
    render(StatusBar);
    const open = app.agent?.id;
    await userEvent.click(screen.getByRole('button', { name: /1 en attente/ }));
    expect(app.cardFocus.id).toBe('a2');
    expect(app.agent?.id).toBe(open);
  });
});

describe('StatusBar branch and sync with the remote', () => {
  const tracked = { upstream: 'origin/main', hasRemote: true };
  const syncButton = () => screen.getByRole('button', { name: /^main/ });
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
    // The way they are in the picker: ahead first; arrows drawn as icons, said in words to a screen reader.
    expect(syncButton()).toHaveAccessibleName('main 1 commit à pousser 3 commits à tirer');
    expect([...syncButton().querySelectorAll('[data-icon]')].map((i) => i.getAttribute('data-icon'))).toEqual(['branch', 'up', 'pull']);
    expect(syncButton().dataset.tip).toContain('origin/main');
    expect(syncButton().dataset.tip).toContain('Dernier fetch : il y a 3 min');
  });

  it('shows the branch of any repository, with no sync to say when it has no remote, and a detached HEAD', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ hasRemote: false }) };
    render(StatusBar);
    expect(syncButton()).toHaveTextContent(/^main$/);
    expect(syncButton().dataset.tip).toBe('Changer de branche');
    app.git = { p1: gitInfo({ ...tracked, branch: '(detached)', upstream: null }) };
    await tick();
    expect(screen.getByRole('button', { name: /^HEAD détachée/ })).toHaveTextContent(/^HEAD détachée$/);
    app.git = { p1: gitInfo({ isRepo: false, branch: '' }) };
    await tick();
    expect(screen.queryByRole('button', { name: /main|HEAD/ })).toBeNull();
    app.git = { p1: gitInfo(tracked) };
    await tick();
    expect(syncButton()).toBeInTheDocument();
    app.ui.view = 'stats';
    await tick();
    expect(screen.queryByRole('button', { name: /main|HEAD/ })).toBeNull();
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
    expect(screen.getByRole('button', { name: /^feat\/x/ })).toHaveTextContent('non publiée');
    app.git = { p1: gitInfo({ hasRemote: true, branch: 'feat/x', upstream: 'origin/feat/x', upstreamGone: true }) };
    await tick();
    const button = screen.getByRole('button', { name: /^feat\/x/ });
    expect(button).toHaveTextContent('distante supprimée');
    expect(button.dataset.tip).toContain("origin/feat/x n'existe plus");
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

  it('gives the focus back to the branch button once a sync from the picker is over', async () => {
    let finish!: (summary: string) => void;
    fakeBackend({ branch_list: () => LIST, git_push: () => new Promise((r) => (finish = r)) });
    app.git = { p1: gitInfo({ ...tracked, ahead: 1 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    await picker();
    await userEvent.click(screen.getByRole('button', { name: /Pousser/ }));
    await waitFor(() => expect(syncButton()).toBeDisabled());
    // A browser takes the focus off a control that becomes disabled: the keyboard would be lost.
    (document.activeElement as HTMLElement | null)?.blur();
    expect(document.body).toHaveFocus();
    finish('1 commit poussé');
    await waitFor(() => expect(syncButton()).toBeEnabled());
    expect(syncButton()).toHaveFocus();
  });

  it('leaves the focus where the user put it while a sync ran', async () => {
    let finish!: (summary: string) => void;
    fakeBackend({ branch_list: () => LIST, git_push: () => new Promise((r) => (finish = r)) });
    app.git = { p1: gitInfo({ ...tracked, ahead: 1 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    await picker();
    await userEvent.click(screen.getByRole('button', { name: /Pousser/ }));
    await waitFor(() => expect(syncButton()).toBeDisabled());
    const sound = screen.getByRole('button', { name: 'Son des notifications' });
    sound.focus();
    finish('1 commit poussé');
    await waitFor(() => expect(syncButton()).toBeEnabled());
    expect(sound).toHaveFocus();
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
    app.usage = usageWith({ fiveHour: null, sevenDay: null, todayCost: 1, updatedAt: 1, accounts: [], current: 'principal' });
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
    app.usage = usageWith({
      fiveHour: { pct: 62, resetsAt: app.now + (1 * 3600 + 48 * 60) * 1000 },
      sevenDay: { pct: 38.4, resetsAt: app.now + (2 * 86400 + 5 * 3600) * 1000 },
      todayCost: 4.12,
      updatedAt: 1,
      accounts: [],
      current: 'principal',
    });
    render(StatusBar);
    expect(screen.getByText('active').closest('.it')).toHaveTextContent('1 active');
    expect(screen.getByRole('button', { name: /1 waiting/ })).toHaveAttribute(
      'data-tip',
      'Go to the card of the next agent that is waiting or needs a look (Ctrl+J)',
    );
    expect(screen.getByText('done').closest('.it')).toHaveTextContent('1 done');
    expect(screen.getByText('5h')).toBeInTheDocument();
    expect(screen.getByText('W')).toBeInTheDocument();
    expect(screen.getByText('62%')).toBeInTheDocument();
    expect(screen.getByText('W').closest('.q')).toHaveAttribute('data-tip', expect.stringMatching(/^38% · resets on /));
    const session = screen.getByRole('meter', { name: '5-hour quota' });
    expect(session).toHaveAttribute('aria-valuenow', '62');
    // A date, in the language: the day depends on the time zone of the machine.
    expect(session.getAttribute('aria-valuetext')).toMatch(
      /^62% · resets on 09\/2[78] at \d{1,2}:\d{2}\s[AP]M · projected at the reset: 97%$/,
    );
    expect(screen.getByRole('meter', { name: '7-day quota' })).toHaveAttribute('aria-valuenow', '38');
    expect(screen.getByText(/Today/)).toHaveTextContent('Today $4.12');
  });

  it('says when a quota is unknown', () => {
    fakeBackend();
    render(StatusBar);
    expect(screen.getByRole('meter', { name: '5-hour quota' })).toHaveAttribute('aria-valuetext', 'Quota unavailable');
    expect(screen.getByRole('meter', { name: '7-day quota' })).toHaveAttribute('aria-valuetext', 'Quota unavailable');
  });

  it('writes the agent processes with their sizes in English, each agent in the tooltip', () => {
    fakeBackend();
    const GB = 1024 ** 3;
    app.resources = {
      instances: 2,
      memory: 1.5 * GB,
      cpu: 12.4,
      agents: [{ id: 'a1', memory: 0.5 * GB, cpu: 2.4 }],
    };
    render(StatusBar);
    const item = screen.getByText(/2 agents/).closest('.it')!;
    expect(item).toHaveTextContent('2 agents · 1.5 GB · 12% CPU');
    expect(item.getAttribute('data-tip')!.split('\n')[0]).toBe('Running agent processes (with the tools and MCP servers they start)');
    expect(item.getAttribute('data-tip')!.split('\n')[1]).toBe('refacto-auth: 512 MB · 2%');
  });

  it('marks the day cost as an estimate while a turn runs, and says why', () => {
    app.agents.a1 = { ...app.agents.a1, liveCost: 0.5 };
    fakeBackend();
    render(StatusBar);
    expect(screen.getByText(/Today/)).toHaveAttribute('data-tip', 'Estimate (public prices) during the turn; exact cost at its end');
    expect(screen.getByText('≈ $0.50')).toBeInTheDocument();
  });

  it('writes the sync with the remote, the picker’s foot and the toast in English', async () => {
    const backend = fakeBackend({ branch_list: () => [branchInfo({ name: 'main', current: true })], git_pull: () => '3 commits pulled' });
    app.git = { p1: gitInfo({ upstream: 'origin/main', hasRemote: true, behind: 3, ahead: 1, lastFetch: app.now - 3 * 60_000 }) };
    render(StatusBar);
    const button = screen.getByRole('button', { name: /^main/ });
    expect(button.dataset.tip).toBe('Tracking origin/main: 3 to pull, 1 to push\nLast fetch: 3 min ago');
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
    const button = screen.getByRole('button', { name: /^feat\/x/ });
    expect(button).toHaveTextContent('not published');
    expect(button.dataset.tip).toBe('Branch not published to the remote repository yet\nLast fetch: never');
    await userEvent.click(button);
    await screen.findByRole('dialog', { name: 'Branches' });
    expect(screen.getByRole('button', { name: 'Publish' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    app.git = { p1: gitInfo({ hasRemote: true, branch: 'feat/x', upstream: 'origin/feat/x', upstreamGone: true }) };
    await tick();
    expect(screen.getByRole('button', { name: /^feat\/x/ })).toHaveTextContent('remote deleted');
    expect(screen.getByRole('button', { name: /^feat\/x/ }).dataset.tip).toContain(
      'The tracked branch origin/feat/x no longer exists on the remote repository',
    );
    app.git = { p1: gitInfo({ hasRemote: false, branch: '(detached)' }) };
    await tick();
    expect(screen.getByRole('button', { name: /^Detached HEAD/ }).dataset.tip).toBe('Switch branch');
  });

  it('writes the update, its countdown and the sound in English', async () => {
    fakeBackend();
    app.update = { version: '1.6.0', notes: '', ready: true };
    app.restartAt = app.now + 25_000;
    render(StatusBar);
    expect(screen.getByText('Restarting in 25 s')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Later' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Notification sound' })).toHaveAttribute('data-tip', 'Notification sound: On');
    // The settings are on the projects' rail now (ESC-158).
    expect(screen.queryByRole('button', { name: /Settings/ })).not.toBeInTheDocument();
    app.restartAt = null;
    await tick();
    expect(screen.getByRole('button', { name: 'Update 1.6.0 ready · Restart' })).toBeInTheDocument();
    app.update = { version: '1.6.0', notes: '', ready: false };
    await tick();
    expect(screen.getByText('Update 1.6.0…')).toBeInTheDocument();
  });
});

describe('StatusBar quotas of the accounts', () => {
  const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
  const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };
  const NOW = new Date(2026, 9, 10, 14, 59).getTime();
  const win = (pct: number, hours: number): RateWindow => ({ pct, resetsAt: NOW + hours * 3_600_000 });
  const read = (id: string, over: Parameters<typeof usageOf>[1] = {}): AccountUsage => usageOf(id, over);
  const two = () => {
    app.settings.accounts = [PRINCIPAL, PRO];
    app.settings.quotaPause = 90;
    // Pro is the current account: the bars are its windows, whatever the other account's are.
    app.usage = usageWith({
      fiveHour: win(42, 3.0167),
      sevenDay: win(12, 108),
      todayCost: 4.12,
      updatedAt: 1,
      current: 'pro',
      accounts: [
        read('principal', { fiveHour: win(95, 1), sevenDay: win(30, 100) }),
        read('pro', { fiveHour: win(42, 3.0167), sevenDay: win(12, 108) }),
      ],
    });
  };
  const group = () => screen.getByRole('button', { name: 'Quotas par compte (compte en cours : Pro)' });
  const panel = () => screen.queryByRole('dialog', { name: 'Quotas des comptes' });

  beforeEach(() => {
    resetApp();
    fakeBackend();
    menu.close();
    app.now = NOW;
    app.usage = usageWith({ ...app.usage, fiveHour: win(42, 3.0167), sevenDay: win(12, 108), todayCost: 4.12 });
  });

  it('is a button with a single account too, named by its harness, which opens the panel and its projections', async () => {
    render(StatusBar);
    const button = screen.getByRole('button', { name: 'Quotas par compte' });
    // The harness in front, not the account.
    expect(button).toHaveTextContent(/^Claude\s*5 h/);
    expect(screen.queryByText('Principal')).not.toBeInTheDocument();
    // The button takes the focus, not its bars.
    expect(screen.getAllByRole('meter').every((m) => !m.hasAttribute('tabindex'))).toBe(true);
    await userEvent.click(button);
    expect(panel()).toBeInTheDocument();
  });

  it('is there before the settings are, as the window is: no account to compare yet', () => {
    app.settings = {} as Settings;
    render(StatusBar);
    expect(screen.getAllByRole('meter')).toHaveLength(2);
    expect(screen.getByRole('button', { name: 'Quotas par compte' })).toBeInTheDocument();
  });

  it('names the harness in front of its gauges, and the current account in the button’s name, with several accounts', () => {
    two();
    render(StatusBar);
    expect(group()).toHaveTextContent(/^Claude\s*5 h\s*42 %/);
    expect(group()).toHaveTextContent('7 j');
    expect(group()).toHaveTextContent('12 %');
    // The resets are in the panel and the tooltips, not in the bar.
    expect(group()).not.toHaveTextContent('reset');
    expect(group()).toHaveAttribute('aria-expanded', 'false');
    expect(group()).toHaveAttribute('aria-haspopup', 'dialog');
    expect(group().getAttribute('aria-controls')).toBeTruthy();
    // The two bars are the current account's; a button around them takes the focus instead of them.
    const meters = within(group()).getAllByRole('meter');
    expect(meters.map((m) => m.getAttribute('aria-valuenow'))).toEqual(['42', '12']);
    expect(meters.every((m) => !m.hasAttribute('tabindex'))).toBe(true);
  });

  it('says the account that goes on in extra usage, while the bar shows the one new agents go to (ESC-147)', () => {
    two();
    app.usage = { ...app.usage, accounts: app.usage.accounts.map((u) => (u.id === 'principal' ? { ...u, overage: true } : u)) };
    render(StatusBar);
    expect(document.getElementById('quota-description')).toHaveTextContent(
      /^Principal : usage supplémentaire en cours, facturé à part. Les agents déjà lancés y restent/,
    );
  });

  it('says new agents go on the account in extra usage too, when it is the one they go to (ESC-147)', () => {
    two();
    // Pro is the current account: it is the one that goes on in extra usage, no other takes the new agents.
    app.usage = { ...app.usage, accounts: app.usage.accounts.map((u) => (u.id === 'pro' ? { ...u, overage: true } : u)) };
    render(StatusBar);
    const said = document.getElementById('quota-description')!;
    expect(said).toHaveTextContent(
      /^Pro\s*: usage supplémentaire en cours, facturé à part\. Les agents déjà lancés y restent, et les nouveaux y partent aussi\./,
    );
    expect(said).not.toHaveTextContent('partent sur un autre compte');
  });

  it('says the agents in extra usage leave it at the end of their turn when « Changer de compte à » is set (ESC-148)', () => {
    two();
    app.settings.switchAt = 95;
    app.usage = { ...app.usage, accounts: app.usage.accounts.map((u) => (u.id === 'principal' ? { ...u, overage: true } : u)) };
    render(StatusBar);
    const said = document.getElementById('quota-description')!;
    expect(said).toHaveTextContent(
      /^Principal\s*: usage supplémentaire en cours, facturé à part\. Ses agents passent sur un autre compte à la fin de leur tour, et les nouveaux y partent\./,
    );
    expect(said).not.toHaveTextContent('y restent');
  });

  it('says nothing of extra usage when no account is in it', () => {
    two();
    render(StatusBar);
    expect(document.getElementById('quota-description')).not.toHaveTextContent(/supplémentaire/);
  });

  it('shows the tooltip of a bar when the pointer is over it, in the button too', async () => {
    two();
    render(StatusBar);
    await userEvent.hover(within(group()).getByRole('meter', { name: 'Quota sur 7 jours' }));
    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      /^12 % · remise à zéro le \d{2}\/\d{2} à \d{2}:\d{2}( · projection à la remise à zéro : \d+ %)?$/,
    );
    await userEvent.unhover(within(group()).getByRole('meter', { name: 'Quota sur 7 jours' }));
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('gives a screen reader the two windows’ values on the group button, which its bars no longer do one by one', () => {
    two();
    render(StatusBar);
    const said =
      /^Quota sur 5 heures : 42 % · remise à zéro le \d{2}\/\d{2} à \d{2}:\d{2}( · projection à la remise à zéro : \d+ %)?\. Quota sur 7 jours : 12 % · remise à zéro le \d{2}\/\d{2} à \d{2}:\d{2}( · projection à la remise à zéro : \d+ %)?$/;
    expect(group()).toHaveAccessibleDescription(said);
    // What the bars read is told as it follows the quota: a reading with a single window has a single bar (Codex's free
    // plan has one).
    app.usage = usageWith({ ...app.usage, fiveHour: null, sevenDay: win(80, 100) });
    flushSync();
    expect(group()).toHaveAccessibleDescription(
      /^Quota sur 7 jours : 80 % · remise à zéro le \d{2}\/\d{2} à \d{2}:\d{2}( · projection à la remise à zéro : \d+ %)?$/,
    );
    // Not read yet: the windows its harness declares, each said unavailable.
    app.usage = usageWith({ ...app.usage, fiveHour: null, sevenDay: null });
    flushSync();
    expect(group()).toHaveAccessibleDescription('Quota sur 5 heures : Quota indisponible. Quota sur 7 jours : Quota indisponible');
  });

  it('shows both values in a tooltip when the group button gets the keyboard focus, until it loses it or Escape is pressed', async () => {
    two();
    render(StatusBar);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    tabTo(group());
    const tip = await screen.findByRole('tooltip');
    expect(tip).toHaveTextContent(/Quota sur 5 heures : 42 % · remise à zéro le \d{2}\/\d{2} à \d{2}:\d{2}/);
    expect(tip).toHaveTextContent(/Quota sur 7 jours : 12 % · remise à zéro le \d{2}\/\d{2} à \d{2}:\d{2}/);
    // Escape puts it away without moving the focus (WCAG 1.4.13), and the panel is not opened.
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    expect(group()).toHaveFocus();
    expect(panel()).not.toBeInTheDocument();
    // It comes back with the next focus.
    group().blur();
    tabTo(group());
    expect(await screen.findByRole('tooltip')).toBeInTheDocument();
    group().blur();
    await waitFor(() => expect(screen.queryByRole('tooltip')).not.toBeInTheDocument());
  });

  it('shows both values in a tooltip when the pointer stays on the group button away from its bars', async () => {
    two();
    render(StatusBar);
    await userEvent.hover(within(group()).getByText('Claude'));
    const tip = await screen.findByRole('tooltip');
    expect(tip).toHaveTextContent(/Quota sur 5 heures : 42 %/);
    expect(tip).toHaveTextContent(/Quota sur 7 jours : 12 %/);
  });

  it('shows no tooltip on a click, which opens the panel instead', async () => {
    two();
    render(StatusBar);
    await userEvent.click(group());
    expect(panel()).toBeInTheDocument();
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    // Enter on the focused button opens the panel too, and the tooltip it showed goes.
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(panel()).not.toBeInTheDocument());
    group().blur();
    tabTo(group());
    expect(await screen.findByRole('tooltip')).toBeInTheDocument();
    await userEvent.keyboard('{Enter}');
    expect(panel()).toBeInTheDocument();
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('says a long name of the current account in the button’s name, never in the bar', () => {
    two();
    const name = 'Équipe de développement de la plateforme de paiement (compte partagé)';
    app.settings.accounts = [PRINCIPAL, { ...PRO, name }];
    render(StatusBar);
    const button = screen.getByRole('button', { name: `Quotas par compte (compte en cours : ${name})` });
    expect(within(button).queryByText(name)).not.toBeInTheDocument();
  });

  it('names the current account as it follows the quota: another account takes the front when it becomes the current one', async () => {
    two();
    render(StatusBar);
    app.usage = usageWith({ ...app.usage, current: 'principal', fiveHour: win(95, 1), sevenDay: win(30, 100) });
    await tick();
    const button = screen.getByRole('button', { name: 'Quotas par compte (compte en cours : Principal)' });
    expect(button).toHaveTextContent(/^Claude\s*5 h\s*95 %/);
    expect(
      within(button)
        .getAllByRole('meter')
        .map((m) => m.getAttribute('aria-valuenow')),
    ).toEqual(['95', '30']);
    expect(within(button).getAllByRole('meter')[0]).toHaveClass('hot');
  });

  it('opens the panel of the accounts over the bar on a click, with the focus in it', async () => {
    two();
    render(StatusBar);
    expect(panel()).not.toBeInTheDocument();
    await userEvent.click(group());
    const dialog = panel()!;
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveFocus();
    expect(group()).toHaveAttribute('aria-expanded', 'true');
    expect(group().getAttribute('aria-controls')).toBe(dialog.id);
    expect(
      within(dialog)
        .getAllByRole('group')
        .map((g) => g.getAttribute('aria-label')),
    ).toEqual(['Pro', 'Principal']);
    // Every account's windows, with the percentages written; the one past the threshold signalled.
    expect(within(dialog).getByRole('group', { name: 'Principal' })).toHaveTextContent('95 %');
    expect(within(dialog).getByRole('group', { name: 'Principal' })).toHaveTextContent('au-delà du seuil de pause');
    expect(within(dialog).getByRole('group', { name: 'Pro' })).toHaveTextContent('en cours');
  });

  it('opens with the keyboard', async () => {
    two();
    render(StatusBar);
    group().focus();
    await userEvent.keyboard('{Enter}');
    expect(panel()).toHaveFocus();
  });

  it('closes on Escape and puts the focus back on the button', async () => {
    two();
    render(StatusBar);
    await userEvent.click(group());
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(panel()).not.toBeInTheDocument());
    expect(group()).toHaveAttribute('aria-expanded', 'false');
    expect(group()).toHaveFocus();
  });

  it('puts the focus back on the button even when the click that opened the panel did not focus it, as in Safari', async () => {
    two();
    render(StatusBar);
    // A click that leaves the focus where it was: nothing for the panel to restore, the button is where it goes.
    await fireEvent.click(group());
    expect(group()).not.toHaveFocus();
    expect(panel()).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(panel()).not.toBeInTheDocument());
    expect(group()).toHaveFocus();
  });

  it('closes on a click outside it and puts the focus back on the button', async () => {
    two();
    const { baseElement } = render(StatusBar);
    await userEvent.click(group());
    // Over everything, what is under the pointer is the backdrop.
    await userEvent.click(baseElement.querySelector('.backdrop')!);
    await waitFor(() => expect(panel()).not.toBeInTheDocument());
    expect(group()).toHaveAttribute('aria-expanded', 'false');
    expect(group()).toHaveFocus();
  });

  it('closes on a new click on the group', async () => {
    two();
    render(StatusBar);
    await userEvent.click(group());
    expect(panel()).toBeInTheDocument();
    await userEvent.click(group());
    await waitFor(() => expect(panel()).not.toBeInTheDocument());
    expect(group()).toHaveFocus();
  });

  it('does not let Escape reach the rest of the window while the panel is open', async () => {
    two();
    render(StatusBar);
    const elsewhere = vi.fn();
    document.addEventListener('keydown', elsewhere);
    await userEvent.click(group());
    await userEvent.keyboard('{Escape}');
    document.removeEventListener('keydown', elsewhere);
    expect(elsewhere).not.toHaveBeenCalled();
  });

  it('keeps the panel open when the accounts are down to one, the button named without an account', async () => {
    two();
    render(StatusBar);
    await userEvent.click(group());
    app.settings.accounts = [PRINCIPAL];
    await tick();
    expect(panel()).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Quotas par compte' })).toHaveAttribute('aria-expanded', 'true');
  });

  it('leaves the cost of the day the total of every account', () => {
    two();
    render(StatusBar);
    expect(screen.getByText(/Aujourd'hui/)).toHaveTextContent(/Aujourd'hui\s+4,12/);
  });

  it('leaves the branch button as it was', async () => {
    two();
    app.git = { p1: gitInfo({ upstream: 'origin/main', hasRemote: true }) };
    render(StatusBar);
    expect(screen.getByRole('button', { name: /^main/ })).toBeInTheDocument();
    await userEvent.click(group());
    expect(panel()).toBeInTheDocument();
  });

  it('writes the group in English, Principal being Main', async () => {
    setLang('en');
    two();
    app.usage = usageWith({ ...app.usage, current: 'principal', fiveHour: win(95, 1), sevenDay: win(12, 108) });
    render(StatusBar);
    const button = screen.getByRole('button', { name: 'Quota by account (current account: Main)' });
    expect(button).toHaveTextContent(/^Claude\s*5h\s*95%/);
    expect(button).toHaveTextContent('W');
    expect(button).toHaveTextContent('12%');
    await userEvent.click(button);
    const dialog = screen.getByRole('dialog', { name: 'Accounts quota' });
    expect(within(dialog).getByRole('group', { name: 'Main' })).toHaveTextContent('current');
    expect(within(dialog).getByRole('group', { name: 'Main' })).toHaveTextContent('past the pause or switch threshold');
    expect(within(dialog).getByRole('group', { name: 'Pro' })).not.toHaveTextContent('past the pause or switch threshold');
  });
});

describe('StatusBar quotas of the harness shown (ESC-30)', () => {
  const NOW = new Date(2026, 9, 10, 14, 59).getTime();
  const at = (hours: number) => NOW + hours * 3_600_000;
  const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
  const WORK: Account = { id: 'work', name: 'Travail', engine: 'codex', configDir: 'C:\codex\work', claudePath: '', active: true };
  /** The bare harness with a single window of 30 days. */
  const MONTHLY: EngineDecl = { ...BARE_ENGINE, caps: { ...BARE_ENGINE.caps, quota: 'windows', quotaWindows: [43200] } };

  const setUp = (engine: 'claude' | 'codex', harness: EngineDecl = BARE_ENGINE) => {
    resetApp({ agents: [agent({ id: 'a1', engine })] });
    fakeBackend();
    app.now = NOW;
    app.engines = [CLAUDE_ENGINE, harness];
    app.settings.accounts = [PRINCIPAL, WORK];
    app.usage = {
      todayCost: 1,
      current: { claude: 'principal', codex: 'work' },
      accounts: [
        usageOf('principal', { fiveHour: { pct: 42, resetsAt: at(3) }, sevenDay: { pct: 12, resetsAt: at(100) } }),
        usageOf('work', { engine: 'codex', meters: [{ minutes: 43200, pct: 70, resetsAt: at(200) }] }),
      ],
    };
  };

  it('shows the accounts of Claude Code for an agent of Claude Code', () => {
    setUp('claude');
    render(StatusBar);
    expect(screen.getByRole('button', { name: 'Quotas par compte (compte en cours : Principal)' })).toBeInTheDocument();
    expect(screen.getByText('5 h')).toBeInTheDocument();
    expect(screen.getByText('7 j')).toBeInTheDocument();
  });

  it('shows the windows of the harness of the agent shown, labelled by their length', () => {
    setUp('codex', MONTHLY);
    render(StatusBar);
    expect(screen.getByRole('button', { name: 'Quotas par compte (compte en cours : Travail)' })).toBeInTheDocument();
    expect(screen.getByText('30 j')).toBeInTheDocument();
    // Only the harness at work: no agent of Claude Code.
    expect(screen.queryByText('5 h')).not.toBeInTheDocument();
    expect(screen.getByRole('meter', { name: 'Quota sur 30 jours' })).toHaveAttribute('aria-valuenow', '70');
  });

  it('shows no bar for a harness without a quota, only the cost of the day', () => {
    setUp('codex');
    render(StatusBar);
    // The harness's group: its cost of the day, no bar (Claude Code's group, the project's harness, has its own).
    const group = screen.getByText('Harness').closest('.grp')!;
    expect(group.querySelector('[role="meter"]')).toBeNull();
    expect(group).toHaveTextContent('0,00 $');
    expect(screen.getByText(/Aujourd'hui|Aujourd’hui/)).toBeInTheDocument();
  });

  it('shows the cost of the day of the account of a harness without a quota, out of its daily budget (ESC-45)', () => {
    setUp('codex');
    app.settings.accounts = [PRINCIPAL, { ...WORK, dailyBudget: 2 }];
    app.usage = {
      ...app.usage,
      accounts: [app.usage.accounts[0], usageOf('work', { engine: 'codex', todayCost: 2.5, dailyBudget: 2 })],
    };
    render(StatusBar);
    const button = screen.getByRole('button', { name: 'Quotas par compte (compte en cours : Travail)' });
    expect(button.textContent?.replace(/\s+/g, ' ')).toMatch(/2,50 \$ \/ 2,00 \$/);
    // Reached: told, the autopilot waits for midnight.
    expect(button.querySelector('.day.over')).not.toBeNull();
    expect(button.getAttribute('data-tip')?.replace(/\s+/g, ' ')).toContain(
      'Harness nu n’a pas de quota : son compte a coûté 2,50 $ aujourd’hui, sur un budget de 2,00 $ par jour',
    );
  });

  it('projects the cost of the day to midnight against its budget past 80 % only, unless « Projection des quotas » is off', () => {
    setUp('codex');
    app.settings.accounts = [{ ...WORK, dailyBudget: 2 }];
    app.usage = { ...app.usage, accounts: [usageOf('work', { engine: 'codex', todayCost: 0.75, dailyBudget: 2 })] };
    // 18:00 on the machine's clock: three quarters of the day gone.
    app.now = new Date(2026, 8, 27, 18).getTime();
    render(StatusBar);
    // 1,00 $ at midnight, half the budget: nothing to say in the bar (choice 21), the panel and the tooltip say it.
    expect(screen.queryByText(/→/)).not.toBeInTheDocument();
    const tip = () => screen.getByRole('button', { name: 'Quotas par compte' }).getAttribute('data-tip')?.replace(/\s+/g, ' ');
    expect(tip()).toContain('Projection à minuit : 1,00 $.');
    // « Toujours »: in the bar too, green.
    app.settings.quotaProjection = 'always';
    flushSync();
    expect(screen.getByText('→ 1,00 $')).toHaveClass('ok');
    app.settings.quotaProjection = 'alert';
    app.usage = { ...app.usage, accounts: [usageOf('work', { engine: 'codex', todayCost: 1.8, dailyBudget: 2 })] };
    flushSync();
    expect(screen.getByText('→ 2,40 $')).toHaveClass('over');
    app.settings.quotaProjection = 'off';
    flushSync();
    expect(screen.queryByText(/→/)).not.toBeInTheDocument();
    expect(tip()).not.toContain('Projection à minuit');
  });

  it('names the harness beside its cost of the day with a single account, and no budget', () => {
    setUp('codex');
    app.settings.accounts = [WORK];
    app.usage = { ...app.usage, accounts: [usageOf('work', { engine: 'codex', todayCost: 0.42 })] };
    render(StatusBar);
    const group = screen.getByText('Harness').closest('.grp')!;
    expect(group.textContent?.replace(/\s+/g, ' ').trim()).toMatch(/^Harness ?0,42 \$$/);
    expect(group.querySelector('.day.over')).toBeNull();
    expect(screen.getByRole('button', { name: 'Quotas par compte' }).getAttribute('data-tip')?.replace(/\s+/g, ' ')).toContain(
      'Harness nu n’a pas de quota : son compte a coûté 0,42 $ aujourd’hui, sans budget par jour.',
    );
  });
});

describe('StatusBar groups of gauges, harness by harness (lot 0 of the 2.0 design)', () => {
  const NOW = new Date(2026, 9, 10, 14, 59).getTime();
  const at = (hours: number) => NOW + hours * 3_600_000;
  const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
  const WORK: Account = { id: 'work', name: 'Travail', engine: 'codex', configDir: 'C:\codex\work', claudePath: '', active: true };
  const MONTHLY: EngineDecl = { ...BARE_ENGINE, caps: { ...BARE_ENGINE.caps, quota: 'windows', quotaWindows: [43200] } };

  beforeEach(() => {
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2', engine: 'codex' })] });
    fakeBackend();
    menu.close();
    app.now = NOW;
    app.engines = [CLAUDE_ENGINE, MONTHLY];
    app.settings.accounts = [PRINCIPAL, WORK];
    // No projection to read past the percentages.
    app.settings.quotaProjection = 'off';
    app.usage = {
      todayCost: 1,
      current: { claude: 'principal', codex: 'work' },
      accounts: [
        usageOf('principal', { fiveHour: { pct: 42, resetsAt: at(4.8) }, sevenDay: { pct: 12, resetsAt: at(100) } }),
        usageOf('work', { engine: 'codex', meters: [{ minutes: 43200, pct: 70, resetsAt: at(200) }] }),
      ],
    };
  });

  it('shows a group per harness at work, the one of the project first, each named by its harness', () => {
    render(StatusBar);
    const button = screen.getByRole('button', { name: 'Quotas par compte (compte en cours : Principal)' });
    const groups = [...button.querySelectorAll<HTMLElement>('.grp')];
    expect(groups.map((g) => g.dataset.engine)).toEqual(['claude', 'codex']);
    expect(groups[0]).toHaveTextContent(/^Claude\s*5 h\s*42 %\s*7 j\s*12 %$/);
    expect(groups[1]).toHaveTextContent(/^Harness\s*30 j\s*70 %$/);
    // Said harness by harness, with the account shown.
    expect(button).toHaveAccessibleDescription(
      /^Claude Code, Principal\. Quota sur 5 heures : 42 %.*Harness nu, Travail\. Quota sur 30 jours : 70 %/,
    );
  });

  it('leaves out a harness with no agent, but the one of the project', async () => {
    app.agents = { a1: app.agents.a1 };
    render(StatusBar);
    expect([...document.querySelectorAll<HTMLElement>('.grp')].map((g) => g.dataset.engine)).toEqual(['claude']);
  });

  it('writes « usage supplémentaire » in amber beside the harness whose account answers as extra usage (choice 22)', () => {
    app.usage = { ...app.usage, accounts: app.usage.accounts.map((u) => (u.id === 'work' ? { ...u, overage: true } : u)) };
    render(StatusBar);
    const [claude, codex] = [...document.querySelectorAll<HTMLElement>('.grp')];
    expect(within(codex).getByText('usage supplémentaire')).toHaveClass('over-word');
    expect(within(claude).queryByText('usage supplémentaire')).not.toBeInTheDocument();
  });

  it('offers the update ready with an icon, its restart in bold', async () => {
    app.update = { version: '1.9.0', notes: '', ready: true };
    render(StatusBar);
    const button = screen.getByRole('button', { name: 'Mise à jour 1.9.0 prête · Redémarrer' });
    expect(button.querySelector('[data-icon="download"]')).not.toBeNull();
    expect(within(button).getByText('Redémarrer').tagName).toBe('B');
    await userEvent.click(button);
    expect(app.modal).toEqual({ kind: 'update' });
  });

  it('has no button of the settings any more: they are on the projects’ rail (ESC-158)', () => {
    render(StatusBar);
    expect(screen.queryByRole('button', { name: /Réglages/ })).not.toBeInTheDocument();
  });
});

describe('StatusBar at the first start', () => {
  it('is empty while there is no project, as the rest of the shell (choice 49)', () => {
    resetApp({ projects: [] });
    fakeBackend();
    render(StatusBar);
    expect(document.querySelector('footer')).toBeEmptyDOMElement();
  });
});
