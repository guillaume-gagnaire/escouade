import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  CodeIndexInfo,
  Memory,
  VectorsModelInfo,
  OfferedMemory,
  WorkflowRefineOutcome,
  WorkflowRefinePlan,
  WorkflowScan,
  SentView,
  LeftoverReport,
  Account,
  MachineAccount,
  AccountForm,
  AccountStatus,
  AccountView,
  ForgeForm,
  ForgesView,
  PrFix,
  PrProposal,
  PullRequest,
  Agent,
  AttachedFile,
  Attachment,
  BoardSettings,
  BranchInfo,
  CheckedUpdate,
  ClaudeLocated,
  ClaudeUpdateStatus,
  EngineId,
  EngineInfo,
  ClosedProject,
  CommitScope,
  Container,
  ConvItem,
  ConvSearchResult,
  DownloadedUpdate,
  ExternalIssue,
  FileBase,
  FileChange,
  FileText,
  FileTree,
  FolderInfo,
  GitLog,
  InitialState,
  Integration,
  McpActivityEntry,
  McpDeclaration,
  McpStatus,
  OwnedDiff,
  IsolaService,
  IssuePage,
  Project,
  RepoMerge,
  RunSuggestion,
  SearchQuery,
  SearchResult,
  Service,
  Settings,
  ShareMode,
  ShellInfo,
  SlashCommand,
  StatesView,
  StatsView,
  SubagentRead,
  TaskOutput,
  TermInfo,
  TestRecipe,
  Ticket,
  TicketDraft,
  TicketImageData,
  TranscriptCursor,
  UiEvent,
  UiState,
  VoiceInfo,
  ServiceCatalog,
  SpeechInfo,
  ConvStarted,
  VoiceStarted,
  WorktreeSuggestion,
  SetupInfo,
  Workflow,
  WorkflowTrack,
  DocsCandidate,
  DocsCompared,
  DocsDecision,
  DocsCheck,
  DiscussionPoured,
  DocsPlan,
  DocsPlanCancelled,
  DiscussionStart,
  DocsDocHit,
  DocsDrive,
  DocsDropped,
  DocsFile,
  DocsHit,
  DocsPage,
  DocsText,
  DocsPending,
  DocsShared,
  DocsConflict,
  DocsTree,
  DocsDraft,
  DocsImported,
  DocsMoved,
} from './types';

/**
 * What a terminal tells the app's log (see pty.rs): which renderer draws it, or that an echo was slow. Durations only, in
 * ms (fractional) from `performance.now()`: the window's clock is never compared with the backend's.
 */
export type TermReport =
  /** `detail`: the GPU's name for WebGL (a software one, SwiftShader, would say so), or why WebGL is not used. */
  | { kind: 'renderer'; renderer: 'webgl' | 'dom'; detail?: string }
  /**
   * The write numbered `seq` by `term_write` (`bytes` of UTF-8) took `sentToReceived` from leaving the window to the output
   * that answers it, and `receivedToPainted` from that output to its paint. `sentToAcked`: from the call of `term_write` to
   * the resolution of its promise (the way to the backend and back). `outputEnd`: the bytes of output the terminal had
   * received, the echo's chunk included, when that chunk came in: every read of the backend is one message of the same
   * bytes, in order, so this tells which read was the echo.
   */
  | {
      kind: 'slowEcho';
      seq: number;
      bytes: number;
      sentToReceived: number;
      receivedToPainted: number;
      sentToAcked?: number;
      outputEnd?: number;
    };

/** The repository of a project of several a git command goes to (ESC-169): no key at all for a project of one. */
function inRepo(repo: string | null): { repo?: string } {
  return repo === null ? {} : { repo };
}

export const api = {
  subscribe: (onEvent: (e: UiEvent) => void) => {
    const channel = new Channel<UiEvent>();
    channel.onmessage = onEvent;
    return invoke<InitialState>('subscribe', { channel });
  },
  setUi: (ui: UiState) => invoke<void>('set_ui', { ui }),
  mcpStatus: () => invoke<McpStatus>('mcp_status'),
  /** The MCP server's activity log, oldest first (the last 200). */
  mcpActivity: () => invoke<McpActivityEntry[]>('mcp_activity'),
  mcpClearActivity: () => invoke<void>('mcp_clear_activity'),
  /** « Les agents peuvent piloter Escouade »: saved at once; the declaration in each account of every harness installed follows (`mcpDeclared`). */
  mcpSetEnabled: (enabled: boolean) => invoke<void>('mcp_set_enabled', { enabled }),
  /** Where the server stands in each active account of every harness installed (the command to run, or the entry to paste, by hand has its token hidden). */
  mcpDeclareStatus: () => invoke<McpDeclaration[]>('mcp_declare_status'),
  /** The command that declares the server by hand in the account, with the real token: asked for when it is copied. */
  mcpManualCommand: (account: string) => invoke<string>('mcp_manual_command', { account }),
  /** The answer to « Statistiques d’usage », saved at once. */
  telemetrySet: (consent: boolean) => invoke<void>('telemetry_set', { consent }),
  /** The dictation (Réglages › Voix): the microphones and the models. */
  voiceInfo: () => invoke<VoiceInfo>('voice_info'),
  /** Starts listening with the settings' microphone and model, or says the model is to be downloaded first. */
  voiceStart: () => invoke<VoiceStarted>('voice_start'),
  /** Stops listening and gives the text heard (empty: nothing said). `target`: where it goes, for the usage statistics. */
  voiceStop: (session: number, target: string) => invoke<string>('voice_stop', { session, target }),
  voiceCancel: () => invoke<void>('voice_cancel'),
  /** Its progress comes as `voice` events; resolves once the model is on the disk, its checksum checked. */
  voiceDownload: (model: string) => invoke<void>('voice_download', { model }),
  voiceDownloadCancel: (model: string) => invoke<void>('voice_download_cancel', { model }),
  voiceDeleteModel: (model: string) => invoke<void>('voice_delete_model', { model }),
  /** The services of « En ligne » whose key is in the keychain; never the keys. */
  voiceOnlineKeys: () => invoke<string[]>('voice_online_keys'),
  /** Keeps a service's key in the system keychain (never in a file); fails when the keychain refuses it. */
  voiceOnlineKeySet: (service: string, key: string) => invoke<void>('voice_online_key_set', { service, key }),
  voiceOnlineKeyDelete: (service: string) => invoke<void>('voice_online_key_delete', { service }),
  /** The models and voices of a service on line (ESC-132): what was listed, read again with the key when `refresh`. */
  voiceOnlineCatalog: (service: string, refresh = false) => invoke<ServiceCatalog>('voice_online_catalog', { service, refresh }),
  // The voice conversation (ESC-97).
  speechInfo: () => invoke<SpeechInfo>('speech_info'),
  conversationStart: () => invoke<ConvStarted>('conversation_start'),
  conversationListen: (session: number) => invoke<void>('conversation_listen', { session }),
  conversationHold: (session: number) => invoke<void>('conversation_hold', { session }),
  /** Reads `text` aloud: the number of the reading, which its end (`convSpoken`) and a barge-in carry. */
  conversationSay: (session: number, text: string) => invoke<number>('conversation_say', { session, text }),
  conversationStop: () => invoke<void>('conversation_stop'),
  /** A sentence said, sent to the agent with the instruction to end with a passage to be read aloud. */
  conversationSend: (id: string, text: string) => invoke<void>('conversation_send', { id, text }),
  speechDownload: (id: string) => invoke<void>('speech_download', { id }),
  speechDownloadCancel: (id: string) => invoke<void>('speech_download_cancel', { id }),
  speechDelete: (id: string) => invoke<void>('speech_delete', { id }),
  /** « Écouter »: resolves once heard. */
  speechPreview: (id: string, speaker?: number) => invoke<void>('speech_preview', { id, speaker }),
  /** « ▶ Écouter » on line: a sentence read by `service` with `model` and `voiceId` in `lang`. */
  speechPreviewOnline: (service: string, model: string, voiceId: string, lang: string) =>
    invoke<void>('speech_preview_online', { service, model, voiceId, lang }),
  /** The settings, but for the Claude accounts (their tab changes them with the `account*` calls below). */
  saveSettings: (settings: Settings) => invoke<ShellInfo[]>('save_settings', { settings }),
  /** Where the `claude` of `path` is (blank: the one found by itself), `wslDistro` the distribution for one of WSL. */
  locateClaude: (path: string, wslDistro: string) => invoke<ClaudeLocated>('claude_locate', { path, wslDistro }),
  /** Whether Claude Code is found for the account new agents go to, and by the settings' own path (as at the start). */
  claudeFound: () => invoke<{ claudeFound: boolean; claudePathFound: boolean; enginesFound?: EngineId[] }>('claude_found'),
  /** Where Escouade's update of Claude Code stands (ESC-133). */
  claudeUpdateStatus: () => invoke<ClaudeUpdateStatus>('claude_update_status'),
  dataDir: () => invoke<string>('data_dir'),
  /** Every harness with what is found of it here: its program, version and state (versions read again). */
  engines: () => invoke<EngineInfo[]>('engines'),
  /** The harnesses found now, the PATH of today read first (ESC-186). */
  enginesFoundNow: () => invoke<EngineId[]>('engines_found_now'),
  /** The items of Principal's folder a new account may share (those it has), files first. */
  accountShareable: (engine?: EngineId) => invoke<string[]>('account_shareable', engine ? { engine } : {}),
  /** A new account of `engine` (Claude Code's by default), its folder made with the items `share` of its Principal's linked or copied, saved last. */
  accountCreate: (name: string, share: string[], mode: ShareMode, engine?: EngineId) =>
    invoke<Account>('account_create', engine ? { name, share, mode, engine } : { name, share, mode }),
  /** « Le compte de la machine » of `engine` added (its Principal: its usual folder, as it is). */
  accountAddMachine: (engine: EngineId) => invoke<Account>('account_add_machine', { engine }),
  /** Whether the machine's account of `engine` is signed in, with whom, and whether it is added already. */
  accountMachine: (engine: EngineId) => invoke<MachineAccount>('account_machine', { engine }),
  /** The account's name, `claude`, « Actif » and the defaults of its new agents (its folder stays its own); the accounts as they then are. */
  accountUpdate: (account: Account) => invoke<Account[]>('account_update', { account }),
  /** The accounts in the order of `ids`, as they then are. */
  accountReorder: (ids: string[]) => invoke<Account[]>('account_reorder', { ids }),
  /** « Ordre des harness » set; the order as it then is. */
  engineOrderSet: (order: EngineId[]) => invoke<EngineId[]>('engine_order_set', { order }),
  favoriteModelSet: (engine: EngineId, model: string, favorite: boolean) =>
    invoke<Partial<Record<EngineId, string[]>>>('favorite_model_set', { engine, model, favorite }),
  /** The account removed, its folder left on the disk; refused while an agent not archived runs on it. */
  accountRemove: (id: string) => invoke<Account[]>('account_remove', { id }),
  accountStatus: (id: string) => invoke<AccountStatus>('account_status', { id }),
  /** The menu of the models of an account opened: they are read again when a day old (ESC-186), told by `accountModels`. */
  accountModelsRefresh: (id: string) => invoke<void>('account_models_refresh', { id }),
  /** The API key of a Cursor account other than its machine's (ESC-43), kept in the system's keychain; its status as it then is. */
  accountKey: (id: string, key: string) => invoke<AccountStatus>('account_key', { id, key }),
  /** What stopped the account's sign-in before it was signed in, when it is known (its program not found, an installation cut half-way); null otherwise. */
  accountSignInEnded: (id: string) => invoke<string | null>('account_sign_in_ended', { id }),
  /** The account's `claude` in an interactive terminal, to sign in with (written to and killed as a terminal). */
  accountLogin: (id: string, cols: number, rows: number, onData: (d: ArrayBuffer) => void) => {
    const output = new Channel<ArrayBuffer>();
    output.onmessage = onData;
    return invoke<TermInfo>('account_login', { id, cols, rows, output });
  },
  /** `historyId`: the history of the folder to say (a closed project's, from « Rouvrir »), else its latest closed project's. */
  inspectFolder: (path: string, historyId?: string) => invoke<FolderInfo>('inspect_folder', historyId ? { path, historyId } : { path }),
  /**
   * `restore`: the folder of a closed project gets its history (same id, agents, tickets) back; else a project of its
   * own. `historyId`: which of the folder's histories comes back (the latest closed by default).
   */
  createProject: (a: {
    path: string;
    name: string;
    color: string;
    worktreePerAgent: boolean;
    firstAgent: string | null;
    restore: boolean;
    historyId?: string;
  }) => invoke<Project>('create_project', a),
  /** What the new project's assistant checks in the folder: a commit, its branches, the folders `dirs` missing, the worktrees folder tracked. */
  inspectSetup: (path: string, worktreeDir: string, dirs: string[]) => invoke<SetupInfo>('inspect_setup', { path, worktreeDir, dirs }),
  /** « Faire un premier commit »: the folder made a repository if need be, with an empty first commit. */
  firstCommit: (path: string) => invoke<void>('first_commit', { path }),
  updateProject: (project: Project) => invoke<void>('update_project', { project }),
  /** « Choisir une image… » (`path`, copied under the project) or « Retirer » (null) of its logo (ESC-167). */
  setProjectLogo: (id: string, path: string | null) => invoke<Project>('set_project_logo', { id, path }),
  /** The project’s logo as a `data:` URL, for its tile; null when it has none. */
  projectLogo: (id: string) => invoke<string | null>('project_logo', { id }),
  /** The image of `path` as a logo would be taken (`data:` URL), or why it would be refused. */
  readLogo: (path: string) => invoke<string>('read_logo', { path }),
  /** « Index du code » of a project (ESC-254): whether it is on, what it holds, what its fence leaves out. */
  getCodeIndex: (id: string) => invoke<CodeIndexInfo>('get_code_index', { id }),
  /** Turns it on (answers once the project is read) or off (its database erased), the project's choice; null follows the app's setting again. */
  setCodeIndex: (id: string, enabled: boolean | null) => invoke<CodeIndexInfo>('set_code_index', { id, enabled }),
  /** « Index du code sur tous les projets » (ESC-293), saved at once: on, the projects that follow it read one after the other; off, their index erased. How many follow it. */
  setCodeIndexAll: (enabled: boolean) => invoke<number>('set_code_index_all', { enabled }),
  /** « Recherche par le sens »: the vectors' model, its download (progress in `vectorsDownload`), « Annuler », « Effacer ». */
  vectorsModel: () => invoke<VectorsModelInfo>('vectors_model'),
  vectorsModelDownload: () => invoke<VectorsModelInfo>('vectors_model_download'),
  vectorsModelCancel: () => invoke<void>('vectors_model_cancel'),
  vectorsModelDelete: () => invoke<VectorsModelInfo>('vectors_model_delete'),
  /** The project’s shared memory (ESC-255), by name. */
  memoryList: (projectId: string) => invoke<Memory[]>('memory_list', { projectId }),
  /** « Oublier » a memory of the project’s shared memory. */
  memoryForget: (projectId: string, name: string) => invoke<void>('memory_forget', { projectId, name }),
  /** What Claude Code remembers of the project, offered once: empty once offered, or when it has nothing. */
  claudeMemoryOffer: (projectId: string) => invoke<OfferedMemory[]>('claude_memory_offer', { projectId }),
  /** « Reprendre » the memories ticked (none: « Pas maintenant »): the offer is not made again. */
  claudeMemoryTake: (projectId: string, names: string[]) => invoke<Project>('claude_memory_take', { projectId, names }),
  /** « Faire confiance à ces commandes »: those of the project’s `.escouade.yml` read (`hash`). */
  sharedTrust: (projectId: string, hash: string) => invoke<Project>('shared_trust', { projectId, hash }),
  /** « Exporter vers .escouade.yml » (no `keys`), or « Écrire dans .escouade.yml » of the keys given. */
  sharedWrite: (projectId: string, keys?: string[]) => invoke<Project>('shared_write', keys ? { projectId, keys } : { projectId }),
  /** « Reprendre la valeur du fichier »: a key kept on this machine follows the file again. */
  sharedUnkeep: (projectId: string, key: string) => invoke<Project>('shared_unkeep', { projectId, key }),
  /** The settings the `.escouade.yml` of a folder gives a new project, its commands left out; null without one. */
  sharedPreview: (path: string) => invoke<Project | null>('shared_preview', { path }),
  reorderProjects: (ids: string[]) => invoke<void>('reorder_projects', { ids }),
  /** `erase`: its history goes too (conversations, tickets, settings); else it is kept as a closed project. */
  removeProject: (id: string, erase: boolean) => invoke<void>('remove_project', { id, erase }),
  /** The projects closed with their history kept, the latest closed first. */
  closedProjects: () => invoke<ClosedProject[]>('closed_projects'),
  /** The history of a closed project, erased for good (an entry the app cannot read: its one file). */
  eraseClosedProject: (id: string) => invoke<void>('erase_closed_project', { id }),
  /**
   * `isolated`: in a worktree of its own or not, whatever « Un worktree par agent » says; null: as it says. `base`: the
   * branch its worktree starts from (« Nouvel agent depuis une autre branche… », ESC-160), which asks for one; null: the
   * project's.
   */
  createAgent: (
    projectId: string,
    model: string | null = null,
    engine: EngineId | null = null,
    isolated: boolean | null = null,
    base: string | null = null,
  ) => invoke<Agent>('create_agent', { projectId, model, engine, isolated, base }),
  createAgentOnBranch: (projectId: string, branch: string, model: string | null = null, engine: EngineId | null = null) =>
    invoke<Agent>('create_agent_on_branch', { projectId, branch, model, engine }),
  /**
   * An agent on the new branch `branch`, made first from `base` (ESC-160), else from where a new agent starts; refused
   * for a name git refuses (ESC-155).
   */
  createAgentOnNewBranch: (projectId: string, branch: string, base: string | null = null) =>
    invoke<Agent>('create_agent_on_branch', { projectId, branch, model: null, engine: null, create: true, base }),
  warmAgent: (id: string) => invoke<void>('warm_agent', { id }),
  /** « Relancer »: the agent's process, at rest, started again on the same session (« Outils d'Escouade au prochain démarrage »). */
  restartAgentProcess: (id: string) => invoke<void>('restart_agent_process', { id }),
  getConversation: (id: string) => invoke<ConvItem[]>('get_conversation', { id }),
  /** Searches the conversations of a project's agents (all projects' with null), archived agents included or not. */
  searchConversations: (query: string, projectId: string | null, archived: boolean) =>
    invoke<ConvSearchResult>('search_conversations', { query, projectId, archived }),
  /** `files`: those attached by their path (copies the app holds, or the files themselves): their paths go to Claude after the text. */
  sendMessage: (id: string, text: string, attachments: Attachment[] = [], files: AttachedFile[] = []) =>
    invoke<void>('send_message', { id, text, attachments, files }),
  /** Opens a copy to write, for `owner` (an agent's id, or « draft »): the handle of the write. Refused over the maximum size. */
  attachOpen: (owner: string, name: string, size: number) => invoke<string>('attach_open', { owner, name, size }),
  /** The next bytes of a copy, raw (not base64 in a JSON): the handle goes in a header. */
  attachChunk: (handle: string, bytes: Uint8Array) => invoke<void>('attach_chunk', bytes, { headers: { 'x-handle': handle } }),
  attachFinish: (handle: string) => invoke<AttachedFile>('attach_finish', { handle }),
  /** Erases a copy half written. */
  attachCancel: (handle: string) => invoke<void>('attach_cancel', { handle }),
  /** Erases a copy finished (its chip removed); the backend refuses any file that is not one of `owner`'s copies. */
  attachDiscard: (owner: string, path: string) => invoke<void>('attach_discard', { owner, path }),
  /**
   * A file attached where it is: checked, not copied. `id` is the one `fileOrigin` was asked for: the backend finds the path
   * itself, from what the host read, and refuses an id it does not know or has given out already. The page never gives a path.
   */
  attachOrigin: (owner: string, id: string) => invoke<AttachedFile>('attach_origin', { owner, id }),
  /**
   * The path of the file the window gave the WebView under `id` (see `file-origin.ts`); null when it is no file of the disk. The
   * backend keeps the path for `attachOrigin`: the page has no use for it but to know the file is one of the disk.
   */
  fileOrigin: (id: string) => invoke<string | null>('file_origin', { id }),
  /** The image on the system's clipboard as PNG, empty without one: read for WebKitGTK (Linux), which leaves it out of a paste. */
  clipboardImage: () => invoke<ArrayBuffer>('clipboard_image'),
  interrupt: (id: string) => invoke<void>('interrupt', { id }),
  /** « Arrêter » on a subagent of the agent `id` (`call`: the call that launched it), by Claude Code's `stop_task`. */
  stopSubagent: (id: string, call: string) => invoke<void>('stop_subagent', { id, call }),
  /** « Arrêter » on a background task of the agent `id` (`task`: a `BgTask` id or a `WorkflowRun` id), by Claude Code's `stop_task`. Refused when it does not run, in the Claude Code view, or with no process. */
  stopBgTask: (id: string, task: string) => invoke<void>('stop_bg_task', { id, task }),
  /** The end of the output of a background task of the agent `id` (8 KiB at most while it runs, 256 KiB once it ended): the running process's, else its output file's. */
  bgTaskOutput: (id: string, task: string) => invoke<TaskOutput>('task_output', { id, task }),
  /** The conversation of a subagent in the Claude Code view, read from the transcript Claude Code keeps of it (read-only; no items while there is none). `cursor` is the one the read before gave: null back when its files did not change since. */
  subagentTranscript: (id: string, call: string, cursor: TranscriptCursor | null = null) =>
    invoke<SubagentRead | null>('subagent_transcript', { id, call, cursor }),
  /** A message to a subagent of the agent `id`: the agent is asked to pass it on (Claude Code lets no host write to a subagent). */
  messageSubagent: (id: string, call: string, text: string) => invoke<void>('message_subagent', { id, call, text }),
  answerQuestion: (id: string, requestId: string, answers: Record<string, string>) =>
    invoke<void>('answer_question', { id, requestId, answers }),
  answerPermission: (id: string, requestId: string, decision: 'allow' | 'always' | 'deny', message: string | null = null) =>
    invoke<void>('answer_permission', { id, requestId, decision, message }),
  setAgentOptions: (id: string, o: { model?: string; effort?: string; mode?: string }) =>
    invoke<void>('set_agent_options', { id, model: o.model ?? null, effort: o.effort ?? null, mode: o.mode ?? null }),
  /** The Claude account of an agent that has not started (empty: « Automatique », the backend chooses). */
  setAgentAccount: (id: string, account: string) => invoke<void>('set_agent_account', { id, account }),
  /** « Reprendre sur <compte> »: the agent stopped by the usage limit goes on, with its session, on another account. */
  resumeOnAccount: (agentId: string, account: string) => invoke<void>('resume_on_account', { agentId, account }),
  /** « Continuer sur <harness> » and « Revenir sur <harness> »: the agent goes on on another harness, by a handover. */
  continueOnEngine: (agentId: string, engine: EngineId) => invoke<void>('continue_on_engine', { agentId, engine }),
  /** « Revenir sur <compte> »: a resume on another account failed; the agent goes back to the account it came from. */
  backToPreviousAccount: (agentId: string) => invoke<void>('back_to_previous_account', { agentId }),
  /** Renames the agent, and its own branch after it: why the branch kept its name, when it did. */
  renameAgent: (id: string, name: string) => invoke<string | null>('rename_agent', { id, name }),
  /** A copy of the agent, « <nom> (copie) », whose Claude Code session forks the original’s (refused during its turn). */
  duplicateAgent: (id: string) => invoke<Agent>('duplicate_agent', { id }),
  archiveAgent: (id: string, archived: boolean) => invoke<void>('archive_agent', { id, archived }),
  setAgentView: (id: string, terminal: boolean) => invoke<void>('set_agent_view', { id, terminal }),
  /**
   * Starts the agent's Claude Code interface. `manual`: the user asked for it (« Relancer »); the backend refuses one that is
   * not, after two stops in a row as soon as it started.
   */
  agentTerminal: (id: string, cols: number, rows: number, onData: (d: ArrayBuffer) => void, manual = false) => {
    const output = new Channel<ArrayBuffer>();
    output.onmessage = onData;
    return invoke<TermInfo>('agent_terminal', { id, cols, rows, output, manual });
  },
  /** Whether the agent's interface asks the question about the trust of its folder (Codex, Copilot CLI), answered in its terminal. */
  tuiAsksTrust: (id: string) => invoke<boolean>('tui_asks_trust', { id }),
  /** An image pasted in the agent's Claude Code view (base64), written to a file: the path to type there, as its `claude` reads it. */
  pasteImage: (id: string, mediaType: string, data: string) => invoke<string>('paste_image', { id, mediaType, data }),
  deleteAgent: (id: string, removeWorktree: boolean) => invoke<string | null>('delete_agent', { id, removeWorktree }),
  /** Merges into the agent's base branch; `switchToBase`: the project's folder is switched to it first. */
  mergeAgent: (id: string, squash: boolean, switchToBase = false) => invoke<string>('merge_agent', { id, squash, switchToBase }),
  /** What merging the agent would do in each of its repositories (ESC-169). */
  agentMergeCheck: (id: string) => invoke<RepoMerge[]>('agent_merge_check', { id }),
  /** « Merger api seul » (ESC-169): the merge of some of the agent's repositories. */
  mergeAgentRepos: (id: string, squash: boolean, switchToBase: boolean, repos: string[]) =>
    invoke<string>('merge_agent_repos', { id, squash, switchToBase, repos }),
  /** « Rechercher à nouveau » (ESC-169): the project's repositories looked for again. */
  refreshRepos: (id: string) => invoke<Project>('refresh_repos', { id }),
  /** The project's workflows as last read, read again if a file changed (ESC-192). */
  getWorkflows: (id: string) => invoke<WorkflowScan>('get_workflows', { id }),
  /** « Relire » its workflows. */
  refreshWorkflows: (id: string) => invoke<WorkflowScan>('refresh_workflows', { id }),
  /** What « Affiner… » will do, said before it is asked (ESC-196). */
  workflowRefinePlan: (id: string) => invoke<WorkflowRefinePlan>('workflow_refine_plan', { id }),
  /** « Affiner… »: a short agent refines the workflows, the rules check what it says. */
  refineWorkflows: (id: string) => invoke<WorkflowRefineOutcome>('refine_workflows', { id }),
  /** « Retirer du dépôt »: the file of a workflow this machine does not read (ESC-195). */
  removeWorkflowFile: (id: string, workflow: string) => invoke<WorkflowScan>('remove_workflow_file', { id, workflow }),
  /** « Oublier » a repository gone from the project's folder. */
  forgetRepo: (id: string, repo: string) => invoke<Project>('forget_repo', { id, repo }),
  integrateBase: (id: string) => invoke<Integration>('integrate_base', { id }),
  /** « Ouvrir une PR » (ESC-64): the title and the description proposed from the agent's branch. */
  agentPrPropose: (id: string) => invoke<PrProposal>('agent_pr_propose', { id }),
  /** Pushes the agent's branch, then opens its pull request as the user validated it. */
  agentPrOpen: (id: string, form: { title: string; body: string; base: string; draft: boolean }) =>
    invoke<PullRequest>('agent_pr_open', { id, form }),
  /** Reads the agent's pull requests again: at most every 20 s, unless `now`. */
  agentPrsRefresh: (id: string, now = false) => invoke<void>('agent_prs_refresh', { id, now }),
  /**
   * Switches the agent's worktree to `name` (a remote branch "origin/x": the local one that tracks it, made if need be);
   * the branch it is on. Rejected with `IN_WORKTREE:<agent id>:<agent name>`, or in words.
   */
  agentBranchSwitch: (id: string, name: string) => invoke<string>('agent_branch_switch', { id, name }),
  /** Fetch, pull (fast-forward only) or push of the agent's own folder; a summary. */
  agentGitSync: (id: string, op: 'pull' | 'push' | 'fetch') => invoke<string>('agent_git_sync', { id, op }),
  /** « L'agent corrige » / « L'agent corrige les retours » on pull request `number`. */
  /** `hostRepo`: the repository it was read in, when not the target's (`PullRequest.hostRepo`). */
  agentPrFix: (id: string, number: number, fix: PrFix, hostRepo?: string) =>
    invoke<void>('agent_pr_fix', { id, number, fix, ...(hostRepo ? { hostRepo } : {}) }),
  getCommands: (id: string) => invoke<SlashCommand[]>('get_commands', { id }),
  fileSuggestions: (id: string, query: string) => invoke<string[]>('file_suggestions', { id, query }),
  gitFiles: (projectId: string, agentId: string | null) => invoke<FileChange[]>('git_files', { projectId, agentId }),
  gitDiff: (projectId: string, agentId: string | null, paths: string[]) => invoke<string>('git_diff', { projectId, agentId, paths }),
  gitProjectDiff: (projectId: string) => invoke<OwnedDiff[]>('git_project_diff', { projectId }),
  /** `repo`: one of the project's repositories when it has several (ESC-169); the first one when null. */
  /** The project's workflows as the user corrected them (ESC-195), those dismissed left out: what the Workflow tab shows. */
  effectiveWorkflows: (id: string) => invoke<Workflow[]>('effective_workflows', { id }),
  /** Where each agent of the project is in its workflows, and where it has been (ESC-193). */
  getWorkflowTracks: (id: string) => invoke<WorkflowTrack[]>('get_workflow_tracks', { id }),
  /** The spec's tree by subject, and the spec the project already has when it has no page (ESC-256). */
  docsSpecTree: (id: string) => invoke<DocsTree>('docs_spec_tree', { id }),
  docsDetectSpec: (id: string) => invoke<DocsCandidate[]>('docs_detect_spec', { id }),
  docsSpecPage: (id: string, path: string) => invoke<DocsPage>('docs_spec_page', { id, path }),
  /** `subject`: the name the page's subject is shown under (a new page's), left as it is when null. */
  docsSpecWrite: (id: string, path: string, title: string | null, body: string | null, subject: string | null = null) =>
    invoke<DocsPage>('docs_spec_write', { id, path, title, body, subject }),
  /** « Comparer au code » (ESC-263): `pages`, or the whole spec when null; ends with them compared. */
  docsCompare: (id: string, pages: string[] | null) => invoke<DocsCompared>('docs_compare', { id, pages }),
  docsSpecSearch: (id: string, query: string) => invoke<DocsHit[]>('docs_spec_search', { id, query }),
  docsDecisions: (id: string) => invoke<DocsDecision[]>('docs_decisions', { id }),
  docsDecisionPropose: (id: string, number: number | null, decision: Partial<DocsDecision>) =>
    invoke<DocsDecision>('docs_decision_propose', { id, number, decision }),
  docsDecisionReplace: (id: string, number: number, decision: Partial<DocsDecision>) =>
    invoke<DocsDecision>('docs_decision_replace', { id, number, decision }),
  /** « Accepter »: a human's act, the only way to accept a decision. */
  docsDecisionAccept: (id: string, number: number) => invoke<DocsDecision>('docs_decision_accept', { id, number }),
  docsDocuments: (id: string, folder: string | null = null) => invoke<DocsFile[]>('docs_documents', { id, folder }),
  docsDocumentText: (id: string, path: string, from: number | null = null) => invoke<DocsText>('docs_document_text', { id, path, from }),
  /**
   * A file of this machine put in the drive (ESC-260): in `folder`, or filed by Escouade when null. Over 50 MB without an
   * LFS rule that takes it, it asks to be `confirmed` first.
   */
  docsDocumentAdd: (id: string, folder: string | null, source: string, confirmed = false) =>
    invoke<DocsDropped>('docs_document_add', { id, folder, source, confirmed }),
  docsDrive: (id: string) => invoke<DocsDrive>('docs_drive', { id }),
  docsDocumentCheck: (id: string, folder: string | null, files: { name: string; size: number }[]) =>
    invoke<DocsCheck[]>('docs_document_check', { id, folder, files }),
  docsDocumentMove: (id: string, path: string, folder: string) => invoke<DocsFile>('docs_document_move', { id, path, folder }),
  docsFolderAdd: (id: string, folder: string) => invoke<string>('docs_folder_add', { id, folder }),
  docsDocumentsSearch: (id: string, query: string) => invoke<DocsDocHit[]>('docs_documents_search', { id, query }),
  docsPending: (id: string) => invoke<DocsPending>('docs_pending', { id }),
  docsShare: (id: string, message: string | null = null, withCode = false) => invoke<DocsShared>('docs_share', { id, message, withCode }),
  docsShareResolve: (id: string) => invoke<DocsConflict>('docs_share_resolve', { id }),
  docsShareCancel: (id: string) => invoke<void>('docs_share_cancel', { id }),
  /** « Commencer » of « Nouvelle discussion » (ESC-261): its agent, in the folder of the docs branch, sent its subject. */
  discussionStart: (id: string, start: DiscussionStart) => invoke<Agent>('discussion_start', { id, start }),
  /** « Verser dans la spec »: what the discussion proposes, written on the docs branch (nothing committed). */
  discussionPour: (agent: string) => invoke<DiscussionPoured>('discussion_pour', { agent }),
  /** « Planifier » (ESC-262): the changes poured by the discussion `agent`, or every change without tickets when null,
   * made into tickets « À faire » by the planner (not started). */
  docsPlan: (id: string, agent: string | null = null) => invoke<DocsPlan>('docs_plan', { id, agent }),
  /** « Annuler » of a planning: its tickets not started yet removed, its changes to plan again. */
  docsPlanCancel: (id: string, plan: string) => invoke<DocsPlanCancelled>('docs_plan_cancel', { id, plan }),
  /** Takes a page (its path) or a decision (from 1) out of « Ce qui ira dans la spec ». */
  discussionDrop: (agent: string, page: string | null, decision: number | null) =>
    invoke<void>('discussion_drop', { agent, page, decision }),
  /** How a spec the project has (`path`, one of `docsDetectSpec`'s) would be cut by subject (ESC-258): nothing written. */
  docsImportPlan: (id: string, path: string) => invoke<DocsDraft[]>('docs_import_plan', { id, path }),
  /** Writes the pages of `only` (all when null) of that cut; none overwritten. */
  docsImport: (id: string, path: string, only: string[] | null = null) => invoke<DocsImported>('docs_import', { id, path, only }),
  docsRecent: (id: string) => invoke<DocsMoved[]>('docs_recent', { id }),
  gitLog: (projectId: string, agentId: string | null, repo: string | null = null) =>
    invoke<GitLog>('git_log', { projectId, agentId, ...inRepo(repo) }),
  gitShow: (projectId: string, hash: string, repo: string | null = null) =>
    invoke<string>('git_show', { projectId, hash, ...inRepo(repo) }),
  /** Fetch, pull (fast-forward only) and push of the project's checkout; each returns a summary. */
  gitFetch: (projectId: string, repo: string | null = null) => invoke<string>('git_fetch', { projectId, ...inRepo(repo) }),
  gitPull: (projectId: string, repo: string | null = null) => invoke<string>('git_pull', { projectId, ...inRepo(repo) }),
  gitPush: (projectId: string, repo: string | null = null) => invoke<string>('git_push', { projectId, ...inRepo(repo) }),
  /** The project's branches: the local ones (the folder's first), then the remote ones. */
  branchList: (projectId: string, repo: string | null = null) => invoke<BranchInfo[]>('branch_list', { projectId, ...inRepo(repo) }),
  /** Rejected with why `name` can't be a new branch (git's rules, a branch of that name already there). */
  branchCheck: (projectId: string, name: string, repo: string | null = null) =>
    invoke<void>('branch_check', { projectId, name, ...inRepo(repo) }),
  /**
   * Switches the project's folder to `name` (a remote branch "origin/x": the local one that tracks it, made if need be); the stash's name when
   * `stash` put the uncommitted changes aside. Rejected with `DIRTY`, `IN_WORKTREE:<agent id>:<agent name>`, `AGENT_WORKING:<agent id>:<agent name>`.
   */
  branchSwitch: (projectId: string, name: string, stash = false, repo: string | null = null) =>
    invoke<string | null>('branch_switch', { projectId, name, stash, ...inRepo(repo) }),
  /** Creates `name` at `start` (a branch, a commit; HEAD when empty); `switchTo`: the folder goes there, rejected as `branchSwitch` is. */
  branchCreate: (projectId: string, name: string, start: string, switchTo: boolean, stash = false, repo: string | null = null) =>
    invoke<string | null>('branch_create', { projectId, name, start, switch: switchTo, stash, ...inRepo(repo) }),
  /**
   * Deletes a branch, its remote one too with `remote` (or a remote branch alone, "origin/x"). Rejected with `UNMERGED:<n>` (`n` commits in no
   * other branch, maybe 0) unless `force`, `IN_WORKTREE:<agent id>:<agent name>`, or in words for the folder's own branch. Resolves with a
   * sentence to show when `remote` was asked but only the local branch went (its remote copy is the default branch, the base's, named
   * otherwise, or tracked by another branch), else null.
   */
  branchDelete: (projectId: string, name: string, remote: boolean, force = false, repo: string | null = null) =>
    invoke<string | null>('branch_delete', { projectId, name, remote, force, ...inRepo(repo) }),
  /** The local branches already in the project's base, but the base, the folder's branch and the worktrees' ones. */
  branchesMerged: (projectId: string, repo: string | null = null) => invoke<string[]>('branches_merged', { projectId, ...inRepo(repo) }),
  /** The diff from `a` to `b` (`git diff a b`), two branches or commits. */
  gitDiffRefs: (projectId: string, a: string, b: string, repo: string | null = null) =>
    invoke<string>('git_diff_refs', { projectId, a, b, ...inRepo(repo) }),
  /**
   * Turns an agent's remote control on or off by hand (off sets it apart from the settings that cover it). Resolves with
   * whether the change waits for the next launch of the Claude Code interface that is open: Escouade can't change it there.
   */
  setRemoteControl: (id: string, enabled: boolean) => invoke<boolean>('set_remote_control', { id, enabled }),
  /** The statistics of a period: of every account (null) or of one. */
  stats: (range: string, account: string | null = null, engine: string | null = null) =>
    invoke<StatsView>('stats', { range, account, engine }),
  refreshUsage: () => invoke<void>('refresh_usage'),
  gitDiscard: (projectId: string, agentId: string | null, path: string) => invoke<void>('git_discard', { projectId, agentId, path }),
  /** What a direct commit of the agent's changes takes (null: the project's own checkout, its agents' worktrees apart). */
  commitPreview: (projectId: string, agentId: string | null) => invoke<CommitScope>('commit_preview', { projectId, agentId }),
  /** Haiku's message for a direct commit of `paths`, in the style of the repository's latest commits: only proposed. */
  commitPropose: (projectId: string, agentId: string | null, paths: string[]) =>
    invoke<string>('commit_propose', { projectId, agentId, paths }),
  /** Commits `paths` with the user's message; the commit's short hash. */
  commitDirect: (projectId: string, agentId: string | null, paths: string[], message: string) =>
    invoke<string>('commit_direct', { projectId, agentId, paths, message }),
  fsTree: (projectId: string, agentId: string | null) => invoke<FileTree>('fs_tree', { projectId, agentId }),
  fsRead: (projectId: string, agentId: string | null, path: string) => invoke<FileText>('fs_read', { projectId, agentId, path }),
  /** Refused with "changed" / "deleted" when the file is no longer the one read (`expectedHash`); null forces. */
  fsWrite: (a: {
    projectId: string;
    agentId: string | null;
    path: string;
    text: string;
    eol: 'lf' | 'crlf';
    bom: boolean;
    expectedHash: string | null;
  }) => invoke<string>('fs_write', a),
  /** Creates an empty file, its folders with it; refused when something is already there. */
  fsCreate: (projectId: string, agentId: string | null, path: string) => invoke<void>('fs_create', { projectId, agentId, path }),
  /** Renames or moves a file or a folder (folders of `to` created); refused when something else is at `to`. */
  fsRename: (projectId: string, agentId: string | null, from: string, to: string) =>
    invoke<void>('fs_rename', { projectId, agentId, from, to }),
  /** Sends a file or a folder to the system's trash; never the root nor the agents' worktrees. */
  fsDelete: (projectId: string, agentId: string | null, path: string) => invoke<void>('fs_delete', { projectId, agentId, path }),
  /** Shows a file or a folder in the system's file manager, selected; null opens the source's folder. */
  fsReveal: (projectId: string, agentId: string | null, path: string | null) => invoke<void>('fs_reveal', { projectId, agentId, path }),
  /** The files an agent sent by a call of SendUserFile (ESC-171), as shown now: its images read by the core, the others looked at. */
  sentFiles: (agentId: string, itemId: string) => invoke<SentView[]>('sent_files', { agentId, itemId }),
  /** The file `index` an agent sent, opened with its application or shown in its folder (`reveal`). */
  sentFileOpen: (agentId: string, itemId: string, index: number, reveal: boolean) =>
    invoke<void>('sent_file_open', { agentId, itemId, index, reveal }),
  /** The image a message of the agent shows by a local path: its `data:` URL, or null (not an image, too heavy, gone, refused). */
  /** An image of the web a message shows, downloaded by the core at the user's click (https, 10 MB at most): its `data:` URL. */
  remoteImage: (src: string) => invoke<string>('remote_image', { src }),
  agentLocalImage: (agentId: string, src: string) => invoke<string | null>('agent_local_image', { agentId, src }),
  /** Creates an empty folder, its parents with it; refused when something is already there. */
  fsMkdir: (projectId: string, agentId: string | null, path: string) => invoke<void>('fs_mkdir', { projectId, agentId, path }),
  fsBase: (projectId: string, agentId: string | null, path: string) => invoke<FileBase | null>('fs_base', { projectId, agentId, path }),
  /** The lines of the source's files (the ignored ones and the agents' worktrees left out) that match `query`. */
  codeSearch: (projectId: string, agentId: string | null, query: SearchQuery) =>
    invoke<SearchResult>('code_search', { projectId, agentId, query }),
  setUnsaved: (count: number) => invoke<void>('set_unsaved', { count }),
  cancelResume: (id: string) => invoke<void>('cancel_resume', { id }),
  ticketCreate: (projectId: string, draft: TicketDraft) => invoke<Ticket>('ticket_create', { projectId, draft }),
  ticketUpdate: (id: string, draft: TicketDraft) => invoke<Ticket>('ticket_update', { id, draft }),
  ticketDelete: (id: string) => invoke<void>('ticket_delete', { id }),
  cleanWorktrees: (projectId: string) => invoke<LeftoverReport>('clean_worktrees', { projectId }),
  /** The images of a ticket with their content, for its form; those whose file is gone are left out. */
  ticketImages: (id: string) => invoke<TicketImageData[]>('ticket_images', { id }),
  ticketPrioritize: (id: string) => invoke<void>('ticket_prioritize', { id }),
  /** « Partager avec l’équipe », « Arrêter le partage » (ESC-190). */
  ticketShare: (id: string, on: boolean) => invoke<Ticket>('ticket_share', { id, on }),
  /** « Masquer chez moi » of another one’s shared ticket, or back on the board. */
  ticketHide: (id: string, hidden: boolean) => invoke<Ticket>('ticket_hide', { id, hidden }),
  /** « Garder le mien », « Prendre le sien » of a shared ticket’s file in conflict. */
  sharedResolve: (id: string, keepMine: boolean) => invoke<Ticket>('shared_resolve', { id, keepMine }),
  /** `git config user.name`, offered by the first launch’s window. */
  gitUserName: (projectId?: string) => invoke<string>('git_user_name', { projectId: projectId ?? null }),
  /** "Lancer": starts even with the autopilot off. */
  ticketStart: (id: string) => invoke<void>('ticket_start', { id }),
  ticketResume: (id: string) => invoke<void>('ticket_resume', { id }),
  ticketApprove: (id: string) => invoke<void>('ticket_approve', { id }),
  ticketReject: (id: string, comment: string) => invoke<void>('ticket_reject', { id, comment }),
  ticketResolveConflict: (id: string) => invoke<void>('ticket_resolve_conflict', { id }),
  ticketDismiss: (id: string) => invoke<void>('ticket_dismiss', { id }),
  boardSet: (projectId: string, settings: BoardSettings) => invoke<Project>('board_set', { projectId, settings }),
  /** « Reprendre maintenant »: the autopilot's pause (a quota, a usage limit) is lifted. */
  autopilotResume: () => invoke<void>('autopilot_resume'),
  gitBranches: (projectId: string) => invoke<string[]>('git_branches', { projectId }),
  /** Checks the credentials with the service, then saves them (apart, never sent back). */
  forges: () => invoke<ForgesView>('forges'),
  forgeCheck: () => invoke<ForgesView>('forge_check'),
  forgeConnect: (form: ForgeForm) => invoke<ForgesView>('forge_connect', { form }),
  forgeDisconnect: (host: string) => invoke<ForgesView>('forge_disconnect', { host }),
  forgeGuess: (url: string) => invoke<string | null>('forge_guess', { url }),
  forgeTokenPage: (kind: string, url: string) => invoke<string>('forge_token_page', { kind, url }),
  integrationConnect: (service: Service, account: AccountForm) => invoke<AccountView>('integration_connect', { service, account }),
  integrationDisconnect: (service: Service) => invoke<AccountView[]>('integration_disconnect', { service }),
  /** Jira projects, Trello boards, GitHub repositories (the project's own first). */
  integrationContainers: (service: Service, projectId: string | null) =>
    invoke<Container[]>('integration_containers', { service, projectId }),
  integrationStates: (service: Service, container: string) => invoke<StatesView>('integration_states', { service, container }),
  /** `page`: the `next` of the page before (« Afficher plus »); none for the first one. */
  integrationIssues: (projectId: string, service: Service, text: string, filters: string[], page?: string) =>
    invoke<IssuePage>('integration_issues', { projectId, service, text, filters, page }),
  integrationImport: (projectId: string, issues: ExternalIssue[], maxLoops: number) =>
    invoke<Ticket[]>('integration_import', { projectId, issues, maxLoops }),
  /** « Resynchroniser »: the ticket's failed syncs go again at once; rejected with why they still fail. */
  integrationResync: (ticketId: string) => invoke<void>('integration_resync', { ticketId }),
  /** `agentId` and `subdir` say where it opens: the agent's worktree, a folder of it (see `TermPlace`). */
  termSpawn: (
    a: { projectId: string; agentId?: string | null; subdir?: string; shell: string; name: string; cols: number; rows: number },
    onData: (d: ArrayBuffer) => void,
  ) => {
    const output = new Channel<ArrayBuffer>();
    output.onmessage = onData;
    return invoke<TermInfo>('term_spawn', { ...a, output });
  },
  runStart: (
    a: { projectId: string; commandId: string; agentId?: string; cols: number; rows: number; cursorRow: number },
    onData: (d: ArrayBuffer) => void,
  ) => {
    const output = new Channel<ArrayBuffer>();
    output.onmessage = onData;
    return invoke<TermInfo>('run_start', { ...a, output });
  },
  /** "Préparer le lancement": reserves the agent's ports and asks it for its recipe. */
  agentPrepareLaunch: (id: string) => invoke<void>('agent_prepare_launch', { id }),
  /** « Lancer » in the test modal: the user read `recipe` (the agent's, as shown) and lets it run. */
  testRecipeApprove: (agentId: string, recipe: TestRecipe) => invoke<void>('test_recipe_approve', { agentId, recipe }),
  testRunStart: (
    a: { agentId: string; kind: 'prep' | 'run' | 'isola'; index: number; cols: number; rows: number; cursorRow: number },
    onData: (d: ArrayBuffer) => void,
  ) => {
    const output = new Channel<ArrayBuffer>();
    output.onmessage = onData;
    return invoke<TermInfo>('test_run_start', { ...a, output });
  },
  /** The worktree's .isola.toml, whole, as `isola up` will read it: what is shown before the services start. */
  isolaConfig: (agentId: string) => invoke<string>('isola_config', { agentId }),
  /** « Lancer » for isola: the user read `config` (the .isola.toml) and `open` (the address to open) and lets `isola up` run. */
  isolaApprove: (agentId: string, config: string, open: string) => invoke<void>('isola_approve', { agentId, config, open }),
  /** The services isola runs for the agent's worktree, with their addresses. */
  isolaServices: (agentId: string) => invoke<IsolaService[]>('isola_services', { agentId }),
  /** Stops the services isola runs for the agent's worktree. */
  isolaDown: (agentId: string) => invoke<void>('isola_down', { agentId }),
  /** The setup and teardown Claude suggests for the project's worktrees, from what it reads of it. */
  suggestWorktreeSteps: (projectId: string) => invoke<WorktreeSuggestion>('suggest_worktree_steps', { projectId }),
  /** The launch commands Claude suggests for the project, from what it reads of it, with how many it gave that were refused; nothing is saved. */
  suggestRunCommands: (projectId: string) => invoke<RunSuggestion>('suggest_run_commands', { projectId }),
  /** The same two, for the folder of a project not made yet (the new project's assistant). */
  suggestWorktreeStepsAt: (path: string) => invoke<WorktreeSuggestion>('suggest_worktree_steps_at', { path }),
  suggestRunCommandsAt: (path: string) => invoke<RunSuggestion>('suggest_run_commands_at', { path }),
  /**
   * A newer release, through the network settings (proxy, TLS verification), kept by the backend for its download.
   * `manual`: the user asked, so a failure is always to be shown.
   */
  updateCheck: (manual: boolean) => invoke<CheckedUpdate>('update_check', { manual }),
  /**
   * Downloads the release found, its signature checked, kept by the backend until it installs: ready once it is the
   * one, superseded when a newer one took its place meanwhile.
   */
  updateDownload: (id: number, manual: boolean) => invoke<DownloadedUpdate>('update_download', { id, manual }),
  updateClose: (id: number) => invoke<void>('update_close', { id }),
  /** « Redémarrer maintenant »: the update downloaded installs, the app stopped cleanly first, and the app starts again. */
  updateRestart: () => invoke<void>('update_restart'),
  /** « Plus tard »: the automatic restart planned is called off. */
  updatePostpone: () => invoke<void>('update_postpone'),
  /** What an automatic restart waits for, as the window sees it. */
  updatePresence: (presence: { modal: boolean; testing: boolean; activeAt: number }) => invoke<void>('update_presence', { presence }),
  /** One HTTP request without proxy (2 s): true for any answer. */
  httpReady: (url: string, agentId?: string) => invoke<boolean>('http_ready', { url, agentId }),
  /** The write's number, which the backend's log tells it by. */
  termWrite: (id: string, data: string) => invoke<number>('term_write', { id, data }),
  termReport: (id: string, report: TermReport) => invoke<void>('term_report', { id, report }),
  termResize: (id: string, cols: number, rows: number) => invoke<void>('term_resize', { id, cols, rows }),
  termKill: (id: string) => invoke<void>('term_kill', { id }),
  playChime: () => invoke<void>('play_chime'),
  quit: () => invoke<void>('quit_app'),
};
