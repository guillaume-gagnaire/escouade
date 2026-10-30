// Mirrors the Rust types in src-tauri/src/model.rs.

import type { ThemeChoice } from './appearance';
import type { Lang } from './i18n/locale.svelte';

export type AgentStatus = 'idle' | 'running' | 'waiting' | 'done' | 'error';

export interface Settings {
  claudePath: string;
  sound: boolean;
  osNotifications: boolean;
  /** « Me prévenir pour »: what the chime and the system notifications are for. */
  notifyFor: NotifyFor;
  idleStopMinutes: number;
  /** « Taille maximale d'un fichier joint », in MB: 0 = no limit. */
  attachMaxMb: number;
  /** « Avertir au-delà de », in MB: a file this big is attached with a warning; 0 = never. */
  attachWarnMb: number;
  /** « Joindre le fichier d'origine »: a file dropped is attached where it is, not copied (when its path is known). */
  attachOrigin: boolean;
  /** « Texte de plus de 256 Ko »: sent by its path like any file, or refused. */
  attachBigText: 'path' | 'refuse';
  /** « Purger les copies après », in days: 0 = kept as long as their owner (agent, project, ticket). */
  attachKeepDays: number;
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
  /** « Reprendre sur un autre compte un agent de ticket arrêté par la limite »: such an agent goes on by itself on another usable account, rather than wait for its own to reset. */
  switchOnLimit: boolean;
  /** « Continuer sur le harness suivant quand tous les comptes d’un harness sont épuisés »: an agent whose harness is spent for its project goes on by itself on the next fallback harness (a handover). Off by default. */
  handoverOnLimit: boolean;
  /** « Modèle de passation », by harness: the model an agent handed over to it runs on; absent: its best (the first of its menu). */
  handoverModels: Partial<Record<EngineId, string>>;
  /** The models marked as favorites, by harness, in the order they were marked: first in the composer's menu. */
  favoriteModels?: Partial<Record<EngineId, string[]>>;
  /** « Pause au-delà du quota », in percent (80, 90, 95 or 100): no ticket of any board starts while the 5-hour or the weekly window is used this much. */
  quotaPause: number;
  /** « Changer de compte à » (ESC-148): 80, 90, 95, or 100 for « À la limite » (nothing moves before the limit). */
  switchAt: number;
  /** « Installer les mises à jour automatiquement »: the app restarts by itself for an update once at rest. */
  autoUpdate: boolean;
  /** « Statistiques d’usage »: null until the user answered the question of the first start. */
  telemetry: boolean | null;
  /** What the external ticket systems' links do (their accounts are kept apart, with their secrets). */
  integrations: IntegrationSettings;
  /** « Langue de l’interface »: the system's (French when it is, English otherwise), or a language of its own. */
  language: 'system' | Lang;
  /** « Langue des textes rédigés par Claude »: the interface's, or a language of its own. */
  claudeLanguage: 'ui' | Lang;
  /** « Modèle des textes rédigés »: the harness the texts written by a model go to; null, or one not found: Claude Code
   * when it is found (its Haiku), else the first found (`writerOf`). */
  writerEngine: EngineId | null;
  /** The model of the texts, for a harness that takes the one Escouade chooses (OpenCode: `<provider>/<model>`); empty: its default one. */
  writerModel?: string;
  /** The accounts of every harness, in the order new agents try them; Principal (id "principal") always among them. */
  accounts: Account[];
  /**
   * « Ordre des harness »: every harness once. It gives a project its fallback harnesses after its default one, and the order
   * of the groups of « Comptes ». Changed by `engine_order_set`, not by the settings modal's draft.
   */
  engineOrder?: EngineId[];
  /** « Les agents peuvent piloter Escouade »: the MCP server runs, given to the agents and declared in each active account of every harness installed. Changed by `mcp_set_enabled`, not by the settings modal's draft. */
  mcpEnabled: boolean;
  /** The port of the MCP server on 127.0.0.1 (0 until its first start): the backend's own. */
  mcpPort: number;
  /** « Les agents tiennent une liste de tâches »: Claude Code's task list tools are on for the agents that start, so their progress can be shown (`Agent.plan`). */
  todoTools: boolean;
  /** « Mettre à jour Claude Code automatiquement »: Escouade runs `claude update` at its start then once a day, apart from its agents, each `claude` it launches having its own update off (ESC-133). */
  claudeSelfUpdate: boolean;
  /** « Ouvrir les nouveaux agents dans l'interface de Claude Code »: a new agent (not a ticket's) starts in the Claude Code view. */
  terminalView: boolean;
  /** « Remote control pour tous les agents »: every agent of every project is reachable from claude.ai and the Claude app, unless set apart by hand (`Agent.remoteOff`). */
  remoteControlAll: boolean;
  /** « Nouveaux projets privés »: a project added is private (`Project.private`) from the start. */
  privateProjects: boolean;
  /** « Index du code sur tous les projets » (ESC-293): the index of every project that made no choice of its own (`Project.codeIndexChoice`) is on. Changed by `set_code_index_all`, not by the settings modal's draft. Absent from the settings of the window's tests made before it. */
  codeIndexAll?: boolean;
  /** « Thème »: Système (the system's, and its changes), Clair or Sombre; applied at once, the window with it (lib/appearance.ts). */
  theme: ThemeChoice;
  /** « Couleurs » (PLG-7): the theme of a plugin the window takes, `<plugin id>/<theme id>`; empty or absent, Escouade's (lib/plugin-theme.svelte.ts). */
  pluginTheme?: string;
  /** « N'installer que des versions vérifiées » (STORE-9): a version of the store without the seal is refused. */
  pluginsVerifiedOnly?: boolean;
  /** « Zoom de l’interface », in percent (`ZOOM_LEVELS`): the WebView's zoom, text, spacing and icons together. */
  uiZoom: number;
  /** « Taille du texte des terminaux », in CSS pixels before the zoom. */
  terminalFontSize: number;
  /**
   * « Projection des quotas »: after each quota, where it would end at the pace used so far (« → 64 % »), in the status
   * bar past 80 % (`alert`) or always, or nowhere (`off`). Read through `projectionMode`, which takes the switch of before
   * 2.0.2 (a boolean) too.
   */
  quotaProjection: QuotaProjection;
  /** « Taille du texte de l’éditeur », in CSS pixels before the zoom. */
  editorFontSize: number;
  /** The width of the projects’ rail chosen by its edge (ESC-168), in CSS pixels; 0 (or absent) is its usual width. */
  railWidth?: number;
  /** The width of the sidebar (agents, Kanban) chosen by its edge (ESC-204), in CSS pixels; 0 (or absent) is its usual width. */
  sideWidth?: number;
  /** « Ton nom » (ESC-190): written in the tickets shared with the team. Absent from the window’s tests made before it. */
  userName?: string;
  /** The id beside the name: from the git e-mail, else drawn once. */
  userId?: string;
  /** The first launch’s window asking the name was shown. */
  nameAsked?: boolean;
  /** « Ton nom » emptied on purpose in the settings (ESC-243): its field is no longer filled from git. */
  userNameCleared?: boolean;
  /** « Voix »: the dictation. */
  voice: VoiceSettings;
}

/** The dictation's settings (Réglages › Voix). */
export interface VoiceSettings {
  /** The microphone (its id); empty: the system's default one. */
  device: string;
  /** The Whisper model, downloaded at the first dictation. */
  model: string;
  /** What Whisper is told the speech is in: "ui" (the interface's language), "auto", or a language's code. */
  language: string;
  /** Starts and stops a dictation, held down to talk: `Mod+Shift+Space` (Mod: Ctrl, Cmd on macOS). */
  shortcut: string;
  /** The conversation's voice (ESC-97). */
  speech: SpeechSettings;
  /** Who transcribes a dictation: Whisper on the machine (the default), or the service `onlineService` with the user's key (ESC-110). */
  transcription: 'local' | 'online';
  /** The service of « En ligne »: `openai`, `groq`, `mistral`, `elevenlabs`, `deepgram`. */
  onlineService: string;
  /** The model chosen for each service, by its id (ESC-132); none: its default. */
  onlineModels: Record<string, string>;
  /** « Transcription en temps réel » of each service that offers it, by its id (ESC-136); none: on. */
  onlineLive?: Record<string, boolean>;
}

/** How the agent's passages are read aloud in a conversation. */
export interface SpeechSettings {
  /** The voice of each language (`fr`, `en`) by its id; none: the default one. */
  voices: Record<string, string>;
  /** The speaker of each voice that has several, by the voice's id. */
  speakers: Record<string, number>;
  /** 1: the voice's own pace (0.5 to 2). */
  rate: number;
  /** 0 to 1. */
  volume: number;
  /** The silence that ends what the user says, and sends it (ms). */
  silenceMs: number;
  /** « Voix générée par » (ESC-132): Piper on the machine (the default), or the service `onlineService` with the key of its dictation. */
  engine: 'local' | 'online';
  onlineService: string;
  /** The model chosen for each service; none: its default. */
  onlineModels: Record<string, string>;
  /** The voice chosen, by `<service>|<lang>` (`<service>|<model>|<lang>` when the voices are models of their own); none: the service's default. */
  onlineVoices: Record<string, string>;
}

/** A model of a service on line (ESC-132). */
export interface OnlineModel {
  id: string;
  label: string;
  /** What sets it apart (fast, quality, languages), when the service says it. */
  note: string;
  /** The languages it speaks; empty: those of the service. */
  langs: string[];
}

/** A voice a service on line lists. */
export interface OnlineVoice {
  id: string;
  name: string;
  /** Empty: any language. */
  langs: string[];
  /** The model it belongs to (Deepgram); empty: any. */
  model: string;
  /** Its gender, accent, description. */
  note: string;
  /** Made or cloned on the user's account. */
  own: boolean;
}

/** What Réglages › Voix shows of a service on line: its models, its voices when its API lists them. */
export interface ServiceCatalog {
  service: string;
  transcribe: OnlineModel[];
  speak: OnlineModel[];
  defaultTranscribe: string;
  defaultSpeak: string;
  /** Null: its API lists no voice, the field is free. */
  voices: OnlineVoice[] | null;
  /** The default voice by `<model>|<lang>`; a language missing: the model does not speak it. */
  defaultVoices: Record<string, string>;
  voicesByModel: boolean;
  voiceExample: string;
  voicesUrl: string;
  /** The pace it takes (its bounds); null: its own pace. */
  speed: [number, number] | null;
  /** Read from its API (else the documentation's), and when. */
  listed: boolean;
  listedAt: number;
  error: string | null;
}

/** A voice of the conversation (`voice::speech::catalog`), as Réglages › Voix lists it. */
export interface SpeechVoice {
  id: string;
  lang: string;
  locale: string | null;
  name: string;
  quality: string;
  speakers: number;
  speakerNames: string[];
  sampleRate: number;
  repo: string;
  onnxSize: number;
  jsonSize: number;
  license: string;
  datasetLicense: string | null;
  datasetUrl: string | null;
  /** What must be shown with it (CC BY). */
  attribution: string | null;
  /** Offered by a decision of the user against the rule of the licences (absent otherwise). */
  exception?: boolean;
  downloaded: boolean;
}

export interface SpeechInfo {
  /** Whether the app can speak on this machine (not on Intel Macs). */
  available: boolean;
  voices: SpeechVoice[];
}

/** What starting a conversation answers. */
export type ConvStarted =
  | { kind: 'listening'; session: number; notice: string | null }
  | { kind: 'needsModel'; model: string; size: number }
  | { kind: 'needsVoice'; voice: string; size: number }
  | { kind: 'unavailable' };

/** A microphone, as Réglages › Voix lists it. */
export interface VoiceDevice {
  id: string;
  name: string;
  default: boolean;
}

/** A Whisper model: its size, how fast and how good (1 to 5), and whether it is on the disk. */
export interface VoiceModel {
  id: string;
  size: number;
  speed: number;
  quality: number;
  downloaded: boolean;
}

export interface VoiceInfo {
  devices: VoiceDevice[];
  models: VoiceModel[];
}

/** What starting a dictation answers: it listens, or the model is to be downloaded first. */
export type VoiceStarted = { kind: 'listening'; session: number } | { kind: 'needsModel'; model: string; size: number };

/** The dictation's events: the level while it listens, a listening that ended by itself, a model's download. */
export type VoiceEvent =
  | { kind: 'level'; session: number; level: number; ms: number }
  | { kind: 'ended'; session: number; error: string | null }
  /** What the service has understood of the dictation so far, as one speaks (ESC-136). */
  | { kind: 'partial'; session: number; text: string }
  | { kind: 'download'; model: string; done: number; total: number }
  | { kind: 'speechDownload'; voice: string; done: number; total: number }
  | { kind: 'convLevel'; session: number; level: number; played: number }
  | { kind: 'convSentence'; session: number; reading: number; index: number; text: string }
  | { kind: 'convSpeech'; session: number }
  | { kind: 'convTranscribing'; session: number }
  /** What the service has understood of the sentence under way so far, as one speaks (ESC-136). */
  | { kind: 'convPartial'; session: number; text: string }
  | { kind: 'convHeard'; session: number; text: string; error: string | null }
  | { kind: 'convSpoken'; session: number; reading: number; error: string | null; paused: boolean }
  | { kind: 'convNotice'; session: number; message: string }
  | { kind: 'convBargeIn'; session: number; reading: number }
  | { kind: 'convEnded'; session: number; error: string | null };

/** A Claude account: Claude Code with a configuration folder of its own (its own sign-in, sessions and quota). */
export interface Account {
  id: string;
  name: string;
  /** Its harness (Claude Code for an account saved before 1.8). */
  engine?: EngineId;
  /** Its configuration folder (`CLAUDE_CONFIG_DIR` for Claude Code); empty for Principal, launched without one. */
  configDir: string;
  /** Its own `claude`; empty: the settings' one. */
  claudePath: string;
  /** New agents may go to it. */
  active: boolean;
  /** « Réservé aux projets »: the only projects whose new agents and tickets may go to it; empty: every project. Never Principal's. */
  projects?: string[];
  /**
   * « Budget par jour ($) », for a harness without a quota (OpenCode): once its turns of the day cost this much, the
   * autopilot pauses until midnight. None: no budget.
   */
  dailyBudget?: number | null;
  /** The model, effort and permission mode of its new agents unless they say theirs (ESC-128); empty: its harness's default. */
  defaultModel?: string;
  defaultEffort?: string;
  defaultMode?: string;
  /** Gemini CLI (ESC-275): it goes by its API key, not by the Google sign-in of its folder. */
  apiKey?: boolean;
}

/** What « Ajouter un compte… » says of the machine's account of a harness (its Principal, `accounts::MachineAccount`). */
export interface MachineAccount {
  /** Its harness's program is found. */
  found: boolean;
  /** It is among the accounts already. */
  added: boolean;
  /** Its harness is signed in there (OpenCode, which signs in to none: found). */
  connected: boolean;
  /** Its harness signs in to an account of its own (not OpenCode, whose providers are its config's). */
  signsIn: boolean;
  /** It may be added before it is signed in there, then signed in by its program in a terminal (Antigravity CLI, ESC-271). */
  signsInLater?: boolean;
  /** Its sign-in is out of Escouade's reach (Gemini CLI's Google sign-in, in `~/.gemini`): a new account is to add instead (ESC-275). */
  apart?: boolean;
  /** Who it is signed in as, when the harness says: an email, a GitHub `@login`. */
  email: string | null;
  /** Its folder. */
  dir: string;
}

/** Whether a Claude account is signed in to claude.ai, as its tab shows it. */
export interface AccountStatus {
  /** Claude Code keeps a sign-in for it (an expired one too: it gets a new token when it runs). */
  connected: boolean;
  /** The email of the claude.ai account, when signed in. */
  email: string | null;
  /** Its configuration folder (Principal's: the app's `CLAUDE_CONFIG_DIR`, else `~/.claude`). */
  dir: string;
  /** Which sign-in it is (a fingerprint, never the token): the same while the sign-in is, even out of date, another once the user signs in again. */
  stamp: string | null;
}

/** Where the `claude` of a path of the settings is, as the line under its field says it. */
export interface ClaudeLocated {
  /** Its path (its Linux path for one of WSL); null when it is not found. */
  path: string | null;
  /** Its distribution, for one of WSL. */
  wsl: string | null;
  /** For one of WSL: its network is NAT, out of reach of Escouade's MCP server and of the hooks of the Claude Code view. */
  nat: boolean;
  /** A `claude` of WSL may be named (under Windows). */
  canWsl: boolean;
}

/** Where Escouade's update of Claude Code stands (`claude_update_status`, ESC-133). */
export interface ClaudeUpdateStatus {
  /** Not run yet, under way, up to date, updated, failed; off by the setting, off by the user in Claude Code itself (`line` says how), or no `claude` found (`line` says what to do). */
  state: 'waiting' | 'running' | 'upToDate' | 'updated' | 'failed' | 'off' | 'userOff' | 'missing';
  /** The version it runs (the new one once updated). */
  version: string | null;
  /** The version it was updated from. */
  from: string | null;
  /** The error in a line, or why it is not run. */
  line: string | null;
  /** When it ended, in milliseconds since the epoch. */
  at: number | null;
}

/** How a new account shares Principal's items: linked (one change for both accounts) or copied. */
export type ShareMode = 'link' | 'copy';

/** The languages as the backend resolved them: the interface's, the system's, and that of the texts Claude writes. */
export interface LangInfo {
  ui: Lang;
  system: Lang;
  claude: Lang;
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
export type Service = 'jira' | 'trello' | 'github' | 'linear' | 'clickup' | 'gitlab' | 'azure';

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
  /** Its token stays in integrations.json: the system keychain refused it. */
  inFile?: boolean;
  /** The system keychain did not give its token at this start: no call goes out for it. */
  unread?: boolean;
}

/** The form of « Connecter… »: the fields its service asks for. */
/** What a code host can do, as its provider declares it. */
export interface ForgeCapabilities {
  draft: boolean;
  autoMerge: boolean;
  checks: boolean;
  jobLogs: boolean;
  reviews: boolean;
  reviewThreads: boolean;
  deployments: boolean;
  mergeStrategies: ('merge' | 'squash' | 'rebase' | 'semiLinear')[];
}

/** A code host Escouade knows (GitHub, GitLab, Bitbucket, Gitea / Forgejo, Azure DevOps). */
export interface ForgeProvider {
  kind: string;
  name: string;
  /** « MR » on GitLab, « PR » elsewhere. */
  requestWord: string;
  /** Its public instance (github.com), if it has one. */
  defaultHost: string | null;
  /** Its tokens belong to an e-mail (Bitbucket). */
  needsEmail: boolean;
  /** Its CLI, whose token serves when none is given (gh, glab, az). */
  cli: string | null;
  /** How its CLI is logged in (`gh auth login`, `az login`). */
  cliLogin?: string | null;
  /** What its address field shows when empty (`dev.azure.com/acme`). */
  addressHint?: string | null;
  /** The rights its tokens need. */
  scopes: string[];
  capabilities: ForgeCapabilities;
}

export type ForgeHealth = 'unchecked' | 'connected' | 'expired' | 'missingScopes' | 'unreachable' | 'unread';

/** A code host's account: one per instance. */
export interface ForgeAccountView {
  kind: string;
  name: string;
  host: string;
  url: string;
  label: string;
  viaCli: boolean;
  inFile: boolean;
  /** github.com's: GitHub Issues' account too. */
  issues: boolean;
  status: { health: ForgeHealth; missing: string[]; error: string | null };
  /** The projects whose remote is on its instance. */
  projects: string[];
}

/** A host of the projects' remotes without an account. */
export interface UnknownForgeHost {
  host: string;
  /** Its instance's address, as the remote tells it. */
  url: string;
  kind: string | null;
  projects: string[];
}

export interface ForgesView {
  providers: ForgeProvider[];
  accounts: ForgeAccountView[];
  unknown: UnknownForgeHost[];
  /** The ticket systems' accounts (github.com's is GitHub Issues' one). */
  issues: AccountView[];
}

export interface ForgeForm {
  kind: string;
  url: string;
  email: string;
  token: string;
}

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
  /** Where it is in its service's hierarchy (a ClickUp list's « Space › Folder »): the group of its menu. */
  group?: string;
  /** The project's own repository (its remote's): the import offers to link it at once. */
  own?: boolean;
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
  /** As shown: Story, Bug, Tâche ; Carte ; Issue (in the interface's language for the names Escouade knows). */
  kind: string;
  /** What the type is coloured by, the same in every language: task, subtask, bug, story, epic, card, issue; '' unknown. */
  kindCode: string;
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
  /** What asks for the next page (« Afficher plus »), when the service has more. */
  next: string | null;
  /** How many tickets match in all, when the service says so. */
  total: number | null;
}

/** A harness that runs agents (« Harness » in the interface): Claude Code, Codex, Copilot CLI, Cursor CLI, OpenCode, Mistral Vibe, Antigravity CLI, Gemini CLI. */
export type EngineId = 'claude' | 'codex' | 'copilot' | 'cursor' | 'opencode' | 'vibe' | 'agy' | 'gemini';

/** A mode of Escouade a harness takes, and what it means there (null: what Escouade's menu says of it, Claude Code's meaning). */
export interface ModeCap {
  id: string;
  meaning: string | null;
}

/** What a harness can do, as it declares it: the window shows a function from these, never from the harness's name. A function it lacks is hidden, not greyed out. */
export interface EngineCaps {
  modes: ModeCap[];
  /** Its models take an effort (each model may still have none). */
  effort: boolean;
  images: boolean;
  pdf: boolean;
  /** Copying an agent (« Dupliquer la conversation »), and up to where; 'none': hidden. */
  fork: 'none' | 'entry' | 'turn' | 'event' | 'message';
  resumeOtherAccount: boolean;
  /** A message sent while a turn runs: queued, read at once, or refused. */
  midTurn: 'queued' | 'immediate' | 'refused';
  slashCommands: boolean;
  quota: 'windows' | 'credits' | 'none';
  /** The lengths of the windows of its quota, in minutes: the bars the status bar shows before their first reading. */
  quotaWindows: number[];
  /** What a turn costs: told by the harness, estimated, or unknown (nothing shown). */
  cost: 'exact' | 'estimated' | 'none';
  /** Its own interface in a terminal (« Vue Claude Code »). */
  terminalView: boolean;
  remoteControl: boolean;
  /** Background tasks and workflows, and stopping a subagent. */
  backgroundTasks: boolean;
  todoTools: boolean;
  /** It tells which requests it would refuse by default. */
  defaultRefusal: boolean;
  /** Its commands run in a sandbox, which the modes set (Codex): « Bac à sable » in the agent's header. */
  sandbox: boolean;
  /** Where the usage of its accounts is seen, when it tells Escouade neither quota nor cost (Cursor CLI: its dashboard, ESC-43). */
  usagePage: string | null;
  /** Its accounts but the machine's go by an API key, kept in the keychain (Cursor CLI), rather than a sign-in of their own. */
  accountsByKey: boolean;
  /**
   * Escouade can keep it from writing a file or running a command, in any mode: a discussion of the Docs tab runs on it
   * (ESC-261). Not one that writes the files of its folder without going through Escouade.
   */
  readOnly: boolean;
}

/** The versions of a harness Escouade was tested with. */
export interface EngineTested {
  oldest: string;
  newest: string;
}

/** A model of the composer's menu for a harness, by the id its harness takes, and what it takes. */
export interface ModelDecl {
  id: string;
  /** Its name in the menu until the harness tells which version it runs. */
  label: string;
  effort: boolean;
  auto: boolean;
}

/** A model as its harness lists it for the account (Codex: `model/list`), in place of the declared ones. */
export interface ToldModel {
  id: string;
  /** Its name in the menu (`GPT-6-Luna`). */
  label: string;
  /** The efforts it takes, in its harness's order (none: no menu « Effort »). */
  efforts: string[];
  defaultEffort: string | null;
  /** It reads images. */
  images: boolean;
  /** The one its harness runs unless told otherwise. */
  default: boolean;
  /** The provider it comes from, when its harness has several (OpenCode: `Ollama (local)`). */
  provider?: string;
  /** What it costs beside the others (Copilot: `billing.multiplier`, `1` the base rate). */
  multiplier?: string;
  /** Its tier of price, when its harness tells no multiplier (Copilot). */
  price?: 'low' | 'medium' | 'high' | 'very_high';
  /** Not listed by its harness: the model the account's config names (Codex's `config.toml`, ESC-186). */
  fromConfig?: boolean;
}

/** What a harness declares, as the window gets it at its start. */
export interface EngineDecl {
  id: EngineId;
  name: string;
  /** Its agents' name, as the texts speak of them (« Claude attend ta réponse »): `Claude`, `Codex`. */
  speaker: string;
  tested: EngineTested;
  caps: EngineCaps;
  /** The command that installs it, on Windows and elsewhere (« Aucun harness trouvé »). */
  install: { windows: string; unix: string };
  /** The models of the composer's menu, in its order. */
  models: ModelDecl[];
  /** The efforts of the composer's menu, in its order (empty without `caps.effort`). */
  efforts: string[];
  /** The model that writes the texts (« Modèle des textes rédigés »): `Claude Haiku`. */
  writer: string;
  /** Its texts go to the model chosen in the settings (`writerModel`): OpenCode. */
  writerTakesModel?: boolean;
}

/** A harness with what is found of it here (Réglages). */
export interface EngineInfo extends EngineDecl {
  located: ClaudeLocated;
  /** Its version as it tells it, null when not found or not told. */
  version: string | null;
  /** absent, refused for a version older than the tested ones, no account signed in, ready (a newer version too, ESC-159). */
  state: 'absent' | 'tooOld' | 'unsupported' | 'notConnected' | 'ready';
  /** What its state means: the refusal and its reason. */
  note: string | null;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  color: string;
  /** Its repositories (ESC-169), from its folder with `/`, the root's own as `""`: absent for a project of one repository. */
  repos?: string[];
  /** Those found once and gone since (« retiré »), until forgotten. */
  reposGone?: string[];
  /** « Lettres de la tuile » (ESC-167): empty is the initials of the name. Absent from a project of the window’s tests made before it. */
  tileLetters?: string;
  /** « Logo » (ESC-167): the file of its copy in the app’s data folder, empty for none; set by `setProjectLogo` alone. */
  logo?: string;
  /** « Libellé sous la tuile » (ESC-168): off by default. Absent from a project of the window’s tests made before it. */
  tileLabel?: boolean;
  /** Its text, shorter than the name; empty is the name. */
  tileLabelText?: string;
  /** « Ton nom dans ce projet » (ESC-190): empty is the app’s. Of this machine. */
  userName?: string;
  /** « Partager chaque nouveau ticket » (ESC-190). */
  shareNew?: boolean;
  worktreePerAgent: boolean;
  /** « Créer le worktree au premier message, au nom de l’agent » (ESC-156): on unless switched off. Absent from a project of the window’s tests made before it. */
  worktreeAtFirstMessage?: boolean;
  /** « Branche de départ des nouveaux worktrees » (ESC-160). Absent from a project of the window’s tests made before it: the folder’s branch. */
  worktreeBase?: WorktreeBase;
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
  /** « Commit »: who writes the commits of the files panel's « Commit… » and « Commit tout… ». */
  commitMode: CommitMode;
  /** « Compte préféré »: the Claude account its new agents and tickets go to (an `Account`'s id); empty is « Automatique ». */
  account: string;
  /** « Dossier des worktrees »: where its new worktrees go, from its folder or absolute; empty is `.claude/worktrees`. */
  worktreeDir: string;
  /** « Préfixe des branches des agents »: a new agent's branch is `<prefix><name>`. */
  agentBranchPrefix: string;
  /** « Préfixe des branches des tickets »: a ticket's own branch is `<prefix><key>`. */
  ticketBranchPrefix: string;
  /** « Remote control pour les agents de ce projet »: each of its agents is reachable from claude.ai and the Claude app, unless set apart by hand (`Agent.remoteOff`). */
  remoteControl: boolean;
  /** « Harness par défaut »: the one its new agents and tickets run on unless they say theirs; null: Claude Code. */
  engine: EngineId | null;
  /** « Projet privé »: the agents of the other projects do not see it through the MCP server, and its own see it alone. */
  private: boolean;
  /** « Branche des docs » (ESC-256): the branch that holds `.escouade/spec`, `decisions` and `documents`; empty is the branch checked out. */
  docsBranch?: string;
  /** Its `.escouade.yml`: the keys kept on this machine, the commands trusted, the file as last read. Absent from a project of the window’s tests made before ESC-146. */
  shared?: ProjectShared;
  /**
   * « Comptes partagés à la limite » (a private project): its agents may go on on an account it shares with other projects
   * when its own reach the usage limit (`accounts::confined`).
   */
  sharedOnLimit: boolean;
  /** Claude Code's memory of this project was offered to its shared memory (ESC-255): once, taken or not. */
  claudeMemoryAsked?: boolean;
  /** The corrections of its workflows (ESC-195), ahead of the reading. Absent from a project of the window’s tests made before it. */
  workflowFixes?: WorkflowFix[];
  /** « Relire quand les fichiers changent » (ESC-195): on unless switched off. Absent from a project of the window’s tests made before it. */
  workflowsAutoRead?: boolean;
  /** « Où garder la configuration des workflows » (ESC-195): in the repository (`.escouade/workflows/`, by default) or on this machine. Absent from a project of the window’s tests made before it. */
  workflowsInRepo?: boolean;
  /** Off: « Pas de workflow pour ce projet » (ESC-309), nothing read nor written of its workflows; `workflows.enabled` of its .escouade.yml when shared. Absent: on. */
  workflowsEnabled?: boolean;
  /** « Index du code » (ESC-251), the project's own choice (ESC-293): absent, it follows « Index du code sur tous les projets » (`codeIndexOn`, lib/codeindex.ts). Set by `set_code_index` (Réglages › Projets, ESC-254). */
  codeIndexChoice?: boolean;
}

/** What a project's code index holds (ESC-251). */
export interface CodeIndexStatus {
  files: number;
  symbols: number;
  links: number;
  /** Worktrees with a delta. */
  worktrees: number;
  /** The database's size on the disk. */
  bytes: number;
  /** When the project's folder was last read, in ms; 0 for never. */
  indexedAt: number;
}

/** « Index du code » of a project, as its settings read it (ESC-254). */
export interface CodeIndexInfo {
  /** On: its own choice, or the app's. */
  enabled: boolean;
  /** It made its own choice (ESC-293); else it follows « Index du code sur tous les projets ». */
  chosen: boolean;
  /** Being read for « Index du code sur tous les projets », or waiting for its turn. */
  reading: boolean;
  waiting: boolean;
  status: CodeIndexStatus;
  /** Its own folders its fence leaves out (another project inside it), relative, with `/`. */
  excluded: string[];
}

/** The vectors' model of « Recherche par le sens » (ESC-254): one for every project of this machine. */
export interface VectorsModelInfo {
  id: string;
  license: string;
  size: number;
  /** What is on the disk toward it (a download cut keeps its part). */
  have: number;
  downloaded: boolean;
  downloading: boolean;
}

/** What the code index saved over the statistics' period, for a project (ESC-254). */
export interface IndexSaving {
  agentId: string;
  projectId: string;
  calls: number;
  filesTokens: number;
  answerTokens: number;
  savedTokens: number;
  /** At the input price of the model each agent ran on, in USD. */
  savedCost: number;
}

/** What tells a step added by hand (ESC-195): a command typed, a subagent launched, a file written. */
export type WorkflowSignal = { command: string } | { agent: string } | { file: string };

/** A step as corrected: the steps of the reading it stands for (several once merged; none, added by hand). */
export interface WorkflowFixStep {
  name: string;
  from?: string[];
  signals?: WorkflowSignal[];
}

/** The corrections of one workflow (`workflowfix::Fix`), by its id. */
export interface WorkflowFix {
  workflow: string;
  /** « Ce n'est pas un workflow ». */
  dismissed?: boolean;
  /** Its steps in their order; absent: the reading's. */
  steps?: WorkflowFixStep[];
  /** Steps of the reading taken out. */
  removed?: string[];
}

/** What « Affiner… » will do, said before (`workflowrefine::RefinePlan`). */
export interface WorkflowRefinePlan {
  workflows: number;
  files: number;
  harness: EngineId;
  model: string;
  account: string;
  accountId: string;
  inputTokens: number;
  outputTokens: number;
  /** Dollars at the list price; none when the model's is unknown. */
  cost: number | null;
  private: boolean;
}

/** What an « Affiner… » did (`workflowrefine::Outcome`). */
export interface WorkflowRefineOutcome {
  scan: WorkflowScan;
  refined: { id: string; name: string; added: string[]; described: number; transitions: number }[];
  rejected: string[];
}

/** Something of a `.escouade.yml` that was left out, with its line. */
export interface SharedWarning {
  line: number | null;
  text: string;
}

/** Commands of a `.escouade.yml` the user has not trusted yet: inactive until then. */
export interface SharedPending {
  /** What « Faire confiance à ces commandes » trusts, no other. */
  hash: string;
  /** By key (`worktrees.setup`, `launch`, `board.test_command`…), as the file writes them. */
  commands: Record<string, unknown>;
}

/** A `.escouade.yml` as its project last read it. */
export interface SharedFile {
  /** The keys it sets (`board.merge_strategy`…). */
  keys: string[];
  /** Their values in the file. */
  values: Record<string, unknown>;
  warnings: SharedWarning[];
  /** Why it could not be read at all. */
  error: SharedWarning | null;
  version: number;
  pending: SharedPending | null;
}

export interface ProjectShared {
  /** The keys of the file set otherwise on this machine: their value goes first. */
  local: string[];
  trusted: string;
  /** None while the project has no `.escouade.yml`. */
  file: SharedFile | null;
}

/** « Rédigé par l'agent » (the agent is asked to commit) or « Direct, avec un message proposé ». */
export type CommitMode = 'agent' | 'direct';

/** What a direct commit takes: the files it commits, and the files copied into the worktrees it never does. */
export interface CommitScope {
  files: FileChange[];
  leftOut: string[];
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

/** The launch commands Claude suggests for a project, and how many it gave that were refused (not one plain line, or too long). */
export interface RunSuggestion {
  commands: RunCommand[];
  refused: number;
}

/** A newer release found by the backend, kept there for its install. */
export interface FoundRelease {
  id: number;
  version: string;
  notes: string;
}

/** An update check or download that failed, as the backend explains it. */
export interface UpdateTrouble {
  /** The cause, in the language of the interface. */
  message: string;
  /** Whether to tell the user now: a check asked for, or failures in a row for long enough. Otherwise it is retried in silence. */
  show: boolean;
  /** The page of the release, to take the installer from in the browser when the download is what failed. */
  page: string | null;
}

export type CheckedUpdate = { kind: 'found'; update: FoundRelease } | { kind: 'none' } | { kind: 'failed'; trouble: UpdateTrouble };

export type DownloadedUpdate = { kind: 'ready' } | { kind: 'superseded' } | { kind: 'failed'; trouble: UpdateTrouble };

/** A command run in a worktree once it is made (setup) or before it is removed (teardown). */
export interface WorktreeStep {
  id: string;
  command: string;
  /** Shell id; the system's default one when not found. */
  shell: string;
  /** Folder relative to the worktree's, empty for the worktree itself. */
  cwd: string;
}

/** The setup and teardown Claude suggests for a project's worktrees, and how many steps it gave that were refused (not one plain line, or too long). */
export interface WorktreeSuggestion {
  setup: WorktreeStep[];
  teardown: WorktreeStep[];
  refused: number;
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
  /** The features in place so far, as its agent last listed them (1 to 8 short lines). */
  progress: string[];
  maxLoops: number;
  column: Column;
  /** Order in "À faire": the lowest first. */
  rank: number;
  /** The ids of the tickets of its project it comes after ("Après"): the autopilot starts it once they are all done. */
  after: string[];
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
  /** The same in the language of the texts for Claude (what the comment published for the team quotes); the window shows `outcome`. */
  outcomeClaude: string | null;
  outcomeUrl: string | null;
  /** The commit its merge made on the target; null when it was not merged (pull request, push, kept, nothing to merge) or before 1.8. */
  outcomeCommit: string | null;
  forced: boolean;
  reminded: boolean;
  cost: number;
  createdAt: number;
  startedAt: number | null;
  reviewAt: number | null;
  doneAt: number | null;
  /** Imported from an external ticket system: the ticket there, kept in step. */
  external: ExternalRef | null;
  /** An existing branch its agent takes up; empty: its own, `ticket/<key>`, made at its start. */
  branch: string;
  /** The model its agent runs, over the Kanban's (« Modèle » in its form, or the MCP `start_ticket`); empty: « Auto », the Kanban's, else the account's. */
  model: string;
  /** The effort its agent runs at, over the Kanban's; empty: « Auto », the Kanban's, else the account's. */
  effort: string;
  /** The account its agents start on and stay on (« Compte », ESC-153), by id; empty: « Auto », the current account of its harness. */
  account: string;
  /** Why it went back to « Auto » (its account removed, switched off or signed out), until it is saved again. */
  accountLost: string | null;
  /** The harness its agents run on (the autopilot starts them on it); null: the project's. */
  engine: EngineId | null;
  /** The images pasted or dropped in its form, in their order: its agent gets them with its first message. */
  images: TicketImage[];
  /** The files attached in its form that are no image it can show (any format, any size): its agent gets their paths with its first message. */
  files: TicketFile[];
  /** Shared with the team (ESC-190): its file `.escouade/tickets/<id>.yml`. Absent or null: this machine's alone. */
  shared?: Shared | null;
  /** The pages of the spec and the decisions it implements, as « Planifier » made it (ESC-262). */
  pages?: string[];
  decisions?: DecisionRef[];
}

/** Someone of the team, as the shared tickets' files name them (ESC-190). */
export interface Person {
  name: string;
  id: string;
}

/** What a shared ticket's file says beside the ticket, and how it stands with it (ESC-190). */
export interface Shared {
  /** Its file's name, without `.yml`. */
  id: string;
  /** Its key as its author gave it. */
  key: string;
  author: Person;
  takenBy: Person | null;
  updatedBy: Person | null;
  created: string;
  after: string[];
  external: { source: string; key: string; url: string } | null;
  /** Its author is this machine's user: only then does the autopilot start it. */
  mine: boolean;
  seen: string;
  /** Why its file is not read (a git conflict, a file that cannot be read). */
  problem: string | null;
  conflict: boolean;
  newer: boolean;
  /** Its file is gone from the folder. */
  gone: boolean;
  /** Its author stopped sharing it while this machine's agent had it. */
  left: boolean;
  /** « Masquer chez moi ». */
  hidden: boolean;
}

/** A file of a ticket other than an image: a copy in the ticket's folder, `files/<file>`. */
export interface TicketFile {
  name: string;
  /** `<uniq>/<name>`, under the ticket's `files/` folder. */
  file: string;
  /** In bytes. */
  size: number;
}

/**
 * A file a ticket's form sends: one the ticket has (`file`, `path` empty), or a copy the form made in the drafts (`path`, the
 * copy's own, `file` empty). The backend moves the copies into the ticket's folder and erases the files left out of the list.
 */
export interface DraftFile {
  name: string;
  file: string;
  path: string;
}

/** An image of a ticket: its content is a file of the ticket's folder, never in the tickets' JSON. */
export interface TicketImage {
  name: string;
  mediaType: string;
  /** Its file in the ticket's folder. */
  file: string;
}

/** An image of a ticket with its content (base64), for its form to show. */
export interface TicketImageData extends TicketImage {
  data: string;
}

/** An image a ticket's form sends: one the ticket has (`file`), or one pasted or dropped (`data`, base64); the other one empty. */
export interface DraftImage {
  name: string;
  mediaType: string;
  file: string;
  data: string;
}

/** Where a new agent's worktree starts, its base: the folder's branch (HEAD), the board's target, or a branch named (local, or remote such as `origin/x`). */
export type WorktreeBase = { kind: 'head' } | { kind: 'target' } | { kind: 'branch'; branch: string };

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
  /** The ids of the tickets it comes after, the whole list. */
  after: string[];
  /** An existing branch for its agent to take up; absent or empty: the ticket's own. Given again when editing, to keep it. */
  branch?: string;
  /** Its images, the whole list; absent: they stay as they are. */
  images?: DraftImage[];
  /** Its other files, the whole list; absent: they stay as they are. */
  files?: DraftFile[];
  /** Its harness (« Harness »), null for the project's; absent: it stays as it is. */
  engine?: EngineId | null;
  /** Its account, model and effort (ESC-153), empty for « Auto »; absent: they stay as they are. */
  account?: string;
  model?: string;
  effort?: string;
  /** « Partager avec l’équipe » (ESC-190): absent, as it is (a new ticket: as the project’s « Partager chaque nouveau ticket »). */
  shared?: boolean;
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

/** What the user read and let run of an isola launch: the worktree's .isola.toml and the address to open. */
export interface IsolaApproval {
  config: string;
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
  /** The branch was there before the agent (« Nouvel agent sur une branche… »): deleting the worktree leaves it. */
  existing?: boolean;
  /** The branch was made for the agent under the name the user typed (ESC-155): the agent's, never renamed. */
  namedBranch?: boolean;
  /** Of a project of several repositories (ESC-169): `path` is its folder, with a worktree of each of these at `path/<repo>`, all on `branch`. */
  repos?: RepoWorktree[];
}

/** The worktree of one repository in an agent's folder (ESC-169). */
export interface RepoWorktree {
  repo: string;
  baseBranch: string;
}

/** The state of one repository of a project of several (ESC-169). */
export interface RepoGit {
  repo: string;
  /** "(detached)" for a detached HEAD; empty when its folder is gone. */
  branch: string;
  upstream: string | null;
  ahead: number;
  behind: number;
  hasRemote: boolean;
  hasCommit: boolean;
  modified: number;
  added: number;
  deleted: number;
  gone: boolean;
}

/** What merging an agent would do in one of its repositories (ESC-169). */
export interface RepoMerge {
  repo: string;
  base: string;
  /** Commits of its branch not in the base: none, nothing to merge there. */
  commits: number;
  add: number;
  del: number;
  /** Why it can't be merged. */
  problem: string | null;
  conflicts: string[];
}

/** A worktree of the project's repository an agent without one of its own went into by itself (ESC-161: `EnterWorktree`, a `cd`), as its session last said. */
export interface FoundWorktree {
  path: string;
  branch: string;
}

/** What « Intégrer <base> » did: the base was in the agent's branch already, went in, or stopped on conflicts the agent was asked to resolve (a rebase is undone). */
export type Integration = { kind: 'upToDate' } | { kind: 'done' } | { kind: 'conflict'; files: string[]; rebase: boolean };

/** A request waiting for the user, as the agent's view sums it up: each text capped, the card in the conversation has it whole. */
export interface PendingRequest {
  /** The item's id in the conversation, which the answer names. */
  id: string;
  kind: 'permission' | 'question';
  /** The tool asking (AskUserQuestion for a question). */
  tool: string;
  /** What the tool is asked to act on, as the card sums it up, its line breaks kept. */
  arg: string;
  description: string | null;
  /** Why Claude Code asks. */
  reason: string | null;
  /** `multi`: several options can be ticked. */
  questions: { question: string; options: string[]; multi?: boolean }[];
  /** A text was cut: what is shown is not the whole request. */
  cut: boolean;
  /** Claude Code would refuse it by default: it is not allowed in one click. */
  defaultNo: boolean;
  /** A permission can be allowed for good (« Toujours autoriser ») or, with `alwaysScope`, for the session. */
  canAlways?: boolean;
  /** What « always » stands for: the rest of the session (Codex, Copilot CLI), the agent alone (Cursor CLI), else the rules the harness keeps. */
  alwaysScope?: 'session' | 'agent' | null;
}

/** Where an agent’s remote control comes from. */
export type RemoteSource = 'hand' | 'project' | 'global';

export interface Agent {
  id: string;
  projectId: string;
  name: string;
  named: boolean;
  /** The harness it runs on, one at a time ('claude' for an agent saved before 1.8); a copy takes the original's. */
  engine: EngineId;
  model: string;
  effort: string;
  mode: string;
  /** The mode its work resumes in once a plan is approved (`planResume`, ESC-277): the one before Plan. */
  planFrom?: string;
  sessionId: string | null;
  /** The latest entry of its session (the uuid of its latest assistant message): where a copy made now forks it. */
  lastEntry: string | null;
  /** A copy of another agent: the original’s session, which its starts fork until a turn gives it its own. */
  forkOf: string | null;
  /** Where the copy forks it: the original’s latest entry when it was copied (null: the whole session). */
  forkAt: string | null;
  cwd: string;
  worktree: Worktree | null;
  /** Its worktree waits for its first message (ESC-156): named first, it gets its branch and its folder under that name. Absent once made, or never waited for. */
  worktreePending?: boolean;
  /** Without a worktree of its own: the one it went into by itself (ESC-161), whose pull requests it follows. */
  foundWorktree?: FoundWorktree;
  createdAt: number;
  archived: boolean;
  /** Why its worktree, its ticket over, was kept: work nothing else keeps (ESC-143). */
  worktreeKept?: string;
  status: AgentStatus;
  tokens: number;
  cost: number;
  activeMs: number;
  touchedFiles: string[];
  lastActivity: number;
  prompts: number;
  activeSince: number | null;
  alive: boolean;
  /** « Outils d’Escouade au prochain démarrage »: its live process has none of Escouade’s tools, which its next one gets (ESC-91). */
  escouadeNext: boolean;
  /** Ids of the question and permission items waiting for an answer, in the order they were asked. */
  pending: string[];
  /** The same requests summed up, in the same order: what « Vue d’ensemble » shows without the conversation. */
  requests: PendingRequest[];
  contextTokens: number;
  /** Size of the context window of the conversation's model (0 until a turn told it). */
  contextWindow: number;
  /** Tokens of the running turn so far; `tokens` includes them once the turn ends. */
  liveTokens: number;
  /** Estimated cost (list prices) of the running turn so far; `cost` gets the exact figure at its end. */
  liveCost: number;
  /** Remote Control: reachable from claude.ai / the Claude app (its process stays up), whatever turned it on. */
  remoteControl: boolean;
  /** What turned it on: the user by hand, its project’s setting, or the setting for all agents; null while it is off. */
  remoteSource: RemoteSource | null;
  /** Set apart by hand (« Désactiver le remote control », a setting covering the agent or not): off in spite of a setting, until turned on by hand. */
  remoteOff: boolean;
  /** The Claude Code interface (« Vue Claude Code ») that runs was started with the remote control; null while no interface runs. */
  interfaceRemote: boolean | null;
  remoteSession: string | null;
  /** The session on claude.ai. */
  remoteUrl: string | null;
  /**
   * Link state reported by Claude Code ("ready", "connected"…), null without a live link. In Claude Code's view it is the interface's,
   * whatever the agent's remote control says (an open interface keeps its link until its next launch): "connected" (then `remoteUrl` is
   * its session), "unavailable" when the interface runs but Claude Code did not open the remote control (no claude.ai subscription,
   * network, proxy or organization policy; the agent's conversation says so in a notification), or null.
   */
  remoteState: string | null;
  /** Stopped by the usage limit: when it is sent "continue" by itself (epoch ms). */
  resumeAt: number | null;
  /** The board's ticket it works on. */
  ticketId: string | null;
  /** What the agent last reported of its work through Escouade's MCP server (120 characters at most), until its next report. */
  progressLine: string | null;
  /** Its pull requests (ESC-64), as its code host last told them, the ones to merge first: none when its host has no client yet. */
  prs: PullRequest[];
  /** The code host's word for them: « MR » on GitLab, « PR » elsewhere; null until read. */
  prWord: string | null;
  /** The ticket's protocol, appended to Claude Code's system prompt. */
  appendPrompt: string | null;
  /** First of the 10 ports reserved for its test launches. */
  portBase: number | null;
  /** How to launch its worktree for a test, as it last wrote it. */
  recipe: TestRecipe | null;
  /** The recipe the user read and let run: the same content as `recipe`, else it is asked again. */
  approvedRecipe: TestRecipe | null;
  /** The .isola.toml (and the address to open) the user read before isola ran the worktree's services. */
  approvedIsola: IsolaApproval | null;
  /** The Claude account it runs on (an `Account`'s id), where its session is kept. */
  account: string;
  /** The account it ran on before « Reprendre sur <compte> » moved it, until a turn ends on the new one: a turn that fails there is the resume’s failure. */
  movedFrom: string | null;
  /** The harness it left by a handover (« Continuer sur <harness> »): set aside until `until` (the reset of its quota; null: left with quota to spare), then offered back, never gone back to by itself. */
  handedFrom: HandedFrom | null;
  /** Stopped by a sign-in of its account that Claude Code could not refresh (ESC-162): the account is to sign in again, and the agent goes on by itself once it has. */
  signIn: SignInWait | null;
  /** What it is doing right now ("Lit src/db.ts", "Lance npm test"…), during a turn. */
  activity: string | null;
  /** The setup of its new worktree under way: the step running ("1/2 · npm ci"). */
  setup: string | null;
  /** isola runs its worktree's services: its test launch goes through it, with no recipe. */
  isola: boolean;
  /** How far it is through a plan (its task list, its subagents, its workflows), worked out by the backend from its stream; absent until it has any. */
  plan?: PlanState;
  /** « Vue Claude Code »: it runs in Claude Code's own interface, in a terminal, not in Escouade's conversation view. */
  terminal: boolean;
  /**
   * « Vue OpenCode »: its harness's interface, in a terminal, joined to the process Escouade drives, which goes on (the
   * conversation keeps up with what is done there). Never with `terminal`.
   */
  attached: boolean;
  /** The terminal of that interface (either) while it runs. */
  terminalId: string | null;
  /** How the last run of that interface ended; null until it ended (the backend puts it back to null when it starts one). */
  tuiExit: TuiExit | null;
  /** A discussion of the Docs tab (ESC-261): its role, its subject and what it proposes for the spec. Absent for any other agent. */
  discussion?: Discussion;
}

/** The role of a discussion of the Docs tab (ESC-261): the spec writer, or the product agent of a brainstorm. */
export type DiscussionKind = 'discussion' | 'brainstorm';

/** What makes an agent a discussion of the Docs tab (ESC-261). */
export interface Discussion {
  kind: DiscussionKind;
  subject: string;
  /** The pages it proposes, a change each, not poured yet. */
  pages: DiscussionPage[];
  /** The decisions it proposes, not poured yet (no number until poured). */
  decisions: DocsDecision[];
  /** What « Verser dans la spec » wrote so far; absent until then. */
  poured?: DiscussionPoured;
  /** What « Planifier » made of it (ESC-262); absent until then, and once cancelled. */
  planned?: DocsPlan;
}

/** A ticket « Planifier » made (ESC-262). */
export interface DocsPlanMade {
  id: string;
  key: string;
  title: string;
}

/** What a « Planifier » did (ESC-262): the tickets made in « À faire », those left out because the board has them. */
export interface DocsPlan {
  id: string;
  /** When (ms). */
  at: number;
  /** The discussion it was asked from (its agent); absent: from the Specification. */
  discussion?: string;
  pages: string[];
  decisions: DecisionRef[];
  made: DocsPlanMade[];
  /** Proposed again by the planner, left out: `key` is the ticket of the board it repeats. */
  skipped: { title: string; key: string }[];
  cancelled: boolean;
}

/** What the Specification shows of the planning (ESC-262): the changes without tickets, the latest plan. */
export interface DocsPlanning {
  pages: string[];
  decisions: DecisionRef[];
  last?: DocsPlan;
  running: boolean;
}

/** What « Annuler » of a planning did: the tickets removed, and those kept because they started. */
export interface DocsPlanCancelled {
  removed: string[];
  kept: string[];
}

export interface DiscussionPage {
  path: string;
  /** The page's text before its first change; null: a new page. */
  before: string | null;
  /** The page as it would be written. */
  page: DocsPage;
  added: number;
  removed: number;
}

export interface DiscussionPoured {
  /** When it was last poured (ms). */
  at: number;
  pages: string[];
  /** The decisions proposed in the register (their ids, ESC-303). */
  decisions: DecisionRef[];
}

/** A gap of a page (its index) that a discussion settles: gone from the page once its change is poured. */
export interface DiscussionGap {
  page: string;
  index: number;
}

/** What a gesture of the spec gives « Nouvelle discussion »: its role, its subject, its first message, the gap it settles. */
export interface DiscussionPreset {
  kind: DiscussionKind;
  subject: string;
  message?: string;
  gap?: DiscussionGap;
}

/** What « Nouvelle discussion » asks for. */
export interface DiscussionStart {
  kind: DiscussionKind;
  subject: string;
  /** The documents joined, already filed in the drive. */
  documents: string[];
  /** The first message, when it is not the subject (a gesture of the spec). */
  message?: string | null;
  gap?: DiscussionGap | null;
  engine?: EngineId | null;
  account?: string | null;
  model?: string | null;
}

/** The end of Claude Code's interface (« Vue Claude Code »), as the backend saw it. */
export interface TuiExit {
  /** The exit code of its process (null: none, it was killed). */
  code: number | null;
  /** It stopped less than 20 s after it started: something is wrong, it is not the user quitting. */
  quick: boolean;
  /** Claude Code found no conversation for the session to resume: the backend already replaced it with a new one. */
  sessionLost: boolean;
}

/** Where a plan's task list comes from: the agent's task tools (what the stream tells), or a plan file of its folder. */
export type PlanSource = 'tools' | 'plan';

export type PlanTaskStatus = 'pending' | 'inProgress' | 'done';

/** How a subagent or a workflow run is going; 'interrupted': the process that ran it is gone, or its turn was stopped, before it ended. */
export type RunStatus = 'running' | 'done' | 'failed' | 'stopped' | 'interrupted';

export interface PlanTask {
  /** Claude Code's id for it (a number as text with the task tools, the rank with TodoWrite). */
  id: string;
  title: string;
  status: PlanTaskStatus;
  /** What it is doing while in progress ("Running tests"). */
  active?: string;
  /** Steps done and in all, when a plan file counts them. */
  steps?: [done: number, total: number];
  /** The ids of the tasks it waits for. */
  blockedBy?: string[];
}

/** A subagent the agent launched (a `Task` / `Agent` call). */
export interface SubAgent {
  /** The id of the call that launched it. */
  id: string;
  title: string;
  /** Its type ("Explore"…). */
  kind?: string;
  /**
   * The model it runs on: the one its messages tell ("claude-haiku-5-5"), once one came, else the one its call asked for,
   * as it wrote it ("haiku"). Its call may name none (its agent's definition chooses, or it inherits the agent's).
   */
  model?: string;
  status: RunStatus;
  background: boolean;
  /** Tools it used. */
  tools: number;
  tokens?: number;
  startedAt: number;
  endedAt?: number;
  /** What it does now ("Édite src/a.ts"), while it runs, in the language of the moment. */
  doing?: string;
  /** The id of the task it works on, when it can be told. */
  planTask?: string;
  /** The call of the subagent that launched it, when one did (a subagent's subagent). */
  parent?: string;
  /** Claude Code's id of its task, once a frame told it: what « Arrêter » stops and what the agent writes to. */
  taskId?: string;
}

/** What the conversation keeps of a subagent once it ended, on the item of its call: the plan forgets its row (30 at most, none after a reset). */
export interface SubagentRun {
  status: RunStatus;
  /** The model it ran on, as its row last had it. */
  model?: string;
  tokens?: number;
  tools: number;
  startedAt: number;
  endedAt?: number | null;
}

/** How a subagent's files stood when its transcript was last read (Claude Code view): the transcript's size in bytes, and when it last changed in ms (the later of it and its meta file). Given back to the next read, which answers `null` while nothing changed. */
export interface TranscriptCursor {
  len: number;
  mtime: number;
}

/** What a read of a subagent's transcript gives in the Claude Code view (`subagent_transcript`; `null` instead when the cursor given is still current). */
export interface SubagentRead {
  /** Its whole thread, which replaces the one shown. */
  items: ConvItem[];
  /** The cursor to give back; null while there is no file yet (the next read looks for it again). */
  cursor: TranscriptCursor | null;
  /** How its files say it ended, which the view's frames never do (stopped in Claude Code's interface, an API error): the backend has ended its row of the plan then, if it still ran, and sent the agent. */
  ended: 'stopped' | 'failed' | null;
  /** The model its messages tell (the backend has put it on its row of the plan then). */
  model?: string | null;
}

/** A workflow of Claude Code the agent runs. */
export interface WorkflowRun {
  /** Claude Code's id of its task. */
  id: string;
  name?: string;
  title: string;
  status: RunStatus;
  /** The phase and the agent running now ("Phase 1 : lecteur A"). */
  now?: string;
  tools: number;
  tokens: number;
  startedAt: number;
  endedAt?: number;
}

/**
 * What a background task is: `command` = a Bash call run in the background, `monitor` = the Monitor tool, `remote` = a
 * remote agent, `teammate` = an in-process teammate. (A workflow is a `WorkflowRun`; a subagent has its own row.)
 */
export type BgKind = 'command' | 'monitor' | 'remote' | 'teammate';

/** A task Claude Code runs in the background for an agent (ESC-14). */
export interface BgTask {
  /** Claude Code's id of the task. */
  id: string;
  kind: BgKind;
  /** The tool's description ("Ticker"). */
  title: string;
  /** The command line, 2000 characters at most; absent when unknown. */
  command?: string;
  status: RunStatus;
  /** In ms (epoch). */
  startedAt: number;
  endedAt?: number;
  /** Claude Code's last words on it: 'Background command "Ticker" completed (exit code 0)', 'Monitor "Watch" stream ended'; for one stopped, only its description. */
  summary?: string;
  /** The id of the tool call that launched it. */
  call?: string;
  /** The task id of the subagent that owns it, when one does. */
  parent?: string;
  outputFile?: string;
}

/** A time Claude Code will start a turn of the agent by itself (ESC-138): a job of its session (`CronCreate`) or the next wake-up of a dynamic loop (`ScheduleWakeup`). */
export interface Wake {
  /** The job's id, `loop` for a loop's wake-up. */
  id: string;
  kind: 'cron' | 'loop';
  /** When it fires (the next time, for a recurring job), in ms (epoch). */
  at: number;
  recurring: boolean;
  /** Written down by Claude Code: it outlives the process. */
  durable: boolean;
  /** The job's schedule as Claude Code tells it ("17 8 22 10 *"). */
  schedule?: string;
  /** The job's prompt, the reason of a loop's wake-up. */
  prompt?: string;
}

/** An agent of Escouade the agent set to work through the MCP server (ESC-138): by its id, or by the ticket whose agent it started, sent back or resumed (a key or an id). */
export interface Driven {
  agent?: string;
  ticket?: string;
  /** When it was last driven, in ms (epoch). */
  at: number;
}

/** The end of the output of a background task (`task_output`): 8 KiB at most while it runs, 256 KiB at most once it ended. */
export interface TaskOutput {
  output: string;
  /** The size of the whole output, in bytes. */
  totalBytes: number;
  /** The start of the output is missing. */
  truncated: boolean;
  /** Where it was read: the running process, its output file (the process is gone, or the Claude Code view), or nowhere (a workflow, a remote agent, no file). */
  source: 'process' | 'file' | 'none';
}

/** How far an agent is through a plan: its task list, its subagents, its workflows. */
export interface PlanState {
  source: PlanSource | null;
  /** The plan file of the agent's folder (relative to it) that the tasks come from (source 'plan'), or that an agent keeping its own task list (source 'tools') is following, when one of its tasks is the plan's. Whether there is a task list is `tasks.length`, not `source`. */
  planFile?: string;
  /** The plan's title (the first heading of its file, without « Implementation Plan »), whatever the source; else the window falls back to the ticket's, then the agent's name. */
  title?: string;
  /** At most 100. */
  tasks: PlanTask[];
  /** All the subagents running, then the latest ended: 30 at most. */
  agents: SubAgent[];
  /** Every subagent launched since the plan was reset, those whose row went included. */
  launched: number;
  /** At most 10. */
  workflows: WorkflowRun[];
  /** The tasks running in the background (commands, monitors, remote agents, teammates), then the 10 latest ended: 40 at most. */
  bgTasks: BgTask[];
  /** The times Claude Code will start a turn by itself, 10 at most; absent when none. */
  wakes?: Wake[];
  /** The agents of Escouade it drives, the latest last, 20 at most; absent when none. */
  driven?: Driven[];
}

/** A choice of Claude Code's model picker: an alias or a full id, and the model it stands for. */
export interface ModelInfo {
  value: string;
  /** Full model id, e.g. "claude-sonnet-5-5". */
  resolvedModel: string;
  /** It takes the permission mode `auto`, as Claude Code tells it (absent for a model without it, or a version that tells it of none). */
  supportsAutoMode?: boolean;
  /** Its name in Claude Code's `/model` ("Opus 5.5"), absent when it tells none. */
  displayName?: string;
  /** It takes an effort (absent: none), and which. */
  supportsEffort?: boolean;
  supportedEffortLevels?: string[];
}

/** What the `claude` of an account told of its models at its last reading: its version, its models, and the menu they make. */
export interface AccountModels {
  /** "2.1.302", empty when it was not read. */
  version: string;
  models: ModelInfo[];
  /** The composer's menu of the account's agents, in its order (`default` aside, marking the one it runs): empty while it told none, when the declared models serve (`engine::claude::catalog`). */
  catalog?: ToldModel[];
  /** When they were last read (ms since the epoch), 0 when unknown (ESC-186). */
  readAt?: number;
}

export interface UiState {
  activeProject: string | null;
  view: string;
  selectedAgent: Record<string, string>;
  /** 'split': conversation on the left half, uncommitted files and their diff on the right. */
  layout?: '' | 'split';
}

/** The checkout of an agent's own folder (`GitInfo.agentGit`): the branch HEAD is really on, whatever its worktree was made on. */
export interface AgentGit {
  /** "(detached)" for a detached HEAD; of several repositories, theirs when they share it (empty otherwise). */
  branch: string;
  upstream: string | null;
  upstreamGone: boolean;
  ahead: number;
  behind: number;
  hasRemote: boolean;
  /** Of an agent of several repositories (ESC-169), each one's: the fields above are then their sums. */
  repos?: RepoGit[];
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
  /**
   * The checkout of each agent that works in a folder of its own (its worktree, or the one it went into by itself,
   * ESC-161), as git tells it: what the status bar shows of the agent open. None for an agent in the project's folder.
   */
  agentGit?: Record<string, AgentGit>;
  /** Of a project of several repositories (ESC-169): each one's; the fields above are their sums, `branch` theirs when they share it (empty otherwise). */
  repos?: RepoGit[];
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

/** A branch of the project's repository (`branchList`): a local one, or a remote one named with its remote ("origin/feat"). */
export interface BranchInfo {
  name: string;
  remote: boolean;
  /** Checked out in the project's folder. */
  current: boolean;
  /** The remote branch a local one tracks ("origin/feat"), null when it tracks none. */
  upstream: string | null;
  /** That upstream no longer exists on the remote (as of the last fetch). */
  upstreamGone: boolean;
  /** Commits to push / to pull, against the upstream as last fetched. */
  ahead: number;
  behind: number;
  /** The folder of the worktree that has it checked out, the project's own folder included. */
  worktree: string | null;
  /** A remote one: the local branch that tracks it (switching to the remote one goes there). */
  trackedBy: string | null;
  /** The id of the agent whose worktree holds it: neither switched to nor deleted (« utilisée par l'agent X »). */
  agent: string | null;
  /** When its latest commit was made (ms epoch). */
  lastCommitAt: number;
}

export interface FileChange {
  path: string;
  status: 'M' | 'A' | 'D';
  add: number;
  del: number;
  agentId: string | null;
  /** Listed from the worktree of `agentId` rather than the project's repository. */
  inWorktree: boolean;
  /** Its repository, in a project of several (ESC-169): `path` starts with it. */
  repo?: string;
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
  /** The files of `files` that git ignores: the ones the project copies into its worktrees (« Fichiers copiés dans les worktrees »). */
  ignored: string[];
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

/**
 * What `code_search` looks for in a source's files: `pattern` as written, or with `regex` an expression as JavaScript
 * writes them (git's Perl expressions when it has them, else translated for its extended ones: what these cannot say
 * is refused with « Cette expression n'est pas prise en charge par git sur ce poste : … »). A pattern holds on one line.
 * At most 5 000 matches, whatever is asked.
 */
export interface SearchQuery {
  pattern: string;
  regex: boolean;
  caseSensitive: boolean;
  wholeWord: boolean;
  maxResults: number;
}

/**
 * A matching line: its file from the source's root, its line and the column of its first match (1-based, in
 * characters: code points, an emoji being one). All offsets are in characters too.
 */
export interface SearchMatch {
  path: string;
  line: number;
  col: number;
  /** At most 300 characters of the line: all of it, or a stretch starting 40 characters before its first match. */
  text: string;
  /** Where `text` starts in the line (0 for the line's start). */
  offset: number;
  /** Each match in `text`, `[start, end)` from the start of `text`, as git found them. */
  ranges: [number, number][];
}

export interface SearchResult {
  matches: SearchMatch[];
  /** More lines matched than were asked for, or the search was stopped after 10 s. */
  truncated: boolean;
  /** The 10 s limit stopped the search (`truncated` is set too). */
  timedOut: boolean;
}

/** The version a file is compared with ("HEAD", or the branch a worktree left). */
export interface FileBase {
  reference: string;
  text: string | null;
}

/** « Projection des quotas » (`Settings.quotaProjection`). */
export type QuotaProjection = 'alert' | 'always' | 'off';

export interface RateWindow {
  pct: number;
  resetsAt: number | null;
}

/**
 * One meter of an account's quota: a window of a length (`minutes`: 300 for 5 hours, 10080 for a week), used in percent
 * until it starts again, or the month's credits (`minutes`: null).
 */
export interface QuotaMeter extends RateWindow {
  minutes: number | null;
}

export interface Usage {
  /** What the turns cost today, on every account. */
  todayCost: number;
  /** Every account, of every harness, in the settings' order. */
  accounts: AccountUsage[];
  /**
   * The account new agents of each harness would go to (the first active one under the pause threshold, else the first
   * active one): the one whose meters the status bar shows for that harness.
   */
  current: Partial<Record<EngineId, string>>;
}

/** One account's quota, as last read. */
export interface AccountUsage {
  id: string;
  /** Its harness. */
  engine: EngineId;
  /** Its meters as last read, the shortest window first; none for a harness without a quota, or not read yet. */
  meters: QuotaMeter[];
  /** Signed in, as the last reading tells (an account not read yet counts as signed in). */
  connected: boolean;
  /** Why its quota is not read (not signed in, sign-in expired), in the interface's language. */
  reason: string | null;
  /** What its turns cost today. */
  todayCost: number;
  /** Its « Budget par jour ($) », when it has one. */
  dailyBudget?: number | null;
  /** When its windows were last read (0: not since the app started). */
  updatedAt: number;
  /** Its agents go on answering over their window, as billed extra usage (`rate_limit_event`). */
  overage?: boolean;
}

/** Why no ticket of any board starts, and until when: a quota window over « Pause au-delà du quota », or a usage limit with no resume planned. */
export interface AutopilotPause {
  /** A window over the threshold (of `minutes`), the month's credits, a usage limit, or the day's budget of an account without a quota. */
  reason: 'window' | 'credits' | 'limit' | 'budget';
  /** The window's length, for a window. */
  minutes?: number;
  /** The window's use, 0-100 (null after a limit). */
  pct: number | null;
  /** When the tickets start again: the window's end, or about then after a limit. */
  until: number;
  /** The accounts a ticket could start on, all held back (the reason and the end are those of the first to be free again); none with a single account. */
  accounts?: string[];
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
  /** A field of a form (Copilot: `elicitation`): its name, and what is written in it when it has no choices. */
  id?: string;
  field?: 'text' | 'number';
  required?: boolean;
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
  origin?: 'remote' | 'handover';
}
export interface TextItem extends Base {
  kind: 'text';
  text: string;
  streaming: boolean;
  /** In a voice conversation: the passage the agent wrote to be read aloud, taken out of `text` (ESC-97). */
  spoken?: string;
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
  /** The call of a subagent (`Agent` / `Task`): the report it handed back (`handback_report`, else the `summary` of a notification with no hand-back), 16 000 bytes of UTF-8 at most. */
  report?: string;
  /** The call of a subagent: what it was once it ended. */
  run?: SubagentRun;
  /** A call of `SendUserFile` (ESC-171): its files as the core delivered them (its images copied). Read through `sentFiles`. */
  sent?: unknown[];
}

/** A file an agent sent (`sentFiles`): an image with its `data:` URL, or a file; there (`ok`), gone (`missing`), of a private project (`fenced`). */
export interface SentView {
  name: string;
  kind: 'image' | 'file';
  state: 'ok' | 'missing' | 'fenced';
  size?: number;
  /** An image too heavy for a preview: shown as a file. */
  big?: boolean;
  url?: string;
  /** A program or a script: never opened, only shown in its folder. */
  runnable?: boolean;
}
export interface QuestionItem extends Base {
  kind: 'question';
  toolUseId: string;
  /** Asked by a subagent: the call that launched it. Shown in the agent's conversation and in the subagent's. */
  sub?: string;
  questions: Question[];
  answers: Record<string, string> | null;
  /** A form of an MCP server (Copilot: `elicitation`): its message and the server that asks. */
  form?: { message?: string; source?: string };
  /** Who answered, when it was a client of the MCP server and not the user in the window (an agent's name, « Claude (hors Escouade) »). */
  by?: string;
  cancelled?: boolean;
  ts: number;
}
export interface PermissionItem extends Base {
  kind: 'permission';
  toolUseId: string;
  /** Asked by a subagent: the call that launched it. Shown in the agent's conversation and in the subagent's. */
  sub?: string;
  toolName: string;
  title?: string | null;
  description?: string | null;
  input: Record<string, any>;
  reason?: string | null;
  canAlways: boolean;
  /** What « always » stands for: for the rest of the session (Codex: `acceptForSession`), for the agent alone (Cursor CLI: its own config folder), else the rules Claude Code suggested. */
  alwaysScope?: 'session' | 'agent';
  /** A change of files asked: its diff, in the lines of a tool's `result.patch`. */
  patch?: { oldStart: number; newStart: number; lines: string[] }[];
  defaultNo: boolean;
  decision: 'allow' | 'always' | 'deny' | null;
  message?: string | null;
  /** Who answered, when it was a client of the MCP server and not the user in the window (an agent's name, « Claude (hors Escouade) »). */
  by?: string;
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
  /** Stopped by the usage limit (absent from turns saved before the accounts): the card offers to go on, on another account. */
  limited?: boolean;
  /** Stopped by a sign-in Claude Code could not refresh (ESC-162): its process started again, or the card asks for a new sign-in. */
  signIn?: boolean;
  /** The harness that ran it (absent from turns saved before 1.8, all Claude Code's). */
  engine?: EngineId;
}
/** A handover to another harness: « Suite sur Codex · quota Claude Code épuisé jusqu’à 18:00 ». */
export interface HandoverItem extends Base {
  kind: 'handover';
  ts: number;
  from: EngineId;
  to: EngineId;
  /** It left because every account of its harness serving the project was spent. */
  limited: boolean;
  /** When that quota resets (ms), if known. */
  until: number | null;
}
/** The sign-in an agent waits for (`Agent.signIn`): its account's, another than `stamp`. */
export interface SignInWait {
  account: string;
  stamp: string | null;
}
/** The harness an agent left by a handover (`Agent.handedFrom`). */
export interface HandedFrom {
  engine: EngineId;
  account: string;
  until: number | null;
}
export interface NoticeItem extends Base {
  kind: 'notice';
  ts: number;
  /** 'output': what a local command printed (`/cost`, `! ls`), its lines kept (ESC-154). 'switch': the agent went on on
   * another account past the threshold (ESC-148), a separator. */
  level: 'info' | 'warn' | 'error' | 'output' | 'switch';
  text: string;
}

/** Passed on to Claude by Claude Code itself: a background task that ended, a subagent's message. */
export interface EventItem extends Base {
  kind: 'event';
  /** 'task' (a background task), 'agent' (a subagent or another session), 'wake' (the wake-up of a loop, its command as text), else the origin Claude Code gave. */
  source: string;
  /** The subagent that sent it. */
  from?: string;
  text: string;
  ts: number;
}

export type ConvItem =
  UserItem | TextItem | ThinkingItem | ToolItem | QuestionItem | PermissionItem | TurnItem | NoticeItem | EventItem | HandoverItem;

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

/** A door of a workflow (ESC-192): a command, a skill or a prompt one types; `other`: of the family, no step of it (the « +38 »). */
export interface WorkflowDoor {
  name: string;
  /** As typed: `/gsd:plan-phase`. */
  command: string;
  kind: 'command' | 'skill' | 'agent';
  scope: 'project' | 'user';
  path: string;
  description: string;
  role: 'entry' | 'step' | 'other';
  /** For an entry that runs several steps: which. */
  enters: string[];
}

/** A step of a workflow: its door, its subagents, the files it reads and writes. */
export interface WorkflowStep {
  name: string;
  command: string;
  order: number;
  description: string;
  path: string;
  agents: { name: string; path: string | null; description: string }[];
  reads: string[];
  writes: string[];
  /** « deviné »: dotted until « Garder » or « Retirer » (Réglages › Projets › Workflow). */
  guessed?: boolean;
  /** What else signals it once corrected (ESC-195): the steps merged in, its name as read when renamed. */
  also?: string[];
  /** Named by the user (the window's own mark, ESC-194): its name is shown as is. */
  named?: boolean;
  /** What the rail says of it in words, for a workflow of Escouade's own (« Tickets », ESC-197). */
  note?: string;
}

export interface WorkflowTransition {
  from: string;
  to: string;
  /** A way back to a step already passed (or to the same one, again). */
  back: boolean;
  condition: string | null;
  max: number | null;
  /** How the rail says it, for a workflow of Escouade's own (« Renvoyer : retour à En cours », ESC-197). */
  say?: string;
}

export interface WorkflowProgress {
  done: number;
  total: number;
}

export interface WorkflowStateFile {
  path: string;
  exists: boolean;
  writtenBy: string[];
  readBy: string[];
  says: { phase: WorkflowProgress | null; checked: WorkflowProgress | null; status: string | null } | null;
}

/** A workflow of a project, as the rules read it (ESC-192). */
export interface Workflow {
  id: string;
  name: string;
  kind: 'cycle' | 'prompt';
  harness: EngineId | null;
  /** The repository of a project of several; empty for the root. */
  repo: string;
  doors: WorkflowDoor[];
  steps: WorkflowStep[];
  transitions: WorkflowTransition[];
  stateFiles: WorkflowStateFile[];
  /** The line that says when it is over. */
  exit: string | null;
  promptLoop: {
    via: string;
    prompt: string;
    list: WorkflowStateFile | null;
    steps: { n: number; text: string }[];
    returns: WorkflowTransition[];
    maxTurns: number | null;
  } | null;
}

/** The reading of a project's workflows: what was found, and where it was looked for. */
export interface WorkflowScan {
  readAt: number;
  workflows: Workflow[];
  alone: { name: string; command: string; harness: EngineId; repo: string; path: string; description: string }[];
  untraced: { name: string; harness: EngineId; steps: number }[];
  sources: { path: string; kind: string; modified: number; len: number }[];
  searched: string[];
  /** Where the sources' paths start: the project's folder, Claude Code's folder, the home folder (absent before ESC-195). */
  root?: string;
  claude?: string;
  home?: string;
  /** The files of the workflows (ESC-195): one per workflow; `local` when this machine reads it too; `updated`, what the last reading changed of it. */
  files?: { id: string; path: string; local: boolean; inRepo: boolean; updated: string | null }[];
  /** The files that could not be read, with why. */
  unreadable?: { path: string; why: string }[];
  /** The workflows the user removed (ESC-309): never written again by a reading; `changed` when what found one changed since (proposed again). */
  removed?: { id: string; name: string; changed: boolean }[];
  /** « Pas de workflow pour ce projet » (ESC-309): nothing was read. */
  off?: boolean;
}

/** How an agent's stay in a step, or outside the workflow (`step` null), ended. */
export type WorkflowOutcome = 'running' | 'done' | 'back' | 'failed' | 'stopped' | 'left';

/** One stay of an agent in a step of its project's workflow (ESC-193). */
export interface WorkflowPassage {
  n: number;
  workflow: string;
  step: string | null;
  /** The version of the workflow when it began. */
  version: string;
  /** The nth stay of the agent in this step. */
  pass: number;
  door: string | null;
  at: number;
  ended: number | null;
  ms: number;
  cost: number;
  /** `agent:<subagent_type>`, `skill:<name>`, `file:<path>`. */
  did: string[];
  outcome: WorkflowOutcome;
  /** Why it went back, as the step wrote it, and the file that decided it. */
  reason: string | null;
  decidedBy: string | null;
  backTo: string | null;
  /** What the agent did, outside the workflow. */
  doing: string | null;
}

/** One turn of a prompt run over and over. */
export interface WorkflowTurn {
  n: number;
  at: number;
  ended: number | null;
  ms: number;
  cost: number;
  checked: { done: number; total: number } | null;
  ticked: boolean | null;
  outcome: WorkflowOutcome;
}

/** Where an agent is in its project's workflow, and where it has been; `behind`: the workflow changed since its step began. */
export interface WorkflowTrack {
  agentId: string;
  projectId: string;
  workflow: string | null;
  step: string | null;
  version: string;
  outside: string | null;
  subagents: string[];
  passages: WorkflowPassage[];
  turns: WorkflowTurn[];
  /** `partial`: the harness tells only part (`missing`: subagents, cost, timing). */
  support: { partial: boolean; missing: string[] };
  since: number;
  ms: number;
  cost: number;
  ticketLoop: [number, number] | null;
  updated: number;
  behind: boolean;
}

export type UiEvent =
  | { type: 'workflowTrack'; track: WorkflowTrack }
  | { type: 'agent'; agent: Agent }
  | { type: 'agentRemoved'; id: string; projectId: string }
  | { type: 'conv'; agentId: string; ops: ConvOp[] }
  | { type: 'memory'; projectId: string }
  | { type: 'codeIndex'; projectId: string }
  | { type: 'vectorsDownload'; done: number; total: number }
  | { type: 'git'; projectId: string; git: GitInfo }
  | { type: 'usage'; usage: Usage }
  | { type: 'focus'; projectId: string; agentId: string | null }
  | { type: 'terminalExit'; id: string; code: number | null }
  | { type: 'resources'; resources: Resources }
  | { type: 'models'; models: ModelInfo[] }
  | { type: 'engineModels'; engine: EngineId; models: ToldModel[] }
  | { type: 'accountModels'; account: string; models: AccountModels }
  | { type: 'accounts'; accounts: Settings['accounts']; engineOrder: EngineId[] }
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
  /** Why no ticket of a project that goes to any account starts for now, or null once they may; `projects`: the same for the paused projects that prefer an account. */
  | { type: 'autopilotPause'; pause: AutopilotPause | null; projects?: Record<string, AutopilotPause> }
  | { type: 'toast'; text: string }
  /** The settings changed a language: the window switches to it at once. */
  | { type: 'language'; lang: LangInfo }
  /** The MCP server started, stopped, could not start, or listens on another port. */
  | { type: 'mcpStatus'; status: McpStatus }
  /** A call to the MCP server, or a request refused. */
  | { type: 'mcpActivity'; entry: McpActivityEntry }
  /** Where the MCP server stands in each active account's Claude Code. */
  | { type: 'mcpDeclared'; declared: McpDeclaration[] }
  /** The dictation: its level, its end, a model's download. */
  | { type: 'voice'; voice: VoiceEvent }
  /** A link `escouade://` came (STORE-11): taken by `deep_links_take`. */
  | { type: 'deepLink' }
  /** What a project's docs have to share (the changes not shared yet), and the conflict a « Partager » stopped on. */
  | { type: 'docsShare'; projectId: string; pending: number; conflict: DocsConflict | null }
  /** The pages of a project being compared to the code (ESC-263), told when one starts or ends. */
  | { type: 'docsCompared'; projectId: string; comparing: string[] }
  /** Lines a step of the setup of an agent's worktree wrote since the last event, `total` counting all it wrote; a step starts with none. */
  | ({ type: 'setupOutput'; agentId: string } & SetupOutput);

/** The MCP server as the backend reports it. */
export interface McpStatus {
  running: boolean;
  /** The port it listens on; stopped, the one saved for its next start (0 before the first). */
  port: number;
  /** Why it does not run though the settings want it to. */
  error: string | null;
}

/** One line of the MCP activity log: a call to a tool, or a request refused. */
export interface McpActivityEntry {
  at: number;
  /** The agent's name, « Claude (hors Escouade) », or « Client inconnu ». */
  caller: string;
  /** The tool called; empty for a request refused before it reached one. */
  tool: string;
  summary: string;
  outcome: 'ok' | 'refused' | 'error';
  /** Why it was refused or how it failed. */
  message: string | null;
}

/** Where the server stands in one account's Claude Code. */
export interface McpDeclaration {
  /** The id of the account. */
  account: string;
  /** Its harness. */
  engine: EngineId;
  ok: boolean;
  error: string | null;
  /**
   * The command that declares it by hand (for a harness whose user config Escouade writes itself, the entry to paste in
   * `file`), its token already hidden (`mcp_manual_command` gives the real one when it is copied).
   */
  command: string;
  /** The user config the entry is in, the token written there in plain text: `.claude.json`, `config.toml`, `opencode.json`, `mcp-config.json`. */
  file: string;
  /** `command` is the entry to paste in `file`; else a command to run. */
  paste: boolean;
}

/** What the step running of a worktree's setup wrote: its rank (from 0), how many lines so far, and the last of them. */
export interface SetupOutput {
  step: number;
  total: number;
  lines: string[];
}

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
  /** Claude Code is found for the account new agents go to (its own `claude`, else the settings'). */
  claudeFound: boolean;
  /** The settings' own path to Claude Code (empty: the PATH) leads to it: what the « Chemin de l'exécutable » field's hint says. */
  claudePathFound?: boolean;
  version: string;
  /** Claude Code's models as it last reported them, empty until a process has started. */
  models: ModelInfo[];
  /** The models the harnesses that list them told (Codex: `model/list`), absent before 1.8. */
  engineModels?: Partial<Record<EngineId, ToldModel[]>>;
  /** What the `claude` of each account (by its id) told of its models, absent before ESC-121. */
  accountModels?: Record<string, AccountModels>;
  /** What each harness declares: its capabilities. */
  engines: EngineDecl[];
  /** The harnesses whose program is found: the only ones offered (absent before 1.8: Claude Code, when found). */
  enginesFound?: EngineId[];
  /** Why no ticket of a project's board starts, by project (none: they may). */
  boardIssues?: Record<string, string>;
  /** Why no ticket of a project that goes to any account starts for now (none: they may). */
  autopilotPause?: AutopilotPause | null;
  /** The same for the paused projects that prefer an account, by project. */
  projectPauses?: Record<string, AutopilotPause>;
  /** The external ticket systems' accounts. */
  accounts?: AccountView[];
  /** The update installed since the app's last start, told once. */
  installed?: InstalledUpdate | null;
  /** When the automatic restart planned for an update comes. */
  restartAt?: number | null;
  /** A version that did not install at the last try. */
  failedUpdate?: string | null;
  /** What the step running of each worktree setup under way wrote, by agent. */
  setupOutput?: Record<string, SetupOutput>;
  /** The languages of the interface, of the system and of the texts Claude writes. */
  lang?: LangInfo;
  /** What the page showed when its process last ended (ESC-290): offered again, never reopened by itself. */
  pageCrash?: { projectId: string | null; agentId: string | null } | null;
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

/** What an agent used over the period; `name` is null once the agent is gone. */
export interface AgentShare {
  agentId: string;
  name: string | null;
  projectId: string;
  tokens: number;
  cost: number;
}

/** What the agents of a ticket used over the period, added up. */
export interface TicketShare {
  id: string;
  key: string;
  title: string;
  loops: number;
  cost: number;
}

/** What an account used over the period: its tokens and cost, the turns it ran and for how many agents. */
export interface AccountShare {
  /** The account's id: one the settings no longer list is possible (its turns remain). */
  account: string;
  tokens: number;
  cost: number;
  turns: number;
  agents: number;
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
  /** Every harness side by side (« Par harness »), the one that ran each turn, whichever harness the other fields are for. */
  byEngine: EngineShare[];
  byAgent: AgentShare[];
  byTicket: TicketShare[];
  /** Every account side by side, dearest first, whichever account the other fields are for. */
  byAccount: AccountShare[];
  /** Every account that ever ran a turn (ids), whichever period or account the view is for. */
  accounts: string[];
  /** Every harness that ever ran a turn (ids), whichever period or harness the view is for. */
  engines: string[];
  /** What the code index saved over the period, per project (ESC-254); absent from a view of the window's tests made before it. */
  indexByProject?: IndexSaving[];
}

/** What a harness ran over the period. */
export interface EngineShare {
  engine: string;
  tokens: number;
  cost: number;
  turns: number;
  agents: number;
}

export interface FolderInfo {
  exists: boolean;
  isRepo: boolean;
  branch: string;
  dirty: number;
  name: string;
  /** The closed project this folder was (its history, kept), if it was one. */
  history: ClosedProject | null;
  /** Its repositories when it holds several (ESC-169); empty otherwise. */
  repos?: string[];
  /** Those of `repos` without any commit. */
  reposWithoutCommit?: string[];
}

/** What the new project's assistant found in the folder (`inspect_setup`). */
export interface SetupInfo {
  /** A repository with a commit: worktrees, so tickets, can start from it. */
  hasCommit: boolean;
  /** Its local branches, the checked-out one first. */
  branches: string[];
  /** Those of the folders asked about that are no folder there. */
  missingDirs: string[];
  /** The worktrees folder is one git tracks files in: none can go there. */
  worktreesTracked: boolean;
}

/** A project closed with its history kept: what adding its folder again brings back. */
export interface ClosedProject {
  id: string;
  name: string;
  color: string;
  path: string;
  worktreePerAgent: boolean;
  /** Its agents, the archived ones left out. */
  agents: number;
  tickets: number;
  /** The latest of its agents' and tickets' dates (ms), else when it was closed. */
  lastActivity: number;
  closedAt: number;
  /** What its history takes on the disk (bytes): its file, its conversations and its setup logs. */
  size: number;
  /**
   * A file of the closed projects the app cannot read: `id` is its name without `.json`, the name, the color and
   * the folder are empty, `size` is the file's alone and the dates are its last change. It can only be erased.
   */
  unreadable: boolean;
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

/** A file attached to a message by its path (any file): a copy kept by the backend, or the original (`origin`). */
export interface AttachedFile {
  /** Shown, and given to Claude. */
  name: string;
  /** On this machine (Windows here). */
  path: string;
  /** The same, as the agent's Claude Code reads it (a Linux path under WSL); empty for a draft's. */
  claudePath: string;
  /** In bytes. */
  size: number;
  /** The file itself: nothing was copied. */
  origin: boolean;
}

/** A pull request's state on its code host. */
export type PrState = 'open' | 'closed' | 'merged';

/** The checks of a pull request's last commit, in one word. */
export type CheckState = 'pending' | 'success' | 'failure' | 'skipped';

/** A pull request (a merge request on GitLab) as its code host tells it (`forge::PullRequest`). */
export interface PullRequest {
  number: number;
  url: string;
  title: string;
  state: PrState;
  draft: boolean;
  head: string;
  base: string;
  /** Conflicts with its base: false; not known yet: null. */
  mergeable: boolean | null;
  approvals: number;
  changesRequested: boolean;
  /** Threads of comments on lines not resolved yet. */
  openThreads: number;
  autoMerge: boolean;
  /** Null when it has no check. */
  checks: CheckState | null;
  fork: boolean;
  /** Of an agent of several repositories (ESC-169): the repository it was opened in. */
  repo?: string;
  /** The repository on the host its branch is in (`ada/demo`), when the host tells it: a fork's, for one opened from a fork. */
  headRepo?: string;
  /**
   * The repository on the host it is in (`acme/demo`), when it was read elsewhere than on the remote of the project's
   * target: on the remote the agent's branch tracks, or the repository that one was forked from.
   */
  hostRepo?: string;
  /** Who opened it, when the host says it. */
  author?: string;
  /** When it was opened (Unix seconds), when the host says it. */
  created?: number;
}

/** A check or a status of a pull request's last commit (`forge::Check`). */
export interface PrCheck {
  name: string;
  state: CheckState;
  url: string | null;
  /** What its log is asked by, when the host keeps it. */
  job: string | null;
  /** Unix seconds, when the host says them: a pending one that started is running. */
  started: number | null;
  finished: number | null;
  /** Stopped before its end (a failure to the rest of the app). */
  cancelled: boolean;
}

export interface PrReview {
  author: string;
  state: 'approved' | 'changesRequested' | 'commented';
  body: string;
}

/** A comment of a pull request, on one of its lines or not. */
export interface PrComment {
  id: string;
  author: string;
  body: string;
  path: string | null;
  line: number | null;
  resolved: boolean;
}

export interface PrDeployment {
  environment: string;
  state: string;
  url: string | null;
}

/** The « PR et CI » tab's read of the agent's pull request (`prview::PrDetail`). */
export interface PrDetail {
  checks: PrCheck[];
  reviews: PrReview[];
  comments: PrComment[];
  deployments: PrDeployment[];
  can: { checks: boolean; jobLogs: boolean; rerun: boolean; reviews: boolean; deployments: boolean };
  rateLimit?: { limit: number; remaining: number; reset: number };
}

/** Why the host's answer is missing (`prview::Failure`). */
export interface PrFailure {
  kind: 'refused' | 'rateLimit' | 'unreachable' | 'other' | 'unavailable';
  message: string;
  /** When the host's limit starts over (Unix seconds). */
  reset?: number;
  /** Réglages › Intégrations settles it. */
  settings?: boolean;
}

/** What a read of the tab gave: the data, else why it is missing. */
export interface PrRead<T> {
  data?: T;
  failure?: PrFailure;
}

/** Why an agent has no « Ouvrir une PR… » (`prview::NoPr`). */
export interface NoPr {
  reason: string;
  settings: boolean;
}

/** « Ouvrir une PR »: what the model proposes, which the user reads and edits (`agentprs::PrProposal`). */
export interface PrProposal {
  title: string;
  body: string;
  head: string;
  base: string;
  word: string;
  host: string;
  canDraft: boolean;
  /** Files of its worktree not committed: not in the pull request. */
  uncommitted: number;
  /** The model did not answer: made from the commits. */
  fallback: boolean;
  /** Of an agent of several repositories (ESC-169): one pull request in each. */
  repos?: PrRepo[];
}

/** One repository of an agent's pull requests (ESC-169). */
export interface PrRepo {
  repo: string;
  /** Its repository on the host (`acme/api`). */
  remote: string;
  base: string;
  commits: number;
}

/** What a correction hands the agent: the failed checks, or the review's feedback. */
export type PrFix = 'checks' | 'reviews';

/** What « Nettoyer » found in a project's worktrees (ESC-143): those removed, and those kept with why. */
export interface LeftoverReport {
  removed: number;
  kept: { agentId: string; name: string; reason: string }[];
}

/** What a memory of the shared memory is about (ESC-255), as Claude Code sorts its own. */
export type MemoryKind = 'feedback' | 'project' | 'reference' | 'user';

/** A memory of a project's shared memory: `.escouade/memory/<name>.md` of its folder. */
export interface Memory {
  name: string;
  description: string;
  kind: MemoryKind;
  body: string;
}

/** A memory of Claude Code's offered to the project's; `exists`: one of that name is there, which it replaces. */
export interface OfferedMemory extends Memory {
  exists: boolean;
}

/** A page of the spec (`docs::Page`, ESC-256), under `.escouade/spec` of the branch of the docs. */
export interface DocsPage {
  /** `subject/page`, without `.md`. */
  path: string;
  title: string;
  subject: string;
  sources: string[];
  tickets: string[];
  decisions: DecisionRef[];
  /** When it was last compared to the code (ISO 8601); empty: never. */
  compared: string;
  /** What the last comparison found the code does otherwise (ESC-258). */
  gaps: DocsGap[];
  /** Not settled yet. */
  draft: boolean;
  /** Its place in its subject (0: by name). */
  order: number;
  body: string;
}

/** A gap between a page of the spec and the code (ESC-258). */
export interface DocsGap {
  text: string;
  /** The files of the code concerned, relative to the project. */
  code: string[];
}

export interface DocsPageInfo {
  path: string;
  title: string;
  compared: string;
  tickets: string[];
  decisions: DecisionRef[];
  /** How many gaps with the code its last comparison found. */
  gaps: number;
  draft: boolean;
}

export interface DocsSubject {
  /** Its folder under `spec/`. */
  name: string;
  /** The name it is shown under. */
  title: string;
  pages: DocsPageInfo[];
}

/** A page as an import of an existing spec would write it (ESC-258). */
export interface DocsDraft {
  path: string;
  subject: string;
  title: string;
  source: string;
  lines: number;
  /** A page of that path is already there: left alone. */
  exists: boolean;
}

export interface DocsImported {
  written: string[];
  skipped: string[];
}

/** A committed change of the spec (« Ce qui a bougé »). */
export interface DocsMoved {
  /** Seconds since the epoch. */
  when: number;
  author: string;
  message: string;
  pages: string[];
}

/** A spec the project already has (`SPEC.md`, a `docs/` folder…), offered for import. */
export interface DocsCandidate {
  path: string;
  kind: 'file' | 'folder';
  files: number;
  bytes: number;
}

export interface DocsTree {
  /** The fixed docs branch; empty: the branch checked out. */
  branch: string;
  pages: number;
  subjects: DocsSubject[];
  /** Only when the spec has no page. */
  existingSpec: DocsCandidate[];
  /** The pages being compared to the code now (ESC-263). */
  comparing?: string[];
  /** The changes without tickets and the latest plan (ESC-262). */
  planning?: DocsPlanning | null;
}

/** What a « Comparer au code » did (ESC-263). */
export interface DocsCompared {
  pages: number;
  gaps: number;
  failed: number;
  error?: string;
}

export interface DocsHit {
  path: string;
  title: string;
  lines: string[];
}

export type DecisionState = 'proposed' | 'accepted' | 'superseded';

export interface DecisionChange {
  page: string;
  added: number;
  removed: number;
}

/** How a decision is cited (`docs::Ref`, ESC-303): an accepted one by its number, a proposal by its id (it has no number until accepted). */
export type DecisionRef = number | string;

/** A decision of the register (`docs::Decision`): an accepted one is never rewritten, a new one replaces it. */
export interface DocsDecision {
  /** Given when it is accepted (ESC-303); null for a proposal (but those numbered before). */
  number: number | null;
  /** The slug of its title, unique in the register: how a proposal is cited; kept once accepted. */
  id: string;
  title: string;
  date: string;
  state: DecisionState;
  origin: string;
  originRef: string;
  pages: string[];
  tickets: string[];
  documents: string[];
  supersedes: number[];
  supersededBy: number | null;
  changes: DecisionChange[];
  acceptedAt: string;
  acceptedBy: string;
  context: string;
  decision: string;
  consequences: string;
}

export interface DocsFile {
  /** Under `documents/`. */
  path: string;
  name: string;
  folder: string;
  size: number;
  kind: 'text' | 'pdf' | 'word' | 'image' | 'other';
  /** Escouade reads its text (text, PDF, Word, OpenDocument). */
  readable: boolean;
  /** When its file last changed, in milliseconds since the epoch. */
  modified: number;
  /** Who added it last, as git says (this machine's user while it is not committed). */
  author: string;
  /** Times the agents read it. */
  reads: number;
  /** Pages of the spec and decisions that cite it. */
  cited: number;
}

/** A folder of the drive (ESC-260). */
export interface DocsFolder {
  /** Under `documents/`. */
  path: string;
  name: string;
  /** The files in it and in its folders. */
  files: number;
}

export interface DocsDrive {
  folders: DocsFolder[];
  files: DocsFile[];
}

/** Where a file dropped went, and why: chosen (dropped in it), name, sort, meaning, words, or new. */
export interface DocsPlacement {
  folder: string;
  /** The folder was made for it. */
  new: boolean;
  by: 'chosen' | 'name' | 'sort' | 'meaning' | 'words' | 'new';
}

export interface DocsDropped {
  file: DocsFile;
  placement: DocsPlacement;
  /** The repository's LFS rules take it. */
  lfs: boolean;
}

/** What the window learns of a file before it drops it. */
export interface DocsCheck {
  name: string;
  size: number;
  lfs: boolean;
  /** Over 50 MB and no LFS rule takes it: asked first. */
  large: boolean;
}

/** A document found by « Rechercher dans les documents ». */
export interface DocsDocHit {
  path: string;
  name: string;
  folder: string;
  kind: string;
  excerpt: string;
}

export interface DocsText {
  path: string;
  kind: string;
  /** Null: Escouade does not extract the text of this format (yet). */
  text: string | null;
  total: number;
  from: number;
  truncated: boolean;
  /** What is known of a file whose content is not read (an image: its format and size). */
  meta?: string;
}

/** A conflict between two versions of a page of the docs, handed to the writer agent (« L'agent résout » / « Annuler »). */
export interface DocsConflict {
  /** The pages both sides changed. */
  files: string[];
  /** The branch that was being merged in. */
  target: string;
  /** The writer agent resolving it, once « L'agent résout » was clicked. */
  agentId: string | null;
}

/** The changes of a project's docs not shared yet. */
export interface DocsPending {
  files: string[];
  count: number;
  /** Commits of code not pushed yet: « Partager » asks to be confirmed (`withCode`) before publishing them. */
  codeCommits: number;
  conflict: DocsConflict | null;
}

/** How a « Partager » ended. */
export type DocsShared =
  | { kind: 'nothing' }
  | { kind: 'done'; committed: number; message: string; pulled: boolean }
  | { kind: 'conflict'; conflict: DocsConflict };
