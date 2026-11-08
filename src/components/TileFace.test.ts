// The project's tile made one's own (ESC-167): the letters chosen in place of the initials, the logo in place of the
// letters, wherever the tile is drawn (the rail, Réglages › Projets, the assistant), its state over it all the same.

import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { forgetLogos } from '../lib/project-logo.svelte';
import { settingsForm } from '../lib/settings.svelte';
import { cleanLetters, tileSize, tileText } from '../lib/project-rail';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import NewProjectModal from './modals/NewProjectModal.svelte';
import SettingsModal from './modals/SettingsModal.svelte';
import ProjectRail from './ProjectRail.svelte';
import { afterGesture } from '../test/settings';

const PNG = 'data:image/png;base64,iVBORw0KGgo=';
const SVG = 'data:image/svg+xml;base64,PHN2Zy8+';
const tile = (name: string) => screen.getByRole('tab', { name: new RegExp(`^${name}`) });
const letters = () => screen.getByRole('textbox', { name: 'Lettres de la tuile' }) as HTMLInputElement;
const preview = () => screen.getByRole('img', { name: 'Tuile du projet' });

describe('cleanLetters', () => {
  it('keeps up to three letters or digits, or one emoji alone', () => {
    expect(cleanLetters('')).toBe('');
    expect(cleanLetters('ab')).toBe('ab');
    expect(cleanLetters('ABCD')).toBe('ABC');
    expect(cleanLetters(' v 2 ')).toBe('v2');
    expect(cleanLetters('É-t.é')).toBe('Été');
    expect(cleanLetters('e\u0301a')).toBe('éa');
    expect(cleanLetters('🚀')).toBe('🚀');
    expect(cleanLetters('👩‍💻x')).toBe('👩‍💻');
    expect(cleanLetters('a🚀')).toBe('a');
    expect(cleanLetters('-!?')).toBe('');
  });

  it('says the initials of the name when no letters are chosen, and how big to draw them', () => {
    expect(tileText({ name: 'demo-api', tileLetters: '' })).toBe('DA');
    expect(tileText({ name: 'demo-api' })).toBe('DA');
    expect(tileText({ name: 'demo-api', tileLetters: 'API' })).toBe('API');
    expect(tileSize('API')).toBe('three');
    expect(tileSize('🚀')).toBe('emoji');
    expect(tileSize('DA')).toBeNull();
  });
});

describe('the tile on the rail', () => {
  beforeEach(() => {
    forgetLogos();
    resetApp({
      projects: [
        project({ tileLetters: 'API' }),
        project({ id: 'p2', name: 'studio-web', logo: 'logo-1.png', private: true }),
        project({ id: 'p3', name: 'infra', tileLetters: '' }),
      ],
      agents: [agent({ id: 'b1', projectId: 'p2', status: 'waiting' }), agent({ id: 'b2', projectId: 'p2', status: 'running' })],
    });
  });

  it('shows the letters chosen, else the initials', () => {
    fakeBackend();
    render(ProjectRail);
    expect(tile('demo-api').querySelector('.ini')).toHaveTextContent('API');
    expect(tile('demo-api').querySelector('.ini')).toHaveClass('three');
    expect(tile('infra').querySelector('.ini')).toHaveTextContent('In');
  });

  it('shows the logo in place of the letters, as an image, its state over it', async () => {
    const backend = fakeBackend({ project_logo: ({ id }: { id: string }) => (id === 'p2' ? PNG : null) });
    render(ProjectRail);
    const t = tile('studio-web');
    await waitFor(() => expect(t.querySelector('img.logo')).toHaveAttribute('src', PNG));
    expect(t.querySelector('.ini')).toBeNull();
    // Its agents waiting, at work, its lock: still on it, and said as before.
    expect(t.querySelector('.ask')).toHaveTextContent('1');
    expect(t.querySelector('.busy')).not.toBeNull();
    expect(t.querySelector('.lock')).not.toBeNull();
    expect(t).toHaveAccessibleName('studio-web, 1 en attente, 1 au travail, privé');
    // Drawn after the logo: over it.
    expect(t.lastElementChild).toHaveClass('busy');
    // Asked once, for the project that has one.
    expect(backend.called('project_logo').map((c) => c.args.id)).toEqual(['p2']);
  });
});

describe('Réglages › Projets', () => {
  beforeEach(() => {
    forgetLogos();
    resetApp({ projects: [project()] });
    app.modal = { kind: 'settings' };
  });

  it('chooses the tile’s letters, empty showing the initials, and saves them', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    expect(letters().value).toBe('');
    expect(letters().placeholder).toBe('DA');
    expect(preview()).toHaveTextContent('DA');
    await userEvent.type(letters(), 'apiv');
    expect(letters().value).toBe('api');
    expect(preview()).toHaveTextContent('api');
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', tileLetters: 'api' });
    expect(app.projects[0].tileLetters).toBe('api');
    expect(backend.called('set_project_logo')).toHaveLength(0);

    // Emptied: the initials again.
    app.modal = { kind: 'settings' };
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    await userEvent.clear(screen.getAllByRole('textbox', { name: 'Lettres de la tuile' }).at(-1)!);
    await afterGesture();
    expect(backend.called('update_project')[1].args.project.tileLetters).toBe('');
  });

  it('chooses a logo, shown before it is saved, copied by the backend once saved, and removes it', async () => {
    const backend = fakeBackend({
      'plugin:dialog|open': () => 'C:\\images\\logo.svg',
      read_logo: () => SVG,
      set_project_logo: ({ path }: { path: string | null }) => ({ ...app.projects[0], logo: path ? 'logo-9.svg' : '' }),
    });
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    expect(screen.queryByRole('button', { name: 'Retirer' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Choisir une image…' }));
    await waitFor(() => expect(preview().querySelector('img')).toHaveAttribute('src', SVG));
    expect(backend.called('read_logo')[0].args).toEqual({ path: 'C:\\images\\logo.svg' });
    await afterGesture();
    expect(backend.called('set_project_logo')[0].args).toEqual({ id: 'p1', path: 'C:\\images\\logo.svg' });
    expect(app.projects[0].logo).toBe('logo-9.svg');
    // The rail shows it without asking for it again.
    render(ProjectRail);
    expect(tile('demo-api').querySelector('img.logo')).toHaveAttribute('src', SVG);
    expect(backend.called('project_logo')).toHaveLength(0);

    // « Retirer »: the letters again, once saved.
    app.modal = { kind: 'settings' };
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    const dialogs = screen.getAllByRole('dialog');
    const last = within(dialogs.at(-1)!);
    await userEvent.click(last.getByRole('button', { name: 'Retirer' }));
    expect(last.getByRole('img', { name: 'Tuile du projet' }).querySelector('img')).toBeNull();
    await afterGesture();
    expect(backend.called('set_project_logo')[1].args).toEqual({ id: 'p1', path: null });
    expect(app.projects[0].logo).toBe('');
  });

  it('says why an image is refused, and keeps the tile as it was', async () => {
    const backend = fakeBackend({
      'plugin:dialog|open': () => 'C:\\images\\big.png',
      read_logo: () => {
        throw '« big.png » pèse 3 Mo : un logo pèse 1 Mo au plus.';
      },
    });
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: 'Choisir une image…' }));
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toContain('« big.png » pèse 3 Mo : un logo pèse 1 Mo au plus.'));
    expect(preview().querySelector('img')).toBeNull();
    expect(settingsForm.changed('project')).toBe(false);
    expect(backend.called('set_project_logo')).toHaveLength(0);
  });
});

describe('the new project’s assistant', () => {
  beforeEach(() => {
    forgetLogos();
    resetApp();
    app.modal = { kind: 'newProject' };
  });

  it('sets the tile’s letters and logo of the project it makes', async () => {
    const made = project({ id: 'p9', name: 'demo', path: 'C:\\code\\demo' });
    const backend = fakeBackend({
      inspect_folder: () => ({ exists: true, isRepo: true, branch: 'main', dirty: 0, name: 'demo', history: null }),
      create_project: () => made,
      'plugin:dialog|open': () => 'C:\\images\\logo.png',
      read_logo: () => PNG,
      set_project_logo: () => ({ ...made, logo: 'logo-2.png' }),
    });
    render(NewProjectModal);
    await userEvent.type(screen.getByPlaceholderText(/^C:/), 'C:\\code\\demo');
    await waitFor(() => expect(screen.getByRole('button', { name: /^Créer sans l.assistant$/ })).toBeEnabled());
    expect(letters().placeholder).toBe('De');
    // An emoji comes whole (a keyboard of emojis, a paste), not one half of it at a time.
    await fireEvent.input(letters(), { target: { value: '🚀x' } });
    expect(letters().value).toBe('🚀');
    await userEvent.click(screen.getByRole('button', { name: 'Choisir une image…' }));
    await waitFor(() => expect(document.querySelector('.tile img.logo')).toHaveAttribute('src', PNG));
    await userEvent.click(screen.getByRole('button', { name: /^Créer sans l.assistant$/ }));
    await waitFor(() => expect(backend.called('set_project_logo')).toHaveLength(1));
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p9', tileLetters: '🚀' });
    expect(backend.called('set_project_logo')[0].args).toEqual({ id: 'p9', path: 'C:\\images\\logo.png' });
    await waitFor(() => expect(app.projects.find((p) => p.id === 'p9')).toMatchObject({ tileLetters: '🚀', logo: 'logo-2.png' }));
  });
});
