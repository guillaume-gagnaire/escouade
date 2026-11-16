import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { CloneView } from '../../lib/team-projects';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import { listedProject, repo } from '../../test/team-projects';
import NewProjectModal from './NewProjectModal.svelte';

// « Cloner » of « Projets de l'équipe » (TS-8) in the wizard « Nouveau projet »: the addresses shown whole before the
// clone, the list's name and colour proposed, the clone by the member's accounts, the access asked for, a project of
// several repositories that says which one is missing, then the project made and attached to the team.

const API = listedProject({ logo: 'data:image/png;base64,iVBORw0KGgo=' });
const PLATFORM = listedProject({
  id: 'cc'.repeat(16),
  name: 'Plateforme',
  color: '',
  logo: null,
  repos: [repo('git@github.com:acme/front.git', { folder: 'front' }), repo('https://github.com/acme/infra.git', { folder: 'infra' })],
});

const done = (path: string, urls: string[]): CloneView => ({
  path,
  complete: true,
  repos: urls.map((url) => ({ url, path, done: true, already: false, noAccess: null, error: null })),
});

describe('« Cloner » in the wizard (TS-8)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project({ id: 'p0', name: 'autre', path: 'C:\\dev\\autre' })] });
  });

  it('shows each address whole before the clone, clones into the folder, and makes a project of the team', async () => {
    app.modal = { kind: 'newProject', clone: { team: 'acme', teamName: 'ACME', project: API } };
    const made = project({ id: 'new', name: 'Acme API', path: 'C:\\dev\\acme-api' });
    const be = fakeBackend({
      team_project_logo: () => 'C:\\data\\teams\\logos\\team-logo-aa.png',
      team_project_clone: () => done('C:\\dev\\acme-api', ['https://github.com/acme/api.git']),
      inspect_folder: () => ({ exists: true, isRepo: true, branch: 'main', dirty: 0, name: 'acme-api', history: null }),
      inspect_setup: () => ({ hasCommit: true, branches: ['main'], missingDirs: [], worktreesTracked: false }),
      create_project: () => made,
      project_set_team: () => ({ ...made, team: 'acme' }),
      set_project_logo: () => ({ ...made, team: 'acme', logo: 'logo.png' }),
      update_project: () => null,
    });
    render(NewProjectModal);
    const block = screen.getByRole('region', { name: 'Cloner « Acme API » de l’équipe ACME' });
    // The address, whole, before anything runs; the accounts said.
    expect(within(block).getByText('https://github.com/acme/api.git')).toBeInTheDocument();
    expect(within(block).getByText(/le serveur d’équipe ne donne aucun accès/)).toBeInTheDocument();
    expect(be.called('team_project_clone')).toHaveLength(0);
    // The list's name proposed, the folder offered next to the other projects.
    expect(screen.getByDisplayValue('Acme API')).toBeInTheDocument();
    expect(within(block).getByDisplayValue('C:\\dev')).toBeInTheDocument();
    expect(within(block).getByDisplayValue('acme-api')).toBeInTheDocument();
    await userEvent.click(within(block).getByRole('button', { name: 'Cloner' }));
    await waitFor(() =>
      expect(be.called('team_project_clone')[0]?.args).toEqual({
        team: 'acme',
        project: API.id,
        parent: 'C:\\dev',
        folder: 'acme-api',
      }),
    );
    // The wizard goes on with the folder cloned.
    expect(await screen.findByText(/Cloné dans .*acme-api\. Le projet sera un projet de l’équipe ACME\./)).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('button', { name: /^Créer sans l.assistant$/ })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: /^Créer sans l.assistant$/ }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(be.called('create_project')[0].args).toMatchObject({ path: 'C:\\dev\\acme-api', name: 'Acme API', color: '#3b82f6' });
    expect(be.called('project_set_team')[0].args).toEqual({ project: 'new', team: 'acme' });
    // The list's logo, taken as « Choisir une image… » would.
    expect(be.called('set_project_logo')[0]?.args).toEqual({ id: 'new', path: 'C:\\data\\teams\\logos\\team-logo-aa.png' });
  });

  it('says the access to ask for, and which repository of several is missing; « Réessayer » clones the rest', async () => {
    app.modal = { kind: 'newProject', clone: { team: 'acme', teamName: 'ACME', project: PLATFORM } };
    let tries = 0;
    const be = fakeBackend({
      team_project_clone: (): CloneView => {
        tries += 1;
        return tries === 1
          ? {
              path: 'C:\\dev\\plateforme',
              complete: false,
              repos: [
                {
                  url: 'git@github.com:acme/front.git',
                  path: 'C:\\dev\\plateforme\\front',
                  done: true,
                  already: false,
                  noAccess: null,
                  error: null,
                },
                {
                  url: 'https://github.com/acme/infra.git',
                  path: 'C:\\dev\\plateforme\\infra',
                  done: false,
                  already: false,
                  noAccess: 'Demande l’accès à acme/infra sur GitHub',
                  error: null,
                },
              ],
            }
          : done('C:\\dev\\plateforme', ['git@github.com:acme/front.git', 'https://github.com/acme/infra.git']);
      },
      inspect_folder: () => ({ exists: true, isRepo: false, branch: '', dirty: 0, name: 'plateforme', history: null }),
      inspect_setup: () => ({ hasCommit: true, branches: [], missingDirs: [], worktreesTracked: false }),
    });
    render(NewProjectModal);
    const block = screen.getByRole('region', { name: 'Cloner « Plateforme » de l’équipe ACME' });
    expect(within(block).getByText('git@github.com:acme/front.git')).toBeInTheDocument();
    expect(within(block).getByText('https://github.com/acme/infra.git')).toBeInTheDocument();
    await userEvent.click(within(block).getByRole('button', { name: 'Cloner' }));
    expect(await within(block).findByText('Demande l’accès à acme/infra sur GitHub')).toBeInTheDocument();
    expect(within(block).getByText('1 dépôt manque : le projet attend qu’il soit cloné.')).toBeInTheDocument();
    expect(within(block).getByText('Cloné')).toBeInTheDocument();
    await userEvent.click(within(block).getByRole('button', { name: 'Réessayer' }));
    await waitFor(() => expect(be.called('team_project_clone')).toHaveLength(2));
    expect(await screen.findByText(/Cloné dans .*plateforme/)).toBeInTheDocument();
  });
});
