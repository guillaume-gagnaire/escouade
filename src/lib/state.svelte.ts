// Global application state (Svelte 5 runes) fed by the backend event channel.

import { openUrl } from '@tauri-apps/plugin-opener';
import { api } from './ipc';
import { applyConvOps, dropConversation, releaseIdle } from './conversations.svelte';
import { buffers } from './editor/buffers.svelte';
import { navHistory } from './editor/history';
import { recentFiles } from './editor/quick-open';
import { fileSearches } from './editor/search.svelte';
import { ancestors, movedPath } from './editor/tree';
import { trees } from './editor/trees.svelte';
import { basename, isAbsPath, plural, relPath } from './format';
import { readPref, writePref } from './prefs';
import { testCommand } from './recipe';
import type { SettingsTab } from './settings.svelte';
import { applyTheme } from './theme';
import type {
  AccountView,
  Agent,
  AgentStatus,
  AutopilotPause,
  FileTree,
  GitInfo,
  LaunchState,
  ModelInfo,
  Project,
  Resources,
  Settings,
  SetupOutput,
  ShellInfo,
  TermInfo,
  Ticket,
  UiEvent,
  UiState,
  Usage,
} from './types';

export type Modal =
  | { kind: 'newProject' }
  /**
   * On a tab (Claude Code by default) and, for a project's tabs, a project (the one on screen by default),
   * scrolled to a section of it; `resume`: with the draft it had, back from a modal it opened; `suggest`: Claude
   * reads the project at once to suggest its launch commands.
   */
  | { kind: 'settings'; tab?: SettingsTab; projectId?: string; section?: 'launch'; resume?: boolean; suggest?: boolean }
  /** `wholeProject`: every uncommitted file the project's list shows, worktrees of the agents included. */
  | { kind: 'diff'; projectId: string; agentId: string | null; paths: string[]; title: string; commit?: string; wholeProject?: boolean }
  | {
      kind: 'confirm';
      title: string;
      body: string;
      confirm: string;
      danger?: boolean;
      option?: { label: string; value: boolean };
      /** A third choice, between cancelling and confirming. */
      alt?: { label: string; onClick: () => void | Promise<void> };
      onConfirm: (option: boolean) => void | Promise<void>;
      /** Where cancelling goes back to (no modal by default). */
      onCancel?: () => void;
    }
  | { kind: 'rename'; title: string; value: string; onSubmit: (v: string) => void | Promise<void> }
  | { kind: 'testLaunch'; agentId: string }
  | { kind: 'import'; projectId: string }
  /** The update downloaded (`app.update`), which a restart installs. */
  | { kind: 'update' }
  /** The release notes of the update installed since the last start. */
  | { kind: 'notes'; version: string; notes: string }
  /** « Rechercher dans les conversations » (Ctrl+K). */
  | { kind: 'convSearch' }
  /** « Ouvrir un fichier » (Ctrl+P): the files of `source` ('project' or an agent's id) in a project. */
  | { kind: 'quickOpen'; projectId: string; source: string }
  /** A direct commit of the agent's changes; `agentId` null: of the project's own checkout. */
  | { kind: 'commit'; projectId: string; agentId: string | null };

export interface Toast {
  id: number;
  text: string;
  kind: 'info' | 'error' | 'ok';
  /** A button of the toast, which closes it. */
  action?: ToastAction;
}

export interface ToastAction {
  label: string;
  onClick: () => void;
}

/** Statuses that call for the user: a question, the end of a turn, an error. */
const ALERT: ReadonlySet<AgentStatus> = new Set(['waiting', 'done', 'error']);

/** How many of the last lines of the step running a worktree's setup the window keeps (as the backend does). */
const SETUP_LINES = 500;

export interface UpdateInfo {
  version: string;
  /** Its release notes (markdown). */
  notes: string;
  /** Downloaded, its signature checked: a restart installs it. */
  ready: boolean;
}

/** What the editor shows for one source of a project. */
export interface EditorPlace {
  open: string[];
  active: string | null;
  /** Folders shown open in the tree. */
  expanded: Record<string, boolean>;
}

/** The editor of a project: shown or not, its source ('project' or an agent id) and, per source, its tabs. */
export interface EditorState {
  on: boolean;
  source: string;
  places: Record<string, EditorPlace>;
  /** A line to bring into view, the cursor on its column (in characters) when given (`seq` changes for each request). */
  reveal: { path: string; line: number; col?: number; seq: number } | null;
}

class AppState {
  ready = $state(false);
  projects = $state<Project[]>([]);
  agents = $state<Record<string, Agent>>({});
  ui = $state<UiState>({ activeProject: null, view: 'project', selectedAgent: {} });
  settings = $state<Settings>({} as Settings);
  usage = $state<Usage>({ fiveHour: null, sevenDay: null, todayCost: 0, updatedAt: 0 });
  resources = $state<Resources>({ instances: 0, memory: 0, cpu: 0, agents: [] });
  git = $state<Record<string, GitInfo>>({});
  shells = $state<ShellInfo[]>([]);
  terminals = $state<TermInfo[]>([]);
  exitedTerms = $state<Record<string, number | null>>({});
  /**
   * What the step running of each agent's worktree setup wrote, its last lines, by agent. Replaced whole at each
   * change (a few hundred lines, often): not made deeply reactive.
   */
  setupOutput = $state.raw<Record<string, SetupOutput>>({});
  /** Launch commands' latest runs, by command id. */
  launches = $state<Record<string, LaunchState>>({});
  selectedLaunch = $state<Record<string, string | null>>({});
  selectedTerm = $state<Record<string, string | null>>({});
  claudeFound = $state(true);
  /** Claude Code's models as it last reported them: the version each alias runs. */
  models = $state<ModelInfo[]>([]);
  version = $state('');
  filesOpen = $state(false);
  filesScope = $state<'agent' | 'project'>('agent');
  /** Tab of the side panel: uncommitted files or the repository history. */
  panelTab = $state<'files' | 'history'>('files');
  /** Side-by-side diffs (else unified), shared by the diff dialog and the split layout. */
  diffSplit = $state(readPref('diffSplit') === '1');
  showArchived = $state(false);
  modal = $state<Modal | null>(null);
  toasts = $state<Toast[]>([]);
  now = $state(Date.now());
  gitTick = $state(0);
  update = $state<UpdateInfo | null>(null);
  /** When the automatic restart for the update comes (the backend warned of it), if one is planned. */
  restartAt = $state<number | null>(null);
  /** A version that did not install at the last try: told of once it is downloaded again. */
  failedUpdate = $state<string | null>(null);
  focusComposer = $state(0);
  /**
   * Agents that asked a question, finished or failed while not on screen, and that the user
   * has not looked at since: their project tab and their card blink.
   */
  attention = $state<Record<string, true>>({});
  /** Every project's tickets, by id. */
  tickets = $state<Record<string, Ticket>>({});
  /** Projects whose board fills the main area (not saved). */
  board = $state<Record<string, boolean>>({});
  /** Why no ticket of a project's board starts (its target branch has no commit yet, or is gone), by project. */
  boardIssues = $state<Record<string, string>>({});
  /** Why no ticket of any board starts for now (a quota window, a usage limit), as the backend says. */
  autopilotPause = $state<AutopilotPause | null>(null);
  /** The external ticket systems' accounts (their secrets stay in the backend). */
  accounts = $state<AccountView[]>([]);
  editor = $state<Record<string, EditorState>>({});

  project = $derived(this.projects.find((p) => p.id === this.ui.activeProject) ?? null);
  /** The board of the project on screen is shown. */
  boardOn = $derived(this.ui.view === 'project' && !!this.project && !!this.board[this.project.id]);
  /** The editor of the project on screen is open, and on screen: the statistics hide it. */
  editorOn = $derived(this.ui.view === 'project' && !!(this.project && this.editor[this.project.id]?.on));
  split = $derived(this.ui.layout === 'split');
  /** Estimated cost of the turns running now (their exact cost joins `usage.todayCost` at their end). */
  liveCost = $derived(Object.values(this.agents).reduce((sum, a) => sum + (a.liveCost ?? 0), 0));

  projectAgents = $derived.by(() => {
    const p = this.project;
    if (!p) return [] as Agent[];
    return Object.values(this.agents)
      .filter((a) => a.projectId === p.id && !a.archived)
      .sort((a, b) => a.createdAt - b.createdAt);
  });

  archivedAgents = $derived.by(() => {
    const p = this.project;
    if (!p) return [] as Agent[];
    return Object.values(this.agents)
      .filter((a) => a.projectId === p.id && a.archived)
      .sort((a, b) => b.lastActivity - a.lastActivity);
  });

  agent = $derived.by(() => {
    const p = this.project;
    if (!p) return null;
    const sel = this.ui.selectedAgent[p.id];
    const a = sel ? this.agents[sel] : undefined;
    if (a && a.projectId === p.id) return a;
    return this.projectAgents[0] ?? null;
  });

  term = $derived.by(() => {
    const p = this.project;
    if (!p) return null;
    const id = this.selectedTerm[p.id];
    return this.terminals.find((t) => t.id === id) ?? null;
  });

  /** The launch command whose log fills the main area, if any: one of the project's, or a step of an agent's recipe. */
  runCommand = $derived.by(() => {
    const p = this.project;
    if (!p) return null;
    const id = this.selectedLaunch[p.id];
    if (!id) return null;
    return p.runCommands.find((c) => c.id === id) ?? testCommand(id, this.agents, this.shells[0]?.id ?? '');
  });

  /** What else goes when the backend removes an agent: what other modules (the launches' logs) hold of it. */
  private removalHooks = new Set<(agentId: string) => void>();
  /** What goes stale when an agent's launch recipe changes: its test launches, kept by the index of their step. */
  private recipeHooks = new Set<(agentId: string) => void>();
  /** What is over when an agent's ticket goes "Terminé": its test launches. */
  private ticketDoneHooks = new Set<(agentId: string) => void>();
  /** What must be written before the app restarts by itself for an update (the composers' drafts). */
  private restartHooks = new Set<() => void>();

  /** `f` runs when the backend removes an agent, before the state forgets it. Returns what unregisters it. */
  onAgentRemoved(f: (agentId: string) => void) {
    this.removalHooks.add(f);
    return () => void this.removalHooks.delete(f);
  }

  /** `f` runs when an agent the window knows gets another launch recipe. Returns what unregisters it. */
  onRecipeChanged(f: (agentId: string) => void) {
    this.recipeHooks.add(f);
    return () => void this.recipeHooks.delete(f);
  }

  /** `f` runs, with its agent, when a ticket goes "Terminé". Returns what unregisters it. */
  onTicketDone(f: (agentId: string) => void) {
    this.ticketDoneHooks.add(f);
    return () => void this.ticketDoneHooks.delete(f);
  }

  /** A version that did not install: the app restarts no more by itself for it, the user tries again or not. */
  tellFailedUpdate(version: string) {
    // The update's window, open, already says why its restart failed.
    if (this.modal?.kind === 'update') return;
    this.toast(`La mise à jour vers ${version} n’a pas pu s’installer.`, 'error', {
      label: 'Réessayer',
      onClick: () => (this.modal = { kind: 'update' }),
    });
  }

  /** `f` runs when the backend warns of an automatic restart, before the app stops. Returns what unregisters it. */
  onRestartWarned(f: () => void) {
    this.restartHooks.add(f);
    return () => void this.restartHooks.delete(f);
  }

  /** Each hook on its own: one that throws neither stops the others nor what the state does next. */
  private runHooks(hooks: Set<(agentId: string) => void>, agentId: string) {
    for (const f of hooks) {
      try {
        f(agentId);
      } catch (e) {
        console.error(e);
      }
    }
  }

  private uiTimer: ReturnType<typeof setTimeout> | undefined;
  /** The view « Vue d’ensemble » covers, which closing it goes back to (the project's, after a start on it). */
  private overviewFrom = 'project';
  /** Events received while the initial snapshot is in flight (newer than the snapshot). */
  private early: UiEvent[] | null = null;
  private toastId = 0;

  async init() {
    this.early = [];
    const s = await api.subscribe((e) => (this.early ? this.early.push(e) : this.onEvent(e)));
    this.projects = s.projects;
    this.agents = Object.fromEntries(s.agents.map((a) => [a.id, a]));
    this.tickets = Object.fromEntries((s.tickets ?? []).map((t) => [t.id, t]));
    this.boardIssues = { ...s.boardIssues };
    this.setupOutput = { ...s.setupOutput };
    this.autopilotPause = s.autopilotPause ?? null;
    this.accounts = s.accounts ?? [];
    this.attention = {};
    this.ui = { ...s.ui, view: s.ui.view || 'project', selectedAgent: s.ui.selectedAgent ?? {} };
    this.overviewFrom = 'project';
    if (!this.ui.activeProject || !this.projects.some((p) => p.id === this.ui.activeProject)) {
      this.ui.activeProject = this.projects[0]?.id ?? null;
    }
    this.settings = s.settings;
    this.usage = s.usage;
    this.git = s.git;
    this.shells = s.shells;
    this.claudeFound = s.claudeFound;
    this.version = s.version;
    this.models = s.models;
    this.restartAt = s.restartAt ?? null;
    this.failedUpdate = s.failedUpdate ?? null;
    if (s.installed) {
      const { version, notes } = s.installed;
      this.toast(`Escouade ${version} est installée.`, 'ok', {
        label: 'Voir les nouveautés',
        onClick: () => (this.modal = { kind: 'notes', version, notes }),
      });
    }
    const early = this.early;
    this.early = null;
    for (const e of early) this.onEvent(e);
    this.ready = true;
    setInterval(() => (this.now = Date.now()), 1000);
    // The conversations out of sight leave the window's memory after a while, read again when shown.
    setInterval(() => releaseIdle(Date.now(), (id) => this.atWork(id)), 60_000);
  }

  /**
   * An agent working, or waiting for an answer (a pending request always comes with « waiting »): its conversation
   * stays in the window's memory, out of sight too.
   */
  private atWork(id: string): boolean {
    const a = this.agents[id];
    return !!a && (a.status === 'running' || a.status === 'waiting');
  }

  private onEvent(e: UiEvent) {
    switch (e.type) {
      case 'agent': {
        const prev = this.agents[e.agent.id];
        const newRecipe = prev && JSON.stringify(prev.recipe ?? null) !== JSON.stringify(e.agent.recipe ?? null);
        this.agents[e.agent.id] = e.agent;
        this.noteAttention(prev, e.agent);
        if (newRecipe) this.runHooks(this.recipeHooks, e.agent.id);
        // Its setup is over: what it wrote is in its log (and its failure in the conversation).
        if (!e.agent.setup) this.dropSetupOutput(e.agent.id);
        break;
      }
      case 'agentRemoved':
        this.runHooks(this.removalHooks, e.id);
        delete this.agents[e.id];
        delete this.attention[e.id];
        this.dropSetupOutput(e.id);
        dropConversation(e.id);
        this.forgetEditorSource(e.projectId, e.id);
        break;
      case 'ticket': {
        const prev = this.tickets[e.ticket.id];
        this.tickets[e.ticket.id] = e.ticket;
        this.noteTicketDone(prev, e.ticket);
        break;
      }
      case 'ticketRemoved':
        delete this.tickets[e.id];
        break;
      case 'project':
        this.replaceProject(e.project);
        break;
      case 'openUrl':
        // The page a pull request needs (a compare page to finish it): a failure is told, not lost.
        this.run(openUrl(e.url));
        break;
      case 'focusBoard':
        // The click of a ticket's notification.
        this.openBoard(e.projectId);
        break;
      case 'toast':
        this.toast(e.text, 'info');
        break;
      case 'boardIssue':
        if (e.issue) this.boardIssues[e.projectId] = e.issue;
        else delete this.boardIssues[e.projectId];
        break;
      case 'autopilotPause':
        this.autopilotPause = e.pause;
        break;
      case 'setupOutput':
        this.takeSetupOutput(e.agentId, e);
        break;
      case 'conv':
        applyConvOps(e.agentId, e.ops);
        break;
      case 'git':
        this.git[e.projectId] = e.git;
        if (e.projectId === this.ui.activeProject) this.gitTick++;
        break;
      case 'usage':
        this.usage = e.usage;
        break;
      case 'resources':
        this.resources = e.resources;
        break;
      case 'models':
        this.models = e.models;
        break;
      case 'focus':
        this.selectProject(e.projectId);
        if (e.agentId) {
          this.selectAgent(e.agentId);
          // The agent is the point of the notification: show it, not the editor.
          this.closeEditor(e.projectId);
        }
        break;
      case 'terminalExit':
        this.exitedTerms[e.id] = e.code;
        this.onLaunchExit(e.id, e.code);
        break;
      case 'quitRequested':
        this.modal = {
          kind: 'confirm',
          title: 'Quitter Escouade ?',
          body: `${plural(e.unsaved, 'fichier n’est pas enregistré', 'fichiers ne sont pas enregistrés')} dans l’éditeur : leurs modifications seront perdues.`,
          confirm: 'Quitter quand même',
          danger: true,
          onConfirm: () => api.quit(),
        };
        break;
      case 'updateFailed':
        this.failedUpdate = e.version;
        this.tellFailedUpdate(e.version);
        break;
      case 'updateRestart':
        this.restartAt = e.at;
        if (e.at !== null) {
          for (const f of this.restartHooks) {
            try {
              f();
            } catch (err) {
              console.error(err);
            }
          }
        }
        break;
    }
  }

  /**
   * Lines the step `step` of an agent's worktree setup wrote, its `total` so far counting them. A new step, or one
   * starting (no line yet), replaces what the window had; otherwise only the lines it does not have yet are taken (one
   * written while the window opened comes in its first state and again in an event).
   */
  private takeSetupOutput(agentId: string, { step, total, lines }: SetupOutput) {
    const had = this.setupOutput[agentId];
    let next: SetupOutput;
    if (!had || had.step !== step || total === 0) {
      next = { step, total, lines: lines.slice(-SETUP_LINES) };
    } else {
      const fresh = Math.min(total - had.total, lines.length);
      if (fresh <= 0) return;
      next = { step, total, lines: [...had.lines, ...lines.slice(lines.length - fresh)].slice(-SETUP_LINES) };
    }
    this.setupOutput = { ...this.setupOutput, [agentId]: next };
  }

  private dropSetupOutput(agentId: string) {
    if (!(agentId in this.setupOutput)) return;
    const rest = { ...this.setupOutput };
    delete rest[agentId];
    this.setupOutput = rest;
  }

  /** True when the user can see `id`'s conversation: selected, shown, window in front. */
  private onScreen(id: string): boolean {
    const shown = this.ui.view === 'project' && this.agent?.id === id && !this.term && !this.runCommand && !this.editorOn && !this.boardOn;
    return shown && (typeof document === 'undefined' || document.hasFocus());
  }

  /** An agent that comes to ask, finish or fail out of sight needs a look. */
  private noteAttention(prev: Agent | undefined, next: Agent) {
    if (!ALERT.has(next.status) || next.archived) {
      delete this.attention[next.id];
    } else if (
      prev &&
      prev.status !== next.status &&
      !this.onScreen(next.id) &&
      (next.status === 'waiting' || !this.ticketDoing(next.id))
    ) {
      this.attention[next.id] = true;
    }
  }

  /** The agent on screen has been seen (called whenever what is on screen may have changed). */
  markSeen() {
    const id = this.agent?.id;
    if (id && this.attention[id] && this.onScreen(id)) delete this.attention[id];
  }

  /** Agents of `projectId` that need a look. */
  attentionIn(projectId: string): Agent[] {
    return Object.keys(this.attention)
      .map((id) => this.agents[id])
      .filter((a) => a && a.projectId === projectId && !a.archived);
  }

  persistUi() {
    clearTimeout(this.uiTimer);
    this.uiTimer = setTimeout(() => api.setUi($state.snapshot(this.ui)).catch(() => {}), 250);
  }

  applyTheme() {
    applyTheme(this.ui.view === 'project' && this.project ? this.project.color : null);
  }

  selectProject(id: string) {
    this.ui.activeProject = id;
    this.ui.view = 'project';
    this.persistUi();
  }

  openStats() {
    this.ui.view = 'stats';
    this.persistUi();
  }

  /** Shows « Vue d’ensemble » (every agent of every project) in place of the project or the statistics. */
  openOverview() {
    if (this.ui.view === 'overview') return;
    this.overviewFrom = this.ui.view;
    this.ui.view = 'overview';
    this.persistUi();
  }

  /** Goes back from « Vue d’ensemble » to the view it covered. */
  closeOverview() {
    if (this.ui.view !== 'overview') return;
    this.ui.view = this.overviewFrom;
    this.persistUi();
  }

  /** Its button and Ctrl+Shift+A open « Vue d’ensemble », and close it when it is open. */
  toggleOverview() {
    if (this.ui.view === 'overview') this.closeOverview();
    else this.openOverview();
  }

  toggleLayout() {
    this.ui.layout = this.split ? '' : 'split';
    this.persistUi();
  }

  setDiffSplit(on: boolean) {
    this.diffSplit = on;
    writePref('diffSplit', on ? '1' : '0');
  }

  selectAgent(id: string) {
    const a = this.agents[id];
    if (!a) return;
    if (this.ui.activeProject !== a.projectId) this.ui.activeProject = a.projectId;
    this.ui.view = 'project';
    this.ui.selectedAgent[a.projectId] = id;
    this.selectedTerm[a.projectId] = null;
    this.selectedLaunch[a.projectId] = null;
    this.board[a.projectId] = false;
    const ed = this.editor[a.projectId];
    if (ed?.on) {
      ed.source = a.worktree ? a.id : 'project';
      ed.places[ed.source] ??= { open: [], active: null, expanded: {} };
    }
    this.persistUi();
    this.focusComposer++;
  }

  selectTerm(id: string | null) {
    const p = this.project;
    if (!p) return;
    this.selectedTerm[p.id] = id;
    if (id) {
      this.selectedLaunch[p.id] = null;
      this.board[p.id] = false;
      this.closeEditor(p.id);
    }
  }

  selectLaunch(commandId: string | null) {
    const p = this.project;
    if (!p) return;
    this.selectedLaunch[p.id] = commandId;
    if (commandId) {
      this.selectedTerm[p.id] = null;
      this.board[p.id] = false;
      this.closeEditor(p.id);
    }
  }

  /** Shows the board of a project in the main area, in place of the agents, a terminal, a launch log and the editor. */
  openBoard(projectId = this.ui.activeProject) {
    if (!projectId) return;
    this.ui.activeProject = projectId;
    this.ui.view = 'project';
    this.board[projectId] = true;
    this.selectedTerm[projectId] = null;
    this.selectedLaunch[projectId] = null;
    this.closeEditor(projectId);
    this.persistUi();
  }

  closeBoard(projectId = this.ui.activeProject) {
    if (projectId) this.board[projectId] = false;
  }

  /** A project as the backend now has it (its board settings, mostly). */
  replaceProject(p: Project) {
    const i = this.projects.findIndex((x) => x.id === p.id);
    if (i >= 0) this.projects[i] = p;
  }

  /** The ticket of an agent: the one it works on ("En cours", "À tester"), else its last one. */
  ticketOf(agentId: string): Ticket | undefined {
    const mine = Object.values(this.tickets).filter((t) => t.agentId === agentId);
    return mine.find((t) => t.column === 'doing' || t.column === 'review') ?? mine.at(-1);
  }

  /** The agent's ticket is "En cours": the board tells about its turns. */
  ticketDoing(agentId: string): boolean {
    return Object.values(this.tickets).some((t) => t.agentId === agentId && t.column === 'doing');
  }

  /** Tickets "À tester" of a project (the badge of "Kanban"). */
  reviewCount(projectId: string): number {
    return Object.values(this.tickets).filter((t) => t.projectId === projectId && t.column === 'review').length;
  }

  /**
   * Opens the editor of a project on `source`; with a file (`path` from the source's root, or an absolute `abs`), shows
   * it, at `line` and `col` when given (1-based, the column in characters: an emoji is one).
   */
  async openEditor(req: { projectId?: string; source: string; path?: string; abs?: string; line?: number; col?: number }) {
    const projectId = req.projectId ?? this.ui.activeProject;
    if (!projectId) return;
    // The file comes first: one outside the source's folder opens nothing, the editor stays as it was.
    let path = req.path;
    if (!path && req.abs) {
      const t = trees.get(projectId, req.source) ?? (await trees.load(projectId, req.source).catch(() => undefined));
      if (t) {
        const found = this.sourcePath(projectId, req.source, t, req.abs);
        if (found === null) {
          this.toast(`${basename(req.abs)} est en dehors du dossier ${req.source === 'project' ? 'du projet' : 'de cet agent'}.`);
          return;
        }
        path = found;
      }
    }
    this.ui.activeProject = projectId;
    this.ui.view = 'project';
    this.board[projectId] = false;
    this.selectedTerm[projectId] = null;
    this.selectedLaunch[projectId] = null;
    // Always work through the stored `$state` proxies, not the raw objects the `??=` expressions return:
    // a write on a raw object is lost for whoever already read the proxy.
    this.editor[projectId] ??= { on: true, source: req.source, places: {}, reveal: null };
    const st = this.editor[projectId];
    st.on = true;
    st.source = req.source;
    st.places[req.source] ??= { open: [], active: null, expanded: {} };
    const place = st.places[req.source];
    if (path) {
      recentFiles.note(projectId, req.source, path);
      if (!place.open.includes(path)) place.open.push(path);
      place.active = path;
      this.expandEditorDir(projectId, req.source, ancestors(path).at(-1) ?? '');
      if (req.line) {
        const seq = (st.reveal?.seq ?? 0) + 1;
        st.reveal = req.col ? { path, line: req.line, col: req.col, seq } : { path, line: req.line, seq };
      }
    }
    this.persistUi();
  }

  /**
   * `abs` from the root of a source's tree, null when it is outside. The root is taken as git spells it, then as the
   * app does (the project's folder, the agent's worktree): the two differ with 8.3 short names or a junction.
   */
  private sourcePath(projectId: string, source: string, tree: FileTree, abs: string): string | null {
    const folder = source === 'project' ? this.projects.find((p) => p.id === projectId)?.path : this.agents[source]?.worktree?.path;
    for (const root of [tree.root, folder]) {
      if (!root) continue;
      const path = relPath(root, abs);
      if (isAbsPath(path) || path.split('/').includes('..')) continue;
      // Spelled in another case, the tree's file is the same one on Windows and macOS: not a second tab.
      if (tree.files.includes(path)) return path;
      const lower = path.toLowerCase();
      return tree.files.find((f) => f.toLowerCase() === lower) ?? path;
    }
    return null;
  }

  closeEditor(projectId = this.ui.activeProject) {
    const st = projectId ? this.editor[projectId] : undefined;
    if (st) st.on = false;
  }

  closeEditorTab(projectId: string, source: string, path: string) {
    const place = this.editor[projectId]?.places[source];
    if (!place) return;
    place.open = place.open.filter((p) => p !== path);
    if (place.active === path) place.active = place.open.at(-1) ?? null;
  }

  /**
   * `from` (a file, or a folder holding files) renamed `to` in a source: its tabs and open folders follow, the same
   * files with their unsaved changes, and so do the history of the jumps and the files opened last.
   */
  renameEditorPath(projectId: string, source: string, from: string, to: string) {
    buffers.move(projectId, source, from, to);
    navHistory.rename(projectId, source, from, to);
    recentFiles.rename(projectId, source, from, to);
    const st = this.editor[projectId];
    const place = st?.places[source];
    if (!st || !place) return;
    const moved = (p: string) => movedPath(p, from, to) ?? p;
    // A tab left open at the new path (its file was deleted) is the one renamed now.
    place.open = [...new Set(place.open.map(moved))];
    if (place.active) place.active = moved(place.active);
    place.expanded = Object.fromEntries(Object.entries(place.expanded).map(([d, open]) => [moved(d), open]));
    if (st.reveal && st.source === source) st.reveal = { ...st.reveal, path: moved(st.reveal.path) };
  }

  /**
   * `path` (a file, or a folder holding files) deleted from a source: its tabs close, with the changes the user gave
   * up (`discarded`, see `buffers.closePath`). The tab of a file typed in since stays, its text with it.
   */
  closeEditorPath(projectId: string, source: string, path: string, discarded?: ReadonlyMap<string, string>) {
    const left = new Set(buffers.closePath(projectId, source, path, discarded));
    const place = this.editor[projectId]?.places[source];
    if (!place) return;
    const gone = (p: string) => movedPath(p, path, path) !== null && !left.has(p);
    place.open = place.open.filter((p) => !gone(p));
    if (place.active && gone(place.active)) place.active = place.open.at(-1) ?? null;
    place.expanded = Object.fromEntries(Object.entries(place.expanded).filter(([d]) => !gone(d)));
  }

  /** Opened, a folder opens the ones above it too: the tree may show them all on its row. Closed, it closes alone. */
  toggleEditorDir(projectId: string, source: string, dir: string) {
    const place = this.editor[projectId]?.places[source];
    if (!place) return;
    // A new object rather than a key written: the `$state` proxy drops a write of a key Object has (`constructor`).
    if (Object.hasOwn(place.expanded, dir) && place.expanded[dir]) place.expanded = { ...place.expanded, [dir]: false };
    else this.expandEditorDir(projectId, source, dir);
  }

  /** Opens a folder of the tree and the ones above it ('' is the root, always open). */
  expandEditorDir(projectId: string, source: string, dir: string) {
    const place = this.editor[projectId]?.places[source];
    if (!place || !dir) return;
    place.expanded = { ...place.expanded, ...Object.fromEntries([...ancestors(dir), dir].map((d) => [d, true])) };
  }

  collapseEditorDirs(projectId: string, source: string) {
    const place = this.editor[projectId]?.places[source];
    if (place) place.expanded = {};
  }

  /**
   * A ticket that goes "Terminé" ends what other modules hold of its agent's test (its launches), and one done by a
   * merge removing its worktree takes its agent's editor source with it.
   */
  private noteTicketDone(prev: Ticket | undefined, next: Ticket) {
    if (next.column !== 'done' || prev?.column === 'done' || !next.agentId) return;
    this.runHooks(this.ticketDoneHooks, next.agentId);
    const b = this.projects.find((p) => p.id === next.projectId)?.board;
    if (b?.action === 'merge' && b.cleanup) this.forgetEditorSource(next.projectId, next.agentId);
  }

  /** A deleted agent's worktree is no source any more: its files and tabs go, the editor shows the project instead. */
  private forgetEditorSource(projectId: string, agentId: string) {
    buffers.closeSource(projectId, agentId);
    trees.closeSource(projectId, agentId);
    recentFiles.closeSource(projectId, agentId);
    const st = this.editor[projectId];
    if (!st) return;
    delete st.places[agentId];
    if (st.source !== agentId) return;
    st.source = 'project';
    st.places.project ??= { open: [], active: null, expanded: {} };
    // The line was asked for in the worktree's file, not in the project's of the same name.
    st.reveal = null;
  }

  /** Forgets a project the backend removed, with its editor (unsaved files included: the close was confirmed). */
  forgetProject(id: string) {
    this.projects = this.projects.filter((x) => x.id !== id);
    if (this.ui.activeProject === id) this.ui.activeProject = this.projects[0]?.id ?? null;
    delete this.editor[id];
    delete this.board[id];
    delete this.boardIssues[id];
    buffers.closeProject(id);
    trees.closeProject(id);
    recentFiles.closeProject(id);
    fileSearches.closeProject(id);
    this.persistUi();
  }

  /** A launch command's process is up; it may have ended, or been stopped, in the meantime. */
  launchStarted(commandId: string, ptyId: string) {
    const l = this.launches[commandId];
    if (!l || l.stopping) api.termKill(ptyId).catch(() => {});
    if (!l) {
      delete this.exitedTerms[ptyId];
      return;
    }
    l.ptyId = ptyId;
    if (ptyId in this.exitedTerms) this.onLaunchExit(ptyId, this.exitedTerms[ptyId]);
  }

  /** A launch command's process ended: stopped on purpose, finished, or crashed. */
  private onLaunchExit(ptyId: string, code: number | null) {
    const l = Object.values(this.launches).find((x) => x.ptyId === ptyId);
    if (!l) return;
    delete this.exitedTerms[ptyId];
    const status = l.stopping ? 'stopped' : code === 0 ? 'done' : 'crashed';
    Object.assign(l, { status, code, ptyId: null, stopping: false });
    if (status === 'crashed') this.toast(`« ${l.name} » s'est arrêté en erreur (code ${code ?? '?'})`, 'error');
  }

  async newAgent(projectId = this.ui.activeProject) {
    if (!projectId) return;
    try {
      const a = await api.createAgent(projectId);
      this.agents[a.id] = a;
      // Its composer is what a new agent is for: closed first, the editor keeps the source it was on.
      this.closeEditor(projectId);
      this.selectAgent(a.id);
    } catch (e) {
      this.toast(String(e), 'error');
    }
  }

  /** A copy of the agent (« Dupliquer la conversation »), shown at once, like a new agent. */
  async duplicateAgent(id: string) {
    try {
      const a = await api.duplicateAgent(id);
      this.agents[a.id] = a;
      // Its conversation is what the copy is for: the editor gives way to it.
      this.closeEditor(a.projectId);
      this.selectAgent(a.id);
    } catch (e) {
      this.toast(String(e), 'error');
    }
  }

  /** Next agent waiting for an answer or needing a look, across all projects (Ctrl+J). */
  nextWaiting() {
    const list = Object.values(this.agents)
      .filter((a) => !a.archived && (a.status === 'waiting' || this.attention[a.id]))
      .sort((a, b) => a.lastActivity - b.lastActivity);
    if (!list.length) return;
    const cur = this.agent?.id;
    const idx = list.findIndex((a) => a.id === cur);
    const next = list[(idx + 1) % list.length];
    this.selectAgent(next.id);
    // Its question or its end of turn is what Ctrl+J is for: show the conversation, not the editor.
    this.closeEditor(next.projectId);
  }

  toast(text: string, kind: Toast['kind'] = 'info', action?: ToastAction) {
    const id = ++this.toastId;
    this.toasts.push(action ? { id, text, kind, action } : { id, text, kind });
    // One with a button stays long enough to reach it.
    setTimeout(() => this.dismissToast(id), action ? 12_000 : kind === 'error' ? 7000 : 3500);
  }

  dismissToast(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  async run<T>(p: Promise<T>): Promise<T | undefined> {
    try {
      return await p;
    } catch (e) {
      this.toast(String(e), 'error');
      return undefined;
    }
  }
}

export const app = new AppState();
