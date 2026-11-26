// « La mémoire d'équipe » (TS-10, `src-tauri/src/teammemory.rs`): the shared memory of a team's project on the team
// server. Each memory with its signed provenance (who, which agent, when, by hand or by an agent, whether that agent had
// read a third party's data); a memory an agent writes is born proposed and becomes the team's once a human keeps it
// (at its ticket's validation, or in Réglages › Projets › Équipe › Mémoire partagée); an administrator puts in quarantine by a
// rule, after a preview. Pure functions here, the screens in `components`.

import { invoke } from './transport';
import type { MemoryKind } from './types';

export interface AgentOf {
  name: string;
  ticket: string | null;
  harness: string;
  model: string;
}

export interface Provenance {
  member: string;
  device: string;
  /** The member's name, as their device says it. */
  name: string;
  at: number;
  by: 'hand' | 'agent' | string;
  agent: AgentOf | null;
  thirdParty: boolean;
  imported: boolean;
}

export type Standing = 'accepted' | 'proposed' | 'rejected';

export interface TeamMemoryEntry {
  id: string;
  name: string;
  description: string;
  kind: MemoryKind;
  body: string;
  provenance: Provenance;
  standing: Standing;
  reviewedBy: string | null;
  removed: boolean;
  removedBy: string | null;
  quarantined: string | null;
  /** Its removal proposed by a tool (ESC-647): still read until a member settles it here. */
  removal?: RemovalAsk | null;
  mine: boolean;
  untrusted: boolean;
  /** The hosts of the addresses it names that the project does not know. */
  unknown: string[];
  at: number;
}

/** A removal a tool proposed (ESC-647): the member whose agent (or outside client) asked, and when. */
export interface RemovalAsk {
  member: string;
  name: string;
  agent: AgentOf | null;
  at: number;
}

export interface QuarantineRule {
  ticket?: string | null;
  member?: string | null;
  device?: string | null;
  since?: number | null;
}

export interface RuleView extends QuarantineRule {
  id: string;
  by: string;
  byName: string;
  at: number;
  count: number;
}

export interface AuthorView {
  member: string;
  device: string;
  name: string;
}

export interface TeamMemoryView {
  on: boolean;
  review: boolean;
  reviewLockedBy: string | null;
  admin: boolean;
  me: string;
  memories: TeamMemoryEntry[];
  proposals: TeamMemoryEntry[];
  /** The removals a tool proposed (ESC-647): « Retirer » or « Garder ». */
  removals?: TeamMemoryEntry[];
  withdrawn: TeamMemoryEntry[];
  rules: RuleView[];
  authors: AuthorView[];
  tickets: string[];
  refused: number;
}

export interface QuarantinePreview {
  rule: QuarantineRule;
  entries: TeamMemoryEntry[];
  applied: string | null;
}

export const teamMemoryApi = {
  view: (projectId: string) => invoke<TeamMemoryView>('team_memory_view', { projectId }),
  decide: (projectId: string, id: string, keep: boolean) => invoke<TeamMemoryView>('team_memory_decide', { projectId, id, keep }),
  decideRemoval: (projectId: string, id: string, remove: boolean) =>
    invoke<TeamMemoryView>('team_memory_decide_removal', { projectId, id, remove }),
  quarantine: (projectId: string, rule: QuarantineRule, apply: boolean) =>
    invoke<QuarantinePreview>('team_memory_quarantine', { projectId, rule, apply }),
  lift: (projectId: string, ruleId: string) => invoke<TeamMemoryView>('team_memory_lift', { projectId, ruleId }),
  ofTicket: (projectId: string, key: string) => invoke<TeamMemoryEntry[]>('team_memory_of_ticket', { projectId, key }),
  setReview: (projectId: string, on: boolean) => invoke<unknown>('team_memory_set_review', { projectId, on }),
};

/** A member or a device, short: its first eight hexadecimal digits. */
export function short(hex: string): string {
  return hex.slice(0, 8);
}

/** Who wrote it, as one line reads it: the agent (its ticket, harness and model) or the member by hand. */
export function author(p: Provenance): { who: string; agent: string | null } {
  const who = p.name.trim() || short(p.member);
  if (p.by !== 'agent' || !p.agent) return { who, agent: null };
  const a = p.agent;
  const bits = [a.ticket, a.harness, a.model].filter((b): b is string => !!b && !!b.trim());
  return { who, agent: [a.name.trim() || null, ...bits].filter(Boolean).join(' · ') };
}

/** What a quarantine with this rule would take out of `entries`, as the core judges it (`Rule::takes`). */
export function takes(rule: QuarantineRule, e: TeamMemoryEntry): boolean {
  const empty = !rule.ticket && !rule.member && !rule.device && rule.since == null;
  if (empty) return false;
  const p = e.provenance;
  if (rule.ticket && (p.agent?.ticket ?? '').toUpperCase() !== rule.ticket.toUpperCase()) return false;
  if (rule.member && p.member !== rule.member) return false;
  if (rule.device && p.device !== rule.device) return false;
  if (rule.since != null && p.at < rule.since) return false;
  return true;
}

/** The milliseconds of the start of a day typed as `YYYY-MM-DD`, in local time; null when it is not one. */
export function dayStart(text: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text.trim());
  if (!m) return null;
  const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  return Number.isNaN(d.getTime()) ? null : d.getTime();
}
