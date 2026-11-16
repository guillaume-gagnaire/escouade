import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { teams } from '../../lib/teams.svelte';
import type { Listed } from '../../lib/team-projects';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import { ALICE, listedProject, ME, repo, teamProjects } from '../../test/team-projects';
import TeamProjects from './TeamProjects.svelte';

// « Projets de l'équipe » (TS-8): the projects of the list already here (attached by the member's gesture, their own
// name kept and said), the others with « Cloner », which opens the wizard with the project; the administrator's form.

const HERE = listedProject({
  id: 'bb'.repeat(16),
  name: 'Site ACME',
  repos: [repo('https://github.com/acme/site.git')],
  here: { project: 'p1', name: 'site', attached: false, otherTeam: false, repos: [0], nameDiffers: true },
});
const API = listedProject();
const PLATFORM = listedProject({
  id: 'cc'.repeat(16),
  name: 'Plateforme',
  branch: 'develop',
  repos: [
    repo('git@github.com:acme/front.git', { folder: 'front' }),
    repo('ssh://git@gitlab.com/acme/infra', { folder: 'infra', host: 'gitlab.com', forge: 'GitLab', path: 'acme/infra' }),
  ],
});

describe('« Projets de l’équipe » (TS-8)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project({ id: 'p1', name: 'site', path: 'C:\\dev\\site' })] });
    teams.list = [];
    app.modal = { kind: 'team', step: 'projects', teamId: 'acme' };
  });

  it('shows the projects here and the others, each address whole, and « Cloner » opens the wizard with the project', async () => {
    fakeBackend({ team_projects: () => teamProjects([API, PLATFORM, HERE]) });
    render(TeamProjects, { teamId: 'acme', onback: () => {} });
    const here = await screen.findByRole('region', { name: 'Sur ce poste' });
    const site = within(here).getByRole('article', { name: 'Site ACME' });
    expect(within(site).getByText('site')).toBeInTheDocument();
    expect(within(site).getByText('https://github.com/acme/site.git')).toBeInTheDocument();
    // Its own name stays, said.
    expect(within(site).getByText('Ici « site », dans la liste « Site ACME » : ton nom et ton logo restent.')).toBeInTheDocument();
    expect(within(site).queryByRole('button', { name: 'Cloner' })).toBeNull();
    const elsewhere = screen.getByRole('region', { name: 'À cloner' });
    const plat = within(elsewhere).getByRole('article', { name: 'Plateforme' });
    expect(within(plat).getByText('git@github.com:acme/front.git')).toBeInTheDocument();
    expect(within(plat).getByText('ssh://git@gitlab.com/acme/infra')).toBeInTheDocument();
    expect(within(plat).getByText(/Branche develop/)).toBeInTheDocument();
    // A member: no form.
    expect(screen.queryByRole('button', { name: 'Ajouter un projet' })).toBeNull();
    await userEvent.click(within(within(elsewhere).getByRole('article', { name: 'Acme API' })).getByRole('button', { name: 'Cloner' }));
    expect(app.modal).toMatchObject({ kind: 'newProject', clone: { team: 'acme', teamName: 'ACME', project: { id: API.id } } });
  });

  it('attaches a clone already here to the team without a new clone', async () => {
    const attached = { ...HERE, here: { ...HERE.here!, attached: true } };
    let view = teamProjects([HERE]);
    const be = fakeBackend({
      team_projects: () => view,
      project_set_team: () => {
        view = teamProjects([attached]);
        return project({ id: 'p1', team: 'acme' });
      },
    });
    render(TeamProjects, { teamId: 'acme', onback: () => {} });
    await userEvent.click(await screen.findByRole('button', { name: 'Rattacher à l’équipe' }));
    await waitFor(() => expect(be.called('project_set_team')[0]?.args).toEqual({ project: 'p1', team: 'acme' }));
    expect(await screen.findByText('Projet de l’équipe')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Rattacher à l’équipe' })).toBeNull();
    expect(be.called('team_project_clone')).toHaveLength(0);
  });

  it('says a project kept to other members, and offers no clone of it', async () => {
    fakeBackend({
      team_projects: () => teamProjects([listedProject({ openToMe: false, access: { kind: 'members', members: [ALICE] } })]),
    });
    render(TeamProjects, { teamId: 'acme', onback: () => {} });
    expect(await screen.findByText('Réservé à d’autres membres de l’équipe.')).toBeInTheDocument();
    expect(screen.getByText(/1 membre/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Cloner' })).toBeNull();
  });

  it('lets an administrator add a project of several repositories, change it and take it out', async () => {
    let view = teamProjects([], { admin: true });
    const be = fakeBackend({
      team_projects: () => view,
      team_project_put: ({ listed }: { listed: Listed }) => {
        view = teamProjects([listedProject({ name: listed.name })], { admin: true });
        return view;
      },
      team_project_remove: () => (view = teamProjects([], { admin: true })),
    });
    render(TeamProjects, { teamId: 'acme', onback: () => {} });
    expect(await screen.findByText('La liste de l’équipe ne compte encore aucun projet : ajoute le premier.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter un projet' }));
    const form = screen.getByRole('region', { name: 'Ajouter un projet à la liste' });
    await userEvent.type(within(form).getByRole('textbox', { name: 'Nom' }), 'Plateforme');
    await userEvent.type(within(form).getByRole('textbox', { name: 'Adresse du dépôt 1' }), 'git@github.com:acme/front.git');
    await userEvent.click(within(form).getByRole('button', { name: 'Ajouter un dépôt' }));
    await userEvent.type(within(form).getByRole('textbox', { name: 'Adresse du dépôt 2' }), 'https://gitlab.com/acme/infra');
    await userEvent.type(within(form).getByRole('textbox', { name: 'Dossier du dépôt 2' }), 'infra');
    await userEvent.type(within(form).getByRole('textbox', { name: 'Branche par défaut' }), 'develop');
    await userEvent.click(within(form).getByRole('radio', { name: 'Certains membres' }));
    await userEvent.click(within(form).getByRole('button', { name: 'Enregistrer' }));
    await waitFor(() => expect(be.called('team_project_put')).toHaveLength(1));
    expect(be.called('team_project_put')[0].args).toEqual({
      team: 'acme',
      project: null,
      listed: {
        name: 'Plateforme',
        color: '',
        logo: null,
        repos: [
          { url: 'git@github.com:acme/front.git', folder: '' },
          { url: 'https://gitlab.com/acme/infra', folder: 'infra' },
        ],
        branch: 'develop',
        access: { kind: 'members', members: [ME] },
      },
    });
    // Changed: by its id.
    await userEvent.click(await screen.findByRole('button', { name: 'Modifier' }));
    const edit = screen.getByRole('region', { name: 'Modifier « Plateforme »' });
    await userEvent.click(within(edit).getByRole('button', { name: 'Enregistrer' }));
    await waitFor(() => expect(be.called('team_project_put')[1]?.args.project).toBe(API.id));
    // Taken out, after a confirmation.
    await userEvent.click(await screen.findByRole('button', { name: 'Retirer' }));
    await userEvent.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Retirer' }));
    await waitFor(() => expect(be.called('team_project_remove')[0]?.args).toEqual({ team: 'acme', project: API.id }));
  });

  it('says the list’s refusal of a dangerous address', async () => {
    fakeBackend({
      team_projects: () => teamProjects([], { admin: true }),
      team_project_put: () => {
        throw 'La liste refuse ce projet : file:///c/api: a local path is not an address of the team';
      },
    });
    render(TeamProjects, { teamId: 'acme', onback: () => {} });
    await userEvent.click(await screen.findByRole('button', { name: 'Ajouter un projet' }));
    const form = screen.getByRole('region', { name: 'Ajouter un projet à la liste' });
    await userEvent.type(within(form).getByRole('textbox', { name: 'Nom' }), 'API');
    await userEvent.type(within(form).getByRole('textbox', { name: 'Adresse du dépôt 1' }), 'file:///c/api');
    await userEvent.click(within(form).getByRole('button', { name: 'Enregistrer' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('file:///c/api: a local path is not an address of the team');
  });

  it('says an alarm of the list, and « Réessayer »', async () => {
    const be = fakeBackend({
      team_projects: () => teamProjects([], { alarm: true, refused: 2 }),
      team_projects_retry: () => teamProjects([]),
    });
    render(TeamProjects, { teamId: 'acme', onback: () => {} });
    expect(await screen.findByText(/ne peut pas être/)).toBeInTheDocument();
    expect(screen.getByText(/2 entrées de la liste refusées/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Réessayer' }));
    await waitFor(() => expect(be.called('team_projects_retry')).toHaveLength(1));
  });
});
