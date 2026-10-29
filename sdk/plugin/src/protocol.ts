// Generated from schemas/plugin-protocol-1.json by sdk/plugin/scripts/generate.mjs: do not edit, run it again.
/** JSON-RPC 2.0 over the standard input and output of a plugin's program, one message per line. The one source of the protocol's types: the TypeScript of @escouade-dev/plugin is generated from it (sdk/plugin/scripts/generate.mjs), and the Rust tests of the core hold it to what the core sends (plugins_protocol_schema). */
export const PROTOCOL_VERSION = 1;

/** The facts of Escouade. */
export type FactName =
  | 'agent.created'
  | 'agent.status'
  | 'agent.waiting'
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
  | 'app.started';

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
  /** The column it left (ticket.column). */
  from?: string;
  /** The column it is in (ticket.column). */
  column?: string;
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
  /** When, in RFC 3339. */
  at: string;
  project?: Project;
  ticket?: Ticket;
  agent?: Agent;
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

export interface LogParams {
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
}

/** Every fact, in the order of the schema. */
export const FACT_NAMES: readonly FactName[] = [
  'agent.created',
  'agent.status',
  'agent.waiting',
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
}

/** What the plugin sends to Escouade: each method, its parameters and its answer (`void` for a notification). */
export interface ToEscouade {
  /** One of its secrets (a key of permissions.secrets), or null when none is kept. */
  'secrets/get': { params: SecretsGetParams; result: string | null };
  /** A line of its journal. */
  log: { params: LogParams; result: void };
}

/** The delay of each of Escouade's calls, in milliseconds; past it, its default answer applies. */
export const TIMEOUTS_MS = {
  initialize: 10000,
  shutdown: 1000,
  'permissions/decide': 500,
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
} as const;
