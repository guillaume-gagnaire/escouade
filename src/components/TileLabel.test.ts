// A project's label under its tile (ESC-168), and the rail's width, chosen by its right edge: the label's text and its
// cut, the rail's tiles and their labels, the width kept for the whole app and held to the window's room, the setting in
// Réglages › Projets and in the assistant.

import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { forgetLogos } from '../lib/project-logo.svelte';
import { cleanLabel, labelText, MAX_LABEL } from '../lib/project-rail';
import { RAIL_LABELLED, RAIL_MAX, RAIL_MIN, railRoom, railShown, chosenWidth, railWidth } from '../lib/rail-width.svelte';
import { app } from '../lib/state.svelte';
import { besideEdge } from '../lib/tip';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import '../test/pointer';
import NewProjectModal from './modals/NewProjectModal.svelte';
import SettingsModal from './modals/SettingsModal.svelte';
import ProjectRail from './ProjectRail.svelte';
import { afterGesture } from '../test/settings';

const tile = (name: string) => screen.getByRole('tab', { name: new RegExp(`^${name}`) });
const edge = () => screen.getByRole('separator', { name: 'Largeur de la barre des projets' });
const labelField = () => screen.getByRole('textbox', { name: 'Texte du libellé' }) as HTMLInputElement;
const labelSwitch = () => screen.getByRole('switch', { name: 'Libellé sous la tuile' });

/** The rail as the app has it at first: its usual width, in a window of 1 440 px. */
function freshRail() {
  railWidth.window = 1440;
}

/** The width last saved in the settings. */
const saved = (backend: ReturnType<typeof fakeBackend>) => backend.called('save_settings').at(-1)?.args.settings.railWidth;

describe('the label’s text', () => {
  it('is on one line, its spaces one, twelve letters at most, an emoji one', () => {
    expect(cleanLabel('')).toBe('');
    expect(cleanLabel(' demo')).toBe('demo');
    expect(cleanLabel('API  v2')).toBe('API v2');
    expect(cleanLabel('a\nb\tc')).toBe('a b c');
    expect(cleanLabel('pagination-service')).toBe('pagination-s');
    expect([...cleanLabel('x'.repeat(40))]).toHaveLength(MAX_LABEL);
    expect(cleanLabel('👩‍💻 équipe front')).toBe('👩‍💻 équipe fro');
    expect(cleanLabel('e\u0301tat')).toBe('état');
  });

  it('says the text chosen, else the name', () => {
    expect(labelText({ name: 'demo-api' })).toBe('demo-api');
    expect(labelText({ name: 'demo-api', tileLabelText: ' ' })).toBe('demo-api');
    expect(labelText({ name: 'demo-api', tileLabelText: 'API' })).toBe('API');
  });
});

describe('the rail’s width', () => {
  it('is 52 px, 72 px with labels, held to the room the window leaves', () => {
    expect(railShown(null, false, 1440)).toBe(RAIL_MIN);
    expect(railShown(null, true, 1440)).toBe(RAIL_LABELLED);
    expect(railShown(240, false, 1440)).toBe(240);
    expect(railShown(999, false, 1440)).toBe(RAIL_MAX);
    expect(railShown(10, true, 1440)).toBe(RAIL_MIN);
    // A window of 1 028 px leaves the rail 80 px; from 1 248 px, all of its 300.
    expect(railRoom(1028)).toBe(80);
    expect(railShown(300, false, 1028)).toBe(80);
    expect(railShown(null, true, 1028)).toBe(RAIL_LABELLED);
    expect(railRoom(1000)).toBe(RAIL_MIN);
    expect(railRoom(1248)).toBe(RAIL_MAX);
    expect(railRoom(1920)).toBe(RAIL_MAX);
  });

  it('reads the width kept in the settings, and nothing that is not one', () => {
    for (const [kept, read] of [
      [200, 200],
      [120.6, 121],
      [900, RAIL_MAX],
      [10, RAIL_MIN],
      [0, null],
      [undefined, null],
      [NaN, null],
      [-4, null],
    ] as const)
      expect(chosenWidth(kept)).toBe(read);
  });
});

describe('the rail', () => {
  beforeEach(() => {
    forgetLogos();
    freshRail();
    resetApp({
      projects: [
        project({ tileLabel: true }),
        project({ id: 'p2', name: 'studio-web', tileLabel: true, tileLabelText: 'Studio', private: true }),
        project({ id: 'p3', name: 'infra' }),
      ],
      agents: [agent({ id: 'b1', projectId: 'p2', status: 'waiting' }), agent({ id: 'b2', projectId: 'p2', status: 'waiting' })],
    });
    app.ui.view = 'project';
    app.ui.activeProject = 'p1';
  });

  it('gives each project its label: its name or its own text, under the tile when on, for the eyes alone', () => {
    fakeBackend();
    render(ProjectRail);
    expect(tile('demo-api')).toHaveClass('labelled');
    expect(tile('demo-api').querySelector('.lbl')).toHaveTextContent('demo-api');
    expect(tile('demo-api').querySelector('.lbl')).toHaveAttribute('aria-hidden', 'true');
    expect(tile('studio-web')).toHaveClass('labelled');
    expect(tile('studio-web').querySelector('.lbl .tx')).toHaveTextContent('Studio');
    // Its agents waiting, at the end of its row once the rail is a list; on the tile as before.
    expect(tile('studio-web').querySelector('.lbl .n')).toHaveTextContent('2');
    expect(tile('studio-web').querySelector('.ask')).toHaveTextContent('2');
    // Off: not under the tile, its name kept for a rail widened to a list.
    expect(tile('infra')).not.toHaveClass('labelled');
    expect(tile('infra').querySelector('.lbl')).toHaveTextContent('infra');
    // Its name and state said as before: the label adds nothing to it.
    expect(tile('studio-web')).toHaveAccessibleName('studio-web, 2 en attente, privé');
    expect(tile('demo-api')).toHaveAttribute('aria-selected', 'true');
    // The tiles that are no project's have their names too, for a wide rail.
    expect(screen.getByRole('button', { name: 'Vue d’ensemble' }).querySelector('.lbl')).toHaveTextContent('Vue d’ensemble');
    expect(screen.getByRole('button', { name: 'Ajouter un projet' }).querySelector('.lbl')).toHaveTextContent('Ajouter un projet');
  });

  it('has a usual width that follows the labels until its edge is dragged', () => {
    fakeBackend();
    expect(railWidth.shown).toBe(RAIL_LABELLED);
    app.projects = app.projects.map((p) => ({ ...p, tileLabel: false }));
    expect(railWidth.shown).toBe(RAIL_MIN);
    app.projects[1] = { ...app.projects[1], tileLabel: true };
    expect(railWidth.shown).toBe(RAIL_LABELLED);
  });

  it('is widened by its edge, by the keyboard too, kept for the whole app, and a double click gives its usual width back', async () => {
    const backend = fakeBackend();
    render(ProjectRail);
    const e = edge();
    expect(e).toHaveAttribute('aria-orientation', 'vertical');
    expect(e).toHaveAttribute('aria-valuenow', String(RAIL_LABELLED));
    expect(e).toHaveAttribute('aria-valuemin', String(RAIL_MIN));
    expect(e).toHaveAttribute('aria-valuemax', String(RAIL_MAX));
    e.focus();
    await userEvent.keyboard('{ArrowRight}');
    expect(railWidth.shown).toBe(RAIL_LABELLED + 16);
    await waitFor(() => expect(saved(backend)).toBe(RAIL_LABELLED + 16));
    await userEvent.keyboard('{End}');
    expect(railWidth.shown).toBe(RAIL_MAX);
    expect(e).toHaveAttribute('aria-valuenow', String(RAIL_MAX));
    await userEvent.keyboard('{Home}');
    expect(railWidth.shown).toBe(RAIL_MIN);
    await waitFor(() => expect(saved(backend)).toBe(RAIL_MIN));
    // Dragged: the width follows the pointer, kept once let go.
    const saves = backend.called('save_settings').length;
    await fireEvent.pointerDown(e, { button: 0, pointerId: 1, clientX: 52 });
    await fireEvent.pointerMove(e, { pointerId: 1, buttons: 1, clientX: 252 });
    expect(railWidth.shown).toBe(252);
    await fireEvent.pointerUp(e, { pointerId: 1 });
    await waitFor(() => expect(saved(backend)).toBe(252));
    // Saved once it is let go, not at each move.
    expect(backend.called('save_settings')).toHaveLength(saves + 1);
    // The same width for every project.
    app.ui.activeProject = 'p3';
    expect(railWidth.shown).toBe(252);
    // A narrow window draws it narrower; the width chosen comes back with the room.
    railWidth.window = 1028;
    expect(railWidth.shown).toBe(80);
    railWidth.window = 1440;
    expect(railWidth.shown).toBe(252);
    // Double click: the usual width, which follows the labels again.
    await fireEvent.dblClick(e);
    expect(railWidth.shown).toBe(RAIL_LABELLED);
    await waitFor(() => expect(saved(backend)).toBe(0));
  });

  it('opens its tooltips beside its edge, not over the names of a wide rail', () => {
    const rail = document.createElement('nav');
    rail.dataset.tipEdge = '';
    const button = document.createElement('button');
    rail.append(button);
    document.body.append(rail);
    rail.getBoundingClientRect = () => ({ left: 0, right: 300, top: 0, bottom: 900, width: 300, height: 900 }) as DOMRect;
    const at = { right: 44, top: 100, height: 36 };
    expect(besideEdge(button, at)).toEqual({ right: 292, top: 100, height: 36 });
    // A narrow rail: beside the tile, as before.
    rail.getBoundingClientRect = () => ({ left: 0, right: 52, top: 0, bottom: 900, width: 52, height: 900 }) as DOMRect;
    expect(besideEdge(button, at)).toBe(at);
    // Out of a rail: beside the element.
    expect(besideEdge(document.body, at)).toBe(at);
    rail.remove();
  });
});

describe('Réglages › Projets', () => {
  beforeEach(() => {
    forgetLogos();
    freshRail();
    resetApp({ projects: [project()] });
    app.modal = { kind: 'settings' };
  });

  it('turns the label on, with a shorter text, and saves them on this machine', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    expect(labelSwitch()).toHaveAttribute('aria-checked', 'false');
    expect(labelField().value).toBe('');
    expect(labelField().placeholder).toBe('demo-api');
    await userEvent.click(labelSwitch());
    await userEvent.type(labelField(), 'Démo API de paiement');
    expect(labelField().value).toBe('Démo API de ');
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({
      id: 'p1',
      tileLabel: true,
      tileLabelText: 'Démo API de',
    });
    expect(app.projects[0]).toMatchObject({ tileLabel: true, tileLabelText: 'Démo API de' });
    // The rail makes room for it.
    expect(railWidth.shown).toBe(RAIL_LABELLED);
  });
});

describe('the new project’s assistant', () => {
  beforeEach(() => {
    forgetLogos();
    freshRail();
    resetApp();
    app.modal = { kind: 'newProject' };
  });

  it('sets the label of the project it makes', async () => {
    const made = project({ id: 'p9', name: 'demo', path: 'C:\\code\\demo' });
    const backend = fakeBackend({
      inspect_folder: () => ({ exists: true, isRepo: true, branch: 'main', dirty: 0, name: 'demo', history: null }),
      create_project: () => made,
    });
    render(NewProjectModal);
    await userEvent.type(screen.getByPlaceholderText(/^C:/), 'C:\\code\\demo');
    await waitFor(() => expect(screen.getByRole('button', { name: /^Créer sans l.assistant$/ })).toBeEnabled());
    expect(labelSwitch()).toHaveAttribute('aria-checked', 'false');
    expect(labelField().placeholder).toBe('demo');
    await userEvent.click(labelSwitch());
    await userEvent.type(labelField(), 'Démo');
    await userEvent.click(screen.getByRole('button', { name: /^Créer sans l.assistant$/ }));
    await waitFor(() => expect(backend.called('update_project')).toHaveLength(1));
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p9', tileLabel: true, tileLabelText: 'Démo' });
    await waitFor(() => expect(app.projects.find((p) => p.id === 'p9')).toMatchObject({ tileLabel: true, tileLabelText: 'Démo' }));
  });

  it('leaves a project made without it as it was: no label', async () => {
    const made = project({ id: 'p9', name: 'demo', path: 'C:\\code\\demo' });
    const backend = fakeBackend({
      inspect_folder: () => ({ exists: true, isRepo: true, branch: 'main', dirty: 0, name: 'demo', history: null }),
      create_project: () => made,
    });
    render(NewProjectModal);
    await userEvent.type(screen.getByPlaceholderText(/^C:/), 'C:\\code\\demo');
    await waitFor(() => expect(screen.getByRole('button', { name: /^Créer sans l.assistant$/ })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: /^Créer sans l.assistant$/ }));
    await waitFor(() => expect(app.projects.some((p) => p.id === 'p9')).toBe(true));
    expect(backend.called('update_project')).toHaveLength(0);
    expect(railWidth.shown).toBe(RAIL_MIN);
  });
});
