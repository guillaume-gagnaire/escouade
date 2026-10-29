import { appJwt, GithubError, type GithubApp } from './github';

// What the submission reads of a repository, through the App's installation on it (Contents and Metadata, read
// only; docs/STORE.md §3.2): the repository, its tags and releases, a tag resolved into its commit, the commit's
// archive, the license GitHub detects, the release's files and their attestations. Each download is bounded while
// it arrives: a file past its size stops there, before it fills the memory. The tests give their own `Source`.

export interface TagChoice {
  name: string;
  commit: string;
  /** Whether a release is attached to it (the release's date then), as the submission page shows it. */
  release: boolean;
  date: string | null;
}

export interface Asset {
  name: string;
  size: number;
  url: string;
}

export interface Source {
  repo(fullName: string): Promise<{ id: number; fullName: string; private: boolean }>;
  tags(fullName: string): Promise<TagChoice[]>;
  /** The commit a tag names now, an annotated tag unwrapped. */
  resolve(fullName: string, tag: string): Promise<{ commit: string; date: string | null }>;
  tarball(fullName: string, commit: string, max: number): Promise<Buffer>;
  /** The SPDX id GitHub detects in the commit's license file (`NOASSERTION` if it does not know it), null without one. */
  license(fullName: string, commit: string): Promise<string | null>;
  /** The files of the release attached to the tag, null if the tag has none. */
  release(fullName: string, tag: string): Promise<Asset[] | null>;
  asset(asset: Asset, max: number): Promise<Buffer>;
  attestations(fullName: string, sha256: string): Promise<unknown[]>;
}

export class TooBig extends Error {
  constructor(readonly size: number) {
    super(`past ${size} bytes`);
  }
}

/** The body of `res`, `max` bytes at most: past them, it stops reading and throws `TooBig`. */
async function bounded(res: Response, max: number): Promise<Buffer> {
  const declared = Number(res.headers.get('content-length') ?? NaN);
  if (declared > max) {
    await res.body?.cancel();
    throw new TooBig(declared);
  }
  if (!res.body) return Buffer.alloc(0);
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of res.body as unknown as AsyncIterable<Uint8Array>) {
    size += chunk.length;
    if (size > max) {
      await res.body?.cancel().catch(() => {});
      throw new TooBig(size);
    }
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}

const tokens = new Map<number, { token: string; until: number }>();

/** The installation's token (one hour at GitHub), asked with the App's JWT, kept until five minutes before its end. */
async function installationToken(app: GithubApp, installationId: number): Promise<string> {
  const kept = tokens.get(installationId);
  if (kept && kept.until > Date.now()) return kept.token;
  const res = await fetch(`${app.api}/app/installations/${installationId}/access_tokens`, {
    method: 'POST',
    headers: {
      accept: 'application/vnd.github+json',
      authorization: `Bearer ${appJwt(app)}`,
      'user-agent': 'escouade-store',
      'x-github-api-version': '2022-11-28',
    },
    signal: AbortSignal.timeout(15_000),
  });
  if (!res.ok) throw new GithubError(`GitHub answered ${res.status} to the installation's token`, res.status);
  const body = (await res.json()) as { token: string; expires_at?: string };
  const until = (body.expires_at ? Date.parse(body.expires_at) : Date.now() + 3600_000) - 300_000;
  tokens.set(installationId, { token: body.token, until });
  return body.token;
}

const repoPath = (fullName: string) => fullName.split('/').map(encodeURIComponent).join('/');

/** The repositories of `installationId`, read through the App. */
export function githubSource(app: GithubApp, installationId: number): Source {
  const get = async (path: string, init: { accept?: string; timeout?: number } = {}) => {
    const token = await installationToken(app, installationId);
    return fetch(path.startsWith('http') ? path : `${app.api}${path}`, {
      headers: {
        accept: init.accept ?? 'application/vnd.github+json',
        authorization: `Bearer ${token}`,
        'user-agent': 'escouade-store',
        'x-github-api-version': '2022-11-28',
      },
      redirect: 'follow',
      signal: AbortSignal.timeout(init.timeout ?? 15_000),
    });
  };
  const json = async <T>(path: string, missing?: T): Promise<T> => {
    const res = await get(path);
    if (res.status === 404 && missing !== undefined) return missing;
    if (!res.ok) throw new GithubError(`GitHub answered ${res.status} to ${new URL(res.url || `${app.api}${path}`).pathname}`, res.status);
    return (await res.json()) as T;
  };
  return {
    async repo(fullName) {
      const r = await json<{ id: number; full_name: string; private: boolean }>(`/repos/${repoPath(fullName)}`);
      return { id: r.id, fullName: r.full_name, private: r.private };
    },
    async tags(fullName) {
      const [tags, releases] = await Promise.all([
        json<{ name: string; commit: { sha: string } }[]>(`/repos/${repoPath(fullName)}/tags?per_page=100`),
        json<{ tag_name: string; draft: boolean; published_at: string | null }[]>(`/repos/${repoPath(fullName)}/releases?per_page=100`, []),
      ]);
      const published = new Map(releases.filter((r) => !r.draft).map((r) => [r.tag_name, r.published_at]));
      return tags.map((t) => ({ name: t.name, commit: t.commit.sha, release: published.has(t.name), date: published.get(t.name) ?? null }));
    },
    async resolve(fullName, tag) {
      let ref = await json<{ object: { type: string; sha: string } }>(
        `/repos/${repoPath(fullName)}/git/ref/tags/${encodeURIComponent(tag)}`,
      );
      let date: string | null = null;
      for (let depth = 0; ref.object.type === 'tag' && depth < 5; depth++) {
        const t = await json<{ object: { type: string; sha: string }; tagger?: { date?: string } }>(
          `/repos/${repoPath(fullName)}/git/tags/${ref.object.sha}`,
        );
        date ??= t.tagger?.date ?? null;
        ref = t;
      }
      if (ref.object.type !== 'commit') throw new GithubError(`the tag ${tag} names no commit`);
      return { commit: ref.object.sha, date };
    },
    async tarball(fullName, commit, max) {
      const res = await get(`/repos/${repoPath(fullName)}/tarball/${commit}`, { timeout: 120_000 });
      if (!res.ok) throw new GithubError(`GitHub answered ${res.status} to the archive`, res.status);
      return bounded(res, max);
    },
    async license(fullName, commit) {
      const body = await json<{ license?: { spdx_id?: string | null } } | null>(
        `/repos/${repoPath(fullName)}/license?ref=${encodeURIComponent(commit)}`,
        null,
      );
      return body ? (body.license?.spdx_id ?? 'NOASSERTION') : null;
    },
    async release(fullName, tag) {
      const body = await json<{ assets: { name: string; size: number; url: string }[] } | null>(
        `/repos/${repoPath(fullName)}/releases/tags/${encodeURIComponent(tag)}`,
        null,
      );
      return body ? body.assets.map((a) => ({ name: a.name, size: a.size, url: a.url })) : null;
    },
    async asset(asset, max) {
      if (asset.size > max) throw new TooBig(asset.size);
      if (!asset.url.startsWith(`${app.api}/`)) throw new GithubError('an asset outside the API');
      const res = await get(asset.url, { accept: 'application/octet-stream', timeout: 120_000 });
      if (!res.ok) throw new GithubError(`GitHub answered ${res.status} to ${asset.name}`, res.status);
      return bounded(res, max);
    },
    async attestations(fullName, sha256) {
      const body = await json<{ attestations?: unknown[] }>(`/repos/${repoPath(fullName)}/attestations/sha256:${sha256}`, {});
      return body.attestations ?? [];
    },
  };
}
