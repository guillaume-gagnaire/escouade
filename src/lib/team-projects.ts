// « Projets de l'équipe » (TS-8, `src-tauri/src/team_projects.rs`): the team's list of projects, encrypted under the
// team's key, next to the projects of this device; « Cloner » opens the wizard « Nouveau projet » with the repositories
// filled in. Pure functions here, the screens in `components/team`.

import { invoke } from '@tauri-apps/api/core';

export type Access = { kind: 'team' } | { kind: 'members'; members: string[] };

export interface ListedRepo {
  url: string;
  folder: string;
}

/** A project of the list, as an administrator writes it. */
export interface Listed {
  name: string;
  color: string;
  logo: string | null;
  repos: ListedRepo[];
  branch: string;
  access: Access;
}

export interface RepoView {
  url: string;
  folder: string;
  host: string;
  path: string;
  forge: string | null;
  scheme: 'https' | 'ssh' | '';
}

export interface Here {
  project: string;
  name: string;
  attached: boolean;
  otherTeam: boolean;
  repos: number[];
  nameDiffers: boolean;
}

export interface TeamProjectView {
  id: string;
  name: string;
  color: string;
  logo: string | null;
  branch: string;
  access: Access;
  repos: RepoView[];
  by: string;
  openToMe: boolean;
  here: Here | null;
}

export interface MemberView {
  id: string;
  admin: boolean;
  me: boolean;
}

export interface TeamProjectsView {
  team: string;
  name: string;
  admin: boolean;
  me: string;
  members: MemberView[];
  projects: TeamProjectView[];
  outside: string[];
  alarm: boolean;
  refused: number;
}

export interface RepoClone {
  url: string;
  path: string;
  done: boolean;
  already: boolean;
  noAccess: string | null;
  error: string | null;
}

export interface CloneView {
  path: string;
  repos: RepoClone[];
  complete: boolean;
}

/** What « Cloner » hands the wizard: the team and the project of its list. */
export interface CloneFrom {
  team: string;
  teamName: string;
  project: TeamProjectView;
}

export const teamProjectsApi = {
  get: (team: string) => invoke<TeamProjectsView>('team_projects', { team }),
  put: (team: string, project: string | null, listed: Listed) => invoke<TeamProjectsView>('team_project_put', { team, project, listed }),
  remove: (team: string, project: string) => invoke<TeamProjectsView>('team_project_remove', { team, project }),
  retry: (team: string) => invoke<TeamProjectsView>('team_projects_retry', { team }),
  clone: (team: string, project: string, parent: string, folder: string) =>
    invoke<CloneView>('team_project_clone', { team, project, parent, folder }),
  logo: (team: string, project: string) => invoke<string | null>('team_project_logo', { team, project }),
};

/** The projects of the list already on this device, and the others (« Cloner »). */
export function split(view: TeamProjectsView): { here: TeamProjectView[]; elsewhere: TeamProjectView[] } {
  return {
    here: view.projects.filter((p) => p.here),
    elsewhere: view.projects.filter((p) => !p.here),
  };
}

/** The local projects whose repository is in the list: TS-13's box gives way to the list for them. */
export function listedHere(view: TeamProjectsView | null | undefined): Set<string> {
  return new Set((view?.projects ?? []).flatMap((p) => (p.here ? [p.here.project] : [])));
}

/** The folder a clone goes to by default: the project's name made a folder name, else its repository's. */
export function folderName(p: TeamProjectView): string {
  const fromName = p.name
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, '-')
    .replace(/^[-.]+|-+$/g, '');
  if (fromName) return fromName;
  return p.repos[0]?.path.split('/').pop() || 'projet';
}

/** The parent folder offered for a clone: the one that holds most of the projects of this device. */
export function parentOf(paths: string[]): string {
  const counts = new Map<string, number>();
  for (const p of paths) {
    const parent = p.replace(/[\\/]+$/, '').replace(/[\\/][^\\/]*$/, '');
    if (parent && parent !== p) counts.set(parent, (counts.get(parent) ?? 0) + 1);
  }
  let best = '';
  let n = 0;
  for (const [k, v] of counts) if (v > n) [best, n] = [k, v];
  return best;
}

/** A draft of the administrator's form, from a project of the list (or empty). */
export function draftOf(p: TeamProjectView | null): Listed {
  if (!p) return { name: '', color: '', logo: null, repos: [{ url: '', folder: '' }], branch: '', access: { kind: 'team' } };
  return {
    name: p.name,
    color: p.color,
    logo: p.logo,
    repos: p.repos.map((r) => ({ url: r.url, folder: p.repos.length > 1 ? r.folder : '' })),
    branch: p.branch,
    access: JSON.parse(JSON.stringify(p.access)) as Access,
  };
}

/** The draft as it is sent: the empty repositories left out. */
export function listedOf(d: Listed): Listed {
  return {
    ...d,
    name: d.name.trim(),
    branch: d.branch.trim(),
    repos: d.repos.map((r) => ({ url: r.url.trim(), folder: r.folder.trim() })).filter((r) => r.url),
  };
}

/** The logos the list takes: raster images only (an SVG can carry a script). */
export function logoOk(url: string | null): boolean {
  return !url || /^data:image\/(png|jpeg|webp);base64,/.test(url);
}
