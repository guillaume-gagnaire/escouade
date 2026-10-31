// The agent's view « Page » (ESC-311): its bar, what its buttons and its shortcut ask the backend, its states said in
// French and in English, and the page hidden when it cannot show.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { devpages } from '../lib/devpage.svelte';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import { flows } from '../lib/test-launch.svelte';
import type { Agent, LaunchState } from '../lib/types';
import { agent, fakeBackend, resetApp } from '../test/ipc';
import DevPage from './DevPage.svelte';

let A: Agent;
const URL1 = 'http://localhost:41731/users?search=marie@';

/** Its launch: a process running (`up`), or ended. */
function launch(up: boolean) {
  app.launches['test:a7:run:0'] = {
    status: up ? 'running' : 'crashed',
    ptyId: up ? 't1' : null,
    name: 'web',
    stopping: false,
    code: up ? null : 1,
    startedAt: 0,
  } as LaunchState;
  flows.all.a7 = {
    lines: [{ id: 'test:a7:run:0', label: 'web', state: up ? 'ready' : 'failed', detail: '', launchId: 'test:a7:run:0' }],
    phase: up ? 'ready' : 'failed',
    error: null,
    opened: URL1,
    where: 'agent',
  };
}

beforeEach(() => {
  setLang('fr');
  A = agent({ id: 'a7', name: 'pagination-users' });
  resetApp({ agents: [A] });
  app.launches = {};
  flows.all = {};
  devpages.pages = { a7: { url: URL1, title: 'pagination-users · web', phase: 'ready', detached: false } };
  devpages.shown = { a7: true };
});

describe('the bar of a launch’s page', () => {
  it('has back, forward, reload, its address, whose page it is, devtools and two ways out', () => {
    launch(true);
    fakeBackend();
    render(DevPage, { agent: A });
    for (const name of ['Retour', 'Avancer', 'Recharger', 'Ouvrir dans une fenêtre à part', 'Ouvrir dans le navigateur du système'])
      expect(screen.getByRole('button', { name })).toBeTruthy();
    expect(screen.getByRole('textbox', { name: 'Adresse de la page' })).toHaveProperty('value', URL1);
    expect(screen.getByText('pagination-users · web')).toBeTruthy();
    expect(screen.getByRole('button', { name: /Outils de développement de cette page \(F12\)/ })).toBeTruthy();
  });

  it('asks the backend for each action, on this agent’s page', async () => {
    launch(true);
    const b = fakeBackend();
    render(DevPage, { agent: A });
    await fireEvent.click(screen.getByRole('button', { name: 'Retour' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Avancer' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Recharger' }));
    await fireEvent.click(screen.getByRole('button', { name: /Outils de développement/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Ouvrir dans le navigateur du système' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Ouvrir dans une fenêtre à part' }));
    expect(b.called('devpage_go').map((c) => [c.args.agentId, c.args.action])).toEqual([
      ['a7', 'back'],
      ['a7', 'forward'],
      ['a7', 'reload'],
    ]);
    expect(b.called('devpage_devtools').map((c) => c.args.agentId)).toEqual(['a7']);
    expect(b.called('open_url').map((c) => c.args.url)).toEqual([URL1]);
    expect(b.called('devpage_detach').map((c) => c.args)).toEqual([{ agentId: 'a7', detached: true }]);
  });

  it('goes to an address typed, its scheme taken from the page when left out', async () => {
    launch(true);
    const b = fakeBackend();
    render(DevPage, { agent: A });
    const field = screen.getByRole('textbox', { name: 'Adresse de la page' });
    await fireEvent.input(field, { target: { value: 'localhost:41731/factures' } });
    await fireEvent.submit(field.closest('form')!);
    expect(b.called('devpage_go').map((c) => c.args)).toEqual([{ agentId: 'a7', action: 'to', to: 'http://localhost:41731/factures' }]);
  });

  it('opens the devtools by F12 and Ctrl+Shift+I', async () => {
    launch(true);
    const b = fakeBackend();
    render(DevPage, { agent: A });
    await fireEvent.keyDown(window, { key: 'F12' });
    await fireEvent.keyDown(window, { key: 'I', code: 'KeyI', ctrlKey: true, shiftKey: true });
    expect(b.called('devpage_devtools')).toHaveLength(2);
  });
});

describe('its states', () => {
  it('says the page does not answer, with reload and the logs', async () => {
    launch(true);
    devpages.pages.a7.phase = 'stalled';
    fakeBackend();
    render(DevPage, { agent: A });
    expect(screen.getByText('localhost:41731 ne répond pas')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: 'Recharger' })).toHaveLength(2);
    expect(screen.getByRole('button', { name: 'Voir les logs' })).toBeTruthy();
  });

  it('says the launch stopped, with « Relancer », and hides the page', async () => {
    launch(false);
    const b = fakeBackend();
    render(DevPage, { agent: A });
    expect(screen.getByText('Le lancement s’est arrêté')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeTruthy();
    await tick();
    await new Promise((r) => requestAnimationFrame(r));
    expect(b.called('devpage_place').at(-1)?.args).toEqual({ agentId: 'a7', bounds: null });
  });

  it('says the launch is starting again', () => {
    launch(true);
    flows.all.a7.phase = 'running';
    fakeBackend();
    render(DevPage, { agent: A });
    expect(screen.getByText('Le lancement démarre…')).toBeTruthy();
  });

  it('says where the page went once in its window, with the way back', () => {
    launch(true);
    devpages.pages.a7.detached = true;
    fakeBackend();
    render(DevPage, { agent: A });
    expect(screen.getByText('La page est dans la fenêtre « Pages » de demo-api.')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Aller à la fenêtre' })).toBeTruthy();
  });

  it('in English too', () => {
    setLang('en');
    launch(false);
    fakeBackend();
    render(DevPage, { agent: A });
    expect(screen.getByText('The launch stopped')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Launch again' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Open in the system browser' })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Developer tools of this page/ })).toBeTruthy();
  });
});
