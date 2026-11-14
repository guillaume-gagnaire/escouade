// Le client de l'API distante, réduit au faux serveur de l'étude (`shared/fake-server`).
import type { UserItem, TextItem, PermissionItem } from '../../../src/lib/types';

/** Un message tel que le faux serveur le rejoue : les champs de `ConvItem` d'Escouade dont la maquette a besoin. */
export type MobileItem =
  | (Pick<UserItem, 'id' | 'kind' | 'text' | 'ts'>)
  | (Pick<TextItem, 'id' | 'kind' | 'text'> & { ts: number })
  | { id: string; kind: 'tool'; name: string; status: string; ts: number; input: Record<string, string>; output?: string; diff?: string }
  | (Pick<PermissionItem, 'id' | 'kind' | 'toolUseId' | 'toolName'> & { ts: number; input: { command: string; description: string } });

export interface WaitingAgent {
  id: string;
  name: string;
  project: string;
  waiting: 'permission' | 'question' | null;
  preview: string;
}

const KEY = 'ms.pairing';

export interface Pairing {
  url: string;
  key: string;
  name: string;
}

export function pairing(): Pairing {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved) return JSON.parse(saved);
  } catch {
    /* pas de stockage : celui de la compilation */
  }
  return { url: import.meta.env.VITE_MS_SERVER ?? 'http://10.0.2.2:8445', key: import.meta.env.VITE_MS_KEY ?? '', name: '' };
}

/** Lit `escouade://pair?u=…&k=…&n=…`. */
export function parsePairUri(uri: string): Pairing | null {
  const m = uri.match(/^escouade:\/\/pair\?(.*)$/);
  if (!m) return null;
  const q = new URLSearchParams(m[1]);
  const url = q.get('u');
  const key = q.get('k');
  if (!url || !key) return null;
  return { url, key, name: q.get('n') ?? '' };
}

export function savePairing(p: Pairing) {
  try {
    localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    /* tant pis */
  }
}

const base = () => pairing().url;

export async function getJson<T>(path: string): Promise<T> {
  const r = await fetch(base() + path);
  if (!r.ok) throw new Error(`${path} : ${r.status}`);
  return r.json();
}

export async function postJson(path: string, body: unknown) {
  const r = await fetch(base() + path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
  if (!r.ok) throw new Error(`${path} : ${r.status}`);
  return r.json();
}

/** Un repère pour le banc (démarrage à froid, conversation affichée…), noté par le faux serveur à l'heure de l'hôte. */
export function mark(app: string, ev: string, v?: string | number) {
  const q = new URLSearchParams({ app, ev });
  if (v !== undefined) q.set('v', String(v));
  fetch(`${base()}/api/mark?${q}`).catch(() => {});
}
