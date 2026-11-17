/**
 * Where the window stands with the core (ESC-495): `local` on Tauri's IPC (it never loses it), else the remote API's
 * WebSocket, opening, open, lost (a new try is under way) or refused (the tries stopped: `reason` says why). The band
 * `ConnectionBand` reads it; `transport.ts` writes it.
 */
export type ConnectionState = 'local' | 'connecting' | 'open' | 'lost' | 'refused';

export const connection = $state<{ state: ConnectionState; reason: string }>({ state: 'local', reason: '' });
