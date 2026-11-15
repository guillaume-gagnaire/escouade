import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend, project, resetApp } from '../test/ipc';
import { setLang } from './i18n';
import { askCloseProject } from './project-actions';
import { app } from './state.svelte';
import { viewUnsaved } from './vscode/unsaved.svelte';

describe('askCloseProject', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    fakeBackend();
  });
  afterEach(() => vi.restoreAllMocks());

  it('asks before closing, and says its history is kept and the files on the disk stay', () => {
    askCloseProject(project());
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Fermer « demo-api » ?',
      body: "Le projet quitte l'application : ses agents, ses terminaux et ses lancements s'arrêtent. Son historique (conversations, tickets, réglages) est gardé : rouvrir ce dossier le retrouve. Les fichiers et les worktrees sur le disque ne sont pas touchés.",
      confirm: 'Fermer le projet',
      danger: true,
    });
  });

  it('offers to erase the history too, a box unchecked at first', () => {
    askCloseProject(project());
    expect((app.modal as any).option).toEqual({ label: 'Effacer aussi son historique (conversations, tickets, réglages)', value: false });
  });

  it('asks in English', () => {
    setLang('en');
    askCloseProject(project());
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Close “demo-api”?',
      body: 'The project leaves the app: its agents, terminals and launches stop. Its history (conversations, tickets, settings) is kept: adding this folder again brings it back. The files and worktrees on disk are not touched.',
      confirm: 'Close the project',
      option: { label: 'Also erase its history (conversations, tickets, settings)', value: false },
    });
  });

  it('closes the project and keeps its history unless the box is checked', async () => {
    const backend = fakeBackend();
    askCloseProject(project());
    await (app.modal as any).onConfirm(false);
    expect(backend.called('remove_project')[0].args).toEqual({ id: 'p1', erase: false });
    expect(app.projects).toEqual([]);
  });

  it('erases the history with the project when the box is checked', async () => {
    const backend = fakeBackend();
    askCloseProject(project());
    await (app.modal as any).onConfirm(true);
    expect(backend.called('remove_project')[0].args).toEqual({ id: 'p1', erase: true });
    expect(app.projects).toEqual([]);
  });

  it('asks next to save its files modified in the VS Code view, and closes nothing on « Annuler » (ESC-464)', async () => {
    const backend = fakeBackend({ set_unsaved: () => null });
    const reverted: string[][] = [];
    const forgotten: string[] = [];
    viewUnsaved.serve({
      save: async () => true,
      revert: async (u) => void reverted.push(u),
      forget: async (id) => void forgotten.push(id),
    });
    const file = { projectId: 'p1', source: 'project', path: 'src/app.ts', uri: 'file:///c:/code/demo-api/src/app.ts' };
    viewUnsaved.set([file]);
    const back = vi.fn();
    askCloseProject(project(), back);
    await (app.modal as any).onConfirm(false);
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Enregistrer les modifications de « app.ts » ?',
      alt: { label: 'Ne pas enregistrer' },
    });
    expect(backend.called('remove_project')).toHaveLength(0);
    // « Annuler »: the project stays, back where it was asked from.
    (app.modal as any).onCancel();
    expect(back).toHaveBeenCalledTimes(1);
    expect(app.projects).toHaveLength(1);
    // « Ne pas enregistrer »: given up, then closed, out of the view.
    askCloseProject(project(), back);
    await (app.modal as any).onConfirm(false);
    await (app.modal as any).alt.onClick();
    expect(reverted).toEqual([[file.uri]]);
    expect(backend.called('remove_project')[0].args).toEqual({ id: 'p1', erase: false });
    expect(forgotten).toEqual(['p1']);
    expect(back).toHaveBeenCalledTimes(2);
  });
});
