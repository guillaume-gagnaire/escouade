// Fixtures of the team's settings (TS-13) for the tests of src/lib/team-settings.ts and of the screens.
import type { SettingsView, TeamFile, TeamSettingsView, VersionView } from '../lib/team-settings';

export const NOW = 1_800_000_000;

export function file(over: Partial<TeamFile> = {}): TeamFile {
  return { version: 1, defaults: {}, enforced: {}, proposed: {}, reasons: {}, quorum: null, signers: [], ...over };
}

export function version(number: number, f: TeamFile, over: Partial<VersionView> = {}): VersionView {
  return {
    number,
    digest: `${number}`.padStart(64, '0'),
    by: 'Claire Martin',
    author: '11'.repeat(16),
    at: NOW - 300,
    note: '',
    file: f,
    signers: ['11'.repeat(16)],
    need: 1,
    signedByMe: false,
    status: { kind: 'superseded' },
    ...over,
  };
}

export function settings(
  versions: VersionView[],
  over: Partial<TeamSettingsView> = {},
  view: Partial<SettingsView> = {},
): TeamSettingsView {
  const applied = versions.find((v) => v.status.kind === 'applied')?.number ?? null;
  return {
    team: 'acme',
    name: 'Acme',
    admin: false,
    me: '22'.repeat(16),
    yaml: 'version: 1\n',
    seen: null,
    signerName: '',
    view: {
      applied,
      versions,
      refusals: [],
      alarm: null,
      quorum: false,
      admins: 1,
      effective: { defaults: {}, enforced: {}, accepted: {}, pending: [], unknown: [], reasons: {} },
      ...view,
    },
    ...over,
  };
}
