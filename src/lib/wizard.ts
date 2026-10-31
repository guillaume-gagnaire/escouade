// The new project's assistant (« Nouveau projet » in steps): the settings it starts from, those it copies from another
// project and checks in the new folder, and what it tells would keep the way of working from working.

import { dedicated, serves } from './accounts';
import { engineName } from './engines';
import { t } from './i18n';
import type { Account, BoardSettings, CommitMode, EngineId, Project, RunCommand, SetupInfo, WorktreeBase, WorktreeStep } from './types';
import { DEFAULT_AGENT_PREFIX, DEFAULT_TICKET_PREFIX } from './worktrees';

export type { SetupInfo };

/**
 * Where the assistant starts from: the folder's .escouade.yml (ESC-146), another project's settings, the recommended ones,
 * Escouade's defaults, or (an existing project's) its own.
 */
export type Start = 'file' | 'copy' | 'recommended' | 'blank' | 'current';

/** The steps, in their order. */
export type StepId = 'start' | 'copied' | 'worktrees' | 'launch' | 'kanban' | 'account' | 'integrations' | 'summary';

/** The steps of the assistant: « Réglages copiés » only when copying. */
export function stepsFor(start: Start): StepId[] {
  const all: StepId[] = ['start', 'copied', 'worktrees', 'launch', 'kanban', 'account', 'integrations', 'summary'];
  return start === 'copy' ? all : all.filter((s) => s !== 'copied');
}

/**
 * The settings the assistant sets: a project's, but its name, color, folder, integrations and tickets' key (fixed by its
 * first ticket); and the accounts to reserve to it, which are the accounts' own.
 */
export interface WizardConfig {
  worktreePerAgent: boolean;
  /** « Créer le worktree au premier message, au nom de l’agent » (ESC-156). */
  worktreeAtFirstMessage: boolean;
  /** « Branche de départ des nouveaux worktrees » (ESC-160). */
  worktreeBase: WorktreeBase;
  worktreeDir: string;
  agentBranchPrefix: string;
  ticketBranchPrefix: string;
  /** The patterns of the files copied into new worktrees, one per line. */
  copy: string;
  worktreeSetup: WorktreeStep[];
  worktreeTeardown: WorktreeStep[];
  runCommands: RunCommand[];
  /** Its Kanban, but the key of its tickets (`prefix`, `nextNumber`), the project's own. */
  board: Omit<BoardSettings, 'prefix' | 'nextNumber'>;
  commitMode: CommitMode;
  account: string;
  remoteControl: boolean;
  /** Its default harness, copied with its agents' settings. */
  engine: EngineId | null;
  /** « Projet privé » (ESC-140). */
  private: boolean;
  /** « Comptes partagés à la limite » (a private project, ESC-141). */
  sharedOnLimit: boolean;
  /**
   * The accounts reserved to it alone once saved (ESC-163; a private project's, « Réserver <compte> à ce projet »): those
   * it has already, and those to reserve. Never copied from another project.
   */
  reserved: string[];
  /** Off: « Pas de workflow pour ce projet » (ESC-309). */
  workflowsEnabled: boolean;
}

/** What a new project starts with besides the assistant's settings: « Nouveaux projets privés » (Réglages › Projets). */
export interface NewProjectDefaults {
  private?: boolean;
}

/** A Kanban as a new project has it (`BoardSettings::default`). */
const DEFAULT_BOARD: WizardConfig['board'] = {
  action: 'merge',
  target: '',
  strategy: 'squash',
  draft: false,
  testsFirst: false,
  testCommand: '',
  cleanup: true,
  conventional: true,
  conflict: 'ask',
  maxParallel: 2,
  model: '',
  effort: '',
  mode: '',
  autopilot: true,
};

/** « Partir de zéro »: Escouade's defaults, those of a project made without the assistant. */
export function blankConfig(defaults: NewProjectDefaults = {}): WizardConfig {
  return {
    worktreePerAgent: false,
    worktreeAtFirstMessage: true,
    worktreeBase: { kind: 'head' },
    worktreeDir: '',
    agentBranchPrefix: DEFAULT_AGENT_PREFIX,
    ticketBranchPrefix: DEFAULT_TICKET_PREFIX,
    copy: '.env*',
    worktreeSetup: [],
    worktreeTeardown: [],
    runCommands: [],
    board: { ...DEFAULT_BOARD },
    commitMode: 'agent',
    account: '',
    remoteControl: false,
    engine: null,
    private: !!defaults.private,
    sharedOnLimit: false,
    reserved: [],
    workflowsEnabled: true,
  };
}

/**
 * « Configuration recommandée »: a worktree per agent, a ticket validated by a squash merge into the current branch
 * (an empty target), its worktree removed once validated, conventional commits, conflicts resolved by the agent,
 * commits written by the agent; the rest as by default.
 */
export function recommendedConfig(defaults: NewProjectDefaults = {}): WizardConfig {
  const c = blankConfig(defaults);
  return {
    ...c,
    worktreePerAgent: true,
    commitMode: 'agent',
    board: { ...c.board, action: 'merge', target: '', strategy: 'squash', cleanup: true, conventional: true, conflict: 'agent' },
  };
}

/**
 * The settings of an existing project, as the assistant edits them (« Garder la configuration actuelle »), with the
 * accounts reserved to it alone among `accounts`.
 */
export function configOf(p: Project, accounts: Account[] = []): WizardConfig {
  const { prefix: _prefix, nextNumber: _next, ...board } = p.board;
  return structuredClone({
    worktreePerAgent: p.worktreePerAgent,
    // Absent from a project of the window's tests made before ESC-156: on, as the backend reads it.
    worktreeAtFirstMessage: p.worktreeAtFirstMessage ?? true,
    // Absent from a project of the window's tests made before ESC-160: the folder's branch.
    worktreeBase: p.worktreeBase ?? { kind: 'head' },
    worktreeDir: p.worktreeDir,
    agentBranchPrefix: p.agentBranchPrefix,
    ticketBranchPrefix: p.ticketBranchPrefix,
    copy: p.worktreeCopy.join('\n'),
    worktreeSetup: p.worktreeSetup,
    worktreeTeardown: p.worktreeTeardown,
    runCommands: p.runCommands,
    board,
    commitMode: p.commitMode,
    account: p.account,
    remoteControl: p.remoteControl,
    engine: p.engine ?? null,
    private: !!p.private,
    sharedOnLimit: !!p.sharedOnLimit,
    reserved: accounts.filter((a) => dedicated(a, p.id)).map((a) => a.id),
    // Absent from a project of the window's tests made before ESC-309: on.
    workflowsEnabled: p.workflowsEnabled ?? true,
  });
}

/** The project with the settings of the assistant, as saved: trimmed, steps that run nothing left out; its own key of tickets kept. */
export function withConfig(p: Project, c: WizardConfig): Project {
  const steps = (list: WorktreeStep[]) =>
    list.map((s) => ({ ...s, command: s.command.trim(), cwd: s.cwd.trim() })).filter((s) => s.command);
  const testCommand = c.board.testCommand.trim();
  return {
    ...p,
    worktreePerAgent: c.worktreePerAgent,
    worktreeAtFirstMessage: c.worktreeAtFirstMessage,
    worktreeBase: c.worktreeBase.kind === 'branch' && !c.worktreeBase.branch.trim() ? { kind: 'head' } : c.worktreeBase,
    worktreeDir: c.worktreeDir.trim(),
    agentBranchPrefix: c.agentBranchPrefix.trim(),
    ticketBranchPrefix: c.ticketBranchPrefix.trim(),
    worktreeCopy: c.copy
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean),
    worktreeSetup: steps(c.worktreeSetup),
    worktreeTeardown: steps(c.worktreeTeardown),
    runCommands: c.runCommands
      .map((r) => ({ ...r, name: r.name.trim(), command: r.command.trim(), cwd: r.cwd.trim() }))
      .filter((r) => r.command),
    commitMode: c.commitMode,
    account: c.account,
    remoteControl: c.remoteControl,
    engine: c.engine,
    private: c.private,
    // Only a private project's: off for one that is not.
    sharedOnLimit: c.private && c.sharedOnLimit,
    workflowsEnabled: c.workflowsEnabled,
    board: { ...p.board, ...c.board, testCommand, testsFirst: c.board.testsFirst && !!testCommand },
  };
}

// ---------- Copy ----------

/** How the copied settings are grouped. */
export type CopyGroup = 'worktrees' | 'launch' | 'kanban' | 'account';
export const COPY_GROUPS: CopyGroup[] = ['worktrees', 'launch', 'kanban', 'account'];

/** What checking a copied setting in the new folder found: fine there, to look at, or replaced. `plain`: nothing to check. */
export type CopyStatus = 'ok' | 'warn' | 'bad' | 'plain';

/** A setting of the other project, as the step « Réglages copiés » lists it. */
export interface CopyItem {
  /** `worktreePerAgent`, `run:<id>`, `setup:<id>`… */
  id: string;
  group: CopyGroup;
  label: string;
  /** What it is set to there, as shown. */
  value: string;
  status: CopyStatus;
  /** What the check found, when there is something to say. */
  note: string;
  /** Taken by default: all but what cannot work here. */
  checked: boolean;
}

/** A folder relative to the project (or its worktrees) that names one: not empty, not `.`. */
const realDir = (d: string) => {
  const s = d.trim().replace(/\\/g, '/').replace(/\/+$/, '');
  return s && s !== '.' ? s : '';
};

/** The folders of the other project's commands (launch, setup, teardown) to look for in the new one. */
export function copyDirs(src: Project): string[] {
  const all = [...src.runCommands, ...src.worktreeSetup, ...src.worktreeTeardown].map((c) => realDir(c.cwd)).filter(Boolean);
  return [...new Set(all)];
}

/** A « Dossier des worktrees » written whole (or from the home folder): the other project's own place, not this one's. */
export function absoluteDir(setting: string): boolean {
  const s = setting.trim().replace(/^"|"$/g, '');
  return /^([A-Za-z]:[\\/]|[\\/]|~([\\/]|$))/.test(s);
}

/** The other project's target branch is one of the new folder's (or none: the current branch). */
function targetFound(target: string, info: SetupInfo | null): boolean {
  return !target.trim() || !!info?.branches.includes(target.trim());
}

/** The account a project prefers, if it may serve a new project: active, and reserved to no other project. */
function usableAccount(id: string, accounts: Account[]): Account | undefined {
  return accounts.find((a) => a.id === id && a.active && serves(a, undefined));
}

/**
 * The settings of `src` the assistant may take, by group, each checked in the new folder (`info`; null: not read, as
 * for a folder that is no repository): the target branch is one of its branches, the folders of the commands are there,
 * the worktrees folder is relative, the account preferred is still usable. What cannot work here is not checked by
 * default; a target branch not found is replaced by the current branch. Neither its integrations nor its tickets are
 * among them.
 */
export function copyItems(
  src: Project,
  info: SetupInfo | null,
  accounts: Account[],
  accountName: (a: Account) => string = (a) => a.name,
): CopyItem[] {
  const items: CopyItem[] = [];
  const add = (i: Omit<CopyItem, 'checked' | 'status' | 'note'> & Partial<CopyItem>) =>
    items.push({ status: 'plain', note: '', checked: true, ...i });
  const missing = (d: string) => !!realDir(d) && (!info || info.missingDirs.includes(realDir(d)));
  const dirNote = (d: string) => t('wizard.copy.dirMissing', { dir: realDir(d) });
  const yes = (on: boolean) => (on ? t('wizard.yes') : t('wizard.no'));

  add({ id: 'worktreePerAgent', group: 'worktrees', label: t('settings.project.worktreePerAgent'), value: yes(src.worktreePerAgent) });
  const absolute = absoluteDir(src.worktreeDir);
  add({
    id: 'worktreeDir',
    group: 'worktrees',
    label: t('settings.project.worktreeDir'),
    value: src.worktreeDir.trim() || '.claude/worktrees',
    status: absolute ? 'warn' : 'ok',
    note: absolute ? t('wizard.copy.dirAbsolute') : t('wizard.copy.dirRelative'),
    checked: !absolute,
  });
  add({
    id: 'prefixes',
    group: 'worktrees',
    label: t('wizard.copy.prefixes'),
    value: `${src.agentBranchPrefix} · ${src.ticketBranchPrefix}`,
  });
  add({ id: 'copy', group: 'worktrees', label: t('settings.project.copy'), value: src.worktreeCopy.join(' ') || '—' });
  for (const [kind, list] of [
    ['setup', src.worktreeSetup],
    ['teardown', src.worktreeTeardown],
  ] as const) {
    for (const s of list) {
      const gone = missing(s.cwd);
      add({
        id: `${kind}:${s.id}`,
        group: 'worktrees',
        label: t(kind === 'setup' ? 'settings.project.setupTitle' : 'settings.project.teardownTitle'),
        value: s.command,
        status: realDir(s.cwd) ? (gone ? 'warn' : 'ok') : 'plain',
        note: realDir(s.cwd) ? (gone ? dirNote(s.cwd) : t('wizard.copy.dirFound', { dir: realDir(s.cwd) })) : '',
        checked: !gone,
      });
    }
  }
  for (const c of src.runCommands) {
    const gone = missing(c.cwd);
    add({
      id: `run:${c.id}`,
      group: 'launch',
      label: c.name || t('wizard.copy.unnamed'),
      value: c.command,
      status: realDir(c.cwd) ? (gone ? 'warn' : 'ok') : 'plain',
      note: realDir(c.cwd) ? (gone ? dirNote(c.cwd) : t('wizard.copy.dirFound', { dir: realDir(c.cwd) })) : '',
      checked: !gone,
    });
  }

  const b = src.board;
  const found = targetFound(b.target, info);
  const current = info?.branches[0] ?? '';
  add({
    id: 'validation',
    group: 'kanban',
    label: t('wizard.copy.validation'),
    value: validationText(b),
    status: !b.target.trim() ? 'plain' : found ? 'ok' : 'bad',
    note: !b.target.trim()
      ? ''
      : found
        ? t('wizard.copy.branchFound')
        : current
          ? t('wizard.copy.branchReplaced', { branch: current })
          : t('wizard.copy.branchReplacedNone'),
  });
  add({
    id: 'tests',
    group: 'kanban',
    label: t('boardSettings.tab.testsFirst'),
    value: b.testsFirst && b.testCommand.trim() ? b.testCommand : t('wizard.no'),
  });
  add({ id: 'cleanup', group: 'kanban', label: t('boardSettings.tab.cleanup'), value: yes(b.cleanup) });
  add({ id: 'conventional', group: 'kanban', label: t('boardSettings.tab.commitMessage'), value: yes(b.conventional) });
  add({ id: 'conflict', group: 'kanban', label: t('boardSettings.tab.onConflict'), value: t(`boardSettings.tab.conflicts.${b.conflict}`) });
  add({
    id: 'parallel',
    group: 'kanban',
    label: t('wizard.copy.parallel'),
    value: t('wizard.copy.parallelValue', { count: b.maxParallel, autopilot: yes(b.autopilot) }),
  });
  add({
    id: 'agents',
    group: 'kanban',
    label: t('wizard.copy.agents'),
    // Its harness goes with them: the models are its own.
    value:
      [src.engine ? engineName(src.engine) : '', b.model, b.effort, b.mode].filter(Boolean).join(' · ') || t('boardSettings.tab.default'),
  });

  add({
    id: 'commitMode',
    group: 'account',
    label: t('settings.project.commit'),
    value: t(`settings.project.commitModes.${src.commitMode}`),
  });
  if (src.account) {
    const a = usableAccount(src.account, accounts);
    const named = accounts.find((x) => x.id === src.account);
    add({
      id: 'account',
      group: 'account',
      label: t('accounts.project.label'),
      value: named ? accountName(named) : src.account,
      status: a ? 'ok' : 'warn',
      note: a ? t('wizard.copy.accountActive') : t('wizard.copy.accountGone'),
      checked: !!a,
    });
  } else {
    add({ id: 'account', group: 'account', label: t('accounts.project.label'), value: t('accounts.project.auto') });
  }
  add({ id: 'remoteControl', group: 'account', label: t('settings.project.remote'), value: yes(src.remoteControl) });
  return items;
}

/** « merge squash → main », « PR → main »… as the copy lists it. */
export function validationText(b: Pick<BoardSettings, 'action' | 'strategy' | 'target' | 'draft'>): string {
  const target = b.target.trim() || t('wizard.currentBranch');
  if (b.action === 'merge') return `merge ${b.strategy} → ${target}`;
  if (b.action === 'pr') return `${b.draft ? t('wizard.draftPr') : 'PR'} → ${target}`;
  return t(`wizard.actions.${b.action}`);
}

/** A copy of a command or a step, with an id of its own: the launches of the other project are kept by theirs. */
const fresh = <T extends { id: string }>(x: T): T => ({ ...structuredClone(x), id: crypto.randomUUID() });

/** `base` with the settings of `src` that are `checked` (by id), checked in the new folder as `copyItems` says. */
export function applyCopy(base: WizardConfig, src: Project, checked: Set<string>, info: SetupInfo | null): WizardConfig {
  const c = structuredClone(base);
  const on = (id: string) => checked.has(id);
  // When its worktree is made goes with it (ESC-156), and where it starts (ESC-160): a branch the new folder does not
  // have makes its agents start from the folder's, as they say.
  if (on('worktreePerAgent')) {
    c.worktreePerAgent = src.worktreePerAgent;
    c.worktreeAtFirstMessage = src.worktreeAtFirstMessage ?? true;
    c.worktreeBase = structuredClone(src.worktreeBase ?? { kind: 'head' });
  }
  if (on('worktreeDir')) c.worktreeDir = src.worktreeDir;
  if (on('prefixes')) {
    c.agentBranchPrefix = src.agentBranchPrefix;
    c.ticketBranchPrefix = src.ticketBranchPrefix;
  }
  if (on('copy')) c.copy = src.worktreeCopy.join('\n');
  const setup = src.worktreeSetup.filter((s) => on(`setup:${s.id}`));
  const teardown = src.worktreeTeardown.filter((s) => on(`teardown:${s.id}`));
  const run = src.runCommands.filter((r) => on(`run:${r.id}`));
  if (setup.length) c.worktreeSetup = setup.map(fresh);
  if (teardown.length) c.worktreeTeardown = teardown.map(fresh);
  if (run.length) c.runCommands = run.map(fresh);
  const b = src.board;
  if (on('validation')) {
    c.board.action = b.action;
    c.board.strategy = b.strategy;
    c.board.draft = b.draft;
    // A branch the new folder does not have: its current one.
    c.board.target = targetFound(b.target, info) ? b.target : '';
  }
  if (on('tests')) {
    c.board.testsFirst = b.testsFirst;
    c.board.testCommand = b.testCommand;
  }
  if (on('cleanup')) c.board.cleanup = b.cleanup;
  if (on('conventional')) c.board.conventional = b.conventional;
  if (on('conflict')) c.board.conflict = b.conflict;
  if (on('parallel')) {
    c.board.maxParallel = b.maxParallel;
    c.board.autopilot = b.autopilot;
  }
  if (on('agents')) {
    c.board.model = b.model;
    c.board.effort = b.effort;
    c.board.mode = b.mode;
    c.engine = src.engine ?? null;
  }
  if (on('commitMode')) c.commitMode = src.commitMode;
  if (on('account')) c.account = src.account;
  if (on('remoteControl')) c.remoteControl = src.remoteControl;
  return c;
}

// ---------- Checks ----------

/** What would keep the way of working from working, each with its correction. */
export type ControlId = 'noCommit' | 'noTarget' | 'tracked' | 'noAccount' | 'noClaude';

/** The step each check is told in, besides the summary. */
export const CONTROL_STEP: Record<ControlId, StepId> = {
  noCommit: 'start',
  noTarget: 'kanban',
  tracked: 'worktrees',
  noAccount: 'account',
  noClaude: 'account',
};

/**
 * The checks that fail for the folder (`info`; null: no repository read) and the settings: no commit (no ticket
 * starts), a target branch the folder does not have, worktrees in a folder git tracks files in, no Claude account that
 * may serve the project (`projectId`: an existing one's reservations count), Claude Code not found.
 */
export function controls(
  info: SetupInfo | null,
  c: WizardConfig,
  ctx: { accounts: Account[]; claudeFound: boolean; projectId?: string },
): ControlId[] {
  const out: ControlId[] = [];
  if (!info?.hasCommit) out.push('noCommit');
  if (info?.hasCommit && (c.board.action === 'merge' || c.board.action === 'pr') && !targetFound(c.board.target, info))
    out.push('noTarget');
  if (info?.worktreesTracked) out.push('tracked');
  if (!ctx.accounts.some((a) => a.active && serves(a, ctx.projectId))) out.push('noAccount');
  if (!ctx.claudeFound) out.push('noClaude');
  return out;
}
