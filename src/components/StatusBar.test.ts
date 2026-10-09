import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, gitInfo, resetApp } from '../test/ipc';
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
    };
    render(StatusBar);
    const week = screen.getByText(/Hebdo/).closest('.it')!;
    expect(week).toHaveTextContent('38 %');
    expect(week).toHaveTextContent('reset 2j 5h');
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

describe('StatusBar sync with the remote', () => {
  const tracked = { upstream: 'origin/main', hasRemote: true };
  const syncButton = () => screen.getByRole('button', { name: /⎇ main/ });
  const item = (label: string) => menu.open!.items.find((i) => i.label === label)!;

  beforeEach(() => {
    resetApp();
    menu.close();
    app.now = Date.UTC(2026, 8, 27, 20, 0, 0);
  });

  it('shows the branch with the commits to pull and to push, and when it was last fetched', () => {
    fakeBackend();
    app.git = { p1: gitInfo({ ...tracked, behind: 3, ahead: 1, lastFetch: app.now - 3 * 60_000 }) };
    render(StatusBar);
    expect(syncButton()).toHaveTextContent('↓3');
    expect(syncButton()).toHaveTextContent('↑1');
    expect(syncButton().title).toContain('origin/main');
    expect(syncButton().title).toContain('Dernier fetch : il y a 3 min');
  });

  it('only shows up in a project whose repository has a remote and a branch checked out', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ hasRemote: false }) };
    render(StatusBar);
    expect(screen.queryByText(/⎇/)).toBeNull();
    app.git = { p1: gitInfo({ ...tracked, branch: '(detached)', upstream: null }) };
    await tick();
    expect(screen.queryByText(/⎇/)).toBeNull();
    app.git = { p1: gitInfo(tracked) };
    await tick();
    expect(syncButton()).toBeInTheDocument();
    app.ui.view = 'stats';
    await tick();
    expect(screen.queryByText(/⎇/)).toBeNull();
  });

  it('offers to pull and push only what there is to pull or push', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ ...tracked, ahead: 2 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Pull', 'Push', '', 'Fetch']);
    expect(item('Pull')).toMatchObject({ hint: '↓0', disabled: true });
    expect(item('Push')).toMatchObject({ hint: '↑2', disabled: false });
    expect(item('Fetch')).toMatchObject({ hint: 'maintenant' });
    expect(item('Fetch').disabled).toBeFalsy();
  });

  it('offers to publish a branch that tracks none', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ hasRemote: true, branch: 'feat/x' }) };
    render(StatusBar);
    const button = screen.getByRole('button', { name: /⎇ feat\/x/ });
    expect(button).toHaveTextContent('non publiée');
    await userEvent.click(button);
    expect(item('Pull').disabled).toBe(true);
    expect(item('Publier la branche').disabled).toBeFalsy();
  });

  it('offers to publish again a branch deleted from the remote', async () => {
    fakeBackend();
    app.git = { p1: gitInfo({ hasRemote: true, branch: 'feat/x', upstream: 'origin/feat/x', upstreamGone: true }) };
    render(StatusBar);
    const button = screen.getByRole('button', { name: /⎇ feat\/x/ });
    expect(button).toHaveTextContent('distante supprimée');
    expect(button.title).toContain("origin/feat/x n'existe plus");
    await userEvent.click(button);
    expect(item('Pull').disabled).toBe(true);
    expect(item('Publier la branche').disabled).toBeFalsy();
  });

  it('pulls, then says what came in', async () => {
    const backend = fakeBackend({ git_pull: () => '3 commits tirés' });
    app.git = { p1: gitInfo({ ...tracked, behind: 3 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    expect(item('Pull').disabled).toBe(false);
    item('Pull').onClick!();
    await waitFor(() => expect(app.toasts).toEqual([expect.objectContaining({ text: '3 commits tirés', kind: 'ok' })]));
    expect(backend.called('git_pull')[0].args).toEqual({ projectId: 'p1' });
  });

  it('shows the push running and allows nothing else meanwhile', async () => {
    let finish!: (summary: string) => void;
    const backend = fakeBackend({ git_push: () => new Promise((r) => (finish = r)) });
    app.git = { p1: gitInfo({ ...tracked, ahead: 1 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    item('Push').onClick!();
    await waitFor(() => expect(syncButton()).toBeDisabled());
    expect(syncButton()).toHaveTextContent('Push…');
    expect(backend.called('git_push')[0].args).toEqual({ projectId: 'p1' });
    finish('1 commit poussé');
    await waitFor(() => expect(syncButton()).toBeEnabled());
    expect(app.toasts.map((t) => [t.text, t.kind])).toEqual([['1 commit poussé', 'ok']]);
  });

  it('fetches on demand', async () => {
    const backend = fakeBackend({ git_fetch: () => 'Fetch terminé : déjà à jour' });
    app.git = { p1: gitInfo(tracked) };
    render(StatusBar);
    await userEvent.click(syncButton());
    item('Fetch').onClick!();
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['Fetch terminé : déjà à jour']));
    expect(backend.called('git_fetch')[0].args).toEqual({ projectId: 'p1' });
  });

  it('reports a failed pull as an error', async () => {
    const refusal = 'La branche locale et origin/main ont divergé : pull impossible en avance rapide.';
    fakeBackend({
      git_pull: () => {
        throw refusal;
      },
    });
    app.git = { p1: gitInfo({ ...tracked, ahead: 1, behind: 1 }) };
    render(StatusBar);
    await userEvent.click(syncButton());
    item('Pull').onClick!();
    await waitFor(() => expect(app.toasts).toEqual([expect.objectContaining({ text: refusal, kind: 'error' })]));
    expect(syncButton()).toBeEnabled();
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
    app.usage = { fiveHour: null, sevenDay: null, todayCost: 1, updatedAt: 1 };
    fakeBackend();
    render(StatusBar);
    expect(screen.getByText('≈ 1,75 $')).toBeInTheDocument();
  });
});
