import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  AccountForm,
  AccountView,
  Agent,
  Attachment,
  BoardSettings,
  CommitScope,
  Container,
  ConvItem,
  ConvSearchResult,
  ExternalIssue,
  FileBase,
  FileChange,
  FileText,
  FileTree,
  FolderInfo,
  FoundRelease,
  GitLog,
  InitialState,
  OwnedDiff,
  IsolaService,
  IssuePage,
  Project,
  SearchQuery,
  SearchResult,
  Service,
  Settings,
  ShellInfo,
  SlashCommand,
  StatesView,
  StatsView,
  TermInfo,
  TestRecipe,
  Ticket,
  TicketDraft,
  UiEvent,
  UiState,
  WorktreeSuggestion,
} from './types';

export const api = {
  subscribe: (onEvent: (e: UiEvent) => void) => {
    const channel = new Channel<UiEvent>();
    channel.onmessage = onEvent;
    return invoke<InitialState>('subscribe', { channel });
  },
  setUi: (ui: UiState) => invoke<void>('set_ui', { ui }),
  saveSettings: (settings: Settings) => invoke<ShellInfo[]>('save_settings', { settings }),
  inspectFolder: (path: string) => invoke<FolderInfo>('inspect_folder', { path }),
  createProject: (a: { path: string; name: string; color: string; worktreePerAgent: boolean; firstAgent: string | null }) =>
    invoke<Project>('create_project', a),
  updateProject: (project: Project) => invoke<void>('update_project', { project }),
  reorderProjects: (ids: string[]) => invoke<void>('reorder_projects', { ids }),
  removeProject: (id: string) => invoke<void>('remove_project', { id }),
  createAgent: (projectId: string, model: string | null = null) => invoke<Agent>('create_agent', { projectId, model }),
  warmAgent: (id: string) => invoke<void>('warm_agent', { id }),
  getConversation: (id: string) => invoke<ConvItem[]>('get_conversation', { id }),
  /** Searches the conversations of a project's agents (all projects' with null), archived agents included or not. */
  searchConversations: (query: string, projectId: string | null, archived: boolean) =>
    invoke<ConvSearchResult>('search_conversations', { query, projectId, archived }),
  sendMessage: (id: string, text: string, attachments: Attachment[] = []) => invoke<void>('send_message', { id, text, attachments }),
  interrupt: (id: string) => invoke<void>('interrupt', { id }),
  answerQuestion: (id: string, requestId: string, answers: Record<string, string>) =>
    invoke<void>('answer_question', { id, requestId, answers }),
  answerPermission: (id: string, requestId: string, decision: 'allow' | 'always' | 'deny', message: string | null = null) =>
    invoke<void>('answer_permission', { id, requestId, decision, message }),
  setAgentOptions: (id: string, o: { model?: string; effort?: string; mode?: string }) =>
    invoke<void>('set_agent_options', { id, model: o.model ?? null, effort: o.effort ?? null, mode: o.mode ?? null }),
  renameAgent: (id: string, name: string) => invoke<void>('rename_agent', { id, name }),
  archiveAgent: (id: string, archived: boolean) => invoke<void>('archive_agent', { id, archived }),
  deleteAgent: (id: string, removeWorktree: boolean) => invoke<string | null>('delete_agent', { id, removeWorktree }),
  /** Merges into the agent's base branch; `switchToBase`: the project's folder is switched to it first. */
  mergeAgent: (id: string, squash: boolean, switchToBase = false) => invoke<string>('merge_agent', { id, squash, switchToBase }),
  getCommands: (id: string) => invoke<SlashCommand[]>('get_commands', { id }),
  fileSuggestions: (id: string, query: string) => invoke<string[]>('file_suggestions', { id, query }),
  gitFiles: (projectId: string, agentId: string | null) => invoke<FileChange[]>('git_files', { projectId, agentId }),
  gitDiff: (projectId: string, agentId: string | null, paths: string[]) => invoke<string>('git_diff', { projectId, agentId, paths }),
  gitProjectDiff: (projectId: string) => invoke<OwnedDiff[]>('git_project_diff', { projectId }),
  gitLog: (projectId: string, agentId: string | null) => invoke<GitLog>('git_log', { projectId, agentId }),
  gitShow: (projectId: string, hash: string) => invoke<string>('git_show', { projectId, hash }),
  /** Fetch, pull (fast-forward only) and push of the project's checkout; each returns a summary. */
  gitFetch: (projectId: string) => invoke<string>('git_fetch', { projectId }),
  gitPull: (projectId: string) => invoke<string>('git_pull', { projectId }),
  gitPush: (projectId: string) => invoke<string>('git_push', { projectId }),
  setRemoteControl: (id: string, enabled: boolean) => invoke<void>('set_remote_control', { id, enabled }),
  stats: (range: string) => invoke<StatsView>('stats', { range }),
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
  fsBase: (projectId: string, agentId: string | null, path: string) => invoke<FileBase | null>('fs_base', { projectId, agentId, path }),
  /** The lines of the source's files (the ignored ones and the agents' worktrees left out) that match `query`. */
  codeSearch: (projectId: string, agentId: string | null, query: SearchQuery) =>
    invoke<SearchResult>('code_search', { projectId, agentId, query }),
  setUnsaved: (count: number) => invoke<void>('set_unsaved', { count }),
  cancelResume: (id: string) => invoke<void>('cancel_resume', { id }),
  ticketCreate: (projectId: string, draft: TicketDraft) => invoke<Ticket>('ticket_create', { projectId, draft }),
  ticketUpdate: (id: string, draft: TicketDraft) => invoke<Ticket>('ticket_update', { id, draft }),
  ticketDelete: (id: string) => invoke<void>('ticket_delete', { id }),
  ticketPrioritize: (id: string) => invoke<void>('ticket_prioritize', { id }),
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
  integrationConnect: (service: Service, account: AccountForm) => invoke<AccountView>('integration_connect', { service, account }),
  integrationDisconnect: (service: Service) => invoke<AccountView[]>('integration_disconnect', { service }),
  /** Jira projects, Trello boards, GitHub repositories (the project's own first). */
  integrationContainers: (service: Service, projectId: string | null) =>
    invoke<Container[]>('integration_containers', { service, projectId }),
  integrationStates: (service: Service, container: string) => invoke<StatesView>('integration_states', { service, container }),
  integrationIssues: (projectId: string, service: Service, text: string, filters: string[]) =>
    invoke<IssuePage>('integration_issues', { projectId, service, text, filters }),
  integrationImport: (projectId: string, issues: ExternalIssue[], maxLoops: number) =>
    invoke<Ticket[]>('integration_import', { projectId, issues, maxLoops }),
  termSpawn: (a: { projectId: string; shell: string; name: string; cols: number; rows: number }, onData: (d: ArrayBuffer) => void) => {
    const output = new Channel<ArrayBuffer>();
    output.onmessage = onData;
    return invoke<TermInfo>('term_spawn', { ...a, output });
  },
  runStart: (
    a: { projectId: string; commandId: string; cols: number; rows: number; cursorRow: number },
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
  /** A newer release, through the network settings (proxy, TLS verification), kept by the backend for its download. */
  updateCheck: () => invoke<FoundRelease | null>('update_check'),
  /** Downloads the release found, its signature checked, kept by the backend until it installs: true once it is the one ready. */
  updateDownload: (id: number) => invoke<boolean>('update_download', { id }),
  updateClose: (id: number) => invoke<void>('update_close', { id }),
  /** « Redémarrer maintenant »: the update downloaded installs, the app stopped cleanly first, and the app starts again. */
  updateRestart: () => invoke<void>('update_restart'),
  /** « Plus tard »: the automatic restart planned is called off. */
  updatePostpone: () => invoke<void>('update_postpone'),
  /** What an automatic restart waits for, as the window sees it. */
  updatePresence: (presence: { modal: boolean; testing: boolean; activeAt: number }) => invoke<void>('update_presence', { presence }),
  /** One HTTP request without proxy (2 s): true for any answer. */
  httpReady: (url: string) => invoke<boolean>('http_ready', { url }),
  termWrite: (id: string, data: string) => invoke<void>('term_write', { id, data }),
  termResize: (id: string, cols: number, rows: number) => invoke<void>('term_resize', { id, cols, rows }),
  termKill: (id: string) => invoke<void>('term_kill', { id }),
  playChime: () => invoke<void>('play_chime'),
  quit: () => invoke<void>('quit_app'),
};
