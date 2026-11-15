import { describe, expect, it } from 'vitest';
import { allows, bandOf, bandText, chain, changesOf, diff, shown, statusLabel } from './team-settings';

import { file, NOW, settings, version } from '../test/team-settings';

describe('the diff of the team’s settings (TS-13)', () => {
  it('says what each version adds, removes and changes, part by part', () => {
    const a = file({
      enforced: { 'permissions.modes': ['plan'], 'telemetry.enabled': false },
      defaults: { 'board.merge_strategy': 'squash' },
    });
    const b = file({
      enforced: { 'permissions.modes': ['plan', 'default'], 'proxy.url': 'http://proxy:3128' },
      defaults: { 'board.merge_strategy': 'squash' },
      proposed: { 'worktrees.setup': ['npm ci'] },
      quorum: true,
    });
    expect(diff(a, b)).toEqual([
      { part: 'enforced', key: 'permissions.modes', kind: 'changed', before: ['plan'], after: ['plan', 'default'] },
      { part: 'enforced', key: 'proxy.url', kind: 'added', after: 'http://proxy:3128' },
      { part: 'enforced', key: 'telemetry.enabled', kind: 'removed', before: false },
      { part: 'proposed', key: 'worktrees.setup', kind: 'added', after: ['npm ci'] },
      { part: 'team', key: 'quorum', kind: 'changed', before: null, after: true },
    ]);
    // The first version: everything it sets is added.
    expect(diff(null, file({ enforced: { 'telemetry.enabled': false } }))).toEqual([
      { part: 'enforced', key: 'telemetry.enabled', kind: 'added', after: false },
    ]);
    expect(diff(a, a)).toEqual([]);
    expect(shown(['npm ci'])).toBe('["npm ci"]');
    expect(shown('pr')).toBe('pr');
  });

  it('keeps the history in the chain, newest first, each version with its change', () => {
    const v1 = version(1, file({ enforced: { 'telemetry.enabled': false } }));
    const v2 = version(2, file({ enforced: { 'telemetry.enabled': false, 'permissions.modes': ['plan'] } }), {
      status: { kind: 'applied' },
    });
    const v3 = version(3, file(), { status: { kind: 'proposed' }, need: 2 });
    const bad = version(2, file(), { digest: 'f'.repeat(64), status: { kind: 'refused', reason: { kind: 'removedAdmin' } } });
    const s = settings([v3, bad, v2, v1]);
    expect(chain(s.view).map((v) => v.number)).toEqual([2, 1]);
    expect(changesOf(s.view, v2)).toEqual([{ part: 'enforced', key: 'permissions.modes', kind: 'added', after: ['plan'] }]);
    expect(statusLabel(v2)).toBe('En vigueur');
    expect(statusLabel(v1)).toBe('Remplacée');
    expect(statusLabel(v3)).toBe('En attente d’une seconde signature (1/2)');
    expect(statusLabel(bad)).toBe('Refusée : signée par un administrateur retiré depuis');
    expect(
      statusLabel(
        version(4, file(), {
          status: { kind: 'refused', reason: { kind: 'never', hit: { rule: 'bypass', part: 'enforced', key: 'permissions.modes' } } },
        }),
      ),
    ).toBe('Refusée : enforced.permissions.modes n’entre jamais dans le fichier d’équipe');
  });
});

describe('the band of a team’s settings', () => {
  const v1 = version(1, file({ enforced: { 'telemetry.enabled': false } }));
  const v2 = version(
    2,
    file({ enforced: { 'telemetry.enabled': false, 'permissions.modes': ['plan'], 'harnesses.allowed': ['claude'] } }),
    {
      status: { kind: 'applied' },
    },
  );

  it('says what changed since the version the member saw, who signed it and when', () => {
    const b = bandOf(settings([v2, v1], { seen: 1 }));
    expect(b).toMatchObject({ team: 'acme', number: 2, changes: 2, by: 'Claire Martin', pending: 0 });
    expect(bandText(b!, NOW * 1000)).toBe('L’équipe Acme a changé 2 réglages · Claire Martin, il y a 5 min');
    // Never seen: everything the version in force sets.
    expect(bandOf(settings([v2, v1]))?.changes).toBe(3);
  });

  it('is gone once seen, and comes back for what waits for this device’s consent', () => {
    expect(bandOf(settings([v2, v1], { seen: 2 }))).toBeNull();
    const pending = settings(
      [v2, v1],
      { seen: 2 },
      {
        effective: {
          defaults: {},
          enforced: {},
          accepted: {},
          unknown: [],
          reasons: {},
          pending: [{ part: 'enforced', key: 'proxy.url', value: 'http://p', sort: 'sensitive', fingerprint: 'ab' }],
        },
      },
    );
    const b = bandOf(pending);
    expect(b).toMatchObject({ changes: 0, pending: 1 });
    expect(bandText(b!, NOW * 1000)).toBe('Acme demande ton accord pour 1 réglage');
    // No version in force: no band.
    expect(bandOf(settings([]))).toBeNull();
  });

  it('says in clear words what a consent allows', () => {
    expect(allows({ key: 'proxy.authorities', sort: 'sensitive' })).toBe('Ce certificat permet de lire le trafic TLS d’Escouade.');
    expect(allows({ key: 'proxy.url', sort: 'sensitive' })).toMatch(/tout le trafic d’Escouade/);
    expect(allows({ key: 'worktrees.setup', sort: 'executable' })).toMatch(/s’exécutera sur ce poste/);
  });
});
