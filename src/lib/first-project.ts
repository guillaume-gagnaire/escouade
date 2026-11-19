// The station of the first project (ESC-505, docs/ONBOARDING.md § 4.4): what it proposes and reads, apart from its
// screen. A folder of the machine, a clone (an address typed, or a repository of a code host connected), or the
// demonstration project; nothing is cloned, written nor opened before the user's gesture.

import { BY_FORGE } from './integrations';
import { api } from './ipc';

/** The same folder, whatever the spelling (`\` or `/`, a trailing separator, the case). */
export function samePath(a: string, b: string): boolean {
  const norm = (p: string) => p.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase();
  return !!a && !!b && norm(a) === norm(b);
}

/** The folder a clone of `url` goes to by default: the last part of its path, without `.git`. */
export function cloneName(url: string): string {
  const path = url
    .trim()
    .replace(/[?#].*$/, '')
    .replace(/\/+$/, '');
  const last = path.split(/[/:]/).pop() ?? '';
  return last.replace(/\.git$/i, '');
}

/**
 * The address of a repository of a code host connected, from what its ticket source lists: GitHub's `owner/repo` on
 * its host, GitLab's `host/group/project` as it is.
 */
export function repoUrl(id: string, host: string): string {
  const first = id.split('/')[0] ?? '';
  const whole = first.includes('.') || first.includes(':') ? id : `${host.replace(/^https?:\/\//, '').replace(/\/+$/, '')}/${id}`;
  return `https://${whole}.git`;
}

/**
 * The folder a clone goes into by default: the one that holds the projects of this device, else the user's home, read
 * from where the demonstration lives (`<home>/.escouade/demo/bac-a-sable`).
 */
export function cloneParent(projects: string[], demo: string): string {
  const parents = new Map<string, number>();
  for (const p of projects) {
    const parent = p.replace(/[\\/]+$/, '').replace(/[\\/][^\\/]*$/, '');
    if (parent && parent !== p) parents.set(parent, (parents.get(parent) ?? 0) + 1);
  }
  let best = '';
  let n = 0;
  for (const [k, v] of parents) if (v > n) [best, n] = [k, v];
  if (best) return best;
  // `<data>/demo/bac-a-sable`: the data folder's parent.
  const parts = demo.replace(/[\\/]+$/, '').split(/[\\/]/);
  return parts.length > 4 ? parts.slice(0, -3).join(demo.includes('\\') ? '\\' : '/') : '';
}

/** What an `.escouade.yml` sets, said in a few of its keys (« board.merge_strategy, launch… »). */
export function ymlList(keys: string[], max = 4): string {
  const shown = keys.slice(0, max).join(', ');
  return keys.length > max ? `${shown}…` : shown;
}

/** The repositories of the code hosts connected, each host's in a group; none when none is (or none answers). */
export async function hostRepos(): Promise<{ host: string; urls: string[] }[]> {
  const forges = await api.forges().catch(() => null);
  if (!forges) return [];
  const out: { host: string; urls: string[] }[] = [];
  for (const acc of forges.accounts) {
    if (!acc.issues || acc.status.health !== 'connected') continue;
    const service = Object.keys(BY_FORGE).find((k) => BY_FORGE[k] === acc.kind);
    if (!service) continue;
    const containers = await api.integrationContainers(service, null).catch(() => []);
    const urls = containers.map((c) => repoUrl(c.id, acc.host));
    if (urls.length) out.push({ host: acc.host, urls });
  }
  return out;
}
