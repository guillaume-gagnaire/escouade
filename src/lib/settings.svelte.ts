// The settings modal: one draft of all it sets (the app's settings, each project's own and its
// board's), saved at once by « Enregistrer » and dropped by « Annuler ».

import { t } from './i18n';
import { api } from './ipc';
import { forgetLaunches, runsOfCommand } from './launch-actions';
import { app } from './state.svelte';
import type {
  BoardSettings,
  CommitMode,
  EngineId,
  Project,
  ProjectIntegrations,
  RunCommand,
  Settings,
  WorktreeBase,
  WorktreeStep,
  WorktreeSuggestion,
} from './types';
import { branchPrefixProblem, worktreesRootProblem } from './worktrees';

export type SettingsTab =
  'app' | 'claude' | 'accounts' | 'notifications' | 'voice' | 'projects' | 'board' | 'integrations' | 'terminals' | 'network' | 'about';

interface TabInfo {
  id: SettingsTab;
  label: string;
  icon: string;
  desc: string;
  scoped?: boolean;
}

/** A tab whose label and description are read where shown, so that they follow a change of language. */
function tabInfo(id: SettingsTab, icon: string, scoped = false): TabInfo {
  return {
    id,
    icon,
    ...(scoped ? { scoped } : {}),
    get label() {
      return t(`settings.tabs.${id}.label`);
    },
    get desc() {
      return t(`settings.tabs.${id}.desc`);
    },
  };
}

export const SETTINGS_TABS: TabInfo[] = [
  tabInfo('app', 'Aa'),
  tabInfo('claude', '✳'),
  // Saved at once, each change (not part of the draft).
  tabInfo('accounts', '◎'),
  tabInfo('notifications', '♪'),
  tabInfo('voice', '◖'),
  tabInfo('projects', '▤', true),
  tabInfo('board', '▦', true),
  tabInfo('integrations', '⧉', true),
  tabInfo('terminals', '$_'),
  tabInfo('network', '⇄'),
  tabInfo('about', 'ⓘ'),
];

/** The app's settings each tab sets. */
const SETTINGS_OF: Partial<Record<SettingsTab, (keyof Settings)[]>> = {
  app: ['language', 'claudeLanguage', 'writerEngine', 'writerModel'],
  claude: [
    'claudePath',
    'autoResume',
    'switchOnLimit',
    'handoverOnLimit',
    'handoverModels',
    'todoTools',
    'claudeSelfUpdate',
    'idleStopMinutes',
    'attachMaxMb',
    'attachWarnMb',
    'attachOrigin',
    'attachBigText',
    'attachKeepDays',
    'remoteControlAll',
  ],
  // What a project added starts with: shown under the project's own settings.
  projects: ['privateProjects'],
  notifications: ['sound', 'osNotifications', 'notifyFor'],
  voice: ['voice'],
  terminals: ['pwshPath', 'bashPath', 'wslDistro'],
  network: ['proxyUrl', 'noProxy', 'proxyTerminals', 'insecureTls'],
  // The quotas are the account's: one pause for every project's board.
  board: ['quotaPause'],
  integrations: ['integrations'],
  about: ['autoUpdate', 'telemetry'],
};

/** What the modal sets of a project, as the backend's `update_project` takes it, its board apart. */
type ProjectFields = Pick<
  Project,
  | 'name'
  | 'color'
  | 'worktreePerAgent'
  | 'worktreeAtFirstMessage'
  | 'worktreeBase'
  | 'runCommands'
  | 'worktreeCopy'
  | 'worktreeSetup'
  | 'worktreeTeardown'
  | 'commitMode'
  | 'account'
  | 'worktreeDir'
  | 'agentBranchPrefix'
  | 'ticketBranchPrefix'
  | 'remoteControl'
  | 'engine'
  | 'private'
  | 'sharedOnLimit'
>;

/** Which commands of a project's worktrees: run once one is made, or before one is removed. */
export type StepKind = 'setup' | 'teardown';

export interface ProjectDraft {
  name: string;
  color: string;
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
  board: BoardSettings;
  /** Its sources in external ticket systems (the « Intégrations » tab). */
  integrations: ProjectIntegrations;
}

function draftOf(p: Project): ProjectDraft {
  const {
    name,
    color,
    worktreePerAgent,
    worktreeAtFirstMessage,
    worktreeBase,
    runCommands,
    worktreeCopy,
    worktreeSetup,
    worktreeTeardown,
    commitMode,
    account,
    worktreeDir,
    agentBranchPrefix,
    ticketBranchPrefix,
    remoteControl,
    engine,
    private: isPrivate,
    sharedOnLimit,
    board,
    integrations,
  } = $state.snapshot(p);
  return {
    name,
    color,
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

const worktreesProblem = (path: string, d: ProjectDraft) => {
  const p = worktreesProblems(path, d);
  return !!p.dir || p.agentPrefix || p.ticketPrefix;
};

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
    account: d.account,
    worktreeDir: d.worktreeDir.trim(),
    agentBranchPrefix: d.agentBranchPrefix.trim(),
    ticketBranchPrefix: d.ticketBranchPrefix.trim(),
    remoteControl: d.remoteControl,
    engine: d.engine,
    private: d.private,
    sharedOnLimit: d.sharedOnLimit,
  };
}

function boardOf(d: ProjectDraft): BoardSettings {
  const testCommand = d.board.testCommand.trim();
  return { ...d.board, testCommand, testsFirst: d.board.testsFirst && !!testCommand };
}

/** A whole number of 0 or more: an emptied number field is null, and the backend expects a number. */
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

class SettingsForm {
  tab = $state<SettingsTab>('claude');
  /** The project the "Projets" and "Kanban" tabs set. */
  projectId = $state<string | null>(null);
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
  /** As last opened or saved: what a change is measured against. */
  #base = $state.raw<{ settings: Settings; projects: Record<string, ProjectDraft> }>({ settings: {} as Settings, projects: {} });

  project = $derived(this.projectId ? this.projects[this.projectId] : undefined);

  /** What keeps the draft from being saved. */
  problem = $derived.by(() => {
    for (const p of app.projects) {
      const d = this.projects[p.id];
      if (!d) continue;
      if (!d.name.trim()) return t('settings.problem.projectName', { name: p.name });
      if (d.runCommands.some((c) => !c.name.trim() || !c.command.trim())) {
        return t('settings.problem.command', { name: d.name.trim() });
      }
      if (worktreesProblem(p.path, d)) return t('settings.problem.worktrees', { name: d.name.trim() });
    }
    return null;
  });

  /** A fresh draft of everything, on `tab` and the project given (else the one on screen). */
  open({ tab = 'claude', projectId }: { tab?: SettingsTab; projectId?: string } = {}) {
    this.resume({ tab, projectId });
    const settings = $state.snapshot(app.settings);
    const projects = Object.fromEntries(app.projects.map((p) => [p.id, draftOf(p)]));
    this.#base = { settings, projects };
    this.settings = structuredClone(settings);
    this.projects = structuredClone(projects);
    // What was proposed to the draft dropped goes with it.
    this.proposal = {};
    this.launchProposal = {};
  }

  /** The draft as it is, on `tab` and the project given (else the one on screen). */
  resume({ tab = 'claude', projectId }: { tab?: SettingsTab; projectId?: string } = {}) {
    const has = (id: string | null | undefined) => !!id && app.projects.some((p) => p.id === id);
    this.tab = tab;
    this.projectId = has(projectId) ? projectId! : has(app.project?.id) ? app.project!.id : (app.projects[0]?.id ?? null);
  }

  /** A tab has something to save. */
  changed(tab: SettingsTab): boolean {
    const keys = SETTINGS_OF[tab];
    const own = !!keys && keys.some((k) => k in changes(settingsOf(this.#base.settings), settingsOf(this.settings)));
    if (tab === 'integrations') return own || Object.keys(this.projects).some((id) => this.#linksChanged(id));
    if (tab === 'board') return own || Object.keys(this.projects).some((id) => !empty(this.#boardChanges(id)));
    if (tab === 'projects') return own || Object.keys(this.projects).some((id) => !empty(this.#projectChanges(id)));
    if (keys) return own;
    return false;
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
   * Saves what changed, each part over what the app has now (the backend moves a board on by
   * itself). Returns whether all of it was saved; what was is no longer a change.
   */
  async save(): Promise<boolean> {
    if (this.problem || this.busy) return false;
    this.busy = true;
    let ok = true;
    try {
      const diff = changes(settingsOf(this.#base.settings), settingsOf(this.settings));
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
    const fields = this.#projectChanges(id);
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
