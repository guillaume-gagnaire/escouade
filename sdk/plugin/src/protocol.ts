// Generated from schemas/plugin-protocol-1.json by sdk/plugin/scripts/generate.mjs: do not edit, run it again.
/** JSON-RPC 2.0 over the standard input and output of a plugin's program, one message per line. The one source of the protocol's types: the TypeScript of @escouade-dev/plugin is generated from it (sdk/plugin/scripts/generate.mjs), and the Rust tests of the core hold it to what the core sends (plugins_protocol_schema). */
export const PROTOCOL_VERSION = 1;

/** The facts of Escouade. */
export type FactName =
  | 'agent.created'
  | 'agent.renamed'
  | 'agent.status'
  | 'agent.stopped'
  | 'agent.handover'
  | 'turn.ended'
  | 'request.permission'
  | 'request.question'
  | 'request.answered'
  | 'ticket.created'
  | 'ticket.column'
  | 'ticket.report'
  | 'ticket.approved'
  | 'ticket.rejected'
  | 'ticket.merged'
  | 'ticket.conflict'
  | 'project.added'
  | 'project.closed'
  | 'worktree.created'
  | 'worktree.removed'
  | 'launch.started'
  | 'launch.ready'
  | 'launch.stopped'
  | 'tests.result'
  | 'app.started'
  | 'docs.written'
  | 'docs.filed'
  | 'docs.gaps'
  | 'memory.written'
  | 'memory.forgotten'
  | 'decision.proposed'
  | 'decision.accepted'
  | 'pr.opened'
  | 'account.switched'
  | 'workflow.step_entered'
  | 'workflow.step_left'
  | 'workflow.looped_back'
  | 'workflow.left'
  | 'workflow.changed';

/** A project, by its id and its name. */
export interface Project {
  id: string;
  name: string;
  /** A private project: nothing of it goes to a plugin that is not on for it. */
  private: boolean;
}

/** A ticket, by its id, its key and its title. */
export interface Ticket {
  id: string;
  /** Like ESC-12. */
  key: string;
  title: string;
}

/** An agent, by its id and its name. */
export interface Agent {
  id: string;
  name: string;
}

/** A fact: what changed, and the project, the ticket and the agent it is about, never the conversation. Ignore the fields you do not know. */
export interface Fact {
  name: FactName;
  /** The version of this fact's fields. */
  version: number;
  /** Its number since Escouade started: two facts come in the order of their numbers. */
  seq: number;
  /** When, in RFC 3339. */
  at: string;
  project?: Project;
  ticket?: Ticket;
  agent?: Agent;
  /** What it says, fact by fact (agent.status: status and was; ticket.column: from and to; turn.ended: end…): see the page Events of the plugins’ documentation. */
  data: Record<string, unknown>;
  [field: string]: unknown;
}

/** What Escouade can do for the plugin. */
export interface EscouadeCapabilities {
  /** Batches, both ways. */
  batch?: boolean;
  /** secrets/get. */
  secrets?: boolean;
  /** The log notification. */
  log?: boolean;
  /** What each agent attached is given again after a restart. */
  attach?: boolean;
}

export interface InitializeParams {
  /** The version Escouade speaks. */
  protocolVersion: number;
  escouade: {
    version: string;
  };
  /** The plugin, as its manifest says. */
  plugin: {
    id: string;
    version: string;
  };
  /** The language of the interface (fr, en), for what the plugin writes for the user. A plugin in WASM has no other way to know it. */
  lang?: string;
  capabilities: EscouadeCapabilities;
}

export interface InitializeResult {
  /** The version the plugin speaks; another than Escouade's stops it. */
  protocolVersion: number;
  /** What the plugin can do. */
  capabilities: Record<string, unknown>;
}

/** What the agent asks to do. */
export interface PermissionRequest {
  id: string;
  /** Like command, edit, fetch. */
  kind: string;
  /** The harness's tool, like Bash. */
  tool?: string;
  /** What the tool was given, like { command: 'npm test' }. */
  input?: Record<string, unknown>;
}

export interface DecideParams {
  request: PermissionRequest;
  project: Project;
  ticket?: Ticket;
  agent?: Agent;
}

/** A refusal wins over an allowance; ask leaves it to the user. */
export interface Decision {
  decision: 'allow' | 'deny' | 'ask';
  /** Why: told to the agent with a refusal. */
  message?: string;
}

export interface SecretsGetParams {
  /** A key of permissions.secrets. */
  key: string;
}

export interface SettingsGetParams {
  /** The id of a project (project.id of a fact): its .escouade.yml's settings for the plugin go over the user's. None: the user's alone. */
  project?: string;
}

export interface FilesReadParams {
  /** The file's path in the plugin's folder: rules.json, data/words.txt. */
  path: string;
}

export interface NetFetchParams {
  /** An https URL to a host of permissions.network; http too for a host of `{account}` or `local:shared`. */
  url: string;
  /** GET when left out. */
  method?: string;
  /** The headers of the request (Host is set by Escouade). */
  headers?: Record<string, string>;
  /** The body of the request, as text. */
  body?: string | null;
  /** The body of the request, as base64, for bytes: body or bodyBase64, not both. */
  bodyBase64?: string;
}

export interface NetFetchResult {
  /** The HTTP status; a 3xx is given as it is, never followed. */
  status: number;
  /** Its headers, by lower case name; a repeated one joined by ", ". */
  headers: Record<string, string>;
  /** Its body, when it is UTF-8. */
  body?: string;
  /** Its body as base64, when it is not UTF-8. */
  bodyBase64?: string;
}

export interface NetStreamParams {
  /** An https URL to a host of permissions.network; http too for a host of `{account}` or `local:shared`. */
  url: string;
  /** GET when left out. */
  method?: string;
  /** The headers of the request (Host is set by Escouade). */
  headers?: Record<string, string>;
  /** The body of the request, as text. */
  body?: string | null;
  /** The body of the request, as base64, for bytes: body or bodyBase64, not both. */
  bodyBase64?: string;
  /** A stream of events, that may stay silent: it is never closed for that. */
  sse?: boolean;
}

export interface NetStreamResult {
  /** What names it. */
  stream: number;
  /** The HTTP status; a 3xx is given as it is, never followed. */
  status: number;
  /** Its headers, by lower case name; a repeated one joined by ", ". */
  headers: Record<string, string>;
}

export interface NetChunkParams {
  /** The stream. */
  stream: number;
  /** The piece as text, when it is UTF-8: a character cut by a read waits for the next piece. */
  data?: string;
  /** The piece as base64, when it is not UTF-8. */
  dataBase64?: string;
}

export interface NetEndParams {
  /** The stream. */
  stream: number;
  /** Why it broke; none for a stream read to its end. */
  error?: string;
}

export interface NetCreditParams {
  /** The stream. */
  stream: number;
  /** How many bytes of it were consumed. */
  bytes: number;
}

export interface NetCancelParams {
  /** The stream. */
  stream: number;
}

export interface WsOpenParams {
  /** A wss URL to a host of net/fetch. */
  url: string;
  /** The headers of the handshake (Host is set by Escouade); never in the journal. */
  headers?: Record<string, string>;
}

export interface WsOpenResult {
  /** What names it. */
  socket: number;
}

export interface WsSendParams {
  /** The socket. */
  socket: number;
  /** A text message. */
  text?: string;
  /** A binary message, as base64: text or dataBase64, not both. */
  dataBase64?: string;
}

export interface WsCloseParams {
  /** The socket. */
  socket: number;
}

export interface WsMessageParams {
  /** The socket. */
  socket: number;
  /** A text message. */
  text?: string;
  /** A binary message, as base64. */
  dataBase64?: string;
}

export interface WsClosedParams {
  /** The socket. */
  socket: number;
  /** The close code; 1006 for a connection that broke, 1009 for a message over 1 MiB. */
  code: number;
  /** Why, when it is said. */
  reason: string;
}

export interface TimerSetParams {
  /** In how long, in milliseconds; less than 10 is 10. */
  ms: number;
}

export interface TimerSetResult {
  /** What names it. */
  timer: number;
}

export interface TimerParams {
  /** The timer. */
  timer: number;
}

export interface LogParams {
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
}

/** What fs/* name: a path relative to root, or a path of the host (C:\wt\a.txt) under a root of the agent. root plugin and temp need no agent. */
export interface FsPath {
  /** The key of the agent, as agent/attach gave it. */
  agent?: string;
  /** A root of permissions.files. */
  root?: 'plugin' | 'agent' | 'agentRepo' | 'agentHome' | 'temp';
  /** Relative to root when there is one, absolute otherwise; never a .. nor a ~. */
  path: string;
}

/** As FsPath, for fs/read. */
export interface FsReadParams {
  /** As FsPath. */
  agent?: string;
  /** As FsPath. */
  root?: 'plugin' | 'agent' | 'agentRepo' | 'agentHome' | 'temp';
  /** As FsPath. */
  path: string;
  /** A file past the limit gives its end, from a whole character, instead of failing: what a log that grows is read by. */
  tail?: boolean;
}

export interface FsReadResult {
  /** The file, when it is UTF-8. */
  text?: string;
  /** The file, when it is not. */
  dataBase64?: string;
  /** Its start is left out (tail): its first line may be whole or not. */
  cut?: boolean;
}

export interface FsWriteParams {
  /** As FsPath. */
  agent?: string;
  /** As FsPath. */
  root?: 'plugin' | 'agent' | 'agentRepo' | 'agentHome' | 'temp';
  /** As FsPath. */
  path: string;
  /** What to write, as text. */
  text?: string;
  /** What to write, in base64, in place of text. */
  dataBase64?: string;
  /** At the end of the file. */
  append?: boolean;
}

export interface FsWriteResult {
  /** Its real path. */
  path: string;
  bytes: number;
}

export interface FsStatResult {
  kind: 'file' | 'dir' | 'other';
  bytes: number;
  /** In milliseconds since 1970. */
  modified?: number | null;
}

export interface FsEntry {
  name: string;
  kind: 'file' | 'dir' | 'link' | 'other';
  bytes: number;
}

export interface FsListResult {
  entries: FsEntry[];
  /** Past 4096 entries. */
  truncated: boolean;
}

export interface FsCanonResult {
  path: string;
}

export interface ProcSpawnParams {
  /** The program's name, the first word of an entry of permissions.commands. */
  command: string;
  /** Its arguments. */
  args?: string[];
  /** Where it runs: a root, never a path. A private project refuses it unless the plugin was turned on for it. Left out: the plugin's temporary folder. */
  dir?: 'agent' | 'project';
  /** The key of the agent, for dir. */
  agent?: string;
  /** The id of a project, for dir project; the agent's when left out. */
  project?: string;
  /** The variables it names, over those of an agent; never ESCOUADE_*, PATH, LD_*, DYLD_*. */
  env?: Record<string, string>;
}

export interface ProcSpawnResult {
  proc: number;
}

export interface ProcRef {
  proc: number;
}

export interface ProcWriteParams {
  proc: number;
  /** What to write on its input. */
  data?: string;
  /** Ends its input, after data. */
  close?: boolean;
}

export interface ProcOutParams {
  proc: number;
  stream: 'stdout' | 'stderr';
  data: string;
}

export interface ProcExitParams {
  proc: number;
  code: number | null;
}

/** What the conversation holds of an id, for a view that starts on it. */
export interface ViewKnown {
  id: string;
  /** A call's tool: none for what is no call. */
  name?: string;
  /** How a call stands (running, ok…). */
  status?: string;
}

/** Lines of the view of an agent, and what Escouade holds of it as they come. */
export interface ViewParams {
  /** The view, by a key of its own: another key is another conversation, which starts from nothing. */
  agent: string;
  /** The harness, for a plugin that has several. */
  harness?: string;
  /** The first lines of this view: known is given, and gone may be dropped. */
  fresh?: boolean;
  known?: ViewKnown[];
  /** The view this one replaces. */
  gone?: string;
  cwd: string;
  mode?: string;
  model?: string;
  /** The language of the app. */
  lang?: string;
  /** The model the agent last ran on. */
  mainModel?: string;
  session?: string | null;
  /** A turn runs. */
  active?: boolean;
  /** The user stopped it. */
  interrupted?: boolean;
  /** The agent is a copy, its session not its own yet. */
  copy?: boolean;
  /** A discussion of the Docs tab: it writes nothing. */
  discussion?: boolean;
  /** The message of the turn under way, as the interface took it. */
  prompt?: string | null;
  lines: string[];
}

/** What the lines mean, in their order: the items of agent/frames, and those of a view (op: typed, spoke, wrote, waits, waitsOn, asks, runs, goes, viewTurn, viewEnd, viewUsage, viewError, row, bridged, linked). An op Escouade does not know is left aside. */
export interface ViewItems {
  items: Array<{
    op: string;
  }>;
}

/** Every fact, in the order of the schema. */
export const FACT_NAMES: readonly FactName[] = [
  'agent.created',
  'agent.renamed',
  'agent.status',
  'agent.stopped',
  'agent.handover',
  'turn.ended',
  'request.permission',
  'request.question',
  'request.answered',
  'ticket.created',
  'ticket.column',
  'ticket.report',
  'ticket.approved',
  'ticket.rejected',
  'ticket.merged',
  'ticket.conflict',
  'project.added',
  'project.closed',
  'worktree.created',
  'worktree.removed',
  'launch.started',
  'launch.ready',
  'launch.stopped',
  'tests.result',
  'app.started',
  'docs.written',
  'docs.filed',
  'docs.gaps',
  'memory.written',
  'memory.forgotten',
  'decision.proposed',
  'decision.accepted',
  'pr.opened',
  'account.switched',
  'workflow.step_entered',
  'workflow.step_left',
  'workflow.looped_back',
  'workflow.left',
  'workflow.changed',
];

/** What Escouade sends to the plugin: each method, its parameters and its answer (`void` for a notification). */
export interface ToPlugin {
  /** The first message: the version of the protocol Escouade speaks and what it can do. Answer with your own. */
  initialize: { params: InitializeParams; result: InitializeResult };
  /** Escouade closes: answer, then end. */
  shutdown: { params: null; result: null };
  /** A fact of permissions.events, from a project where the plugin is on. A notification: no answer. */
  'events/fact': { params: Fact; result: void };
  /** A permission an agent asks for, before the user is asked (permissions.decide). Past the delay, the user is asked. */
  'permissions/decide': { params: DecideParams; result: Decision };
  /** What a program wrote, as text cut between two characters. Escouade stops reading it past 256 KiB not yet written on the input of the module. A notification: no answer. */
  'proc/out': { params: ProcOutParams; result: void };
  /** A program ended, after all it wrote; code is null when it was killed. A notification: no answer. */
  'proc/exit': { params: ProcExitParams; result: void };
  /** A piece of a stream. */
  'net/chunk': { params: NetChunkParams; result: void };
  /** The end of a stream; none after a net/cancel. */
  'net/end': { params: NetEndParams; result: void };
  /** A message received on a socket. */
  'ws/message': { params: WsMessageParams; result: void };
  /** A socket ended, by either side. */
  'ws/closed': { params: WsClosedParams; result: void };
  /** A timer came due. */
  'timer/fired': { params: TimerParams; result: void };
  /** The terminal view of an agent of a harness: Escouade reads the file the harness's own interface writes, and hands its lines on with what it holds of the agent; the plugin answers what they mean, as the items of agent/frames and those of a view. A line is what the file holds, or a note of Escouade, an object with a field escouade: end (the running turn is over: error, durationMs, models), hook (a hook of the interface, under hook), frame (a message of the harness read as a process's, under frame). In an instance of its own, by the same turn as the rest, on the plugin's time for a turn: it may ask nothing of the host. A harness written in WASM answers it when it has a terminal view; any other refuses it as an unknown method. */
  'view/translate': { params: ViewParams; result: ViewItems };
}

/** What the plugin sends to Escouade: each method, its parameters and its answer (`void` for a notification). */
export interface ToEscouade {
  /** One of its secrets (a key of permissions.secrets), or null when none is kept. */
  'secrets/get': { params: SecretsGetParams; result: string | null };
  /** The values of its settings (contributes.settings), as the user set them; with a project, those its .escouade.yml gives the plugin (plugins.<id>) over them, where the plugin is on. */
  'settings/get': { params: SettingsGetParams; result: Record<string, string | number | boolean> };
  /** A file of the plugin's own folder, as text (UTF-8): a path relative to the folder, with / between its parts. Refused (refused) for a path that leaves the folder (absolute, .., a link that goes out of it), a hidden file or folder (.env, .git) and the manifest; notFound for a file that is not there; failed for a file over 256 KiB or not UTF-8. Both transports: the SDK's p.file(path). */
  'files/read': { params: FilesReadParams; result: string };
  /** An HTTP request, for a plugin in WASM (run.js, run.wasm): the SDK's standard fetch goes by it. Escouade sends it in https only, to a host of permissions.network or one of its subdomains, never to an address of the machine or of the local network (judged on the addresses the name resolves to). `{account}` in permissions.network opens the host the user typed for one of the plugin's accounts, on that port and that host alone, in http or on the local network; `local:shared` opens http to 127.0.0.1 on a port the core read in the output of the process it started for the plugin (Launch.shared.listens), while it lives. The proxy and the TLS of Réglages › Réseau are followed: through a proxy only the name is judged, and the journal says so. No redirection is followed (the plugin gets the 3xx). The answer comes within 10 s and holds 1 MiB at most (failed past either), unless the manifest's limits ask for more (120 s, 32 MiB at most). A plugin run by a command uses its own network, and is answered methodNotFound; the same for net/stream, ws/* and timer/*. */
  'net/fetch': { params: NetFetchParams; result: NetFetchResult };
  /** A response in pieces, for a plugin in WASM: the rules of net/fetch at its opening, the answer once its headers are read. Then net/chunk, and net/end. The plugin holds 256 KiB it has not consumed (net/credit): past it Escouade does not read the socket. 8 streams at once. 120 s without a byte end a stream that is not declared sse. */
  'net/stream': { params: NetStreamParams; result: NetStreamResult };
  /** The plugin consumed these bytes of a stream: as many more may come, never past 256 KiB waiting. */
  'net/credit': { params: NetCreditParams; result: void };
  /** The plugin lets go of a stream: its connection closes. */
  'net/cancel': { params: NetCancelParams; result: void };
  /** A WebSocket, for a plugin in WASM: wss only, to the hosts of net/fetch, the proxy and the TLS of Réglages › Réseau. 4 sockets at once, 1 MiB a message. Its headers never go to the journal. */
  'ws/open': { params: WsOpenParams; result: WsOpenResult };
  /** A message on a socket: refused past 1 MiB, or when what waits to be sent would pass 256 KiB (a long message goes when nothing waits). */
  'ws/send': { params: WsSendParams; result: null };
  /** Closes a socket: ws/closed follows. */
  'ws/close': { params: WsCloseParams; result: null };
  /** A timer, for a plugin in WASM: 10 ms at least, 64 at once. The prelude makes setTimeout of it. It goes with the instance. */
  'timer/set': { params: TimerSetParams; result: TimerSetResult };
  /** Takes a timer back. */
  'timer/cancel': { params: TimerParams; result: void };
  /** A file under a root: { text }, or { dataBase64 } when it is not UTF-8. With tail, what is past the limit below is cut from its start. For a plugin in WASM, by escouade.call (run.wasm) or by a line (run.js), on permissions.files: refused for a root it does not declare, a .., a ~, or a path that, links followed, is not under a root of that agent or that its fence refuses (« Projet privé »). failed past 256 KiB, or limits.fileBytes (4 MiB at most). */
  'fs/read': { params: FsReadParams; result: FsReadResult };
  /** Writes a file under a root in write access, the folders it needs made; each write goes to the plugin's journal. For a plugin in WASM, by escouade.call (run.wasm) or by a line (run.js), on permissions.files: refused for a root it does not declare, a .., a ~, or a path that, links followed, is not under a root of that agent or that its fence refuses (« Projet privé »). 4 MiB at most. */
  'fs/write': { params: FsWriteParams; result: FsWriteResult };
  /** What a path is. For a plugin in WASM, by escouade.call (run.wasm) or by a line (run.js), on permissions.files: refused for a root it does not declare, a .., a ~, or a path that, links followed, is not under a root of that agent or that its fence refuses (« Projet privé »). */
  'fs/stat': { params: FsPath; result: FsStatResult };
  /** The entries of a folder, by name, 4096 at most; a link is a link, not followed. For a plugin in WASM, by escouade.call (run.wasm) or by a line (run.js), on permissions.files: refused for a root it does not declare, a .., a ~, or a path that, links followed, is not under a root of that agent or that its fence refuses (« Projet privé »). */
  'fs/list': { params: FsPath; result: FsListResult };
  /** The real path of a path, links followed. For a plugin in WASM, by escouade.call (run.wasm) or by a line (run.js), on permissions.files: refused for a root it does not declare, a .., a ~, or a path that, links followed, is not under a root of that agent or that its fence refuses (« Projet privé »). */
  'fs/canon': { params: FsPath; result: FsCanonResult };
  /** Whether git cannot run anything of an agent's in its folder: its configuration protected and, when it commits, its hooks. For a harness in WASM, which neither reads .git nor runs git, by escouade.call: the answer is false when git does not answer within a second. Refused without the root agentRepo in permissions.files, and for a folder that is no agent's of this plugin. */
  'fs/gitProtected': {
    params: {
      /** The agent's folder. */
      path: string;
      /** It commits: its hooks count too. True when left out. */
      commits?: boolean;
    };
    result: {
      protected: boolean;
    };
  };
  /** Whether a write of a path strictly in an agent's folder reaches a file alone: a file there (not a link), or nothing there and no submodule of git's index at it or under it, its links leading nowhere out of the folder. For a harness in WASM whose rule of a write covers what is under its path, which reads no worktree, by escouade.call. Needs no root of permissions.files: it gives one boolean of the agent's own folder. A folder, a link, and a git that does not answer within a second give false. Refused for a path that is not absolute, strictly in the folder of an agent of this plugin, and out of its fence. */
  'fs/fileOnly': {
    params: {
      /** The agent, as agent/attach named it. */
      agent: string;
      /** The absolute path the write names. */
      path: string;
    };
    result: {
      fileOnly: boolean;
    };
  };
  /** Runs a program of permissions.commands, for a plugin in WASM: the command and its arguments must be an entry word for word, more following only an entry that ends in *. The program is found on the PATH, never a path; no shell; 4 at once; it ends with the plugin. Its environment is an agent's, without Escouade's variables, plus env. Its output comes by proc/out, its end by proc/exit; proc/out may come before this answer. Each start goes to the journal. */
  'proc/spawn': { params: ProcSpawnParams; result: ProcSpawnResult };
  /** Writes on the input of a program; close ends that input. failed past 1 MiB it has not read. */
  'proc/write': { params: ProcWriteParams; result: null };
  /** Ends a program and what it started; proc/exit follows. */
  'proc/kill': { params: ProcRef; result: null };
  /** A line of its journal. */
  log: { params: LogParams; result: void };
}

/** The delay of each call that has one, in milliseconds: past it, Escouade gives up a call to the plugin (its default answer applies), and answers a call of the plugin at the latest. */
export const TIMEOUTS_MS = {
  initialize: 10000,
  shutdown: 1000,
  'permissions/decide': 500,
  'files/read': 5000,
  'net/fetch': 10000,
  'net/stream': 10000,
  'ws/open': 10000,
} as const;

/** The delay of the other calls, in milliseconds. */
export const DEFAULT_TIMEOUT_MS = 5000;

/** The error codes of the protocol. */
export const ERRORS = {
  /** A line that is not JSON. */
  parse: -32700,
  /** A message that is not JSON-RPC 2.0. */
  invalidRequest: -32600,
  /** A method the other side does not serve. */
  methodNotFound: -32601,
  /** Parameters that do not hold. */
  invalidParams: -32602,
  /** The handler failed. */
  internal: -32603,
  /** A secret that is not one of the plugin's own. */
  notYours: -32001,
  /** The system's keychain cannot be reached. */
  keychain: -32002,
  /** What the plugin may not reach: a host outside permissions.network, http, an address of the machine or of the local network (net/fetch); a path outside its folder, hidden, or its manifest (files/read); a path out of its roots (fs/*); a program, an argument, a folder or a variable it may not name (proc/spawn). A capability it does not declare says the same. */
  refused: -32003,
  /** A request that failed on the way: no answer within its delay, an answer or a file too big, a file that is not UTF-8. */
  failed: -32004,
  /** No such file in the plugin's folder (files/read), or under a root (fs/*). */
  notFound: -32005,
} as const;
