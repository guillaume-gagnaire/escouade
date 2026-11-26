import { describe, expect, it } from 'vitest';
import { customPageOf, customPageProblem, relativeToProject } from './custom-page';
import { pendingGroups } from './shared-file';
import type { Project, SharedFile } from './types';
import { project } from '../test/ipc';

/** A project whose .escouade.yml proposes `page`, waiting for trust or not, with what it trusted before. */
function proposing(page: string, { waiting = false, trusted = '' } = {}): Project {
  const file: SharedFile = {
    keys: ['pages.custom'],
    values: { 'pages.custom': page },
    warnings: [],
    error: null,
    version: 1,
    pending: waiting ? { hash: 'h', commands: { 'pages.custom': page } } : null,
  } as SharedFile;
  return project({ fileCustomPage: trusted || undefined, shared: { file, local: [], trusted: '' } as never });
}

describe('custom-page (AP-7)', () => {
  it('none without a page of the machine or of the file', () => {
    expect(customPageOf(project())).toEqual({ from: 'none' });
    expect(customPageOf(project({ customPage: '  ' }))).toEqual({ from: 'none' });
    expect(customPageOf(undefined)).toEqual({ from: 'none' });
  });

  it('the file’s waits for trust, then is the project’s', () => {
    expect(customPageOf(proposing('design/revue.html', { waiting: true }))).toEqual({
      from: 'waiting',
      path: 'design/revue.html',
    });
    expect(customPageOf(proposing('design/revue.html', { trusted: 'design/revue.html' }))).toEqual({
      from: 'repo',
      path: 'design/revue.html',
    });
    // A new value waits; the trusted one stays the project's until then.
    expect(customPageOf(proposing('design/autre.html', { waiting: true, trusted: 'design/revue.html' }))).toEqual({
      from: 'repo',
      path: 'design/revue.html',
    });
  });

  it('the machine’s comes first, and says the file’s it is ahead of', () => {
    const p = { ...proposing('design/revue.html', { trusted: 'design/revue.html' }), customPage: 'mine.html' };
    expect(customPageOf(p)).toEqual({ from: 'machine', path: 'mine.html', file: 'design/revue.html' });
    // The draft being typed, not the saved value.
    expect(customPageOf(p, '')).toEqual({ from: 'repo', path: 'design/revue.html' });
    expect(customPageOf(project({ customPage: 'C:/pages/a.html' }))).toEqual({ from: 'machine', path: 'C:/pages/a.html', file: null });
  });

  it('an .html document or none', () => {
    for (const ok of ['', 'a.html', 'design/Revue.HTM', 'C:\\pages\\a.html']) expect(customPageProblem(ok), ok).toBeNull();
    for (const bad of ['a.js', 'design/', 'page.html.txt']) expect(customPageProblem(bad), bad).toBe('notHtml');
  });

  it('a file chosen in the project’s folder is kept relative to it', () => {
    expect(relativeToProject('C:\\code\\demo-api\\design\\revue.html', 'C:\\code\\demo-api')).toBe('design/revue.html');
    expect(relativeToProject('c:\\Code\\Demo-API\\a.html', 'C:\\code\\demo-api\\')).toBe('a.html');
    expect(relativeToProject('C:\\code\\demo-api-2\\a.html', 'C:\\code\\demo-api')).toBe('C:\\code\\demo-api-2\\a.html');
    expect(relativeToProject('/home/me/pages/a.html', '/home/me/app')).toBe('/home/me/pages/a.html');
    expect(relativeToProject('/home/me/app/a.html', '/home/me/app')).toBe('a.html');
  });

  it('the file’s page is read whole among what waits for trust', () => {
    const groups = pendingGroups(proposing('design/revue.html', { waiting: true }));
    expect(groups).toEqual([{ kind: 'page', commands: [{ name: '', command: 'design/revue.html', cwd: '', shell: '' }] }]);
  });
});
