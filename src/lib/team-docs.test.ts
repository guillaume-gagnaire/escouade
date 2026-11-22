import { describe, expect, it } from 'vitest';
import { bands, bodyOf, fieldText, megabytes, someFiles, syncState } from './team-docs';
import { teamDocsView } from '../test/team-docs';

describe('the bands above the docs of a team’s project (TS-9)', () => {
  it('shows nothing for docs kept in the repository', () => {
    expect(
      bands(teamDocsView({ onServer: false, conflicts: [{ path: 'a/b', title: 'B', mine: '', theirs: '', base: '', agentId: null }] })),
    ).toEqual([]);
    expect(bands(null)).toEqual([]);
    expect(syncState(teamDocsView({ onServer: false }))).toBeNull();
  });

  it('puts what was set aside first, then the conflicts, what waits last', () => {
    const v = teamDocsView({
      refused: [{ kind: 'page', key: '../x', device: 'ab', member: null, mine: false }],
      waitingNumbers: ['choix'],
      repoFiles: ['.escouade/spec/a/b.md'],
      tooLarge: ['video.mp4'],
      decisionConflicts: [{ id: 'd', title: 'D', field: 'context', mine: 'x', theirs: 'y', stamp: '01' }],
      conflicts: [{ path: 'a/b', title: 'B', mine: 'm', theirs: 't', base: '', agentId: null }],
    });
    expect(bands(v).map((b) => b.kind)).toEqual(['refused', 'conflict', 'lost', 'repoFiles', 'tooLarge', 'waiting']);
  });

  it('says offline, the changes waiting, or in step', () => {
    expect(syncState(teamDocsView({ online: false, pending: 3 }))).toBe('offline');
    expect(syncState(teamDocsView({ pending: 2 }))).toBe('pending');
    expect(syncState(teamDocsView())).toBe('synced');
  });

  it('writes a field on one line, lists the first files, the bound in megabytes', () => {
    expect(fieldText(['a', 'b'])).toBe('a, b');
    expect(fieldText(null)).toBe('');
    expect(fieldText({ page: 'x' })).toBe('{"page":"x"}');
    expect(someFiles(['1', '2', '3', '4', '5'])).toEqual({ list: ['1', '2', '3'], more: 2 });
    expect(someFiles(['1'])).toEqual({ list: ['1'], more: 0 });
    expect(megabytes(20 * 1024 * 1024)).toBe(20);
    expect(bodyOf('---\ntitle: "A"\n---\n\nLe texte.\n')).toBe('Le texte.\n');
    expect(bodyOf('Sans en-tête.')).toBe('Sans en-tête.');
  });
});
