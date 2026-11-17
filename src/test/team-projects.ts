// Factories of « Projets de l'équipe » (TS-8) for the tests: a project of the team's list, the view of a team's list.
import type { RepoView, TeamProjectView, TeamProjectsView } from '../lib/team-projects';

export const ME = '02'.repeat(16);
export const ALICE = '01'.repeat(16);

export function repo(url: string, over: Partial<RepoView> = {}): RepoView {
  const m = /github\.com[/:](.+?)(\.git)?$/.exec(url);
  return {
    url,
    folder: (m?.[1] ?? url).split('/').pop() ?? '',
    host: 'github.com',
    path: m?.[1] ?? '',
    forge: 'GitHub',
    scheme: url.startsWith('https') ? 'https' : 'ssh',
    ...over,
  };
}

export function listedProject(over: Partial<TeamProjectView> = {}): TeamProjectView {
  return {
    id: 'aa'.repeat(16),
    name: 'Acme API',
    color: '#3b82f6',
    logo: null,
    branch: 'main',
    access: { kind: 'team' },
    repos: [repo('https://github.com/acme/api.git')],
    by: ALICE,
    openToMe: true,
    here: null,
    board: '',
    proposed: null,
    ...over,
  };
}

export function teamProjects(projects: TeamProjectView[], over: Partial<TeamProjectsView> = {}): TeamProjectsView {
  return {
    team: 'acme',
    name: 'ACME',
    admin: false,
    me: ME,
    members: [
      { id: ALICE, admin: true, me: false },
      { id: ME, admin: false, me: true },
    ],
    projects,
    outside: [],
    alarm: false,
    refused: 0,
    ...over,
  };
}
