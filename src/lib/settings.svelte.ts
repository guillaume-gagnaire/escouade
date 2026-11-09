// The settings window (ESC-380): what its fields show of the app's settings, each project's own and its board's, saved
// as soon as they change — a choice at once, a text when it loses the focus or after a pause in the typing. What cannot
// be saved as typed (a project without a name, a launch command without one, a number below 0) is said on its row and
// left as it was saved.

import { t } from './i18n';
import { api } from './ipc';
import { forgetLaunches, runsOfCommand } from './launch-actions';
import { knowLogo, logoUrl, pickLogo } from './project-logo.svelte';
import { app } from './state.svelte';
import type {
  BoardSettings,
  CommitMode,
  PagesOpen,
  EngineId,
  Project,
  ProjectIntegrations,
  RunCommand,
  Settings,
  WorkflowFix,
  WorktreeBase,
  WorktreeStep,
  WorktreeSuggestion,
} from './types';
import { branchPrefixProblem, worktreesRootProblem } from './worktrees';
import { rowsOf, SETTINGS_PAGES, type RowInfo, type SettingsPage } from './settings-pages';

export * from './settings-pages';

/** The app's settings each page sets, read from its rows. */
export const SETTINGS_OF = {} as Record<SettingsPage, (keyof Settings)[]>;
for (const p of SETTINGS_PAGES) SETTINGS_OF[p.id] = [...new Set(rowsOf(p.id).flatMap((r) => r.settings))];

/** Where the window opens or comes back to: a page, and the project of a project's page. */
export interface SettingsPlace {
  page?: SettingsPage;
  projectId?: string;
}

/** What the modal sets of a project, as the backend's `update_project` takes it, its board apart. */
type ProjectFields = Pick<
  Project,
  | 'name'
  | 'color'
  | 'tileLetters'
  | 'logo'
  | 'tileLabel'
  | 'tileLabelText'
  | 'worktreePerAgent'
  | 'worktreeAtFirstMessage'
  | 'worktreeBase'
  | 'runCommands'
  | 'worktreeCopy'
  | 'worktreeSetup'
  | 'worktreeTeardown'
  | 'commitMode'
  | 'pagesOpen'
  | 'account'
  | 'worktreeDir'
  | 'agentBranchPrefix'
  | 'ticketBranchPrefix'
  | 'remoteControl'
  | 'engine'
  | 'private'
  | 'sharedOnLimit'
  | 'workflowFixes'
  | 'workflowsAutoRead'
  | 'workflowsInRepo'
  | 'workflowsEnabled'
  | 'userName'
  | 'shareNew'
  | 'docsBranch'
>;

/** Which commands of a project's worktrees: run once one is made, or before one is removed. */
export type StepKind = 'setup' | 'teardown';

export interface ProjectDraft {
  /** « Branche des docs » (ESC-258): empty for the current branch. */
  docsBranch: string;
  name: string;
  color: string;
  /** « Lettres de la tuile » (ESC-167): empty is the initials of the name. */
  tileLetters: string;
  /** « Logo »: the project's file, or the path of the image chosen (copied once saved), or empty for none. */
  logo: string;
  /** « Libellé sous la tuile » (ESC-168): off by default. */
  tileLabel: boolean;
  /** Its text: empty is the name. */
  tileLabelText: string;
  worktreePerAgent: boolean;
  /** « Créer le worktree au premier message, au nom de l’agent ». */
  worktreeAtFirstMessage: boolean;
  /** « Branche de départ des nouveaux worktrees » (ESC-160). */
  worktreeBase: WorktreeBase;
  runCommands: RunCommand[];
  /** The patterns of the files copied into new worktrees, one per line. */
  copy: string;
  worktreeSetup: WorktreeStep[];
  worktreeTeardown: WorktreeStep[];
  /** « Commit »: the agent writes the commits, or the app commits with the message the user read. */
  commitMode: CommitMode;
  /** « Ouvrir les pages des lancements » (ESC-311). */
  pagesOpen: PagesOpen;
  /** « Compte préféré »: the Claude account its agents and tickets go to; empty is « Automatique ». */
  account: string;
  /** « Dossier des worktrees », as typed: empty is `.claude/worktrees`. */
  worktreeDir: string;
  /** « Préfixe des branches des agents ». */
  agentBranchPrefix: string;
  /** « Préfixe des branches des tickets ». */
  ticketBranchPrefix: string;
  /** « Remote control pour les agents de ce projet »: its own value, kept while the setting for all agents covers it too. */
  remoteControl: boolean;
  /** « Harness par défaut »: null is Claude Code. */
  engine: EngineId | null;
  /** « Projet privé ». */
  private: boolean;
  /** « Comptes partagés à la limite ». */
  sharedOnLimit: boolean;
  /** The corrections of its workflows (ESC-195). */
  workflowFixes: WorkflowFix[];
  /** « Relire quand les fichiers changent ». */
  workflowsAutoRead: boolean;
  /** « Où garder la configuration des workflows »: in the repository (`.escouade/workflows/`), or on this machine. */
  workflowsInRepo: boolean;
  /** Off: « Pas de workflow pour ce projet » (ESC-309). */
  workflowsEnabled: boolean;
  /** « Ton nom dans ce projet » (ESC-190): empty is the app's. */
  userName: string;
  /** « Partager chaque nouveau ticket » (ESC-190). */
  shareNew: boolean;
  board: BoardSettings;
  /** Its sources in external ticket systems (the « Intégrations » tab). */
  integrations: ProjectIntegrations;
}

function draftOf(p: Project): ProjectDraft {
  const {
    name,
    color,
    tileLetters,
    logo,
    tileLabel,
    tileLabelText,
    worktreePerAgent,
    worktreeAtFirstMessage,
    worktreeBase,
    runCommands,
    worktreeCopy,
    worktreeSetup,
    worktreeTeardown,
    commitMode,
    pagesOpen,
    account,
    worktreeDir,
    agentBranchPrefix,
    ticketBranchPrefix,
    remoteControl,
    engine,
    private: isPrivate,
    sharedOnLimit,
    workflowFixes,
    workflowsAutoRead,
    workflowsInRepo,
    workflowsEnabled,
    userName,
    shareNew,
    docsBranch,
    board,
    integrations,
  } = $state.snapshot(p);
  return {
    name,
    color,
    // Absent from a project of the window's tests made before ESC-167.
    tileLetters: tileLetters ?? '',
    logo: logo ?? '',
    // Absent from a project of the window's tests made before ESC-168.
    tileLabel: tileLabel ?? false,
    tileLabelText: tileLabelText ?? '',
    worktreePerAgent,
    // Absent from a project of the window's tests made before ESC-156: on, as the backend has it.
    worktreeAtFirstMessage: worktreeAtFirstMessage ?? true,
    // Absent from a project of the window's tests made before ESC-160: the folder's branch.
    worktreeBase: worktreeBase ?? { kind: 'head' },
    runCommands,
    copy: worktreeCopy.join('\n'),
    worktreeSetup,
    worktreeTeardown,
    commitMode,
    // Absent from a project saved before ESC-311: in the agent.
    pagesOpen: pagesOpen ?? 'agent',
    account,
    worktreeDir,
    agentBranchPrefix,
    ticketBranchPrefix,
    remoteControl,
    // Absent from a project of the window's tests made before 1.8.
    engine: engine ?? null,
    // Absent from a project of the window's tests made before ESC-139.
    private: !!isPrivate,
    // Absent from a project of the window's tests made before ESC-141.
    sharedOnLimit: !!sharedOnLimit,
    // Absent from a project of the window's tests made before ESC-195.
    workflowFixes: workflowFixes ?? [],
    workflowsAutoRead: workflowsAutoRead ?? true,
    workflowsInRepo: workflowsInRepo ?? true,
    // Absent from a project of the window's tests made before ESC-309.
    workflowsEnabled: workflowsEnabled ?? true,
    // Absent from a project of the window's tests made before ESC-190.
    userName: userName ?? '',
    shareNew: !!shareNew,
    // Absent from a project of the window's tests made before ESC-256.
    docsBranch: docsBranch ?? '',
    board,
    integrations,
  };
}

/** What keeps a draft's « Dossier des worktrees » and branch prefixes from being saved, field by field. */
export function worktreesProblems(path: string, d: Pick<ProjectDraft, 'worktreeDir' | 'agentBranchPrefix' | 'ticketBranchPrefix'>) {
  return {
    dir: worktreesRootProblem(path, d.worktreeDir),
    agentPrefix: branchPrefixProblem(d.agentBranchPrefix.trim()),
    ticketPrefix: branchPrefixProblem(d.ticketBranchPrefix.trim()),
  };
}

/** How long a text waits after its last key before it is saved. */
export const SAVE_PAUSE_MS = 700;

/** The numbers of the app's settings typed in a field: a whole number of 0 or more each. */
const NUMBERS = ['idleStopMinutes', 'attachMaxMb', 'attachWarnMb', 'attachKeepDays'] as const;
export type NumberSetting = (typeof NUMBERS)[number];

/** A number typed that cannot be saved: emptied (null), not a number, or below 0. */
const badNumber = (n: unknown) => n === null || n === undefined || n === '' || !Number.isFinite(Number(n)) || Number(n) < 0;

/** The steps as saved: trimmed, without those that run nothing. */
const stepsOf = (steps: WorktreeStep[]) =>
  steps.map((x) => ({ ...x, command: x.command.trim(), cwd: x.cwd.trim() })).filter((x) => x.command);

/** The links as saved: one without a container is none, a column is commented once. */
function linksOf(d: ProjectDraft): ProjectIntegrations {
  return { links: d.integrations.links.filter((l) => l.container.trim()), comments: [...new Set(d.integrations.comments)] };
}

/** The draft as it would be saved: trimmed, the patterns split into lines. */
function fieldsOf(d: ProjectDraft): ProjectFields {
  return {
    name: d.name.trim(),
    color: d.color,
    tileLetters: d.tileLetters.trim(),
    logo: d.logo,
    tileLabel: d.tileLabel,
    tileLabelText: d.tileLabelText.trim(),
    worktreePerAgent: d.worktreePerAgent,
    worktreeAtFirstMessage: d.worktreeAtFirstMessage,
    // A branch not chosen yet (« Une branche… » with no name) leaves it as the folder's.
    worktreeBase: d.worktreeBase.kind === 'branch' && !d.worktreeBase.branch.trim() ? { kind: 'head' } : d.worktreeBase,
    runCommands: d.runCommands.map((c) => ({ ...c, name: c.name.trim(), command: c.command.trim(), cwd: c.cwd.trim() })),
    worktreeCopy: d.copy
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean),
    worktreeSetup: stepsOf(d.worktreeSetup),
    worktreeTeardown: stepsOf(d.worktreeTeardown),
    commitMode: d.commitMode,
    pagesOpen: d.pagesOpen,
    account: d.account,
    worktreeDir: d.worktreeDir.trim(),
    agentBranchPrefix: d.agentBranchPrefix.trim(),
    ticketBranchPrefix: d.ticketBranchPrefix.trim(),
    remoteControl: d.remoteControl,
    engine: d.engine,
    private: d.private,
    sharedOnLimit: d.sharedOnLimit,
    workflowFixes: d.workflowFixes,
    workflowsAutoRead: d.workflowsAutoRead,
    workflowsInRepo: d.workflowsInRepo,
    workflowsEnabled: d.workflowsEnabled,
    userName: d.userName.trim(),
    shareNew: d.shareNew,
    docsBranch: d.docsBranch.trim(),
  };
}

function boardOf(d: ProjectDraft): BoardSettings {
  const testCommand = d.board.testCommand.trim();
  return { ...d.board, testCommand, testsFirst: d.board.testsFirst && !!testCommand };
}

/** A whole number of 0 or more (the field's own refusals apart, `badNumber`). */
const count = (n: number) => Math.max(0, Math.floor(Number(n) || 0));

function settingsOf(s: Settings): Settings {
  return {
    ...s,
    idleStopMinutes: count(s.idleStopMinutes),
    attachMaxMb: count(s.attachMaxMb),
    attachWarnMb: count(s.attachWarnMb),
    attachKeepDays: count(s.attachKeepDays),
  };
}

/** The fields of `next` that differ from `base`. */
function changes<T extends object>(base: T, next: T): Partial<T> {
  const out: Partial<T> = {};
  for (const k of Object.keys(next) as (keyof T)[]) {
    if (JSON.stringify(base[k]) !== JSON.stringify(next[k])) out[k] = next[k];
  }
  return out;
}

const empty = (o: object) => Object.keys(o).length === 0;

/** What it says when `n` commands were put in the draft, `refused` others having been left out for what they would hide. */
const proposed = (n: number, refused = 0) => {
  if (!refused) return t('settings.suggest.proposed', { count: n });
  return refused === 1
    ? t('settings.suggest.proposedLeftOne', { count: n })
    : t('settings.suggest.proposedLeftMany', { count: n, refused });
};

/** The keys that differ from the draft as opened, by where they are kept. */
interface ChangedKeys {
  settings: Set<string>;
  project: Set<string>;
  board: Set<string>;
}

const rowChanged = (r: RowInfo, k: ChangedKeys) =>
  r.settings.some((x) => k.settings.has(x)) || r.project.some((x) => k.project.has(x)) || r.board.some((x) => k.board.has(x));

class SettingsForm {
  /** The page on screen. */
  page = $state<SettingsPage>('accounts');
  /** The project the pages of a project set. */
  projectId = $state<string | null>(null);
  /**
   * The row a search opened (R8): its page unfolds « Avancé » when it is there, and the window lights it. `n` tells
   * one opening from the next of the same row.
   */
  found = $state<{ row: RowInfo; n: number } | null>(null);
  settings = $state<Settings>({} as Settings);
  /** By project. */
  projects = $state<Record<string, ProjectDraft>>({});
  busy = $state(false);
  /** Claude reads the project to suggest its worktree steps, by project. */
  suggesting = $state<Record<string, boolean>>({});
  /** Claude reads the project to suggest its launch commands, by project. */
  suggestingLaunch = $state<Record<string, boolean>>({});
  /**
   * Claude's suggestion of worktree steps, shown in full until it is taken in place of the draft's or ignored, by
   * project: a command longer than its field would only be read by scrolling it, and the setup runs by itself in each
   * new worktree.
   */
  proposal = $state<Record<string, WorktreeSuggestion>>({});
  /** Claude's suggestion of launch commands, shown the same way, by project. */
  launchProposal = $state<Record<string, RunCommand[]>>({});
  /**
   * The name git knows (ESC-243), put in « Ton nom » while none was given and it was not emptied on purpose: in the
   * draft and in what it is measured against, so that it is neither a change nor saved as is (git's name stands in).
   */
  gitName = $state('');
  /** « Ton nom » as given, the name taken from git left out: what the projects' fields fall back on before their git. */
  ownName = $derived.by(() => {
    const n = this.settings.userName?.trim() ?? '';
    return n === this.gitName ? '' : n;
  });
  /** As last opened or saved: what a change is measured against. */
  #base = $state.raw<{ settings: Settings; projects: Record<string, ProjectDraft> }>({ settings: {} as Settings, projects: {} });

  project = $derived(this.projectId ? this.projects[this.projectId] : undefined);

  /** The numbers typed that cannot be saved: said on their row, the value saved before stays. */
  badNumbers = $derived(new Set<NumberSetting>(NUMBERS.filter((k) => badNumber(this.settings[k]))));

  /**
   * What of a project's fields cannot be saved as typed, said on its row: its name emptied, a launch command without a
   * name or a line, its worktrees folder or a branch prefix refused. The value saved before stays until it is fixed.
   */
  projectProblems(id: string) {
    const d = this.projects[id];
    const p = app.projects.find((x) => x.id === id);
    if (!d || !p) return { name: false, commands: false, dir: null, agentPrefix: false, ticketPrefix: false };
    return {
      name: !d.name.trim(),
      commands: d.runCommands.some((c) => !c.name.trim() || !c.command.trim()),
      ...worktreesProblems(p.path, d),
    };
  }

  /** The fields of a project left out of a save, for what `projectProblems` says of them. */
  #heldBack(id: string): (keyof ProjectFields)[] {
    const p = this.projectProblems(id);
    const out: (keyof ProjectFields)[] = [];
    if (p.name) out.push('name');
    if (p.commands) out.push('runCommands');
    if (p.dir) out.push('worktreeDir');
    if (p.agentPrefix) out.push('agentBranchPrefix');
    if (p.ticketPrefix) out.push('ticketBranchPrefix');
    return out;
  }

  #timer: ReturnType<typeof setTimeout> | undefined;
  /** The saves one after the other: a change made while one runs is saved by the next. */
  #chain: Promise<boolean> = Promise.resolve(true);
  /** The saves asked for and not finished: a pause waited, or a save running (the window's `data-saving`). */
  saving = $state(false);
  #asked = 0;

  /** A change made: saved after a pause (a key typed waits for the next), or at once by `flush`. */
  schedule(delay = SAVE_PAUSE_MS) {
    clearTimeout(this.#timer);
    this.saving = true;
    this.#timer = setTimeout(() => void this.flush(), delay);
  }

  /** The saves started, finished (none started by it). */
  settled(): Promise<boolean> {
    return this.#chain;
  }

  /** Saves now what changed and can be saved (a choice made, a field left, the window closed). */
  flush(): Promise<boolean> {
    clearTimeout(this.#timer);
    this.#timer = undefined;
    const asked = ++this.#asked;
    this.saving = true;
    this.#chain = this.#chain
      .then(
        () => this.save(),
        () => this.save(),
      )
      .finally(() => {
        if (asked === this.#asked && !this.#timer) this.saving = false;
      });
    return this.#chain;
  }

  /**
   * What the window shows, read again from the app, on the page and the project given (else the one on
   * screen). A save still running is let finish first, so that what it sends is not shown as a change to undo.
   */
  open(place: SettingsPlace = {}) {
    this.resume(place);
    if (this.busy) {
      void this.#chain.then(() => this.#read());
      return;
    }
    this.#read();
  }

  #read() {
    const settings = $state.snapshot(app.settings);
    const projects = Object.fromEntries(app.projects.map((p) => [p.id, draftOf(p)]));
    this.#base = { settings, projects };
    this.settings = structuredClone(settings);
    this.gitName = '';
    this.projects = structuredClone(projects);
    // What was proposed to the draft dropped goes with it.
    this.proposal = {};
    this.launchProposal = {};
  }

  /** « Ton nom » filled with git's name (ESC-243) when it has none and was not emptied on purpose. */
  prefillName(git: string | null) {
    const base = this.#base.settings;
    if (!git?.trim() || base.userName?.trim() || base.userNameCleared || this.settings.userName?.trim()) return;
    this.gitName = git!.trim();
    this.#base = { ...this.#base, settings: { ...base, userName: this.gitName } };
    this.settings.userName = this.gitName;
  }

  /** « Ton nom » typed: emptied, it is a choice that its field is not filled from git again. */
  setUserName(v: string) {
    this.settings.userName = v;
    if (!v.trim()) this.settings.userNameCleared = true;
    else if (this.settings.userNameCleared) this.settings.userNameCleared = false;
  }

  /**
   * The draft as it is, on the page given (else « Comptes et harness »)
   * and the project given (else the one on screen).
   */
  resume({ page, projectId }: SettingsPlace = {}) {
    const has = (id: string | null | undefined) => !!id && app.projects.some((p) => p.id === id);
    this.page = page ?? 'accounts';
    this.found = null;
    this.projectId = has(projectId) ? projectId! : has(app.project?.id) ? app.project!.id : (app.projects[0]?.id ?? null);
  }

  /** A page has something to save: for the project given, else for any (a page of a project). */
  changed(page: SettingsPage, projectId?: string): boolean {
    const keys = this.#changedKeys(projectId);
    return rowsOf(page).some((r) => rowChanged(r, keys));
  }

  /** What differs from the draft as opened: the app's keys, and those of the project given (else of any). */
  #changedKeys(projectId?: string): ChangedKeys {
    const settings = new Set(Object.keys(changes(settingsOf(this.#base.settings), settingsOf(this.settings))));
    const project = new Set<string>();
    const board = new Set<string>();
    for (const id of projectId ? [projectId] : Object.keys(this.projects)) {
      for (const k of Object.keys(this.#projectChanges(id))) project.add(k === 'worktreeCopy' ? 'copy' : k);
      if (this.#linksChanged(id)) project.add('integrations');
      for (const k of Object.keys(this.#boardChanges(id))) board.add(k);
    }
    return { settings, project, board };
  }

  /**
   * The project as the backend has it now (its `.escouade.yml` applied, ESC-146): its draft follows, unless it holds
   * changes of the user's, which stay as typed.
   */
  refresh(p: Project) {
    if (!this.projects[p.id] || !empty(this.#projectChanges(p.id)) || !empty(this.#boardChanges(p.id)) || this.#linksChanged(p.id)) return;
    const d = draftOf(p);
    this.#base = { ...this.#base, projects: { ...this.#base.projects, [p.id]: d } };
    this.projects[p.id] = structuredClone(d);
  }

  /** A launch command for the project on screen in the modal, run by the first shell found. */
  addCommand() {
    this.project?.runCommands.push({ id: crypto.randomUUID(), name: '', command: '', shell: app.shells[0]?.id ?? 'pwsh', cwd: '' });
  }

  /** A worktree step for the project on screen in the modal, run by the first shell found. */
  addStep(kind: StepKind) {
    const step: WorktreeStep = { id: crypto.randomUUID(), command: '', shell: app.shells[0]?.id ?? 'pwsh', cwd: '' };
    this.project?.[kind === 'setup' ? 'worktreeSetup' : 'worktreeTeardown'].push(step);
  }

  /**
   * « Remplir automatiquement »: Claude reads the project and its suggestion is proposed (`proposal`), to replace the
   * draft's worktree steps once read (nothing is proposed when it finds none, or fails). The steps the backend refused
   * are counted in what is said.
   */
  async suggest(projectId: string) {
    if (this.suggesting[projectId]) return;
    this.suggesting[projectId] = true;
    try {
      const s = await api.suggestWorktreeSteps(projectId);
      const n = s.setup.length + s.teardown.length;
      if (!n) return app.toast(t('settings.suggest.nothing'));
      // Closed and opened again meanwhile: proposed to the fresh draft all the same.
      if (!this.projects[projectId]) return;
      this.proposal[projectId] = s;
      app.toast(proposed(n, s.refused));
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      this.suggesting[projectId] = false;
    }
  }

  /** Claude's worktree steps, read, in place of both lists of the draft. */
  takeProposal(projectId: string) {
    const s = this.proposal[projectId];
    const d = this.projects[projectId];
    delete this.proposal[projectId];
    if (!s || !d) return;
    d.worktreeSetup = s.setup;
    d.worktreeTeardown = s.teardown;
  }

  /**
   * « Remplir automatiquement » of the launch commands: Claude reads the project and its suggestion is proposed
   * (`launchProposal`), to replace the draft's commands once read (nothing is proposed when it finds none, or fails).
   * The commands the backend refused are counted in what is said.
   */
  async suggestLaunch(projectId: string) {
    if (this.suggestingLaunch[projectId]) return;
    this.suggestingLaunch[projectId] = true;
    try {
      const { commands, refused } = await api.suggestRunCommands(projectId);
      if (!commands.length) return app.toast(t('settings.suggest.nothing'));
      // Closed and opened again meanwhile: proposed to the fresh draft all the same.
      if (!this.projects[projectId]) return;
      this.launchProposal[projectId] = commands;
      app.toast(proposed(commands.length, refused));
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      this.suggestingLaunch[projectId] = false;
    }
  }

  /** Claude's launch commands, read, in place of the draft's. */
  takeLaunchProposal(projectId: string) {
    const commands = this.launchProposal[projectId];
    const d = this.projects[projectId];
    delete this.launchProposal[projectId];
    if (commands && d) d.runCommands = commands;
  }

  #projectChanges(id: string): Partial<ProjectFields> {
    const base = this.#base.projects[id];
    const d = this.projects[id];
    return base && d ? changes(fieldsOf(base), fieldsOf(d)) : {};
  }

  #linksChanged(id: string): boolean {
    const base = this.#base.projects[id];
    const d = this.projects[id];
    return !!base && !!d && JSON.stringify(linksOf(base)) !== JSON.stringify(linksOf(d));
  }

  #boardChanges(id: string): Partial<BoardSettings> {
    const base = this.#base.projects[id];
    const d = this.projects[id];
    return base && d ? changes(boardOf(base), boardOf(d)) : {};
  }

  /**
   * Saves what changed, each part over what the app has now (the backend moves a board on by itself), what cannot be
   * saved as typed left out (`badNumbers`, `projectProblems`). Returns whether all that was sent was saved; what was
   * is no longer a change. `flush` calls it, one save at a time.
   */
  async save(): Promise<boolean> {
    if (this.busy) return false;
    this.busy = true;
    let ok = true;
    try {
      const diff = changes(settingsOf(this.#base.settings), settingsOf(this.settings));
      for (const k of this.badNumbers) delete diff[k];
      if (!empty(diff)) {
        const next = { ...$state.snapshot(app.settings), ...diff };
        const shells = await app.run(api.saveSettings(next));
        if (shells) {
          app.settings = next;
          app.shells = shells;
          this.#base = { ...this.#base, settings: { ...this.#base.settings, ...diff } };
          // Another `claude`: the board's banner and the hint of the path say whether it is found now.
          if ('claudePath' in diff || 'wslDistro' in diff) {
            const found = await api.claudeFound().catch(() => null);
            if (found) {
              app.claudeFound = found.claudeFound;
              app.claudePathFound = found.claudePathFound;
              app.enginesFound = found.enginesFound ?? (found.claudeFound ? ['claude'] : []);
            }
          }
        } else ok = false;
      }
      for (const id of Object.keys(this.projects)) {
        if (!(await this.#saveProject(id))) ok = false;
      }
    } finally {
      this.busy = false;
    }
    return ok;
  }

  async #saveProject(id: string): Promise<boolean> {
    let ok = true;
    // The logo goes its own way (ESC-167): the image chosen is copied by the backend.
    const { logo, ...fields } = this.#projectChanges(id);
    for (const k of this.#heldBack(id)) delete fields[k as keyof typeof fields];
    if (logo !== undefined && !(await this.#saveLogo(id, logo))) ok = false;
    const links = this.#linksChanged(id) ? linksOf($state.snapshot(this.projects[id])) : null;
    const current = () => app.projects.find((p) => p.id === id);
    let p = current();
    if (!p) return true;
    if (!empty(fields) || links) {
      // As sent: what is typed meanwhile stays a change.
      const sent: Partial<ProjectDraft> = { ...this.#draftFields(id, fields), ...(links ? { integrations: links } : {}) };
      const next = { ...$state.snapshot(p), ...fields, ...(links ? { integrations: links } : {}) };
      const saved = await app.run(api.updateProject(next).then(() => true));
      p = current();
      if (saved && p) {
        if (fields.runCommands)
          forgetLaunches(
            p.runCommands.filter((c) => !fields.runCommands!.some((x) => x.id === c.id)).flatMap((c) => [c.id, ...runsOfCommand(c.id)]),
          );
        // What was saved, and nothing else of the project.
        Object.assign(p, fields, links ? { integrations: links } : {});
        this.#rebase(id, (b) => ({ ...b, ...sent }));
      } else if (!saved) ok = false;
    }
    const board = this.#boardChanges(id);
    p = current();
    if (p && !empty(board)) {
      const saved = await app.run(api.boardSet(id, { ...$state.snapshot(p.board), ...board }));
      if (saved) {
        app.replaceProject(saved);
        this.#rebase(id, (b) => ({ ...b, board: { ...b.board, ...board } }));
      } else ok = false;
    }
    return ok;
  }

  /** The images chosen as logos, by path: their preview until they are saved. */
  logoPreviews = $state<Record<string, string>>({});

  /** « Choisir une image… »: the image read as a logo would be (a refusal said), then in the draft. */
  async chooseLogo(projectId: string) {
    const picked = await pickLogo();
    const d = this.projects[projectId];
    if (!picked || !d) return;
    this.logoPreviews[picked.path] = picked.url;
    d.logo = picked.path;
  }

  /** What the tile of the draft's project shows as its logo: the image chosen, else its own. */
  logoOf(projectId: string): string | null {
    const d = this.projects[projectId];
    const p = app.projects.find((x) => x.id === projectId);
    if (!d?.logo) return null;
    if (p && d.logo === p.logo) return logoUrl(p);
    return this.logoPreviews[d.logo] ?? null;
  }

  async #saveLogo(id: string, logo: string): Promise<boolean> {
    const saved = await app.run(api.setProjectLogo(id, logo || null));
    const p = app.projects.find((x) => x.id === id);
    if (!saved) return false;
    const url = this.logoPreviews[logo];
    if (url) knowLogo(saved, url);
    if (p) p.logo = saved.logo;
    // The path chosen becomes the copy's name, in the draft too unless another was chosen meanwhile.
    const d = this.projects[id];
    if (d && d.logo === logo) d.logo = saved.logo ?? '';
    this.#rebase(id, (b) => ({ ...b, logo: saved.logo ?? '' }));
    return true;
  }

  /** The draft's own values for the fields saved: the patterns as typed, the commands as edited. */
  #draftFields(id: string, fields: Partial<ProjectFields>): Partial<ProjectDraft> {
    const d = $state.snapshot(this.projects[id]);
    const out: Partial<ProjectDraft> = {};
    for (const k of Object.keys(fields) as (keyof ProjectFields)[]) {
      if (k === 'worktreeCopy') out.copy = d.copy;
      else (out as Record<string, unknown>)[k] = d[k];
    }
    return out;
  }

  #rebase(id: string, f: (b: ProjectDraft) => ProjectDraft) {
    const b = this.#base.projects[id];
    if (b) this.#base = { ...this.#base, projects: { ...this.#base.projects, [id]: f(b) } };
  }
}

export const settingsForm = new SettingsForm();
