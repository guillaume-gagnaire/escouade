// « En parallèle » of a Kanban (ESC-620): the bound the core gives (`board_capacity`: 16 on a desktop, 64 on a server,
// lower under a ceiling the device's policy or the team imposes), and what the window says under it: the ceiling and
// where it comes from, and beyond 6 what the machine holds and that the quota goes faster. An indication, never a
// refusal: the core keeps the bound (`parallel.rs`).

import { t } from './i18n';
import type { ParallelCapacity } from './types';

/** What the window goes by before the core answered: a desktop's bound, no ceiling. */
export const DESKTOP_LIMIT = 16;

/** The most the setting takes. */
export function parallelMax(cap: ParallelCapacity | null): number {
  return cap?.max ?? DESKTOP_LIMIT;
}

/** `n` within 1 and the bound (a field typed empty or beyond). */
export function withinBound(n: number, cap: ParallelCapacity | null): number {
  if (!Number.isFinite(n)) return 1;
  return Math.min(Math.max(Math.round(n), 1), parallelMax(cap));
}

/** « Plafond de 8, imposé par l'équipe Acme »; null when none is imposed. */
export function ceilingLine(cap: ParallelCapacity | null): string | null {
  const c = cap?.ceiling;
  if (!c) return null;
  return c.by.kind === 'team'
    ? t('boardSettings.tab.parallelCeilingTeam', { max: c.value, team: c.by.team })
    : t('boardSettings.tab.parallelCeilingDevice', { max: c.value });
}

/** Beyond `warnAbove`: what the machine holds (its cores, its memory, the room an agent takes) and that the quota goes
 *  faster; null at or below it, or before the core answered. */
export function parallelWarning(n: number, cap: ParallelCapacity | null): string | null {
  if (!cap || n <= cap.warnAbove) return null;
  const machine = t(cap.server ? 'boardSettings.tab.parallelServer' : 'boardSettings.tab.parallelDesktop');
  const gb = (mb: number) => String(Math.round(mb / 1024));
  const said = cap.machine.memoryMb
    ? t('boardSettings.tab.parallelMachine', { machine, cores: cap.machine.cores, memory: gb(cap.machine.memoryMb) })
    : t('boardSettings.tab.parallelMachineCores', { machine, cores: cap.machine.cores });
  return t('boardSettings.tab.parallelWarn', { count: cap.fits, above: cap.warnAbove, machine: said, agent: gb(cap.agentMb) });
}
