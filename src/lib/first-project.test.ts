import { describe, expect, it } from 'vitest';
import { cloneName, cloneParent, repoUrl, samePath, ymlList } from './first-project';

describe('the first project (ESC-505)', () => {
  it('names the folder of a clone after its repository', () => {
    expect(cloneName('https://github.com/acme/api.git')).toBe('api');
    expect(cloneName('git@gitlab.com:acme/front-web.git')).toBe('front-web');
    expect(cloneName('https://example.com/acme/site/')).toBe('site');
    expect(cloneName('')).toBe('');
  });

  it('makes the address of a repository of a code host connected', () => {
    expect(repoUrl('acme/api', 'github.com')).toBe('https://github.com/acme/api.git');
    expect(repoUrl('gitlab.acme.fr/web/site', 'gitlab.acme.fr')).toBe('https://gitlab.acme.fr/web/site.git');
  });

  it('clones by default where the projects are, else in the home the demonstration tells', () => {
    expect(cloneParent(['C:\\dev\\a', 'C:\\dev\\b', 'D:\\x\\c'], '')).toBe('C:\\dev');
    expect(cloneParent([], 'C:\\Users\\ada\\.escouade\\demo\\bac-a-sable')).toBe('C:\\Users\\ada');
    expect(cloneParent([], '/home/ada/.escouade/demo/bac-a-sable')).toBe('/home/ada');
  });

  it('knows a folder whatever its spelling, and says what an .escouade.yml sets in a few keys', () => {
    expect(samePath('C:\\Users\\ada\\.escouade\\demo\\bac-a-sable\\', 'c:/users/ada/.escouade/demo/bac-a-sable')).toBe(true);
    expect(samePath('C:/a', 'C:/b')).toBe(false);
    expect(samePath('', '')).toBe(false);
    expect(ymlList(['a', 'b'])).toBe('a, b');
    expect(ymlList(['a', 'b', 'c', 'd', 'e'])).toBe('a, b, c, d…');
  });
});
