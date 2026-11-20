// The teams of the team server on this device (TS-3, `src-tauri/src/teams.rs`): what the screens of `components/team`
// read and ask. The backend reads each team every few seconds (an administrator online lets the invitees in) and says
// when one changed (`teams`), and the list is read again.

import { invoke } from './transport';
import { teamSettingsApi, type TeamSettingsView } from './team-settings';
import { teamCostsApi } from './team-costs';
import { app } from './state.svelte';

export interface AlarmView {
  kind: 'fork' | 'rollback' | 'rewritten' | 'journal';
  epoch: number | null;
  at: number;
}

export interface InvitationView {
  id: string;
  kind: 'member' | 'device';
  role: 'admin' | 'member' | null;
  /** Seconds since the epoch. */
  expiresAt: number;
  used: boolean;
  ready: boolean;
  reusedAt: number | null;
}

export interface TeamView {
  id: string;
  name: string;
  server: string;
  joined: boolean;
  role: 'admin' | 'member' | 'recovery' | 'none';
  admins: number;
  members: number;
  epoch: number | null;
  fingerprint: string[];
  head: string[];
  escrow: string | null;
  keysInFile: boolean;
  recoveryConfirmed: boolean;
  wordsPending: boolean;
  alarm: AlarmView | null;
  invitations: InvitationView[];
  lastError: string | null;
}

export interface Created {
  team: TeamView;
  words: string[];
}

export interface InviteView {
  url: string;
  deepLink: string;
  expiresAt: number;
  ready: boolean;
  /** The link's QR code, SVG (made by the backend: dark on light). */
  qr: string;
}

export interface JoinPreview {
  server: string;
  fingerprint: string[];
  ownDevice: boolean;
}

export type PairState = 'waiting' | 'waitingForToken' | 'sent' | 'wrongCode';

export interface PairView {
  code: string;
  server: string;
  expiresAt: number;
  state: PairState;
}

export interface EscrowPair {
  public: string;
  secret: string;
  holder: string;
}

export const teamsApi = {
  list: () => invoke<TeamView[]>('team_list'),
  create: (server: string, token: string, name: string, escrowPublic: string | null, escrowHolder: string | null) =>
    invoke<Created>('team_create', { server, token, name, escrowPublic, escrowHolder }),
  escrowNew: (holder: string) => invoke<EscrowPair>('team_escrow_new', { holder }),
  confirmWords: (team: string, typed: [number, string][]) => invoke<boolean>('team_confirm_words', { team, typed }),
  newRecovery: (team: string) => invoke<string[]>('team_new_recovery', { team }),
  invite: (team: string, role: 'admin' | 'member', hours: number) => invoke<InviteView>('team_invite', { team, role, hours }),
  ownDevice: (team: string) => invoke<InviteView>('team_own_device', { team }),
  joinPreview: (link: string) => invoke<JoinPreview>('team_join_preview', { link }),
  join: (link: string, name: string) => invoke<TeamView>('team_join', { link, name }),
  pairStart: (team: string) => invoke<PairView>('team_pair_start', { team }),
  pairStep: (team: string) => invoke<PairView>('team_pair_step', { team }),
  pairCancel: (team: string) => invoke<void>('team_pair_cancel', { team }),
  pairJoin: (server: string, code: string, name: string) => invoke<TeamView>('team_pair_join', { server, code, name }),
  recover: (server: string, token: string, words: string, name: string) => invoke<Created>('team_recover', { server, token, words, name }),
  sync: (team: string) => invoke<TeamView>('team_sync', { team }),
  forget: (team: string) => invoke<void>('team_forget', { team }),
};

/** A backend error as text (the commands answer French sentences). */
export function errorText(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
}

class Teams {
  list = $state<TeamView[]>([]);
  loaded = $state(false);
  /** The signed settings of each team this device is in (TS-13), by team id: the band and the screen read them. */
  settings = $state<Record<string, TeamSettingsView>>({});

  async load() {
    try {
      this.list = (await teamsApi.list()) ?? [];
    } catch {
      this.list = [];
    }
    this.loaded = true;
    await this.loadSettings();
    await this.tellGrain();
  }

  /** The teams whose « par membre » this session offered to tell already (TS-16, C15). */
  private told = new Set<string>();

  /** A team gone « par membre » is told to its member at the next opening, before its totals carry its name. */
  async tellGrain() {
    if (!this.list.some((x) => x.joined)) return;
    let untold: [string, string][] = [];
    try {
      untold = (await teamCostsApi.untold()) ?? [];
    } catch {
      return;
    }
    const next = untold.find(([id]) => !this.told.has(id));
    if (!next || app.modal) return;
    this.told.add(next[0]);
    app.modal = { kind: 'team', step: 'told', teamId: next[0] };
  }

  async loadSettings() {
    const next: Record<string, TeamSettingsView> = {};
    for (const team of this.list.filter((x) => x.joined)) {
      try {
        next[team.id] = await teamSettingsApi.get(team.id);
      } catch {
        // A team that does not open: its card says why.
      }
    }
    this.settings = next;
  }

  putSettings(s: TeamSettingsView) {
    this.settings = { ...this.settings, [s.team]: s };
  }

  byId(id: string | null | undefined): TeamView | undefined {
    return this.list.find((t) => t.id === id);
  }

  /** One team, as the backend answered it: in place of the one listed, or added. */
  put(team: TeamView) {
    const i = this.list.findIndex((t) => t.id === team.id);
    if (i >= 0) this.list[i] = team;
    else this.list = [...this.list, team];
  }
}

export const teams = new Teams();
