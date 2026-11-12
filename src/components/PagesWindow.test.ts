// The bar of a project's « Pages » window (ESC-311), drawn with the app's controls as the agent's view « Page » is
// (ESC-419), and what its buttons ask the backend.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { fakeBackend } from '../test/ipc';
import PagesWindow from './PagesWindow.svelte';

const PAGE = { agentId: 'a7', projectId: 'p1', title: 'pagination-users · web', url: 'http://localhost:41731/users', detached: true };

beforeEach(() => setLang('fr'));

describe('the « Pages » window', () => {
  it('has the bar of the agent’s view, with the app’s icon buttons, buttons, segments and field', async () => {
    fakeBackend({ devpage_list: () => ({ pages: [PAGE], shown: 'a7' }) });
    render(PagesWindow, { projectId: 'p1' });
    const address = await screen.findByRole('textbox', { name: 'Adresse de la page' });
    expect(address).toHaveProperty('value', PAGE.url);
    expect(address.closest('form')?.classList).toContain('field');
    for (const name of ['Retour', 'Avancer', 'Recharger']) expect(screen.getByRole('button', { name }).classList).toContain('icon-btn');
    for (const name of ['Remettre dans l’agent', 'Ouvrir dans le navigateur du système'])
      expect([...screen.getByRole('button', { name }).classList]).toEqual(expect.arrayContaining(['btn', 'icon-only']));
    expect(screen.getByRole('radiogroup', { name: 'Largeur de la page' }).classList).toContain('segmented');
    expect(screen.getByRole('radio', { name: 'Pleine largeur' }).classList).toContain('on');
  });

  it('asks the backend for its buttons, on the page shown', async () => {
    const b = fakeBackend({ devpage_list: () => ({ pages: [PAGE], shown: 'a7' }) });
    render(PagesWindow, { projectId: 'p1' });
    await fireEvent.click(await screen.findByRole('button', { name: 'Recharger' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Remettre dans l’agent' }));
    expect(b.called('devpage_go').map((c) => [c.args.agentId, c.args.action])).toEqual([['a7', 'reload']]);
    expect(b.called('devpage_detach').map((c) => c.args)).toEqual([{ agentId: 'a7', detached: false }]);
  });
});
