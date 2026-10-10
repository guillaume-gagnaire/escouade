import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend, project, resetApp } from '../test/ipc';
import { buffers } from './editor/buffers.svelte';
import { setLang } from './i18n';
import { askCloseProject } from './project-actions';
import { app } from './state.svelte';

describe('askCloseProject', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    fakeBackend();
  });
  afterEach(() => vi.restoreAllMocks());

  it('asks before closing, and says the files on the disk stay', () => {
    askCloseProject(project());
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Fermer « demo-api » ?',
      body: "Le projet et ses agents sont retirés de l'application (conversations comprises). Les fichiers et les worktrees sur le disque ne sont pas touchés.",
      confirm: 'Fermer le projet',
      danger: true,
    });
  });

  it('asks in English, and adds the sentence about the unsaved files after its own', () => {
    setLang('en');
    vi.spyOn(buffers, 'unsavedIn').mockReturnValue(2);
    askCloseProject(project());
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Close “demo-api”?',
      body: 'The project and its agents are removed from the app (conversations included). The files and worktrees on disk are not touched. 2 unsaved files in the editor will be lost.',
      confirm: 'Close the project',
    });
  });
});
