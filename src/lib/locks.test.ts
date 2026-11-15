import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { bounded, denial, deniedBy, lockedBy, lockLine, locks, setLocks } from './locks.svelte';
import { SETTINGS_ROWS } from './settings-pages';
import type { Locks } from './types';

const DEVICE = { kind: 'device', source: 'the machine’s policy' } as const;
const ACME = { kind: 'team', team: 'Acme' } as const;

/** What the core sends (`locks::Locks`): telemetry imposed off by the device, the modes bounded by the device and Acme. */
const LOCKS: Locks = {
  locks: {
    'telemetry.enabled': { key: 'telemetry.enabled', value: false, by: DEVICE, bounds: [] },
    'permissions.modes': {
      key: 'permissions.modes',
      value: null,
      by: null,
      bounds: [
        { by: ACME, allowed: ['plan', 'default', 'acceptEdits'] },
        { by: DEVICE, allowed: ['default', 'acceptEdits', 'auto'] },
      ],
    },
  },
  rows: { telemetry: ['telemetry.enabled'], agentMode: ['permissions.modes'], mode: ['permissions.modes'] },
  warnings: [],
};

describe('the locks of the settings (TS-11)', () => {
  beforeEach(() => {
    setLang('fr');
    setLocks(undefined);
  });

  it('gives each row of the register its locked_by: who imposes its value, none for a bound alone', () => {
    expect(lockedBy('telemetry')).toBeNull();
    setLocks(LOCKS);
    expect(locks.current).toEqual(LOCKS);
    const row = (id: string) => SETTINGS_ROWS.find((r) => r.id === id)!;
    expect(row('telemetry').lockedBy).toEqual(DEVICE);
    // A bound greys choices, not the row.
    expect(row('agentMode').lockedBy).toBeNull();
    expect(row('sound').lockedBy).toBeNull();
    expect(lockLine(DEVICE)).toBe('Imposé par la stratégie de ce poste');
    expect(lockLine(ACME)).toBe('Imposé par l’équipe Acme');
  });

  it('says who denies a choice of a bound, the device first, and greys it with the reason', () => {
    setLocks(LOCKS);
    expect(deniedBy('permissions.modes', 'plan')).toEqual(DEVICE);
    expect(deniedBy('permissions.modes', 'auto')).toEqual(ACME);
    expect(deniedBy('permissions.modes', 'default')).toBeNull();
    expect(denial('permissions.modes', 'auto')).toBe('non permis par l’équipe Acme');
    expect(denial('harnesses.allowed', 'codex')).toBeNull();
    const options = bounded('permissions.modes', [
      { value: 'plan', label: 'Plan' },
      { value: 'default', label: 'Demander' },
      { value: 'auto', label: 'Auto' },
    ]);
    expect(options).toEqual([
      { value: 'plan', label: 'Plan', disabled: true, title: 'Plan · non permis par la stratégie de ce poste' },
      { value: 'default', label: 'Demander' },
      { value: 'auto', label: 'Auto', disabled: true, title: 'Auto · non permis par l’équipe Acme' },
    ]);
  });

  it('names the rows the core’s table locks, each one of the register', () => {
    // locks::KEYS (src-tauri/src/locks.rs): its `rows`, checked there too against this file.
    for (const id of ['allowBypass', 'agentMode', 'mode', 'projectEngine', 'telemetry', 'betaUpdates', 'remoteAll', 'projectRemote'])
      expect(SETTINGS_ROWS.some((r) => r.id === id)).toBe(true);
  });
});
