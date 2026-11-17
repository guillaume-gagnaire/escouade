import { fList, fTime } from './format';
import { t } from './i18n';
import type { PrWriting } from './prthreads';
import type { PrCheck, PrDetail, PrFailure, PullRequest } from './types';

// The control of a pull request in its view (ESC-488, PR-8 of the PR in Escouade): its merge box « prête à merger ? »,
// each line of what is missing with the gesture that unblocks it, the methods the repository allows, and what is said
// of a refusal. Pure: the view sends each gesture on a click, through `pr_write` and `pr_merge`.

export type MergeStrategy = 'merge' | 'squash' | 'rebase' | 'semiLinear';

/** Where a pull request stands for its merge, as its host sums it up (`forge::PrControl`). */
export interface PrControl {
  mergeState: 'clean' | 'behind' | 'blocked' | 'dirty' | 'draft' | 'unstable' | 'unknown';
  behind?: boolean;
  reviewDecision?: 'approved' | 'changesRequested' | 'reviewRequired';
  labels: string[];
  autoMerge?: MergeStrategy;
  deletesBranch: boolean;
  id?: string;
}

/** What the merge box reads (`prwrite::PrControlRead`). */
export interface PrControlRead {
  control?: PrControl;
  options: { strategies: MergeStrategy[]; autoMerge: boolean };
  preferred?: MergeStrategy;
  writing: PrWriting;
}

export interface LabelInfo {
  name: string;
  color?: string;
  description?: string;
}

/** Who may review and the labels of the repository (`forge::Candidates`). */
export interface Candidates {
  reviewers: string[];
  labels: LabelInfo[];
}

/** The merge the user confirmed (`prwrite::MergeAsk`). */
export interface MergeAsk {
  strategy: MergeStrategy;
  deleteBranch: boolean;
  deleteWorktree: boolean;
}

/** What a merge did (`prwrite::Merged`). */
export interface Merged {
  merged: boolean;
  ticket?: string;
  branch?: string;
  branchKept?: string;
  worktree?: string;
  worktreeKept?: string;
}

/** What unblocks a line of the box. */
export type RowAction =
  | { kind: 'ready' }
  | { kind: 'rerunOne'; job: string }
  | { kind: 'rerunAll' }
  | { kind: 'checks' }
  | { kind: 'askReview' }
  | { kind: 'threads' }
  | { kind: 'update' }
  | { kind: 'ticketAgent' }
  | { kind: 'openAgent' }
  | { kind: 'host' };

export interface Row {
  key: 'draft' | 'checks' | 'reviews' | 'threads' | 'branch' | 'rules';
  /** ko: it blocks; wait: it blocks until something happens; ok: done; info: not required. */
  tone: 'ok' | 'ko' | 'wait' | 'info';
  title: string;
  sub?: string;
  action?: RowAction;
  /** What the action's button says. */
  label?: string;
  /** The host does not do the gesture from Escouade: said in place of the button. */
  elsewhere?: string;
}

/** What the box needs to say what is missing. */
export interface BoxInput {
  pr: PullRequest;
  detail: PrDetail | null;
  control: PrControlRead | null;
  /** The open threads, as read whole; else the pull request's count. */
  openThreads: number;
  /** Its reviewers' last say, by name (`prpage::reviewers`). */
  people: { name: string; state: 'approved' | 'changesRequested' | 'commented' | 'requested' }[];
  host: string;
  /** The ticket whose pull request it is, « En revue » with its agent: a conflict goes to that agent (ESC-61). */
  ticketAgent: boolean;
  /** The agent that holds its branch, for a conflict it updates in its conversation. */
  agent: string | null;
}

const failedCheck = (c: PrCheck) => c.state === 'failure';
const running = (c: PrCheck) => c.state === 'pending';

/** Each line of the box, the blocking ones first: what is missing, and the gesture that unblocks it. */
export function boxRows(i: BoxInput): Row[] {
  const { pr, detail, control, host } = i;
  const w = control?.writing;
  const c = control?.control;
  const rows: Row[] = [];
  const notHere = t('prs.page.control.box.notHere', { host });
  const base = pr.base;

  if (pr.draft || c?.mergeState === 'draft')
    rows.push({
      key: 'draft',
      tone: 'ko',
      title: t('prs.page.control.box.draft'),
      sub: t('prs.page.control.box.draftSub'),
      ...(w?.draft ? { action: { kind: 'ready' }, label: t('prs.page.control.box.markReady') } : { elsewhere: notHere }),
    });

  // The checks, as read; else the pull request's word for them.
  const checks = detail?.checks ?? [];
  const failed = checks.filter(failedCheck);
  const pending = checks.filter(running);
  const done = checks.filter((x) => x.state === 'success' || x.state === 'skipped').length;
  if (failed.length || (!checks.length && pr.checks === 'failure')) {
    const one = failed.length === 1 && failed[0].job && w?.rerunCheck ? failed[0] : null;
    const action: Pick<Row, 'action' | 'label' | 'elsewhere'> = one
      ? { action: { kind: 'rerunOne', job: one.job! }, label: t('prs.page.control.box.rerunOne') }
      : detail?.can.rerun
        ? { action: { kind: 'rerunAll' }, label: t('prs.page.control.box.rerunAll') }
        : { action: { kind: 'checks' }, label: t('prs.page.control.box.seeChecks') };
    rows.push({
      key: 'checks',
      tone: 'ko',
      title: t('prs.page.control.box.checksFailed', { count: failed.length || 1 }),
      sub: failed.length ? fList(failed.slice(0, 2).map((x) => x.name)) : undefined,
      ...action,
    });
  } else if (pending.length || (!checks.length && pr.checks === 'pending')) {
    rows.push({
      key: 'checks',
      tone: 'wait',
      title: t('prs.page.control.box.checksRunning'),
      sub: checks.length ? t('prs.page.control.box.ofAll', { done, all: checks.length }) : undefined,
      action: { kind: 'checks' },
      label: t('prs.page.control.box.seeChecks'),
    });
  } else if (checks.length || pr.checks === 'success') {
    rows.push({
      key: 'checks',
      tone: 'ok',
      title: t('prs.page.control.box.checksPassed'),
      sub: checks.length ? t('prs.page.control.box.ofAll', { done, all: checks.length }) : undefined,
    });
  } else {
    rows.push({ key: 'checks', tone: 'info', title: t('prs.page.control.box.noChecks') });
  }

  // The reviews: changes asked, an approval the rules require, approved, or none required.
  const named = (s: string) => i.people.filter((p) => p.state === s).map((p) => p.name);
  const decision = c?.reviewDecision;
  if (pr.changesRequested || decision === 'changesRequested') {
    const who = named('changesRequested');
    rows.push({
      key: 'reviews',
      tone: 'ko',
      title: t('prs.page.control.box.changes'),
      sub: who.length ? t('prs.page.control.box.by', { names: fList(who) }) : undefined,
      ...(i.openThreads ? { action: { kind: 'threads' }, label: t('prs.page.control.box.next') } : {}),
    });
  } else if (decision === 'reviewRequired' && !pr.approvals) {
    const asked = named('requested');
    rows.push({
      key: 'reviews',
      tone: 'wait',
      title: t('prs.page.control.box.reviewRequired'),
      sub: asked.length
        ? t('prs.page.control.box.reviewRequiredFrom', { names: fList(asked) })
        : t('prs.page.control.box.reviewRequiredSub'),
      ...(asked.length
        ? {}
        : w?.reviewers
          ? { action: { kind: 'askReview' }, label: t('prs.page.control.box.askReview') }
          : { elsewhere: notHere }),
    });
  } else if (pr.approvals > 0 || decision === 'approved') {
    const who = named('approved');
    rows.push({
      key: 'reviews',
      tone: 'ok',
      title: t('prs.page.control.box.approved'),
      sub: who.length ? t('prs.page.control.box.by', { names: fList(who) }) : undefined,
    });
  } else {
    rows.push({
      key: 'reviews',
      tone: 'info',
      title: t('prs.page.control.box.noApproval'),
      sub: t('prs.page.control.box.noApprovalSub'),
    });
  }

  if (i.openThreads > 0)
    rows.push({
      key: 'threads',
      tone: 'wait',
      title: t('prs.page.control.box.threadsOpen', { count: i.openThreads }),
      sub: t('prs.page.control.box.threadsSub'),
      action: { kind: 'threads' },
      label: t('prs.page.control.box.next'),
    });

  // The branch: a conflict, behind its base, or clean.
  const canUpdate = !!(w?.updateMerge || w?.updateRebase);
  if (pr.mergeable === false || c?.mergeState === 'dirty') {
    rows.push({
      key: 'branch',
      tone: 'ko',
      title: t('prs.page.control.box.conflict', { base }),
      sub: t('prs.page.control.box.conflictSub'),
      ...(i.ticketAgent
        ? { action: { kind: 'ticketAgent' }, label: t('prs.page.control.box.toTicketAgent') }
        : i.agent
          ? { action: { kind: 'openAgent' }, label: t('prs.page.control.box.toAgent', { agent: i.agent }) }
          : { action: { kind: 'host' }, label: t('prs.page.control.box.seeOnHost', { host }) }),
    });
  } else if (c?.behind || c?.mergeState === 'behind') {
    rows.push({
      key: 'branch',
      tone: c?.mergeState === 'behind' ? 'ko' : 'info',
      title: t('prs.page.control.box.behind', { base }),
      sub: t('prs.page.control.box.behindSub', { base }),
      ...(canUpdate ? { action: { kind: 'update' }, label: t('prs.page.control.box.update') } : { elsewhere: notHere }),
    });
  } else if (pr.mergeable === true) {
    rows.push({
      key: 'branch',
      tone: 'ok',
      title: t('prs.page.control.box.noConflict'),
      sub: c?.behind === false ? t('prs.page.control.box.upToDateSub', { base }) : t('prs.page.control.box.noConflictSub', { base }),
    });
  } else {
    rows.push({ key: 'branch', tone: 'info', title: t('prs.page.control.box.mergeUnknown', { base }) });
  }

  // Held by the base's rules, and nothing above says why: they are on the host.
  const said = rows.some((r) => r.tone === 'ko' || r.tone === 'wait');
  if (c?.mergeState === 'blocked' && !said)
    rows.push({
      key: 'rules',
      tone: 'ko',
      title: t('prs.page.control.box.protected'),
      sub: t('prs.page.control.box.protectedSub', { host, base }),
      action: { kind: 'host' },
      label: t('prs.page.control.box.seeOnHost', { host }),
    });

  const weight = { ko: 0, wait: 1, info: 2, ok: 3 } as const;
  return rows
    .map((r, n) => ({ r, n }))
    .sort((a, b) => weight[a.r.tone] - weight[b.r.tone] || a.n - b.n)
    .map((x) => x.r);
}

/** Nothing blocks: the merge may be asked. */
export const mergeable = (rows: Row[]) => !rows.some((r) => r.tone === 'ko' || r.tone === 'wait');

/** What only waits (checks running, a review asked): the host's auto-merge fits. */
export const onlyWaiting = (rows: Row[]) =>
  rows.some((r) => r.tone === 'wait') && !rows.some((r) => r.tone === 'ko' && r.key !== 'checks' && r.key !== 'reviews');

/** The methods the box offers, the preferred (the Kanban's, else the first allowed) first. */
export function methodsOf(read: PrControlRead | null): MergeStrategy[] {
  const all = read?.options.strategies ?? [];
  const first = read?.preferred && all.includes(read.preferred) ? read.preferred : all[0];
  return first ? [first, ...all.filter((s) => s !== first)] : [];
}

/** A label's colour from its host, when it is one: six hexadecimal digits, nothing else is put in a style. */
export function labelColor(color: string | undefined): string | null {
  return color && /^[0-9a-f]{6}$/i.test(color) ? `#${color.toLowerCase()}` : null;
}

/** The candidates whose name holds each word of `query`, without case. */
export function filterNames<T>(items: T[], name: (x: T) => string, query: string): T[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return items.filter((x) => words.every((w) => name(x).toLowerCase().includes(w)));
}

/** A refusal of a control or of the merge, in the box's words. */
export function controlFailure(f: PrFailure, host: string, base: string, word: string, online = true): string {
  const k = 'prs.page.control.failure';
  switch (f.kind) {
    case 'refused':
      return t(`${k}.refused`, { host });
    case 'forbidden':
      return t(`${k}.forbidden`, { host });
    case 'gone':
      return t(`${k}.gone`, { host, word });
    case 'conflict':
      return t(`${k}.conflict`, { base });
    case 'protected':
      return t(`${k}.protected`, { base, message: f.message });
    case 'checkRequired':
      return t(`${k}.checkRequired`, { message: f.message });
    case 'rateLimit':
      return f.reset ? t(`${k}.rateLimitUntil`, { host, time: fTime(f.reset * 1000) }) : t(`${k}.rateLimit`, { host });
    case 'unreachable':
      return online ? t(`${k}.unreachable`, { host }) : t(`${k}.offline`);
    default:
      return f.message;
  }
}

/** What a merge did, in sentences. */
export function mergedText(m: Merged, i: { base: string; host: string; method: MergeStrategy }): string[] {
  const k = 'prs.page.control.done';
  const out = [
    m.merged ? t(`${k}.merged`, { base: i.base, method: t(`prs.page.control.method.${i.method}`) }) : t(`${k}.queued`, { host: i.host }),
  ];
  if (m.branch) out.push(t(`${k}.branch`, { branch: m.branch }));
  if (m.branchKept) out.push(t(`${k}.branchKept`, { why: m.branchKept }));
  if (m.worktree) out.push(t(`${k}.worktree`, { agent: m.worktree }));
  if (m.worktreeKept) out.push(t(`${k}.worktreeKept`, { why: m.worktreeKept }));
  if (m.ticket) out.push(t(`${k}.ticket`, { key: m.ticket }));
  return out;
}

/**
 * The order of the reads of one thing (ESC-488): each read takes a number when it sets out; its answer is shown only when
 * no newer read's was, and `fence` (after a write) drops every read that set out before it. An answer older than what
 * is shown never brings it back.
 */
export function ordered() {
  let asked = 0;
  let shown = 0;
  return {
    /** A read sets out: its number. */
    start: () => ++asked,
    /** Its answer arrived: true when it may be shown (and it is, from now on, the newest). */
    take(n: number): boolean {
      if (n <= shown) return false;
      shown = n;
      return true;
    },
    /** Something was written: the reads on their way are older than it. */
    fence() {
      shown = asked;
    },
  };
}

/**
 * What a write the host accepted changes in what the view shows, at once (ESC-488): the reread that follows confirms it,
 * and no answer of before brings it back (`ordered`). The pull request, its labels, and its control.
 */
export function written(
  w: import('./prthreads').PrWrite,
  shown: { pr: PullRequest; labels: string[]; read: PrControlRead | null },
): { pr: PullRequest; labels: string[]; read: PrControlRead | null } {
  const pr = { ...shown.pr };
  let labels = shown.labels;
  const c = shown.read?.control ? { ...shown.read.control } : undefined;
  switch (w.kind) {
    case 'setDraft':
      pr.draft = w.draft;
      if (c) c.mergeState = w.draft ? 'draft' : c.mergeState === 'draft' ? 'unknown' : c.mergeState;
      break;
    case 'requestReviewers':
      pr.reviewers = [...new Set([...(pr.reviewers ?? []), ...w.logins])];
      break;
    case 'removeReviewer':
      pr.reviewers = (pr.reviewers ?? []).filter((r) => r !== w.login);
      break;
    case 'addLabels':
      labels = [...new Set([...labels, ...w.labels])];
      break;
    case 'removeLabel':
      labels = labels.filter((l) => l !== w.label);
      break;
    case 'updateBranch':
      if (c) {
        c.behind = false;
        if (c.mergeState === 'behind') c.mergeState = 'unknown';
      }
      break;
    case 'autoMerge':
      pr.autoMerge = true;
      if (c) c.autoMerge = w.strategy;
      break;
    case 'cancelAutoMerge':
      pr.autoMerge = false;
      if (c) c.autoMerge = undefined;
      break;
    default:
      return shown;
  }
  if (c) c.labels = labels;
  return { pr, labels, read: shown.read && c ? { ...shown.read, control: c } : shown.read };
}
