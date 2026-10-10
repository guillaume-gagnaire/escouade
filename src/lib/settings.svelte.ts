// The settings modal: one draft of all it sets (the app's settings, each project's own and its
// board's), saved at once by « Enregistrer » and dropped by « Annuler ».

import { api } from './ipc';
import { forgetLaunches } from './launch-actions';
import { app } from './state.svelte';
import type {
  BoardSettings,
  CommitMode,
  Project,
  ProjectIntegrations,
  RunCommand,
  Settings,
  WorktreeStep,
  WorktreeSuggestion,
} from './types';

export type SettingsTab = 'claude' | 'notifications' | 'projects' | 'board' | 'integrations' | 'terminals' | 'network' | 'about';

export const SETTINGS_TABS: { id: SettingsTab; label: string; icon: string; desc: string; scoped?: boolean }[] = [
  { id: 'claude', label: 'Claude Code', icon: '✳', desc: 'Exécutable, modèle et permissions par défaut' },
  { id: 'notifications', label: 'Notifications', icon: '♪', desc: 'Alertes visuelles et sonores' },
  { id: 'projects', label: 'Projets', icon: '▤', desc: 'Réglages propres à chaque projet', scoped: true },
  { id: 'board', label: 'Kanban', icon: '▦', desc: 'Pilote auto et tickets validés', scoped: true },
  { id: 'integrations', label: 'Intégrations', icon: '⧉', desc: 'Jira, Trello et GitHub Issues', scoped: true },
  { id: 'terminals', label: 'Terminaux', icon: '$_', desc: 'Shells disponibles dans les terminaux intégrés' },
  {
    id: 'network',
    label: 'Réseau',
    icon: '⇄',
    desc: 'Proxy HTTP(S) et certificats TLS pour Claude Code, les intégrations et les mises à jour',
  },
  { id: 'about', label: 'À propos', icon: 'ⓘ', desc: 'Version et données locales' },
];

/** The app's settings each tab sets. */
const SETTINGS_OF: Partial<Record<SettingsTab, (keyof Settings)[]>> = {
  claude: ['claudePath', 'defaultModel', 'defaultEffort', 'defaultMode', 'autoResume', 'idleStopMinutes'],
  notifications: ['sound', 'osNotifications', 'notifyFor'],
  terminals: ['pwshPath', 'bashPath', 'wslDistro'],
  network: ['proxyUrl', 'noProxy', 'proxyTerminals', 'insecureTls'],
  // The quotas are the account's: one pause for every project's board.
  board: ['quotaPause'],
  integrations: ['integrations'],
  about: ['autoUpdate'],
};

/** What the modal sets of a project, as the backend's `update_project` takes it, its board apart. */
type ProjectFields = Pick<
  Project,
  'name' | 'color' | 'worktreePerAgent' | 'runCommands' | 'worktreeCopy' | 'worktreeSetup' | 'worktreeTeardown' | 'commitMode'
>;

/** Which commands of a project's worktrees: run once one is made, or before one is removed. */
export type StepKind = 'setup' | 'teardown';

export interface ProjectDraft {
  name: string;
  color: string;
  worktreePerAgent: boolean;
  runCommands: RunCommand[];
  /** The patterns of the files copied into new worktrees, one per line. */
  copy: string;
  worktreeSetup: WorktreeStep[];
  worktreeTeardown: WorktreeStep[];
  /** « Commit »: the agent writes the commits, or the app commits with the message the user read. */
  commitMode: CommitMode;
  board: BoardSettings;
  /** Its sources in external ticket systems (the « Intégrations » tab). */
  integrations: ProjectIntegrations;
}

function draftOf(p: Project): ProjectDraft {
  const { name, color, worktreePerAgent, runCommands, worktreeCopy, worktreeSetup, worktreeTeardown, commitMode, board, integrations } =
    $state.snapshot(p);
  return {
    name,
    color,
    worktreePerAgent,
    runCommands,
    copy: worktreeCopy.join('\n'),
    worktreeSetup,
    worktreeTeardown,
    commitMode,
    board,
    integrations,
  };
}

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
    runCommands: d.runCommands.map((c) => ({ ...c, name: c.name.trim(), command: c.command.trim(), cwd: c.cwd.trim() })),
    worktreeCopy: d.copy
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean),
    worktreeSetup: stepsOf(d.worktreeSetup),
    worktreeTeardown: stepsOf(d.worktreeTeardown),
    commitMode: d.commitMode,
  };
}

function boardOf(d: ProjectDraft): BoardSettings {
  const testCommand = d.board.testCommand.trim();
  return { ...d.board, testCommand, testsFirst: d.board.testsFirst && !!testCommand };
}

function settingsOf(s: Settings): Settings {
  // An emptied number field is null: the backend expects a number.
  return { ...s, idleStopMinutes: Math.max(0, Math.floor(Number(s.idleStopMinutes) || 0)) };
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

/** What « Remplir automatiquement » says when Claude read the project and found nothing to run. */
const NOTHING_TO_RUN = "Claude n'a trouvé aucune commande à lancer pour ce projet.";

/** What it says when `n` commands were put in the draft, `refused` others having been left out for what they would hide. */
const proposed = (n: number, refused = 0) => {
  const plural = refused > 1 ? 's' : '';
  const left = refused ? `, ${refused} écartée${plural} (caractères invisibles ou trop longue${plural})` : '';
  return n > 1
    ? `${n} commandes proposées${left} : relis-les avant d'enregistrer.`
    : `1 commande proposée${left} : relis-la avant d'enregistrer.`;
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
      if (!d.name.trim()) return `Le projet « ${p.name} » doit garder un nom.`;
      if (d.runCommands.some((c) => !c.name.trim() || !c.command.trim())) {
        return `Chaque commande de lancement de « ${d.name.trim()} » demande un nom et une ligne de commande.`;
      }
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
    if (keys) return own;
    if (tab === 'projects') return Object.keys(this.projects).some((id) => !empty(this.#projectChanges(id)));
    return false;
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
      if (!n) return app.toast(NOTHING_TO_RUN);
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
      if (!commands.length) return app.toast(NOTHING_TO_RUN);
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
          forgetLaunches(p.runCommands.filter((c) => !fields.runCommands!.some((x) => x.id === c.id)).map((c) => c.id));
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
