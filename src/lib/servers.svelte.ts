/**
 * The servers this computer knows (ESC-537, `docs/SERVER-UX.md`, « Se connecter à un serveur »): read from this
 * computer's own core (`local`, whichever core the window speaks to), and what moves the window from one machine to
 * another. The window is on one machine at a time: going to a server, or back on this computer, loads it again on that
 * machine's core (`transport.ts` asks `server_link` as it starts).
 */

import { connection, type ServerInfo, type ServerProblem } from './connection.svelte';
import { locale, t } from './i18n';
import { app } from './state.svelte';
import type { Locks } from './types';
import { local, remoteTransport } from './transport';

export interface SshHost {
  alias: string;
  hostName: string;
  user?: string;
  port?: number;
  proxyJump?: string;
}

export interface ServersView {
  servers: ServerInfo[];
  active: string | null;
  tokensInFile: boolean;
  thisDevice: string;
  /** This computer's policies (ESC-539), whichever core the window speaks to. */
  locks?: Locks;
}

/** The answer of a step of the dialog (`servers::Outcome`). */
export type Outcome<T> = { outcome: 'ok'; value: T } | { outcome: 'problem'; problem: ServerProblem };

export interface Reached {
  attempt: string;
  version: string;
  name: string;
  device: string;
  host: string;
}

export const servers = $state<{ view: ServersView | null }>({ view: null });

/** The servers read again. */
export async function loadServers(): Promise<ServersView | null> {
  try {
    servers.view = await local<ServersView>('servers_view');
  } catch {
    servers.view = null;
  }
  return servers.view;
}

/** Whether this computer knows a server: the machine is said from then on (the title bar, the rail). */
export const knowsServers = (): boolean => (servers.view?.servers.length ?? 0) > 0 || connection.server !== null;

/** « Se connecter à un serveur »: the dialog, on its first step (or pairing again `existing`). */
export function openConnect(existing?: string): void {
  app.modal = existing ? { kind: 'connectServer', existing } : { kind: 'connectServer' };
}

/** The window loads again, on the machine the core says. */
function reload(): void {
  location.reload();
}

/** The window to the server `id`: its tunnel up and this computer's token accepted, then loaded again on it. */
export async function goToServer(id: string): Promise<ServerProblem | null> {
  const r = await local<Outcome<null>>('server_open', { id });
  if (r.outcome === 'problem') {
    void loadServers();
    return r.problem;
  }
  reload();
  return null;
}

/** Back on this computer. */
export async function goHome(): Promise<void> {
  await local('server_leave');
  reload();
}

/** « Réessayer »: the tunnel and the socket tried again now. */
export async function retryNow(): Promise<void> {
  await local('server_retry').catch(() => {});
  remoteTransport()?.retryNow();
}

/** « Retirer » (S15): asked to revoke this computer, then forgotten here. Whether the server revoked it. */
export async function forgetServer(id: string): Promise<boolean> {
  const onIt = connection.server?.id === id;
  const revoked = await local<boolean>('server_forget', { id });
  if (onIt) {
    reload();
    return revoked;
  }
  await loadServers();
  return revoked;
}

/** The ordinal of a try: « 6ᵉ », « 6th ». */
export function nth(n: number): string {
  if (locale.ui === 'fr') return n === 1 ? '1ʳᵉ' : `${n}ᵉ`;
  const rule = new Intl.PluralRules('en', { type: 'ordinal' }).select(n);
  const suffix = { one: 'st', two: 'nd', few: 'rd', other: 'th', zero: 'th', many: 'th' }[rule];
  return `${n}${suffix}`;
}

/** The space's plate: its first letter (a screen stands for this computer). */
export function plateOf(name: string): string {
  const letter = [...name.trim()].find((c) => /\p{L}|\p{N}/u.test(c));
  return (letter ?? '?').toLocaleUpperCase();
}

/** The name the window says of the machine it is on. */
export function machineName(): string {
  return connection.server?.name ?? t('servers.thisComputer');
}

/** Where a server's agents run, as its terminal and its headings say it: « escouade@vps.example.net ». */
export function serverWhere(): string {
  const m = connection.machine ?? connection.server?.machine;
  if (m?.host) return m.user ? `${m.user}@${m.host}` : m.host;
  return connection.server?.host ?? '';
}
