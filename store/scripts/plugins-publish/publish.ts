import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { RESERVED_PREFIX } from '../../../schemas/plugin-manifest.mjs';
import { reportVerdict } from '../plugin-review/admin';
import { catalog, type Source } from '../plugin-review/download';
import type { CatalogVersion } from '../../server/utils/catalog';

// The chief's gestures for a plugin of the first hand (docs/PLUGINS.md §8.6.2), one after the other: its tag, its
// submission to the store as anyone's, its checks, its publication, the local review, then, on the reader's verdict
// alone, the report and the signature. The store only reviews and signs a published version (docs/STORE.md §4.3): it is
// published, unverified and asking for the review, before the review reads it; the app takes none of the first hand
// unsigned. Nothing is kept of a run: where a plugin stands is read from what the gestures left, the catalogue of the
// store (published, signed) and the report beside the others, so a run that stopped takes up there, and a report a
// reader filled in is never written again. The key is read by `plugin-review -- sign` alone.

export interface Plugin {
  /** Its folder under `plugins/`, the short name of its tag. */
  short: string;
  id: string;
  version: string;
}

/** A plugin's folder in the repository and its tag (docs/PLUGINS.md §8.6.2). */
export const folderOf = (p: Plugin) => `plugins/${p.short}`;
export const tagOf = (p: Plugin) => `${folderOf(p)}/v${p.version}`;
const label = (p: Plugin) => `${p.id}@${p.version}`;

/** The run cannot start: nothing was done. */
export class Refused extends Error {}
/** The run stops here, and a run after it takes up where this one stopped. */
export class Stop extends Error {}

export type Tool = (name: string, args: Record<string, unknown>) => Promise<Record<string, unknown>>;

export interface Options {
  /** The app's repository, whose `origin` the tags go to. */
  repo: string;
  /** `owner/name` on GitHub, as the store knows it. */
  repository: string;
  /** The folders named; none: all of them, for `--changed`. */
  names: string[];
  source: Source;
  /** Where `plugin-review` writes its reports. */
  reports: string;
  tool: Tool;
  /** Runs `plugin-review` with these arguments, in this terminal; gives its code. */
  review: (args: string[]) => Promise<number>;
  /** How often a version in its checks is asked after, and for how long. */
  pollMs: number;
  waitMs: number;
  log: (line: string) => void;
}

/** A line of the store or of a plugin, on one line: it goes to the chief's terminal, never as a consigne. */
const line = (s: unknown) =>
  String(s)
    // eslint-disable-next-line no-control-regex
    .replace(/[\u0000-\u001f\u007f]+/g, ' ')
    .trim();

function git(repo: string, args: string[]): { ok: boolean; out: string; err: string } {
  const r = spawnSync('git', args, { cwd: repo, encoding: 'utf8', windowsHide: true });
  if (r.error) throw new Refused(`git ne se lance pas : ${r.error.message}`);
  return { ok: r.status === 0, out: (r.stdout ?? '').trim(), err: (r.stderr ?? '').trim() };
}

/**
 * The categories of a first version whose manifest cannot say them. The module of these three embeds its manifest
 * (`include_str!`, read by `harnesses_of`): a `store` key there would change its fingerprint (docs/PLUGINS.md §8.6.2,
 * §8.29). They take it with their next version, when a new fingerprint is due anyway.
 */
export const EMBEDDED_CATEGORIES: Record<string, string[]> = {
  'dev.escouade.cursor-cli': ['harness'],
  'dev.escouade.gemini-cli': ['harness'],
  'dev.escouade.mistral': ['harness'],
};

/** The plugins of the first hand of `plugins/`, by their folder; `0.0.0` is the mark of one still in the core. */
export function firstHand(repo: string): Plugin[] {
  const root = join(repo, 'plugins');
  return readdirSync(root, { withFileTypes: true })
    .filter((d) => d.isDirectory() && existsSync(join(root, d.name, 'escouade-plugin.json')))
    .map((d) => d.name)
    .sort()
    .flatMap((short) => {
      const m = JSON.parse(readFileSync(join(root, short, 'escouade-plugin.json'), 'utf8')) as { id?: unknown; version?: unknown };
      return typeof m.id === 'string' && m.id.startsWith(RESERVED_PREFIX) && typeof m.version === 'string'
        ? [{ short, id: m.id, version: m.version }]
        : [];
    });
}

/** What the run goes through: those named, or every plugin ported. */
export function chosen(all: Plugin[], names: string[], log: Options['log']): Plugin[] {
  const ported = all.filter((p) => p.version !== '0.0.0');
  if (!names.length) {
    const left = all.length - ported.length;
    if (left) log(`${left} plugin(s) pas encore portés (version 0.0.0) : laissés de côté.`);
    return ported;
  }
  return names.map((name) => {
    const p = all.find((x) => x.short === name);
    if (!p) throw new Refused(`plugins/${name} n’est pas un plugin de la première main`);
    if (p.version === '0.0.0') throw new Refused(`plugins/${name} n’a pas de version à lui (0.0.0) : il n’est pas encore porté`);
    return p;
  });
}

/** A tree that is the commit, and a commit that is on `origin/main`: what is tagged is what everyone reads. */
export function preflight(repo: string): string {
  const dirty = git(repo, ['status', '--porcelain']);
  if (!dirty.ok) throw new Refused(`git status a échoué : ${dirty.err}`);
  if (dirty.out) throw new Refused(`l’arbre n’est pas propre : rien n’est étiqueté\n${dirty.out}`);
  const fetched = git(repo, ['fetch', '--quiet', 'origin', 'main']);
  if (!fetched.ok) throw new Refused(`git fetch origin main a échoué : ${fetched.err}`);
  const head = git(repo, ['rev-parse', 'HEAD']).out;
  if (!git(repo, ['merge-base', '--is-ancestor', head, 'origin/main']).ok)
    throw new Refused(`le commit ${head.slice(0, 12)} n’est pas sur origin/main : pousse-le d’abord, rien n’est étiqueté`);
  return head;
}

/** The tag on `origin`, posed on `head` when it is not there yet. */
function tagged(o: Options, p: Plugin, head: string) {
  const tag = tagOf(p);
  const ref = `refs/tags/${tag}`;
  const remote = git(o.repo, ['ls-remote', 'origin', ref]);
  if (!remote.ok) throw new Stop(`${tag} : git ls-remote a échoué : ${remote.err}`);
  if (remote.out) return o.log(`${tag} : déjà étiqueté.`);
  const local = git(o.repo, ['rev-parse', '--quiet', '--verify', `${ref}^{commit}`]);
  if (local.ok && local.out !== head)
    throw new Stop(`${tag} existe ici sur ${local.out.slice(0, 12)}, pas sur ${head.slice(0, 12)} : à régler à la main`);
  if (!local.ok) {
    const made = git(o.repo, ['tag', tag, head]);
    if (!made.ok) throw new Stop(`${tag} : git tag a échoué : ${made.err}`);
  }
  const pushed = git(o.repo, ['push', '--quiet', 'origin', ref]);
  if (!pushed.ok) throw new Stop(`${tag} : git push a échoué : ${pushed.err}`);
  o.log(`${tag} : étiqueté sur ${head.slice(0, 12)} et poussé.`);
}

interface Check {
  key: string;
  status: string;
  detail?: { fr?: string };
  problems?: { path?: string; why?: { fr?: string } }[];
}

/** What the checks have against a version, one line each. */
function red(checks: Check[]): string {
  const bad = checks.filter((c) => c.status === 'fail' || c.status === 'hold' || c.status === 'skip');
  return bad
    .map((c) => {
      const why = c.problems?.map((x) => `${x.path ? `${x.path} : ` : ''}${x.why?.fr ?? ''}`).join(' ; ');
      return `  ${c.key} : ${line(why || c.detail?.fr || c.status)}`;
    })
    .join('\n');
}

interface Submission {
  id: number;
  state: string;
  first: boolean;
  suggested: { categories: string[]; keywords: string[] };
  checks: Check[];
  error?: string | null;
}

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms));

/** Submits the tag (the store gives back a submission already made) and waits for its checks. */
async function checked(o: Options, p: Plugin): Promise<Submission> {
  const asked = await o.tool('submit_version', { repository: o.repository, path: folderOf(p), tag: tagOf(p) });
  const id = asked.version_id as number;
  o.log(`${label(p)} : ${asked.again ? 'déjà soumise' : 'soumise'} (version ${id}), ses contrôles tournent.`);
  const until = Date.now() + o.waitMs;
  for (;;) {
    const v = (await o.tool('get_version', { version_id: id })) as unknown as Submission;
    if (v.state === 'ready' || v.state === 'published') return v;
    if (v.state === 'rejected') throw new Stop(`${label(p)} : un contrôle est rouge.\n${red(v.checks)}`);
    if (v.state === 'held') throw new Stop(`${label(p)} : retenue pour une personne, à régler dans la file du store.\n${red(v.checks)}`);
    if (v.state === 'error')
      throw new Stop(
        `${label(p)} : le store ou GitHub a échoué (${line(v.error)}) : retry_checks ${id} par le MCP du store, puis relance.`,
      );
    if (Date.now() >= until)
      throw new Stop(`${label(p)} : les contrôles tournent encore (version ${id}) : relance, la commande reprend ici.`);
    await sleep(o.pollMs);
  }
}

/** Published, unverified and asking for the review: what the review reads, what a signature covers. */
async function published(o: Options, p: Plugin, v: Submission) {
  if (v.state === 'published') return;
  const { keywords } = v.suggested;
  const categories = v.suggested.categories.length ? v.suggested.categories : (EMBEDDED_CATEGORIES[p.id] ?? []);
  if (v.first && !categories.length)
    throw new Stop(`${label(p)} : première version sans catégorie : son manifeste doit dire store.categories.`);
  await o.tool('publish_version', { version_id: v.id, ...(v.first ? { categories, keywords } : {}), request_review: true });
  o.log(`${label(p)} : publiée, non vérifiée, en attente de sa revue.`);
}

/** The local review, once: a report a reader filled in stays as it is. */
async function reviewed(o: Options, p: Plugin) {
  const at = join(o.reports, p.id);
  if (existsSync(join(at, `${p.version}.md`)) && existsSync(join(at, `${p.version}.json`))) return;
  const code = await o.review([label(p)]);
  if (code !== 0) throw new Stop(`${label(p)} : la revue locale s’arrête (code ${code}) : son rapport, s’il y en a un, est dans ${at}.`);
}

/** Stops on anything but the reader's « Vérifier »; the script's proposal never makes one. */
function settled(o: Options, p: Plugin) {
  const at = join(o.reports, p.id);
  const report = join(at, `${p.version}.md`);
  const data = JSON.parse(readFileSync(join(at, `${p.version}.json`), 'utf8')) as { proposed?: { verdict: string | null } };
  const verdict = reportVerdict(readFileSync(report, 'utf8'), data as never);
  if (verdict === 'Vérifier') return;
  if (verdict === null)
    throw new Stop(
      `${label(p)} : le rapport attend sa lecture : /review-plugin ${label(p)}, jusqu’à la ligne **Verdict final : Vérifier**, puis relance. Rapport : ${report}`,
    );
  throw new Stop(`${label(p)} : verdict « ${verdict} » : rien n’est déposé ni signé. Rapport : ${report}`);
}

const versionOf = async (o: Options, p: Plugin): Promise<CatalogVersion | undefined> =>
  (await catalog(o.source)).plugins.find((x) => x.id === p.id)?.versions.find((v) => v.version === p.version);

/** Everything a plugin still has to go through. */
async function one(o: Options, p: Plugin, head: string): Promise<boolean> {
  const entry = await versionOf(o, p);
  if (entry?.verified || entry?.yanked) return false;
  if (!entry) {
    tagged(o, p, head);
    await published(o, p, await checked(o, p));
  } else o.log(`${label(p)} : déjà au catalogue, non vérifiée.`);
  await reviewed(o, p);
  settled(o, p);
  for (const step of ['report', 'sign']) {
    const code = await o.review([step, label(p)]);
    if (code !== 0) throw new Stop(`${label(p)} : plugin-review ${step} a échoué (code ${code}) : relance, la commande reprend ici.`);
  }
  if (!(await versionOf(o, p))?.verified) throw new Stop(`${label(p)} : signée, mais le catalogue ne la dit pas vérifiée.`);
  o.log(`${label(p)} : publiée avec son sceau.`);
  return true;
}

/** Goes through the plugins in order, up to the first that stops: throws `Stop` there, gives how many were published. */
export async function publish(o: Options): Promise<number> {
  const plugins = chosen(firstHand(o.repo), o.names, o.log);
  const head = preflight(o.repo);
  let done = 0;
  for (const p of plugins) if (await one(o, p, head)) done++;
  return done;
}

/**
 * A tool of the store's MCP server, as an agent calls it (`POST /mcp`, each POST a whole exchange, docs/STORE.md §2.5): its
 * JSON, third-party mark taken off. A refusal of the store stops the run, with what to do when it is only a limit.
 */
export function storeTool(store: string, token: string): Tool {
  return async (name, args) => {
    const post = () =>
      fetch(`${store}/mcp`, {
        method: 'POST',
        headers: {
          authorization: `Bearer ${token}`,
          'content-type': 'application/json',
          accept: 'application/json, text/event-stream',
          'user-agent': 'escouade-plugins-publish',
        },
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name, arguments: args } }),
        redirect: 'error',
      });
    // git runs in step with this process: a socket the store closed meanwhile is only seen here. Once more, then: a tool
    // sent twice is refused or given back by the store, and a run takes up after a refusal.
    const res = await post().catch((e: unknown) => {
      if (e instanceof TypeError) return post();
      throw e;
    });
    if (res.status === 401) throw new Stop('le store ne prend pas ce jeton sur /mcp : un jeton d’administration, sur un store à jour');
    if (!res.ok) throw new Stop(`le store répond ${res.status} à ${name}`);
    const body = (await res.json()) as {
      error?: { message?: string };
      result?: { isError?: boolean; content: { text: string }[] };
    };
    if (!body.result) throw new Stop(`le store refuse ${name} : ${line(body.error?.message)}`);
    const text = body.result.content
      .at(-1)!
      .text.replace(/^<third_party_data>\n/, '')
      .replace(/\n<\/third_party_data>$/, '');
    const data = JSON.parse(text) as Record<string, unknown>;
    if (!body.result.isError) return data;
    if (data.error === 'too-many' || data.error === 'too-many-requests')
      throw new Stop(
        'la limite de soumissions du store est atteinte (20 par heure et par compte) : relance dans une heure, la commande reprend ici.',
      );
    throw new Stop(`le store refuse ${name} : ${line(data.error)}`);
  };
}
