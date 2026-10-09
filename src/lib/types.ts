// Mirrors the Rust types in src-tauri/src/model.rs.

export type AgentStatus = 'idle' | 'running' | 'waiting' | 'done' | 'error';

export interface Settings {
  claudePath: string;
  defaultModel: string;
  defaultEffort: string;
  defaultMode: string;
  sound: boolean;
  osNotifications: boolean;
  /** « Me prévenir pour »: what the chime and the system notifications are for. */
  notifyFor: NotifyFor;
  idleStopMinutes: number;
  pwshPath: string;
  bashPath: string;
  wslDistro: string;
  proxyUrl: string;
  noProxy: string;
  proxyTerminals: boolean;
  /** TLS certificates are not checked (a proxy with a certificate of its own). */
  insecureTls: boolean;
  /** Send "continue" by itself to an agent stopped by the usage limit, once the quota resets. */
  autoResume: boolean;
  /** « Pause au-delà du quota », in percent (80, 90, 95 or 100): no ticket of any board starts while the 5-hour or the weekly window is used this much. */
  quotaPause: number;
  /** « Installer les mises à jour automatiquement »: the app restarts by itself for an update once at rest. */
  autoUpdate: boolean;
  /** What the external ticket systems' links do (their accounts are kept apart, with their secrets). */
  integrations: IntegrationSettings;
}

/**
 * Which kinds of event chime and show a system notification. One switched off does neither; the agent's tab and card
 * still blink, and the taskbar (the Dock) still signals it.
 */
export interface NotifyFor {
  /** « Questions et autorisations ». */
  questions: boolean;
  /** « Tâches terminées ». */
  done: boolean;
  /** « Erreurs ». */
  errors: boolean;
  /** « Tickets (prêt à tester, bloqué) ». */
  tickets: boolean;
}

/** The sync with the external ticket systems and their automatic import, for every project. */
export interface IntegrationSettings {
  /** « Mettre à jour le statut externe ». */
  syncStates: boolean;
  /** « Publier un résumé à chaque boucle ». */
  loopComments: boolean;
  /** « Extraire les critères d'acceptation ». */
  extractCriteria: boolean;
  /** « Importer les tickets étiquetés ». */
  autoImport: boolean;
  importLabel: string;
  /** Minutes between two automatic imports. */
  importEvery: number;
}

/** An external ticket system. */
export type Service = 'jira' | 'trello' | 'github';

/** A Jira status, a Trello list, or for GitHub "open", "closed" or "label:<name>". */
export interface ExternalState {
  id: string;
  name: string;
}

/** A project's source of tickets: a Jira project, a Trello board, a GitHub repository. */
export interface SourceLink {
  service: Service;
  /** The Jira project's key, the Trello board's id, owner/repo. */
  container: string;
  name: string;
  /** The state an imported ticket's external one gets when it comes into each column (none: left as it is). */
  states: Partial<Record<Column, ExternalState>>;
}

export interface ProjectIntegrations {
  /** One per service at most. */
  links: SourceLink[];
  /** The columns whose arrival writes a comment on the external ticket. */
  comments: Column[];
  /** Every external ticket imported into the project (« service|id »): the backend's own, kept when the links are saved. */
  imported?: string[];
}

/** Where an imported ticket comes from. */
export interface ExternalRef {
  service: Service;
  id: string;
  /** As shown: ATL-1287, #142, #42. */
  key: string;
  container: string;
  url: string;
  /** Why its last sync failed, until one succeeds. */
  error: string | null;
}

export interface AccountView {
  service: Service;
  connected: boolean;
  /** « ada@atlas.dev · atlas.atlassian.net », « @ada ». */
  label: string;
}

/** The form of « Connecter… »: the fields its service asks for. */
export interface AccountForm {
  site: string;
  email: string;
  key: string;
  token: string;
}

/** A Jira project, a Trello board, a GitHub repository. */
export interface Container {
  id: string;
  name: string;
}

export interface StatesView {
  states: ExternalState[];
  defaults: Partial<Record<Column, ExternalState>>;
}

export interface IssueFilter {
  id: string;
  label: string;
}

/** An external ticket as the import lists it. */
export interface ExternalIssue {
  service: Service;
  id: string;
  key: string;
  title: string;
  kind: string;
  meta: string[];
  url: string;
  description: string;
  criteria: string[];
  container: string;
  /** Already on the project's Kanban. */
  imported: boolean;
}

export interface IssuePage {
  issues: ExternalIssue[];
  filters: IssueFilter[];
}

export interface Project {
  id: string;
  name: string;
  path: string;
  color: string;
  worktreePerAgent: boolean;
  createdAt: number;
  /** Commands that launch the project, each in its own read-only terminal. */
  runCommands: RunCommand[];
  /** The board: what validating a ticket does, its agents, its tickets' key. */
  board: BoardSettings;
  /** Untracked files of the project copied into every new worktree (glob patterns). */
  worktreeCopy: string[];
  /** Run in every new worktree, one after the other, before its agent's first message. */
  worktreeSetup: WorktreeStep[];
  /** Run in a worktree before the app removes it. */
  worktreeTeardown: WorktreeStep[];
  /** The external ticket systems its tickets come from, and what moving them does there. */
  integrations: ProjectIntegrations;
}

export interface RunCommand {
  id: string;
  name: string;
  command: string;
  /** Shell id: pwsh, powershell, bash, wsl. */
  shell: string;
  /** Folder relative to the project's, empty for the project itself. */
  cwd: string;
}

/** A newer release found by the backend, kept there for its install. */
export interface FoundRelease {
  id: number;
  version: string;
  notes: string;
}

/** A command run in a worktree once it is made (setup) or before it is removed (teardown). */
export interface WorktreeStep {
  id: string;
  command: string;
  /** Shell id; the system's default one when not found. */
  shell: string;
  /** Folder relative to the worktree's, empty for the worktree itself. */
  cwd: string;
}

/** The setup and teardown Claude suggests for a project's worktrees. */
export interface WorktreeSuggestion {
  setup: WorktreeStep[];
  teardown: WorktreeStep[];
}

/** A service isola runs for a worktree. */
export interface IsolaService {
  name: string;
  /** "running", "stopped"…, as isola says it. */
  status: string;
  /** Its address through isola's proxy, else on its port; empty for a background process. */
  url: string;
  /** What tells it is up: its own port (the proxy answers before it does), else `url`. */
  probe: string;
}

export type Column = 'todo' | 'doing' | 'review' | 'done';

export interface Criterion {
  text: string;
  ok: boolean;
  note: string;
}

export interface Ticket {
  id: string;
  projectId: string;
  key: string;
  title: string;
  description: string;
  criteria: Criterion[];
  /** The features in place so far, as its agent last listed them (3 to 8 short lines). */
  progress: string[];
  maxLoops: number;
  column: Column;
  /** Order in "À faire": the lowest first. */
  rank: number;
  agentId: string | null;
  /** n of "Boucle n/max". */
  iteration: number;
  /** Every loop its agent began, rounds sent back included; 0 for a ticket saved before they were counted. */
  loops: number;
  partial: boolean;
  blocked: string | null;
  /** The block is a merge conflict: "L'agent résout" and "Annuler". */
  conflict: boolean;
  /** The validation step running ("Tests…"…). */
  step: string | null;
  outcome: string | null;
  outcomeUrl: string | null;
  forced: boolean;
  reminded: boolean;
  cost: number;
  createdAt: number;
  startedAt: number | null;
  reviewAt: number | null;
  doneAt: number | null;
  /** Imported from an external ticket system: the ticket there, kept in step. */
  external: ExternalRef | null;
}

export type BoardAction = 'merge' | 'pr' | 'push' | 'keep';

export interface BoardSettings {
  action: BoardAction;
  target: string;
  strategy: 'merge' | 'squash' | 'rebase';
  draft: boolean;
  testsFirst: boolean;
  testCommand: string;
  cleanup: boolean;
  conventional: boolean;
  conflict: 'ask' | 'agent' | 'abort';
  maxParallel: number;
  model: string;
  effort: string;
  mode: string;
  autopilot: boolean;
  prefix: string;
  nextNumber: number;
}

export interface TicketDraft {
  title: string;
  description: string;
  criteria: string[];
  maxLoops: number;
}

export interface RecipeStep {
  command: string;
  /** Relative to the worktree. */
  dir: string;
}

export interface RecipeProcess {
  name: string;
  command: string;
  dir: string;
  env: Record<string, string>;
  /** Answers over HTTP once the process is ready (not waited for when empty). */
  url: string;
}

export interface TestRecipe {
  prepare: RecipeStep[];
  processes: RecipeProcess[];
  /** The address that shows the feature itself; else the first process's url. */
  open: string;
}

export type LaunchStatus = 'running' | 'stopped' | 'done' | 'crashed';

/** A launch command's latest run (none before its first launch). */
export interface LaunchState {
  status: LaunchStatus;
  /** Terminal of the running process. */
  ptyId: string | null;
  name: string;
  /** Stopped on purpose: its exit is not a crash. */
  stopping: boolean;
  code: number | null;
  startedAt: number;
}

export interface Worktree {
  path: string;
  branch: string;
  baseBranch: string;
}

export interface Agent {
  id: string;
  projectId: string;
  name: string;
  named: boolean;
  model: string;
  effort: string;
  mode: string;
  sessionId: string | null;
  cwd: string;
  worktree: Worktree | null;
  createdAt: number;
  archived: boolean;
  status: AgentStatus;
  tokens: number;
  cost: number;
  activeMs: number;
  touchedFiles: string[];
  lastActivity: number;
  prompts: number;
  activeSince: number | null;
  alive: boolean;
  pending: string[];
  contextTokens: number;
  /** Size of the context window of the conversation's model (0 until a turn told it). */
  contextWindow: number;
  /** Tokens of the running turn so far; `tokens` includes them once the turn ends. */
  liveTokens: number;
  /** Estimated cost (list prices) of the running turn so far; `cost` gets the exact figure at its end. */
  liveCost: number;
  /** Remote Control: reachable from claude.ai / the Claude app (its process stays up). */
  remoteControl: boolean;
  remoteSession: string | null;
  /** The session on claude.ai. */
  remoteUrl: string | null;
  /** Link state reported by Claude Code ("ready", "connected"…), null without a live link. */
  remoteState: string | null;
  /** Stopped by the usage limit: when it is sent "continue" by itself (epoch ms). */
  resumeAt: number | null;
  /** The board's ticket it works on. */
  ticketId: string | null;
  /** The ticket's protocol, appended to Claude Code's system prompt. */
  appendPrompt: string | null;
  /** First of the 10 ports reserved for its test launches. */
  portBase: number | null;
  /** How to launch its worktree for a test, as it last wrote it. */
  recipe: TestRecipe | null;
  /** What it is doing right now ("Lit src/db.ts", "Lance npm test"…), during a turn. */
  activity: string | null;
  /** The setup of its new worktree under way: the step running ("1/2 · npm ci"). */
  setup: string | null;
  /** isola runs its worktree's services: its test launch goes through it, with no recipe. */
  isola: boolean;
}

/** A choice of Claude Code's model picker: an alias or a full id, and the model it stands for. */
export interface ModelInfo {
  value: string;
  /** Full model id, e.g. "claude-sonnet-5-5". */
  resolvedModel: string;
}

export interface UiState {
  activeProject: string | null;
  view: string;
  selectedAgent: Record<string, string>;
  /** 'split': conversation on the left half, uncommitted files and their diff on the right. */
  layout?: '' | 'split';
}

export interface GitInfo {
  isRepo: boolean;
  /** "(detached)" for a detached HEAD. */
  branch: string;
  /** The remote branch it tracks ("origin/main"), null when it tracks none. */
  upstream: string | null;
  /** The upstream no longer exists on the remote (deleted, e.g. once merged). */
  upstreamGone: boolean;
  /** Commits to push / to pull, against the upstream as last fetched. */
  ahead: number;
  behind: number;
  hasRemote: boolean;
  /** When the repository was last fetched (ms epoch). */
  lastFetch: number | null;
  modified: number;
  added: number;
  deleted: number;
  total: number;
  agents: Record<string, number>;
}

export interface Commit {
  hash: string;
  parents: string[];
  author: string;
  /** Author date, Unix seconds. */
  time: number;
  /** Branches and tags pointing at it ("HEAD", "main", "origin/main", "tag: v1.0"). */
  refs: string[];
  subject: string;
}

export interface GitLog {
  commits: Commit[];
  /** The branch the agent works on. */
  head: string | null;
}

export interface FileChange {
  path: string;
  status: 'M' | 'A' | 'D';
  add: number;
  del: number;
  agentId: string | null;
  /** Listed from the worktree of `agentId` rather than the project's repository. */
  inWorktree: boolean;
}

/** The unified diff of the listed files that one agent owns in one checkout (null: no agent). */
export interface OwnedDiff {
  agentId: string | null;
  inWorktree: boolean;
  diff: string;
}

/** The files of an editor source, relative to `root`. */
export interface FileTree {
  root: string;
  files: string[];
  truncated: boolean;
}

/** A file as the editor reads it: its text with LF line endings, and how to write it back. */
export interface FileText {
  kind: 'text' | 'binary' | 'tooLarge';
  text: string | null;
  size: number;
  hash: string;
  eol: 'lf' | 'crlf';
  bom: boolean;
}

/** The version a file is compared with ("HEAD", or the branch a worktree left). */
export interface FileBase {
  reference: string;
  text: string | null;
}

export interface RateWindow {
  pct: number;
  resetsAt: number | null;
}

export interface Usage {
  fiveHour: RateWindow | null;
  sevenDay: RateWindow | null;
  todayCost: number;
  updatedAt: number;
}

/** Why no ticket of any board starts, and until when: a quota window over « Pause au-delà du quota », or a usage limit with no resume planned. */
export interface AutopilotPause {
  reason: 'fiveHour' | 'week' | 'limit';
  /** The window's use, 0-100 (null after a limit). */
  pct: number | null;
  /** When the tickets start again: the window's end, or about then after a limit. */
  until: number;
}

/** What the running Claude processes use (each with what it started), per agent and in all. */
export interface Resources {
  instances: number;
  /** Bytes. */
  memory: number;
  /** Share of the whole machine, in percent. */
  cpu: number;
  agents: { id: string; memory: number; cpu: number }[];
}

export interface ShellInfo {
  id: string;
  label: string;
  path: string;
}

export interface TermInfo {
  id: string;
  projectId: string;
  name: string;
  shell: string;
}

export interface PatchHunk {
  oldStart: number;
  newStart: number;
  lines: string[];
}

export interface ToolResult {
  text?: string;
  isError: boolean;
  add?: number;
  del?: number;
  patch?: PatchHunk[];
  filePath?: string;
}

export interface QuestionOption {
  label: string;
  description?: string;
  preview?: string;
}

export interface Question {
  question: string;
  header?: string;
  options: QuestionOption[];
  multiSelect?: boolean;
}

interface Base {
  id: string;
  parent?: string | null;
}

export interface UserItem extends Base {
  kind: 'user';
  text: string;
  images: number;
  /** Names of the other attached files (PDF, text); absent from older logs. */
  files?: string[];
  ts: number;
  queued: boolean;
  /** "remote": sent from claude.ai / the Claude app (Remote Control). */
  origin?: 'remote';
}
export interface TextItem extends Base {
  kind: 'text';
  text: string;
  streaming: boolean;
}
export interface ThinkingItem extends Base {
  kind: 'thinking';
  text: string;
  streaming: boolean;
}
export interface ToolItem extends Base {
  kind: 'tool';
  name: string;
  input: Record<string, any>;
  status: 'running' | 'ok' | 'error' | 'interrupted';
  result?: ToolResult;
  ts: number;
}
export interface QuestionItem extends Base {
  kind: 'question';
  toolUseId: string;
  questions: Question[];
  answers: Record<string, string> | null;
  cancelled?: boolean;
  ts: number;
}
export interface PermissionItem extends Base {
  kind: 'permission';
  toolUseId: string;
  toolName: string;
  title?: string | null;
  description?: string | null;
  input: Record<string, any>;
  reason?: string | null;
  canAlways: boolean;
  defaultNo: boolean;
  decision: 'allow' | 'always' | 'deny' | null;
  message?: string | null;
  cancelled?: boolean;
  ts: number;
}
export interface TurnItem extends Base {
  kind: 'turn';
  ts: number;
  durationMs: number | null;
  cost: number;
  tokens: number;
  isError: boolean;
  interrupted: boolean;
  error: string | null;
}
export interface NoticeItem extends Base {
  kind: 'notice';
  ts: number;
  level: 'info' | 'warn' | 'error';
  text: string;
}

/** Passed on to Claude by Claude Code itself: a background task that ended, a subagent's message. */
export interface EventItem extends Base {
  kind: 'event';
  /** 'task' (a background task), 'agent' (a subagent or another session), else the origin Claude Code gave. */
  source: string;
  /** The subagent that sent it. */
  from?: string;
  text: string;
  ts: number;
}

export type ConvItem = UserItem | TextItem | ThinkingItem | ToolItem | QuestionItem | PermissionItem | TurnItem | NoticeItem | EventItem;

export type ConvOp =
  | { op: 'append'; item: ConvItem }
  | { op: 'patch'; id: string; patch: Record<string, unknown> }
  | { op: 'delta'; id: string; text: string };

/** A message of a conversation that matches a search (Ctrl+K). */
export interface ConvHit {
  agentId: string;
  projectId: string;
  agentName: string;
  archived: boolean;
  /** Its rank among the conversation's items. */
  eventIndex: number;
  itemId: string;
  /** The text around the match, on one line. */
  snippet: string;
  /** Where the match is in `snippet` (string indices). */
  mark: [number, number];
  /** When it was written (epoch ms). */
  at: number;
}

export interface ConvSearchResult {
  /** Agent by agent, the most recently active first; in each, the newest messages first. */
  hits: ConvHit[];
  /** More messages match than the results given (200). */
  capped: boolean;
  /** Stopped after 5 s, before the end of the conversations. */
  timedOut: boolean;
}

export type UiEvent =
  | { type: 'agent'; agent: Agent }
  | { type: 'agentRemoved'; id: string; projectId: string }
  | { type: 'conv'; agentId: string; ops: ConvOp[] }
  | { type: 'git'; projectId: string; git: GitInfo }
  | { type: 'usage'; usage: Usage }
  | { type: 'focus'; projectId: string; agentId: string | null }
  | { type: 'terminalExit'; id: string; code: number | null }
  | { type: 'resources'; resources: Resources }
  | { type: 'models'; models: ModelInfo[] }
  | { type: 'quitRequested'; unsaved: number }
  /** The automatic restart for the update downloaded: when it comes, or null once called off. */
  | { type: 'updateRestart'; at: number | null }
  /** The update downloaded did not install, the app still running: no automatic restart for it any more. */
  | { type: 'updateFailed'; version: string }
  | { type: 'ticket'; ticket: Ticket }
  | { type: 'ticketRemoved'; id: string; projectId: string }
  | { type: 'project'; project: Project }
  | { type: 'openUrl'; url: string }
  | { type: 'focusBoard'; projectId: string }
  /** Why no ticket of the project's board starts (its target branch), or null once they may. */
  | { type: 'boardIssue'; projectId: string; issue: string | null }
  /** Why no ticket of any board starts for now, or null once they may. */
  | { type: 'autopilotPause'; pause: AutopilotPause | null }
  | { type: 'toast'; text: string };

export interface InitialState {
  projects: Project[];
  tickets: Ticket[];
  agents: Agent[];
  ui: UiState;
  settings: Settings;
  usage: Usage;
  git: Record<string, GitInfo>;
  shells: ShellInfo[];
  terminals: TermInfo[];
  claudeFound: boolean;
  version: string;
  /** Claude Code's models as it last reported them, empty until a process has started. */
  models: ModelInfo[];
  /** Why no ticket of a project's board starts, by project (none: they may). */
  boardIssues?: Record<string, string>;
  /** Why no ticket of any board starts for now (none: they may). */
  autopilotPause?: AutopilotPause | null;
  /** The external ticket systems' accounts. */
  accounts?: AccountView[];
  /** The update installed since the app's last start, told once. */
  installed?: InstalledUpdate | null;
  /** When the automatic restart planned for an update comes. */
  restartAt?: number | null;
  /** A version that did not install at the last try. */
  failedUpdate?: string | null;
}

/** An update installed: its version and its release notes (markdown). */
export interface InstalledUpdate {
  version: string;
  notes: string;
}

export interface Bucket {
  label: string;
  start: number;
  input: number;
  cache: number;
  output: number;
  cost: number;
  prompts: number;
}

export interface Share {
  key: string;
  tokens: number;
  cost: number;
}

export interface StatsView {
  range: string;
  buckets: Bucket[];
  tokens: number;
  tokensPrev: number;
  cost: number;
  costAll: number;
  firstTs: number | null;
  prompts: number;
  byProject: Share[];
  byModel: Share[];
}

export interface FolderInfo {
  exists: boolean;
  isRepo: boolean;
  branch: string;
  dirty: number;
  name: string;
}

export interface SlashCommand {
  name: string;
  description: string;
  argumentHint?: string;
}

/** A file attached to a message: `data` is base64 for images and PDFs, the text itself for
 * text files (`text/plain`). */
export interface Attachment {
  name: string;
  mediaType: string;
  data: string;
}
