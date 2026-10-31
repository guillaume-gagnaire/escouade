// Global application state (Svelte 5 runes) fed by the backend event channel.

import { openUrl } from './opener';
import { api } from './ipc';
import { applyConvOps, dropConversation, releaseIdle } from './conversations.svelte';
import { buffers } from './editor/buffers.svelte';
import { navHistory } from './editor/history';
import { recentFiles } from './editor/quick-open';
import { fileSearches } from './editor/search.svelte';
import { ancestors, movedPath } from './editor/tree';
import { trees } from './editor/trees.svelte';
import { bgRow } from './bgtasks';
import type { DocsSpaceId } from './docs';
import { basename, isAbsPath, relPath } from './format';
import { setLang, t } from './i18n';
import { dictation } from './dictation.svelte';
import { conversation } from './conversation.svelte';
import { mcp } from './mcp.svelte';
import { readPref, writePref } from './prefs';
import { errorText, missingAccountOf } from './refusal';
import { parseTestId, parseWorktreeRunId, testCommand, worktreeCommand } from './recipe';
import type { IconName } from './icons';
import type { SettingsTab } from './settings.svelte';
import type { FormFile } from './ticket-files';
import type { FormImage } from './ticket-images';
import { applyTheme } from './theme';
import { workflows } from './workflows.svelte';
import type {
  AccountView,
  Agent,
  AgentStatus,
  AutopilotPause,
  DiscussionPreset,
  DocsConflict,
  FileTree,
  GitInfo,
  LangInfo,
  LaunchState,
  ModelInfo,
  AccountModels,
  ToldModel,
  EngineDecl,
  EngineId,
  Project,
  Resources,
  Settings,
  SetupOutput,
  ShellInfo,
  TermInfo,
  Ticket,
  TicketDraft,
  UiEvent,
  UiState,
  Usage,
  OfferedMemory,
} from './types';

/** What an agent's header, too narrow, no longer shows (ESC-166): « … » offers it in its menu. */
export interface HeaderFolded {
  /** « Fichiers », the figure that opens the side panel. */
  files: boolean;
  /** The layouts, classic and side by side. */
  layout: boolean;
}

/** Where the focus goes back to when a subagent's conversation is left: the element that had it, and the call opened from. */
export interface SubFocus {
  agentId: string;
  call: string;
  el: HTMLElement | null;
  /**
   * The button it was, when it was one of those that open a subagent in the agent's conversation (the stack of dots of its
   * banner, the line of its call, a request it made): `data-open-from`, which the conversation drawn anew finds again.
   */
  opener: string | null;
}

export type Modal =
  /**
   * The window that adds a project, on `path` when it opens on one (read at once). `historyId`: of the folder's closed
   * projects, the one « Rouvrir » was clicked on; it holds only while the path typed is `path`. `projectId`: the assistant
   * opened again on an existing project (Réglages › Projets), to set it up anew.
   */
  | { kind: 'newProject'; path?: string; historyId?: string; projectId?: string }
  /** The question of the first start: the usage statistics. */
  | { kind: 'telemetry' }
  /** The first launch's other question (ESC-190): the user's name for the team; `then`, what waited for it (a share). */
  | { kind: 'teamName'; projectId?: string; then?: () => void }
  /** ESC-255: what Claude Code remembers of a project, offered once to its shared memory, memory by memory. */
  | { kind: 'claudeMemory'; projectId: string; offered: OfferedMemory[] }
  /**
   * On a tab (Claude Code by default) and, for a project's tabs, a project (the one on screen by default),
   * scrolled to a section of it; `resume`: with the draft it had, back from a modal it opened; `suggest`: Claude
   * reads the project at once to suggest its launch commands.
   */
  | { kind: 'settings'; tab?: SettingsTab; projectId?: string; section?: 'launch' | 'sources'; resume?: boolean; suggest?: boolean }
  /**
   * `commit`: what it changed; `refs`: what changes from `from` to `to` (branches, or commits); `wholeProject`: every
   * uncommitted file the project's list shows, worktrees of the agents included.
   */
  | {
      kind: 'diff';
      projectId: string;
      agentId: string | null;
      paths: string[];
      title: string;
      commit?: string;
      refs?: { from: string; to: string };
      wholeProject?: boolean;
      /** The repository `commit` and `refs` are of, in a project of several (ESC-169). */
      repo?: string;
    }
  | {
      kind: 'confirm';
      title: string;
      body: string;
      confirm: string;
      danger?: boolean;
      option?: { label: string; value: boolean };
      /** A third choice, between cancelling and confirming. */
      alt?: { label: string; onClick: () => void | Promise<void> };
      /** Before the title, in amber. */
      icon?: IconName;
      onConfirm: (option: boolean) => void | Promise<void>;
      /** Where cancelling goes back to (no modal by default). */
      onCancel?: () => void;
    }
  | { kind: 'rename'; title: string; value: string; onSubmit: (v: string) => void | Promise<void> }
  /**
   * « Nouvelle discussion » of the Docs tab (ESC-261), its role and subject given by a gesture of the spec (« Mettre la spec
   * à jour », « Écrire une première spec ») with the first message it sends.
   */
  | { kind: 'newDiscussion'; projectId: string; preset?: DiscussionPreset }
  /** « Ajouter un compte… » (a new Claude account, then its sign-in), or « Se connecter… » for the account `accountId`. */
  /** `back`: the settings' tab it came from and goes back to (Comptes Claude by default). */
  | { kind: 'account'; accountId?: string; back?: SettingsTab; engine?: EngineId; reserveFor?: string }
  | { kind: 'testLaunch'; agentId: string }
  | { kind: 'import'; projectId: string }
  /** The update downloaded (`app.update`), which a restart installs; `back`, the settings' tab it was opened from and goes back to. */
  | { kind: 'update'; back?: SettingsTab }
  /** The release notes of the update installed since the last start. */
  | { kind: 'notes'; version: string; notes: string }
  /** « Rechercher dans les conversations » (Ctrl+K). */
  | { kind: 'convSearch' }
  /** « Ouvrir un fichier » (Ctrl+P): the files of `source` ('project' or an agent's id) in a project. */
  | { kind: 'quickOpen'; projectId: string; source: string }
  /**
   * A direct commit of the agent's changes; `agentId` null: of the project's own checkout. `resume`: what it had
   * written, back from a dialog that took its place.
   */
  | { kind: 'commit'; projectId: string; agentId: string | null; resume?: CommitDraft }
  /** « Ouvrir une PR » on an ordinary agent's branch (ESC-64): its title and description proposed, then read and edited. */
  | { kind: 'pullRequest'; agentId: string }
  /** « Merger… » of an agent of several repositories (ESC-169): each repository checked, all or nothing. */
  | { kind: 'mergeRepos'; agentId: string }
  /**
   * « Nouvelle branche » on a project. `start`: where it starts (a branch, or the commit the graph gave), the current
   * branch by default; `resume`: what had been written, back from the question of the stash.
   */
  | { kind: 'newBranch'; projectId: string; start?: string; resume?: NewBranchDraft; repo?: string }
  /** « Branches mergées » : the local branches already in the project's base, to delete together. */
  | { kind: 'mergedBranches'; projectId: string; repo?: string }
  /**
   * The form of a ticket, new or (`ticket`) edited, in a large window. `onSubmit` saves it and tells whether it did (the
   * window closes then, else it stays with what was typed); `resume`: what had been typed, back from a dialog that took
   * its place (the confirmation of its closing).
   */
  | {
      kind: 'ticket';
      projectId: string;
      ticket?: Ticket;
      onSubmit: (d: TicketDraft) => boolean | Promise<boolean>;
      resume?: TicketFormDraft;
    };

/** What a commit's window had written when another dialog took its place: its message, and Haiku's proposal as it came. */
export interface CommitDraft {
  message: string;
  proposed: string;
}

/** What the form of a ticket had in its fields when another dialog took its place (the criteria as the text of their field). */
export interface TicketFormDraft {
  title: string;
  description: string;
  criteria: string;
  maxLoops: number;
  after: string[];
  branch: string;
  /** Its harness (null: the project's); absent: the ticket's. */
  engine?: EngineId | null;
  /** Its account, model and effort (empty: « Auto »); absent: the ticket's. */
  options?: { account: string; model: string; effort: string };
  /** Absent: those of the ticket. */
  images?: FormImage[];
  /** The files of the ticket's images as the window showed them, those gone from its folder left out; absent: the ticket's. */
  held?: string[];
  /**
   * Its other files, the very list of the window (not a copy of it): a copy still being written goes on meanwhile, and the
   * window back shows where it is. Absent: those of the ticket.
   */
  files?: FormFile[];
}

/** What the window of a new branch had written when the question of the stash took its place. */
export interface NewBranchDraft {
  name: string;
  start: string;
  switchTo: boolean;
}

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

/** Its turn is under way: working, or waiting for an answer. */
export const atWork = (a: Pick<Agent, 'status'>) => a.status === 'running' || a.status === 'waiting';

/** How many of the last lines of the step running a worktree's setup the window keeps (as the backend does). */
const SETUP_LINES = 500;

/** The title of the question asked on quitting, in the language of the interface (the open dialog is told by it). */
const quitTitle = () => t('shell.quit.title');

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
  /** Counts the `escouade://` links told (STORE-11): the view Plugins takes them (App.svelte). */
  deepLinks = $state(0);
  projects = $state<Project[]>([]);
  agents = $state<Record<string, Agent>>({});
  ui = $state<UiState>({ activeProject: null, view: 'project', selectedAgent: {} });
  settings = $state<Settings>({} as Settings);
  usage = $state<Usage>({ todayCost: 0, accounts: [], current: {} });
  resources = $state<Resources>({ instances: 0, memory: 0, cpu: 0, agents: [] });
  git = $state<Record<string, GitInfo>>({});
  /** `git config user.name` in each project's folder, by project, once asked (ESC-243). */
  gitNames = $state<Record<string, string>>({});
  shells = $state<ShellInfo[]>([]);
  terminals = $state<TermInfo[]>([]);
  exitedTerms = $state<Record<string, number | null>>({});
  /** How many times the agent's interface was asked to start again from outside its view (« Outils d’Escouade au prochain démarrage »), by agent. */
  relaunched = $state<Record<string, number>>({});
  /** Terminals the window killed itself, whose exit is still to come: nobody reads it. */
  private droppedTerms = new Set<string>();
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
  /** The settings' own path to Claude Code leads to it (what its field's hint says, whatever the current account runs). */
  claudePathFound = $state(true);
  /** Claude Code's models as it last reported them: the version each alias runs. */
  models = $state<ModelInfo[]>([]);
  /** What each harness declares (its capabilities), as the backend gave it at the start. */
  engines = $state.raw<EngineDecl[]>([]);
  /** The models the harnesses that list them told (Codex: `model/list`): the composer's menu of their agents. */
  engineModels = $state.raw<Partial<Record<EngineId, ToldModel[]>>>({});
  /** What the `claude` of each account told of its models: which take the mode `auto` (`modelTakes`). */
  accountModels = $state.raw<Record<string, AccountModels>>({});
  /** The harnesses whose program is found: the only ones the interface offers (Réglages › Harness lists them all). */
  enginesFound = $state.raw<EngineId[]>([]);
  version = $state('');
  filesOpen = $state(false);
  filesScope = $state<'agent' | 'project'>('agent');
  /** Tab of the side panel: uncommitted files, the repository history or the project's workflow (ESC-194). */
  panelTab = $state<'files' | 'history' | 'workflow' | 'hosts'>('files');
  /** A text to put in an agent's composer without sending it (a door of the Workflow tab): taken by its composer, which then clears it. */
  composerPut = $state<{ agentId: string; text: string } | null>(null);
  /** How many messages went back to each agent's draft (`giveBack`): its composer reads the draft again. */
  draftReturned = $state<Record<string, number>>({});
  /** Side-by-side diffs (else unified), shared by the diff dialog and the split layout. */
  diffSplit = $state(readPref('diffSplit') === '1');
  showArchived = $state(false);
  /** The plan banner of an agent's conversation, open or folded, by agent (kept for the session; absent: open if the window is tall enough). */
  planOpen = $state<Record<string, boolean>>({});
  /**
   * The subagent whose conversation fills the main area in place of its agent's, by agent: the id of the call that
   * launched it (ESC-13). Absent or null: the agent's own conversation. Kept for the session.
   */
  subView = $state<Record<string, string | null>>({});
  /**
   * What was typed to a subagent through its agent (« Lui écrire »), by `${agentId}:${call}`: kept when the view is left
   * and after the subagent ended, for the message that was not sent.
   */
  subDrafts = $state<Record<string, string>>({});
  /**
   * Where the focus goes back to when a subagent's conversation is left (not reactive: read once, by the conversation that
   * comes back). `from`: the element that had the focus when the first conversation was opened from the agent's own;
   * `back`: that, once the view was closed by its breadcrumb or Escape (selecting an agent drops it: the composer has the focus then).
   */
  subFocus: { from: SubFocus | null; back: SubFocus | null } = { from: null, back: null };
  /** The list of an agent's subagents under its card, unfolded or folded by the reader, by agent (absent: unfolded while one runs). */
  subsOpen = $state<Record<string, boolean>>({});
  /**
   * The background task whose view fills the main area in place of its agent's conversation, by agent: Claude Code's id of
   * the task (ESC-14). Absent or null: the agent's own conversation. Kept for the session; never at the same time as
   * `subView` (opening one closes the other), and they share the memory of where the focus goes back to (`subFocus`).
   */
  taskView = $state<Record<string, string | null>>({});
  /** The list of an agent's background tasks under its card, unfolded or folded by the reader, by agent (absent: unfolded while one runs). */
  bgOpen = $state<Record<string, boolean>>({});
  modal = $state<Modal | null>(null);
  toasts = $state<Toast[]>([]);
  now = $state(Date.now());
  gitTick = $state(0);
  /** Each project's shared memory changed this many times (ESC-255): its settings' list reads it again. */
  memoryTicks = $state<Record<string, number>>({});
  /** A project's « Index du code » changed (ESC-293): read, erased, waiting for its reading. */
  codeIndexTicks = $state<Record<string, number>>({});
  /** The download of the vectors' model under way (ESC-254): bytes so far, of `total`; null when none is. */
  vectorsProgress = $state<{ done: number; total: number } | null>(null);
  update = $state<UpdateInfo | null>(null);
  /** When the automatic restart for the update comes (the backend warned of it), if one is planned. */
  restartAt = $state<number | null>(null);
  /** A version that did not install at the last try: told of once it is downloaded again. */
  failedUpdate = $state<string | null>(null);
  /** The last download of an update that failed, until one goes through: Réglages › À propos says it (choice 13). */
  updateTrouble = $state<{ version: string; message: string; page: string | null } | null>(null);
  focusComposer = $state(0);
  /** The card of the agents' list to give the focus to (Ctrl+J), `n` counting the asks: the sidebar focuses it. */
  cardFocus = $state<{ id: string; n: number }>({ id: '', n: 0 });
  /** Ctrl+Shift+N: the menu « Harness » of « Nouvel agent » opens (the sidebar watches it). */
  newAgentMenu = $state(0);
  /** « … » in an agent's header: the sidebar opens the agent's menu (its card's right click) at this place. */
  agentMenuAsk = $state<{ id: string; x: number; y: number; seq: number; folded?: HeaderFolded } | null>(null);
  /** Counts the projects and agents chosen, the one on screen again included: its project's tab comes back into view. */
  chosen = $state(0);
  /**
   * Agents that asked a question, finished or failed while not on screen, and that the user
   * has not looked at since: their project tab and their card blink.
   */
  attention = $state<Record<string, true>>({});
  /** Every project's tickets, by id. */
  tickets = $state<Record<string, Ticket>>({});
  /** Projects whose board fills the main area (not saved). */
  board = $state<Record<string, boolean>>({});
  /** Projects whose Docs tab (ESC-258) fills the main area, in place of the agents and the board (not saved). */
  docs = $state<Record<string, boolean>>({});
  /** The space of the Docs tab each project shows: the spec, the decisions or the documents. */
  docsSpace = $state<Record<string, DocsSpaceId>>({});
  /** The page of the spec each project shows; none: the map of the subjects. */
  docsPage = $state<Record<string, string | null>>({});
  /** The discussion (its agent's id) each project's Docs tab shows (ESC-261); none: the space chosen. */
  docsDiscussion = $state<Record<string, string | null>>({});
  /** A page of the spec just made: it opens to be written. */
  docsEdit = $state<string | null>(null);
  /** Why no ticket of a project's board starts (its target branch has no commit yet, or is gone), by project. */
  boardIssues = $state<Record<string, string>>({});
  /** What each project's docs have to share (ESC-257), by project id: the status bar says « N changements à partager ». */
  docsShare = $state<Record<string, { pending: number; conflict: DocsConflict | null }>>({});
  /** The pages of each project being compared to the code (ESC-263). */
  docsComparing = $state<Record<string, string[]>>({});
  /** Why no ticket of any board starts for now (a quota window, a usage limit), as the backend says. */
  autopilotPause = $state<AutopilotPause | null>(null);
  /** The same for the paused projects that prefer an account, by project (they wait for that account alone). */
  projectPauses = $state<Record<string, AutopilotPause>>({});
  /** The external ticket systems' accounts (their secrets stay in the backend). */
  accounts = $state<AccountView[]>([]);
  editor = $state<Record<string, EditorState>>({});
  /** The languages as the backend resolved them from the settings (the system's is named in « Réglages »). */
  lang = $state<LangInfo>({ ui: 'fr', system: 'fr', claude: 'fr' });

  project = $derived(this.projects.find((p) => p.id === this.ui.activeProject) ?? null);
  /** The board of the project on screen is shown. */
  boardOn = $derived(this.ui.view === 'project' && !!this.project && !!this.board[this.project.id]);
  /** The Docs tab of the project on screen is shown (ESC-258). */
  docsOn = $derived(this.ui.view === 'project' && !!this.project && !!this.docs[this.project.id] && !this.board[this.project.id]);
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
    return (
      p.runCommands.find((c) => c.id === id) ??
      worktreeCommand(id, p, this.agents) ??
      testCommand(id, this.agents, this.shells[0]?.id ?? '')
    );
  });

  /** The agent whose conversation is on screen: its project's view, without the board, the editor, a run or a terminal over it. */
  agentShown = $derived(
    this.ui.view === 'project' && !this.term && !this.runCommand && !this.editorOn && !this.boardOn ? this.agent : null,
  );

  /** What else goes when the backend removes an agent: what other modules (the launches' logs) hold of it. */
  private removalHooks = new Set<(agentId: string) => void>();
  /** What goes stale when an agent's launch recipe changes: its test launches, kept by the index of their step. */
  private recipeHooks = new Set<(agentId: string) => void>();
  /** What is over when an agent's ticket goes "Terminé": its test launches. */
  private ticketDoneHooks = new Set<(agentId: string) => void>();
  /** What must be written before the app restarts by itself for an update (the composers' drafts). */
  private restartHooks = new Set<() => void>();
  private turnEndHooks = new Set<(agentId: string) => void>();

  /** `f` runs when the backend removes an agent, before the state forgets it. Returns what unregisters it. */
  onAgentRemoved(f: (agentId: string) => void) {
    this.removalHooks.add(f);
    return () => void this.removalHooks.delete(f);
  }

  /** `f` runs when an agent's turn ends, after the state has its new self. Returns what unregisters it. */
  onTurnEnded(f: (agentId: string) => void) {
    this.turnEndHooks.add(f);
    return () => void this.turnEndHooks.delete(f);
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

  /**
   * The page's process ended while it showed a project (ESC-290): the page came back without it, and offers it again.
   * Never opened by itself, a page that crashes on it would crash again at each load.
   */
  tellPageCrash(crash: { projectId: string | null; agentId: string | null }) {
    const project = crash.projectId ? this.projects.find((p) => p.id === crash.projectId) : undefined;
    if (!project) {
      this.toast(t('shell.pageCrash.other'), 'error');
      return;
    }
    const reopen = () => {
      if (crash.agentId && this.agents[crash.agentId]) this.selectAgent(crash.agentId);
      else this.selectProject(project.id);
    };
    this.toast(
      t('shell.pageCrash.project', { project: project.name }),
      'error',
      { label: t('shell.pageCrash.reopen'), onClick: reopen },
      60_000,
    );
  }

  /** A version that did not install: the app restarts no more by itself for it, the user tries again or not. */
  tellFailedUpdate(version: string) {
    // The update's window, open, already says why its restart failed.
    if (this.modal?.kind === 'update') return;
    this.toast(t('shell.update.failed', { version }), 'error', {
      label: t('common.retry'),
      onClick: () => {
        // Its window shows an update downloaded only: without one, it would be an empty dialog keeping every key.
        if (!this.update?.ready) {
          this.toast(t('shell.update.notReady', { version }));
        } else if (!this.openFromToast({ kind: 'update' })) {
          this.tellFailedUpdate(version);
        }
      },
    });
  }

  /**
   * Opens `modal` from a toast's button, unless a dialog is open: the toasts are above the dialogs, and what is typed in
   * the one open (a commit's message) would be lost. False when it did not, the toast to be shown again for later.
   */
  private openFromToast(modal: Modal): boolean {
    if (this.modal) return false;
    this.modal = modal;
    return true;
  }

  /** Dialogs whose answer is given (a choice running or made, a commit under way or made), by identity. */
  private answered = new WeakSet<Modal>();

  /**
   * `modal` has its answer (`yes`), or has it no more (a commit git refused): once another dialog that took its place
   * is cancelled, an answered one does not come back to be answered twice.
   */
  markAnswered(modal: Modal | null, yes = true) {
    if (!modal) return;
    if (yes) this.answered.add(modal);
    else this.answered.delete(modal);
  }

  /** Whether `modal` comes back once a dialog that took its place is cancelled. */
  private comesBack(modal: Modal): boolean {
    if (this.answered.has(modal)) return false;
    // The update's window shows an update downloaded only: without one, an empty dialog keeping every key.
    return modal.kind !== 'update' || !!this.update?.ready;
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
    // First: what is shown from here on is written in it.
    if (s.lang) this.takeLang(s.lang);
    this.projects = s.projects;
    this.agents = Object.fromEntries(s.agents.map((a) => [a.id, a]));
    this.tickets = Object.fromEntries((s.tickets ?? []).map((t) => [t.id, t]));
    this.boardIssues = { ...s.boardIssues };
    this.setupOutput = { ...s.setupOutput };
    this.autopilotPause = s.autopilotPause ?? null;
    this.projectPauses = s.projectPauses ?? {};
    this.accounts = s.accounts ?? [];
    this.attention = {};
    this.ui = { ...s.ui, view: s.ui.view || 'project', selectedAgent: s.ui.selectedAgent ?? {} };
    this.overviewFrom = 'project';
    if (!this.ui.activeProject || !this.projects.some((p) => p.id === this.ui.activeProject)) {
      // Never the one the page crashed on (ESC-290): it is offered again, not opened.
      this.ui.activeProject = this.projects.find((p) => p.id !== s.pageCrash?.projectId)?.id ?? null;
    }
    this.settings = s.settings;
    this.usage = s.usage;
    this.git = s.git;
    this.shells = s.shells;
    this.claudeFound = s.claudeFound;
    this.claudePathFound = s.claudePathFound ?? s.claudeFound;
    this.version = s.version;
    this.models = s.models;
    this.engines = s.engines ?? [];
    this.engineModels = s.engineModels ?? {};
    this.accountModels = s.accountModels ?? {};
    this.enginesFound = s.enginesFound ?? (s.claudeFound ? ['claude'] : []);
    this.restartAt = s.restartAt ?? null;
    this.failedUpdate = s.failedUpdate ?? null;
    if (s.pageCrash) this.tellPageCrash(s.pageCrash);
    // Not answered yet (closed without a choice: asked again at the next start).
    if (s.settings.telemetry === null) this.modal = { kind: 'telemetry' };
    // The user's name for the shared tickets (ESC-190), asked once, after the statistics' question.
    else if (!s.settings.nameAsked) this.modal = { kind: 'teamName' };
    if (s.installed) {
      const { version, notes } = s.installed;
      const tell = () =>
        this.toast(t('shell.update.installed', { version }), 'ok', {
          label: t('shell.update.seeNotes'),
          onClick: () => {
            if (!this.openFromToast({ kind: 'notes', version, notes })) tell();
          },
        });
      tell();
    }
    const early = this.early;
    this.early = null;
    for (const e of early) this.onEvent(e);
    this.ready = true;
    setInterval(() => (this.now = Date.now()), 1000);
    // The conversations out of sight leave the window's memory after a while, read again when shown.
    setInterval(() => releaseIdle(Date.now(), (id) => this.atWork(id)), 60_000);
  }

  /** The languages the backend resolved: the interface switches to its own at once, without reloading. */
  private takeLang(lang: LangInfo) {
    this.lang = lang;
    setLang(lang.ui);
  }

  /**
   * An agent working, or waiting for an answer (a pending request always comes with « waiting »): its conversation
   * stays in the window's memory, out of sight too.
   */
  private atWork(id: string): boolean {
    const a = this.agents[id];
    return !!a && atWork(a);
  }

  private onEvent(e: UiEvent) {
    switch (e.type) {
      case 'agent': {
        const prev = this.agents[e.agent.id];
        const newRecipe = prev && JSON.stringify(prev.recipe ?? null) !== JSON.stringify(e.agent.recipe ?? null);
        this.agents[e.agent.id] = e.agent;
        // The task it shows is gone from its plan (a new session starts it again): the view has nothing to show.
        const shown = this.taskView[e.agent.id];
        if (shown && !bgRow(e.agent.plan, shown)) this.closeTask(e.agent.id);
        this.noteAttention(prev, e.agent);
        if (newRecipe) this.runHooks(this.recipeHooks, e.agent.id);
        if (prev && atWork(prev) && !atWork(e.agent)) this.runHooks(this.turnEndHooks, e.agent.id);
        // Its setup is over: what it wrote is in its log (and its failure in the conversation).
        if (!e.agent.setup) this.dropSetupOutput(e.agent.id);
        break;
      }
      case 'agentRemoved':
        this.runHooks(this.removalHooks, e.id);
        workflows.drop(e.projectId, e.id);
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
      case 'workflowTrack':
        workflows.take(e.track);
        break;
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
        // For want of an account (ESC-128): with « Ajouter un compte… ».
        if (missingAccountOf(e.text)) this.refused(e.text);
        else this.toast(e.text, 'info');
        break;
      case 'language':
        this.takeLang(e.lang);
        break;
      case 'mcpStatus':
      case 'mcpDeclared':
      case 'mcpActivity':
        mcp.take(e);
        break;
      case 'deepLink':
        this.deepLinks++;
        break;
      case 'voice':
        dictation.take(e.voice);
        conversation.take(e.voice);
        break;
      case 'docsShare':
        this.docsShare[e.projectId] = { pending: e.pending, conflict: e.conflict };
        break;
      case 'docsCompared':
        this.docsComparing[e.projectId] = e.comparing;
        break;
      case 'boardIssue':
        if (e.issue) this.boardIssues[e.projectId] = e.issue;
        else delete this.boardIssues[e.projectId];
        break;
      case 'autopilotPause':
        this.autopilotPause = e.pause;
        this.projectPauses = e.projects ?? {};
        break;
      case 'setupOutput':
        this.takeSetupOutput(e.agentId, e);
        break;
      case 'conv':
        applyConvOps(e.agentId, e.ops);
        break;
      case 'memory':
        this.memoryTicks[e.projectId] = (this.memoryTicks[e.projectId] ?? 0) + 1;
        break;
      case 'codeIndex':
        this.codeIndexTicks[e.projectId] = (this.codeIndexTicks[e.projectId] ?? 0) + 1;
        break;
      case 'vectorsDownload':
        this.vectorsProgress = { done: e.done, total: e.total };
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
      case 'accounts':
        this.settings = { ...this.settings, accounts: e.accounts, engineOrder: e.engineOrder };
        break;
      case 'engineModels':
        this.engineModels = { ...this.engineModels, [e.engine]: e.models };
        break;
      case 'accountModels':
        this.accountModels = { ...this.accountModels, [e.account]: e.models };
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
        if (this.droppedTerms.delete(e.id)) break;
        this.exitedTerms[e.id] = e.code;
        this.onLaunchExit(e.id, e.code);
        break;
      case 'quitRequested': {
        // Cancelled, it goes back to the dialog it takes the place of (a commit's window keeps its message in it), unless
        // that one got its answer meanwhile; asked again while it asks, back to that same one.
        const open = this.modal;
        const again = open?.kind === 'confirm' && open.title === quitTitle();
        this.modal = {
          kind: 'confirm',
          title: quitTitle(),
          body: t('shell.quit.unsaved', { count: e.unsaved }),
          confirm: t('shell.quit.confirm'),
          danger: true,
          onConfirm: () => api.quit(),
          onCancel: again
            ? open.onCancel
            : open
              ? () => {
                  if (this.comesBack(open)) this.modal = open;
                }
              : undefined,
        };
        break;
      }
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
    return this.agentShown?.id === id && (typeof document === 'undefined' || document.hasFocus());
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
  /**
   * The harnesses looked for again, the PATH of today read first (ESC-186): one installed while the app runs is offered
   * without a restart. The window back in front, the menu « Harness » opened. Nothing said when it fails: what was found stays.
   */
  async refreshEngines(): Promise<void> {
    const found = await api.enginesFoundNow().catch(() => null);
    if (found && found.join() !== this.enginesFound.join()) this.enginesFound = found;
  }

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
    this.chosen++;
    this.persistUi();
  }

  openStats() {
    this.ui.view = 'stats';
    this.persistUi();
  }

  /** Shows the view Plugins (PLG-4, choice 213) in place of the project or the statistics. */
  openPlugins() {
    this.ui.view = 'plugins';
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

  /**
   * Opens the menu of the agent `id` under `el` (« … » of its header): the sidebar, which draws it, watches this.
   * `folded`: what the header, too narrow, no longer shows, which the menu then offers (ESC-166).
   */
  askAgentMenu(id: string, el: HTMLElement, folded?: HeaderFolded) {
    const r = el.getBoundingClientRect();
    this.agentMenuAsk = { id, x: r.left, y: r.bottom + 4, seq: (this.agentMenuAsk?.seq ?? 0) + 1, folded };
  }

  toggleLayout() {
    this.ui.layout = this.split ? '' : 'split';
    this.persistUi();
  }

  setDiffSplit(on: boolean) {
    this.diffSplit = on;
    writePref('diffSplit', on ? '1' : '0');
  }

  /**
   * Opens the conversation of the subagent `call` of the agent `agentId` in place of the agent's (and selects the agent).
   * Opened from the agent's own conversation, the element that has the focus now is where it goes back to.
   */
  openSubagent(agentId: string, call: string) {
    const from = this.focusFrom(agentId, call);
    this.selectAgent(agentId);
    if (this.agents[agentId]) {
      this.subView[agentId] = call;
      this.subFocus.from = from;
    }
  }

  /**
   * Where the focus goes back to once the view opened on `call` is left: the element that has it now when no view of the
   * agent was open (the conversation is where it comes from), else the one remembered, whichever view it was.
   */
  private focusFrom(agentId: string, call: string): SubFocus | null {
    if (this.subView[agentId] || this.taskView[agentId]) return this.subFocus.from;
    const active = document.activeElement;
    return {
      agentId,
      call,
      el: active instanceof HTMLElement && active !== document.body ? active : null,
      opener: active instanceof HTMLElement ? (active.dataset.openFrom ?? null) : null,
    };
  }

  /** Back to the agent's own conversation, where the focus goes back to (`takeFocusBack`). */
  closeSubagent(agentId: string) {
    this.subView[agentId] = null;
    this.leaveView(agentId);
  }

  /**
   * A view of `agentId` is left. The memory of the focus is one for the whole window: it is touched only when it is that
   * agent's (`from`: it goes to `back`; `back` left behind is dropped). A view closed out of sight (the task of an agent in the
   * background is gone from its plan) leaves the memory of the view that is on screen as it is.
   */
  private leaveView(agentId: string) {
    const { from, back } = this.subFocus;
    const mine = from?.agentId === agentId;
    if (!mine && back?.agentId !== agentId) return;
    this.subFocus = { from: mine ? null : from, back: mine ? from : null };
  }

  /**
   * Opens the view of the background task `task` (a task of Claude Code, or a workflow) of the agent `agentId` in place of
   * the agent's conversation (and selects the agent), closing the conversation of a subagent if one was open.
   */
  openTask(agentId: string, task: string) {
    const from = this.focusFrom(agentId, task);
    this.selectAgent(agentId);
    if (this.agents[agentId]) {
      this.taskView[agentId] = task;
      this.subFocus.from = from;
    }
  }

  /** Back to the agent's own conversation, where the focus goes back to (`takeFocusBack`). */
  closeTask(agentId: string) {
    this.taskView[agentId] = null;
    this.leaveView(agentId);
  }

  /** Where the focus goes back to, once, when the conversation of `agentId` is drawn again after `closeSubagent`. */
  takeFocusBack(agentId: string): SubFocus | null {
    const { back } = this.subFocus;
    if (back?.agentId !== agentId) return null;
    this.subFocus.back = null;
    return back;
  }

  /** Selecting an agent shows its own conversation, not one of its subagents'. */
  selectAgent(id: string) {
    const a = this.agents[id];
    if (!a) return;
    if (this.subView[id]) this.subView[id] = null;
    if (this.taskView[id]) this.taskView[id] = null;
    this.subFocus = { from: null, back: null };
    if (this.ui.activeProject !== a.projectId) this.ui.activeProject = a.projectId;
    this.ui.view = 'project';
    this.ui.selectedAgent[a.projectId] = id;
    this.selectedTerm[a.projectId] = null;
    this.selectedLaunch[a.projectId] = null;
    this.board[a.projectId] = false;
    this.docs[a.projectId] = false;
    const ed = this.editor[a.projectId];
    if (ed?.on) {
      ed.source = a.worktree ? a.id : 'project';
      ed.places[ed.source] ??= { open: [], active: null, expanded: {} };
    }
    this.persistUi();
    this.focusComposer++;
    this.chosen++;
  }

  selectTerm(id: string | null) {
    const p = this.project;
    if (!p) return;
    this.selectedTerm[p.id] = id;
    if (id) {
      this.selectedLaunch[p.id] = null;
      this.board[p.id] = false;
      this.docs[p.id] = false;
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
      this.docs[p.id] = false;
      this.closeEditor(p.id);
    }
  }

  /** Shows the board of a project in the main area, in place of the agents, a terminal, a launch log and the editor. */
  openBoard(projectId = this.ui.activeProject) {
    if (!projectId) return;
    this.ui.activeProject = projectId;
    this.ui.view = 'project';
    this.board[projectId] = true;
    this.docs[projectId] = false;
    this.selectedTerm[projectId] = null;
    this.selectedLaunch[projectId] = null;
    this.closeEditor(projectId);
    this.persistUi();
  }

  closeBoard(projectId = this.ui.activeProject) {
    if (projectId) this.board[projectId] = false;
  }

  /**
   * Shows the Docs tab of a project (ESC-258), on `space` when given (and on `page` of the spec), in place of the agents,
   * the board, a terminal, a launch log and the editor.
   */
  openDocs(projectId = this.ui.activeProject, space?: DocsSpaceId, page?: string | null) {
    if (!projectId) return;
    this.ui.activeProject = projectId;
    this.ui.view = 'project';
    this.board[projectId] = false;
    this.docs[projectId] = true;
    if (space) this.docsSpace[projectId] = space;
    if (page !== undefined) this.docsPage[projectId] = page;
    // A space chosen: the discussion open leaves the main area.
    if (space) this.docsDiscussion[projectId] = null;
    this.selectedTerm[projectId] = null;
    this.selectedLaunch[projectId] = null;
    this.closeEditor(projectId);
    this.persistUi();
  }

  /** Shows the discussion `agentId` (ESC-261) in its project's Docs tab. */
  openDiscussion(projectId: string, agentId: string) {
    this.openDocs(projectId);
    this.docsDiscussion[projectId] = agentId;
  }

  /** « Nouvelle discussion » (Ctrl+B in the Docs tab). */
  newDiscussion(projectId = this.ui.activeProject, preset?: DiscussionPreset) {
    if (!projectId) return;
    this.modal = { kind: 'newDiscussion', projectId, preset };
  }

  closeDocs(projectId = this.ui.activeProject) {
    if (projectId) this.docs[projectId] = false;
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
      const tree = trees.get(projectId, req.source) ?? (await trees.load(projectId, req.source).catch(() => undefined));
      if (tree) {
        const found = this.sourcePath(projectId, req.source, tree, req.abs);
        if (found === null) {
          this.toast(
            t(req.source === 'project' ? 'shell.openFile.outsideProject' : 'shell.openFile.outsideAgent', { name: basename(req.abs) }),
          );
          return;
        }
        path = found;
      }
    }
    this.ui.activeProject = projectId;
    this.ui.view = 'project';
    this.board[projectId] = false;
    this.docs[projectId] = false;
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
    delete this.docs[id];
    delete this.boardIssues[id];
    buffers.closeProject(id);
    trees.closeProject(id);
    recentFiles.closeProject(id);
    fileSearches.closeProject(id);
    this.persistUi();
  }

  /** A terminal the window kills (its sign-in terminal): its exit is not kept, whether it came already or comes after. */
  dropExit(id: string) {
    if (id in this.exitedTerms) delete this.exitedTerms[id];
    else this.droppedTerms.add(id);
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
    const found = Object.entries(this.launches).find(([, x]) => x.ptyId === ptyId);
    if (!found) return;
    const [id, l] = found;
    delete this.exitedTerms[ptyId];
    const status = l.stopping ? 'stopped' : code === 0 ? 'done' : 'crashed';
    Object.assign(l, { status, code, ptyId: null, stopping: false });
    if (status !== 'crashed') return;
    // « Voir le log »: its log in the main area, in its project.
    const projectId = this.launchProject(id);
    this.toast(
      t('runs.launch.crashed', { name: l.name, code: code ?? '?' }),
      'error',
      projectId ? { label: t('runs.launch.viewLog'), onClick: () => this.showLaunch(projectId, id) } : undefined,
    );
  }

  /** The project a launch runs for: its command's, or that of the agent whose test or worktree it runs in. */
  private launchProject(id: string): string | null {
    const of = parseTestId(id) ?? parseWorktreeRunId(id);
    if (of) return this.agents[of.agentId]?.projectId ?? null;
    return this.projects.find((p) => p.runCommands.some((c) => c.id === id))?.id ?? null;
  }

  /** The log of a launch, in the main area of its project. */
  showLaunch(projectId: string, id: string) {
    if (this.ui.view !== 'project' || this.ui.activeProject !== projectId) this.selectProject(projectId);
    this.selectLaunch(id);
  }

  /**
   * A new agent of the project, on `engine` (« Nouvel agent sur … »), else on the project's harness; `isolated`: in a
   * worktree of its own or not (« Nouvel agent dans un worktree », « sans worktree »), else as the project says; `base`:
   * the branch its worktree starts from (« Nouvel agent depuis une autre branche… », ESC-160), else the project's.
   */
  async newAgent(
    projectId = this.ui.activeProject,
    engine: EngineId | null = null,
    isolated: boolean | null = null,
    base: string | null = null,
  ) {
    if (!projectId) return;
    try {
      const a = await api.createAgent(projectId, null, engine, isolated, base);
      this.agents[a.id] = a;
      // Its composer is what a new agent is for: closed first, the editor keeps the source it was on.
      this.closeEditor(projectId);
      this.selectAgent(a.id);
    } catch (e) {
      this.refused(e);
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
      this.refused(e);
    }
  }

  /**
   * The card of the next agent waiting for an answer or needing a look, across all projects (Ctrl+J): it gets the focus,
   * its question answered there by 1, 2…, and Enter opens it. The conversation open stays: its project's list is shown,
   * not the Kanban, and the project changes when the agent is another one's.
   */
  nextWaiting() {
    const list = Object.values(this.agents)
      .filter((a) => !a.archived && (a.status === 'waiting' || this.attention[a.id]))
      .sort((a, b) => a.lastActivity - b.lastActivity);
    if (!list.length) return;
    const cur = list.some((a) => a.id === this.cardFocus.id) ? this.cardFocus.id : this.agent?.id;
    const idx = list.findIndex((a) => a.id === cur);
    const next = list[(idx + 1) % list.length];
    if (this.ui.view !== 'project' || this.ui.activeProject !== next.projectId) this.selectProject(next.projectId);
    this.board[next.projectId] = false;
    this.docs[next.projectId] = false;
    this.cardFocus = { id: next.id, n: this.cardFocus.n + 1 };
  }

  /** `ms` is for a toast with more to read and do than the usual time allows; without it, the time follows the toast's kind and button. */
  toast(text: string, kind: Toast['kind'] = 'info', action?: ToastAction, ms?: number) {
    const id = ++this.toastId;
    this.toasts.push(action ? { id, text, kind, action } : { id, text, kind });
    // One with a button stays long enough to reach it.
    setTimeout(() => this.dismissToast(id), ms ?? (action ? 12_000 : kind === 'error' ? 7000 : 3500));
  }

  dismissToast(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  async run<T>(p: Promise<T>): Promise<T | undefined> {
    try {
      return await p;
    } catch (e) {
      this.refused(e);
      return undefined;
    }
  }

  /** A refusal told by a toast; one for want of an account of a harness (ESC-128) with « Ajouter un compte… » on it. */
  refused(e: unknown) {
    const engine = missingAccountOf(e);
    if (!engine) return this.toast(errorText(e), 'error');
    this.toast(errorText(e), 'error', { label: t('accounts.tab.add'), onClick: () => (this.modal = { kind: 'account', engine }) });
  }
}

export const app = new AppState();
