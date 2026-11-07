import { fSince } from './format';
import { t } from './i18n';
import { app } from './state.svelte';
import type { Agent, BoardSettings, BranchCi, CiOf, PrCheck, Project, Ticket } from './types';

// The CI of a branch without a pull request (ESC-63): a project's target in its board's header, the branch a ticket
// pushed (« Valider et pousser ») on its card. Read by the backend with the pull requests of the tickets; a click opens
// its jobs (CiModal), their logs in reading only, « Relancer les jobs en échec » and « Envoyer à un agent ».

/** Its state as the pill draws it: red, green, waiting, or no job. */
export type CiTone = 'failure' | 'success' | 'pending' | 'none';

export function ciTone(ci: Pick<BranchCi, 'checks'>): CiTone {
  switch (ci.checks) {
    case 'failure':
      return 'failure';
    case 'pending':
      return 'pending';
    case 'success':
    case 'skipped':
      return 'success';
    default:
      return 'none';
  }
}

/** « main ✗ depuis 2 h », « main ✓ », « main · CI en cours ». */
export function ciPill(ci: BranchCi, now: number): string {
  const tone = ciTone(ci);
  const since = ci.redSince ? fSince(ci.redSince, now) : '';
  return t(`board.ci.pill.${tone}`, { branch: ci.branch, since });
}

/** Its jobs in words: « 2/5 jobs en échec ». */
export function ciState(ci: BranchCi): string {
  const tone = ciTone(ci);
  if (tone === 'none') return t('board.ci.state.none');
  return t(`board.ci.state.${tone}`, { count: ci.checksTotal, failed: ci.checksFailed });
}

/** The pill's tooltip: its state, and why its last reading failed. */
export function ciTitle(ci: BranchCi, now: number): string {
  const head = t('board.ci.pillTitle', { branch: ci.branch, state: ciState(ci) });
  if (!ci.error) return head;
  return `${head}\n${t('board.ci.error', { error: ci.error, since: ci.errorSince ? fSince(ci.errorSince, now) : '' })}`;
}

/** « Pause quand la CI de la cible est rouge » holds the board's tickets back now (the backend's `pause_holds`). */
export function redTargetHolds(s: Pick<BoardSettings, 'pauseOnRedTarget'>, ci: BranchCi | null | undefined): boolean {
  return !!s.pauseOnRedTarget && !!ci && ci.checks === 'failure';
}

/** A pill is worth showing: its branch has jobs, or its reading failed. A branch without any CI shows none. */
export function ciShown(ci: BranchCi | null | undefined): ci is BranchCi {
  return !!ci && (!!ci.checks || !!ci.error);
}

/** The CI of a project's target, as last read; none without a host Escouade reads. */
export function targetCiOf(project: Pick<Project, 'id'>): BranchCi | null {
  return app.targetCi[project.id] ?? null;
}

/** The pill of a ticket's card « Terminé »: the branch it pushed, once read with jobs (or failed to be read). */
export function pushedCi(ticket: Ticket): BranchCi | null {
  const ci = ticket.pushed;
  if (ticket.column !== 'done' || !ciShown(ci)) return null;
  return ci;
}

/** Whose CI a ticket's card opens: the branch it pushed, or its pull request « En revue ». */
export function ciOfTicket(ticket: Ticket): CiOf | null {
  if (ticket.column === 'pr' && ticket.pr) return { kind: 'ticketPr', id: ticket.id };
  if (ticket.pushed) return { kind: 'pushed', id: ticket.id };
  return null;
}

/** A failed job can go to an agent (« Envoyer à un agent »): failed. */
export function canSend(c: PrCheck): boolean {
  return c.state === 'failure' || c.cancelled;
}

/** The agents of a project a log may be sent to: not archived, the most recent first. */
export function sendTargets(agents: Agent[], projectId: string): Agent[] {
  return agents.filter((a) => a.projectId === projectId && !a.archived).sort((x, y) => (y.lastActivity ?? 0) - (x.lastActivity ?? 0));
}

/** A line of a job's log, split into its runs of colour (ANSI SGR), as a terminal shows them. */
export interface LogRun {
  text: string;
  /** Its colour's class (`fg-1`…`fg-15`), bold, dim; none: the terminal's own. */
  cls: string;
}

/** The colours and weights of the SGR sequences (`\x1b[31m`…) a job's log holds, the other escapes dropped. */
export function ansiRuns(text: string): LogRun[] {
  const out: LogRun[] = [];
  let fg = -1;
  let bold = false;
  let dim = false;
  const push = (s: string) => {
    if (!s) return;
    const cls = [fg >= 0 ? `fg-${fg}` : '', bold ? 'b' : '', dim ? 'd' : ''].filter(Boolean).join(' ');
    const last = out[out.length - 1];
    if (last && last.cls === cls) last.text += s;
    else out.push({ text: s, cls });
  };
  // eslint-disable-next-line no-control-regex
  const re = /\x1b\[([0-9;]*)([A-Za-z])/g;
  let at = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    push(text.slice(at, m.index));
    at = m.index + m[0].length;
    if (m[2] !== 'm') continue;
    const codes = m[1] === '' ? [0] : m[1].split(';').map(Number);
    for (let i = 0; i < codes.length; i++) {
      const c = codes[i];
      if (c === 0) [fg, bold, dim] = [-1, false, false];
      else if (c === 1) bold = true;
      else if (c === 2) dim = true;
      else if (c === 22) [bold, dim] = [false, false];
      else if (c === 39) fg = -1;
      else if (c >= 30 && c <= 37) fg = c - 30;
      else if (c >= 90 && c <= 97) fg = c - 90 + 8;
      // 256 colours and true colours: their arguments skipped, the terminal's own colour kept.
      else if (c === 38) i += codes[i + 1] === 5 ? 2 : 4;
    }
  }
  push(text.slice(at));
  return out;
}

/** GitHub's and Gitea's markers of a log's lines (`##[error]`, `##[group]`): an error stands out, a group is a title. */
export function logLineKind(line: string): 'error' | 'warning' | 'group' | null {
  const l = line.replace(/^\S+Z\s/, '');
  if (/^##\[error\]|^ERROR:|^error(\[|:)/.test(l)) return 'error';
  if (/^##\[warning\]/.test(l)) return 'warning';
  if (/^##\[group\]|^section_start:/.test(l)) return 'group';
  return null;
}
