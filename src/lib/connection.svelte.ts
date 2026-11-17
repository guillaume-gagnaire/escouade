/**
 * Where the window stands with the core (ESC-495): `local` on Tauri's IPC (it never loses it), else the remote API's
 * WebSocket, opening, open, lost (a new try is under way) or refused (the tries stopped: `reason` says why). The band
 * `ConnectionBand` reads it; `transport.ts` writes it.
 *
 * `secretsInFile` (ESC-514): the core the window speaks to keeps its secrets in a file of its data readable by its owner
 * alone, not in a keychain (an `escouade-server`, which its `welcome` says): where a secret is given, the window says so.
 */
export type ConnectionState = 'local' | 'connecting' | 'open' | 'lost' | 'refused';

export const connection = $state<{ state: ConnectionState; reason: string; secretsInFile: boolean }>({
  state: 'local',
  reason: '',
  secretsInFile: false,
});
