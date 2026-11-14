// Le client de l'API distante, réduit au faux serveur de l'étude : le même que celui de l'interface Svelte
// (mobile-study/svelte-ui/src/server.ts), le stockage en plus dans le trousseau (expo-secure-store).
import * as SecureStore from 'expo-secure-store';
import type { PermissionItem, TextItem, UserItem } from './escouade';

export type MobileItem =
  | Pick<UserItem, 'id' | 'kind' | 'text' | 'ts'>
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

export interface Pairing {
  url: string;
  key: string;
  name: string;
}

let current: Pairing = {
  url: process.env.EXPO_PUBLIC_MS_SERVER ?? 'http://10.0.2.2:8445',
  key: process.env.EXPO_PUBLIC_MS_KEY ?? '',
  name: '',
};

export const pairing = () => current;

export async function loadPairing() {
  try {
    const saved = await SecureStore.getItemAsync('ms.pairing');
    if (saved) current = JSON.parse(saved);
  } catch {
    /* celui de la compilation */
  }
  return current;
}

export async function savePairing(p: Pairing) {
  current = p;
  await SecureStore.setItemAsync('ms.pairing', JSON.stringify(p));
}

/** Lit `escouade://pair?u=…&k=…&n=…`. */
export function parsePairUri(uri: string): Pairing | null {
  const m = uri.match(/^escouade:\/\/pair\?(.*)$/);
  if (!m) return null;
  const q = new URLSearchParams(m[1]);
  const url = q.get('u');
  const key = q.get('k');
  return url && key ? { url, key, name: q.get('n') ?? '' } : null;
}

export async function getJson<T>(path: string): Promise<T> {
  const r = await fetch(current.url + path);
  if (!r.ok) throw new Error(`${path} : ${r.status}`);
  return r.json();
}

export async function postJson(path: string, body: unknown) {
  const r = await fetch(current.url + path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
  return r.json();
}

export function mark(ev: string, v?: string | number) {
  const q = new URLSearchParams({ app: 'react-native', ev });
  if (v !== undefined) q.set('v', String(v));
  fetch(`${current.url}/api/mark?${q}`).catch(() => {});
}
