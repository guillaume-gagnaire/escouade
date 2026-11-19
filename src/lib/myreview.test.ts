import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { agentOffer, mineFailure, pendingPlace, placePending, sortPending, submitBlock, verdictsOf } from './myreview';
import type { PrWriting } from './prthreads';
import type { PendingNote, PrContentsFile, Ticket } from './types';

// « Ta relecture » (ESC-494): the verdicts a host gives (a comment alone on the user's own pull request), what the
// publication needs, where the remarks waiting go in the tab « Fichiers », the case for the ticket's agent, and a
// refusal in words that say all of it stays waiting.

const GITHUB: PrWriting = {
  comment: true,
  lineComment: true,
  reply: true,
  resolve: true,
  reopen: true,
  review: true,
  approve: true,
  requestChanges: true,
};
const GITLAB: PrWriting = { ...GITHUB, lineComment: false, reopen: false, requestChanges: false };

const note = (over: Partial<PendingNote> = {}): PendingNote => ({
  id: 1,
  path: 'src/a.ts',
  line: 2,
  side: 'right',
  body: 'Pourquoi ?',
  code: ['b'],
  created: 0,
  ...over,
});

const FILE: PrContentsFile = {
  path: 'src/a.ts',
  status: 'modified',
  additions: 1,
  deletions: 1,
  lines: [
    { kind: '@', text: '@@ -1,2 +1,2 @@' },
    { kind: ' ', old: 1, new: 1, text: 'a' },
    { kind: '-', old: 2, text: 'x' },
    { kind: '+', new: 2, text: 'b' },
  ],
  binary: false,
  generated: false,
  cut: false,
};

beforeEach(() => setLang('fr'));

describe('« Ta relecture » (lib/myreview)', () => {
  it('offers the verdicts the host gives, a comment alone on the user’s own pull request with its reason', () => {
    expect(verdictsOf(GITHUB, false, 'GitHub', 'PR').map((v) => [v.verdict, v.offered])).toEqual([
      ['commented', true],
      ['approved', true],
      ['changesRequested', true],
    ]);
    const own = verdictsOf(GITHUB, true, 'GitHub', 'PR');
    expect(own.map((v) => v.offered)).toEqual([true, false, false]);
    expect(own[1].why).toBe('GitHub ne permet que de commenter ta propre PR : ni approuver, ni demander des changements.');
    const gitlab = verdictsOf(GITLAB, false, 'GitLab', 'MR');
    expect(gitlab.map((v) => v.offered)).toEqual([true, true, false]);
    expect(gitlab[2].why).toBe('GitLab ne donne pas ce verdict depuis Escouade.');
    expect(verdictsOf({ ...GITHUB, review: false }, false, 'GitHub', 'PR')[0].offered).toBe(false);
  });

  it('needs a text to ask for changes and something to say to comment', () => {
    expect(submitBlock('changesRequested', ' ', [note()])).toBe('Écris le texte de ta relecture pour demander des changements.');
    expect(submitBlock('commented', '', [])).toBe('Rien à publier : écris un texte ou ajoute une remarque.');
    expect(submitBlock('commented', '', [note()])).toBeNull();
    expect(submitBlock('approved', '', [])).toBeNull();
    expect(submitBlock('changesRequested', 'Deux points.', [])).toBeNull();
  });

  it('puts a remark under its line, a stale one apart in its file, one on a file out of the diff above the files', () => {
    const placed = placePending([FILE], [note(), note({ id: 2, stale: true }), note({ id: 3, path: 'gone.ts' })]);
    expect(placed.under['src/a.ts'][3].map((n) => n.id)).toEqual([1]);
    expect(placed.stale['src/a.ts'].map((n) => n.id)).toEqual([2]);
    expect(placed.top.map((n) => n.id)).toEqual([3]);
    // A line removed is on the base's side.
    const left = placePending([FILE], [note({ id: 4, side: 'left', line: 2 })]);
    expect(left.under['src/a.ts'][2].map((n) => n.id)).toEqual([4]);
    expect(pendingPlace(note({ startLine: 1 }))).toBe('src/a.ts:1-2');
    expect(
      sortPending([note({ id: 2, path: 'b.ts' }), note({ id: 1, line: 9 }), note({ id: 3, line: 1 })], ['src/a.ts', 'b.ts']).map(
        (n) => n.id,
      ),
    ).toEqual([3, 1, 2]);
  });

  it('offers the ticket’s agent the changes asked, checked when asked or on the user’s own pull request', () => {
    const tk = { column: 'pr', agentId: 'a1' } as Ticket;
    expect(agentOffer(tk, 'changesRequested', false)).toEqual({ offered: true, checked: true });
    expect(agentOffer(tk, 'commented', false)).toEqual({ offered: true, checked: false });
    expect(agentOffer(tk, 'commented', true)).toEqual({ offered: true, checked: true });
    expect(agentOffer(tk, 'approved', false).offered).toBe(false);
    expect(agentOffer({ ...tk, agentId: null } as Ticket, 'changesRequested', false).offered).toBe(false);
    expect(agentOffer({ ...tk, column: 'done' } as Ticket, 'changesRequested', false).offered).toBe(false);
    expect(agentOffer(null, 'changesRequested', false).offered).toBe(false);
  });

  it('says a refusal in words that keep the review waiting', () => {
    expect(mineFailure({ kind: 'forbidden', message: '' }, 'GitHub')).toBe(
      'Ton jeton GitHub n’a pas le droit d’écrire sur ce dépôt. Rien n’est parti, ta relecture reste en attente.',
    );
    expect(mineFailure({ kind: 'unreachable', message: '' }, 'GitHub', false)).toBe(
      'Hors ligne : rien n’est parti, ta relecture reste en attente.',
    );
    expect(mineFailure({ kind: 'lines', message: 'GitHub n’a plus la ligne d’une remarque.' }, 'GitHub')).toBe(
      'GitHub n’a plus la ligne d’une remarque. Ta relecture reste en attente.',
    );
  });
});
