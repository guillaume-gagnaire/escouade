import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import CodeIndexAllSection from './CodeIndexAllSection.svelte';

// « Index du code sur tous les projets » (ESC-293): off by default, saved at once; on, the projects that follow it are
// read by the backend (a toast says how many); off, their index is erased after an app's confirmation that says how many.
// The model of the search by meaning is never downloaded by it.

const group = () => screen.getByText('Tous les projets', { selector: 'h2' }).closest('section') as HTMLElement;
const PROJECTS = () => [
  project({ id: 'p1' }),
  project({ id: 'p2', name: 'beta' }),
  project({ id: 'p3', name: 'gamma', codeIndexChoice: false }),
];

describe('the code index on every project (ESC-293)', () => {
  beforeEach(() => {
    resetApp({ projects: PROJECTS() });
    app.modal = { kind: 'settings', tab: 'projects', projectId: 'p1' };
  });

  it('is off by default, says what the index does, that it stays here, the fence, and who follows it', () => {
    fakeBackend({});
    render(CodeIndexAllSection);
    const g = group();
    const sw = within(g).getByRole('switch', { name: 'Index du code sur tous les projets' });
    expect(sw).toHaveAttribute('aria-checked', 'false');
    expect(within(g).getByText(/trouvent le code plus vite/)).toBeInTheDocument();
    expect(within(g).getByText(/Tout reste sur cette machine/)).toBeInTheDocument();
    expect(within(g).getByText(/La clôture des projets privés est respectée/)).toBeInTheDocument();
    expect(within(g).getByText('Suivie par 2 projets sur 3.')).toBeInTheDocument();
  });

  it('turns on at once, says how many projects will be read, and downloads no model', async () => {
    const be = fakeBackend({ set_code_index_all: () => 2 });
    render(CodeIndexAllSection);
    const sw = within(group()).getByRole('switch', { name: 'Index du code sur tous les projets' });
    await userEvent.click(sw);
    expect(be.called('set_code_index_all')[0].args).toEqual({ enabled: true });
    await waitFor(() => expect(sw).toHaveAttribute('aria-checked', 'true'));
    expect(app.settings.codeIndexAll).toBe(true);
    expect(app.toasts.map((t) => t.text)).toEqual([
      'Index du code sur tous les projets allumé : 2 projets seront lus, un par un, en arrière-plan.',
    ]);
    expect(be.called('vectors_model_download')).toHaveLength(0);
    expect(be.called('save_settings')).toHaveLength(0);
  });

  it('asks before turning off, with how many projects lose their index, and erases nothing until confirmed', async () => {
    app.settings.codeIndexAll = true;
    const be = fakeBackend({ set_code_index_all: () => 2 });
    render(CodeIndexAllSection);
    await userEvent.click(within(group()).getByRole('switch', { name: 'Index du code sur tous les projets' }));
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Couper l’index du code de 2 projets ?',
      body: expect.stringContaining('Les 2 projets qui suivent ce réglage perdent leur index'),
      confirm: 'Couper et effacer',
      danger: true,
    });
    expect(be.called('set_code_index_all')).toHaveLength(0);
    await (app.modal as any).onConfirm(false);
    expect(be.called('set_code_index_all')[0].args).toEqual({ enabled: false });
    expect(app.settings.codeIndexAll).toBe(false);
    expect(app.toasts.map((t) => t.text)).toEqual(['Index du code sur tous les projets coupé : l’index de 2 projets est effacé.']);
    // Back to the settings of the project on screen, their draft as it was.
    expect(app.modal).toMatchObject({ kind: 'settings', tab: 'projects', resume: true });
  });

  it('turns off without asking when no project follows it', async () => {
    resetApp({ projects: [project({ codeIndexChoice: true })] });
    app.settings.codeIndexAll = true;
    const be = fakeBackend({ set_code_index_all: () => 0 });
    render(CodeIndexAllSection);
    expect(within(group()).getByText('Aucun projet ne la suit : chacun a fait son propre choix.')).toBeInTheDocument();
    await userEvent.click(within(group()).getByRole('switch', { name: 'Index du code sur tous les projets' }));
    expect(be.called('set_code_index_all')[0].args).toEqual({ enabled: false });
    expect(app.modal).not.toMatchObject({ kind: 'confirm' });
  });
});
