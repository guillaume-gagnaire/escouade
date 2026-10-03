import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  Agent,
  Attachment,
  BoardSettings,
  ConvItem,
  FileBase,
  FileChange,
  FileText,
  FileTree,
  FolderInfo,
  GitLog,
  InitialState,
  Project,
  Settings,
  ShellInfo,
  SlashCommand,
  StatsView,
  TermInfo,
  Ticket,
  TicketDraft,
  UiEvent,
  UiState,
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
  mergeAgent: (id: string, squash: boolean) => invoke<string>('merge_agent', { id, squash }),
  getCommands: (id: string) => invoke<SlashCommand[]>('get_commands', { id }),
  fileSuggestions: (id: string, query: string) => invoke<string[]>('file_suggestions', { id, query }),
  gitFiles: (projectId: string, agentId: string | null) => invoke<FileChange[]>('git_files', { projectId, agentId }),
  gitDiff: (projectId: string, agentId: string | null, paths: string[]) => invoke<string>('git_diff', { projectId, agentId, paths }),
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
  fsBase: (projectId: string, agentId: string | null, path: string) => invoke<FileBase | null>('fs_base', { projectId, agentId, path }),
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
  gitBranches: (projectId: string) => invoke<string[]>('git_branches', { projectId }),
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
  testRunStart: (
    a: { agentId: string; kind: 'prep' | 'run'; index: number; cols: number; rows: number; cursorRow: number },
    onData: (d: ArrayBuffer) => void,
  ) => {
    const output = new Channel<ArrayBuffer>();
    output.onmessage = onData;
    return invoke<TermInfo>('test_run_start', { ...a, output });
  },
  /** One HTTP request without proxy (2 s): true for any answer. */
  httpReady: (url: string) => invoke<boolean>('http_ready', { url }),
  termWrite: (id: string, data: string) => invoke<void>('term_write', { id, data }),
  termResize: (id: string, cols: number, rows: number) => invoke<void>('term_resize', { id, cols, rows }),
  termKill: (id: string) => invoke<void>('term_kill', { id }),
  playChime: () => invoke<void>('play_chime'),
  quit: () => invoke<void>('quit_app'),
};
