import { describe, expect, it } from 'vitest';
import { listedProject, repo, teamProjects } from '../test/team-projects';
import { draftOf, folderName, listedHere, listedOf, logoOk, parentOf, split } from './team-projects';

describe('the team’s projects (TS-8)', () => {
  it('names the folder of a clone after the project, else its repository', () => {
    expect(folderName(listedProject({ name: 'Acme API' }))).toBe('acme-api');
    expect(folderName(listedProject({ name: 'Équipe été 2026' }))).toBe('equipe-ete-2026');
    expect(folderName(listedProject({ name: '…' }))).toBe('api');
    // Never a hidden folder from the address (TS-18: a project listed as `…/.ssh` would be cloned into the parent's
    // `.ssh`): the dots and dashes before it go.
    expect(folderName(listedProject({ name: '…', repos: [repo('git@github.com:x/.ssh')] }))).toBe('ssh');
    expect(folderName(listedProject({ name: '.', repos: [repo('https://github.com/x/...')] }))).toBe('projet');
  });

  it('offers the folder that holds most of the projects of this device', () => {
    expect(parentOf(['C:\\dev\\a', 'C:\\dev\\b', 'D:\\work\\c'])).toBe('C:\\dev');
    expect(parentOf(['/home/alice/src/a/', '/home/alice/src/b'])).toBe('/home/alice/src');
    expect(parentOf([])).toBe('');
  });

  it('splits the list into the projects here and the others, and knows the local ones it holds', () => {
    const here = listedProject({
      id: 'h',
      here: {
        project: 'p1',
        name: 'x',
        attached: true,
        otherTeam: false,
        repos: [0],
        nameDiffers: false,
        boardJoined: false,
        boardOther: false,
      },
    });
    const view = teamProjects([listedProject(), here]);
    expect(split(view).here.map((p) => p.id)).toEqual(['h']);
    expect(split(view).elsewhere).toHaveLength(1);
    expect([...listedHere(view)]).toEqual(['p1']);
    expect(listedHere(null).size).toBe(0);
  });

  it('sends a draft without its empty repositories, each address trimmed', () => {
    const d = draftOf(null);
    d.name = ' API ';
    d.repos = [
      { url: ' https://github.com/acme/api ', folder: '' },
      { url: '  ', folder: '' },
    ];
    expect(listedOf(d)).toMatchObject({ name: 'API', repos: [{ url: 'https://github.com/acme/api', folder: '' }] });
    // A project of one repository has no folder of its own.
    expect(draftOf(listedProject()).repos).toEqual([{ url: 'https://github.com/acme/api.git', folder: '' }]);
  });

  it('takes raster logos only', () => {
    expect(logoOk(null)).toBe(true);
    expect(logoOk('data:image/png;base64,iVBO')).toBe(true);
    expect(logoOk('data:image/svg+xml;base64,PHN2Zz4=')).toBe(false);
  });
});
