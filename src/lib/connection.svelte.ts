/**
 * Where the window stands with the core (ESC-495): `local` on Tauri's IPC (it never loses it), else the remote API's
 * WebSocket, opening, open, lost (a new try is under way) or refused (the tries stopped: `reason` says why). The band
 * `ConnectionBand` reads it; `transport.ts` writes it.
 *
 * `secretsInFile` (ESC-514): the core the window speaks to keeps its secrets in a file of its data readable by its owner
 * alone, not in a keychain (an `escouade-server`, which its `welcome` says): where a secret is given, the window says so.
 *
 * On a server (ESC-537, « Se connecter à un serveur »): `server` is the one the window is on (its name, its SSH host,
 * its machine), `latency` the last way there and back, `lostAt` when the connection went, `tries` and `retryAt` the
 * tries since and when the next one comes, `refusal` what the server answered when it stopped them (a revoked token
 * says when and by whom; another version says which side to update), `problem` why the tunnel itself stopped.
 *
 * `serverUpdate` (ESC-515), in a refusal of the versions and its problem, when the server is the one to update: how to
 * put it at this window's version, `command` (`escouade-server update --to <version>`, run on it: it checks the
 * signature and waits for the turns) or `image` (a new image of its container), with what to run.
 */
export type ConnectionState = 'local' | 'connecting' | 'open' | 'lost' | 'refused';

/** A server the window may be on, as this computer knows it (`servers::ServerView`). */
export interface ServerInfo {
  id: string;
  name: string;
  ssh: string;
  host: string;
  remotePort: number;
  device: string;
  version?: string;
  machine?: { host: string; user: string };
  lastSeen?: number;
  problem?: ServerProblem;
  active: boolean;
}

/** How to put a server at the client's version (`remoteapi::proto::ServerUpdate`, ESC-515). */
export interface ServerUpdate {
  how: 'command' | 'image';
  command: string;
}

/** What went wrong with a server (`servers::Problem`, `docs/SERVER-UX.md` § 3). */
export type ServerProblem =
  | { kind: 'policy'; message: string }
  | { kind: 'unreachable'; why: string; output: string; command: string }
  | { kind: 'hostKey'; changed: boolean; output: string; command: string }
  | { kind: 'noEscouade'; output: string; command: string }
  | { kind: 'code'; reason: string; message: string }
  | { kind: 'version'; server: string; client: string; update: string; serverUpdate?: ServerUpdate }
  | { kind: 'revoked'; revoked?: { device: string; at: number; by?: string } }
  | { kind: 'refused'; code: string; message: string };

/** What the server answered when it stopped the tries. */
export interface Refusal {
  code: string;
  error: string;
  serverVersion?: string;
  update?: string;
  /** This window's version, as it said it. */
  clientVersion?: string;
  revoked?: { device: string; at: number; by?: string };
  /** For the server to update: how (ESC-515). */
  serverUpdate?: ServerUpdate;
}

/** How a connection to a core stands: the window's (`connection`), or a space's own (ESC-539). */
export interface Connection {
  state: ConnectionState;
  reason: string;
  secretsInFile: boolean;
  server: ServerInfo | null;
  machine: { host: string; user: string } | null;
  latency: number | null;
  lostAt: number | null;
  tries: number;
  retryAt: number | null;
  refusal: Refusal | null;
  problem: ServerProblem | null;
  /** The events of the state the session missed during the last cut (ESC-538); null without one. */
  missed: number | null;
}

const fresh = (): Connection => ({
  state: 'local',
  reason: '',
  secretsInFile: false,
  server: null,
  machine: null,
  latency: null,
  lostAt: null,
  tries: 0,
  retryAt: null,
  refusal: null,
  problem: null,
  missed: null,
});

export const connection = $state<Connection>(fresh());

/** A connection of its own, for another space the window reads (its pile, « Partout »). */
export function connectionOf(): Connection {
  const c = $state<Connection>({ ...fresh(), state: 'connecting' });
  return c;
}

/** The window is on a server (not this computer's core). */
export const onServer = (): boolean => connection.server !== null;
