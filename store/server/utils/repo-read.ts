import { createError } from 'h3';
import { normalizePath, tagFor } from '../../shared/folder';
import type { Account } from './accounts';
import type { Db } from './db';
import { cachedFolders, folderList, judgeFolder, listingTag, treeRead } from './folder';
import { GithubError } from './github';
import { TooBig } from './source';
import { pickRepo, type Deps } from './submissions';
import { ARCHIVE_MAX, readArchive } from './tar';
import { compareVersions, tagVersion } from './versions';

// What the submission reads of a repository before a version is submitted, the same for the site's routes
// (/api/submit/folders, /api/submit/tags) and the MCP server's tools (ESC-317): the same checks, the same refusals.

/**
 * The plugin's folder, picked before its version (ESC-313, docs/STORE.md §3.6): the folders of the repository that
 * hold a manifest, read in the tree of a tag's commit (`at`, else `listingTag`; its archive only past the tree's
 * bound, kept for the last commits read), and the folder typed by the author judged: absolute, climbing, through a
 * link, without a manifest. The checks judge it again on the tag submitted.
 */
export async function repoFolders(
  db: Db,
  deps: Pick<Deps, 'source'>,
  account: Account,
  q: { repo?: unknown; path?: unknown; at?: unknown },
) {
  const repo = await pickRepo(db, account.id, q.repo);
  if ('error' in repo) throw createError({ statusCode: repo.error === 'not-yours' ? 404 : 409, statusMessage: repo.error });
  const typed = normalizePath(q.path ?? '');
  const path = 'path' in typed ? typed.path : null;
  try {
    const source = deps.source(repo.installationId);
    const tags = await source.tags(repo.fullName);
    const at = (typeof q.at === 'string' && tags.find((t) => t.name === q.at)) || listingTag(tags, path);
    if (!at) return { repository: repo.fullName, tag: null, commit: null, folders: [], more: false, path, problem: null };
    const list = await cachedFolders(`${repo.fullName}@${at.commit}`, async () => {
      // The commit's tree, without a byte of its files; the archive only when GitHub cut the tree short.
      const tree = await source.tree(repo.fullName, at.commit);
      if (!tree.truncated) return folderList(treeRead(tree.entries));
      try {
        return folderList(readArchive(await source.tarball(repo.fullName, at.commit, ARCHIVE_MAX)));
      } catch (e) {
        if (e instanceof TooBig) return { folders: [], more: false, all: new Set<string>(), links: [] };
        throw e;
      }
    });
    const problem = 'problem' in typed ? typed.problem : judgeFolder(list, typed.path);
    return { repository: repo.fullName, tag: at.name, commit: at.commit, folders: list.folders, more: list.more, path, problem };
  } catch (e) {
    console.error('[store] folders:', e instanceof Error ? e.message : e);
    throw createError({
      statusCode: e instanceof GithubError && e.status === 404 ? 404 : 502,
      statusMessage: 'GitHub did not give the archive',
    });
  }
}

/**
 * The tags and releases of a repository the author picked, the newest version first, each with the commit it names
 * now (docs/STORE.md §3.1: "v1.2.0 → 3f9c2e1"). With a folder (`path`, ESC-313), that folder's tags alone (`v1.2.3`
 * at the root; `<folder>/v1.2.3`, `<name>@1.2.3` or `<name>-v1.2.3` in a folder), and how many others were left out.
 */
export async function repoTags(db: Db, deps: Pick<Deps, 'source'>, account: Account, q: { repo?: unknown; path?: unknown }) {
  const repo = await pickRepo(db, account.id, q.repo);
  if ('error' in repo) throw createError({ statusCode: repo.error === 'not-yours' ? 404 : 409, statusMessage: repo.error });
  const folder = q.path === undefined ? null : normalizePath(q.path);
  if (folder && 'problem' in folder) throw createError({ statusCode: 400, statusMessage: `path-${folder.problem}` });
  const path = folder?.path ?? null;
  try {
    const all = await deps.source(repo.installationId).tags(repo.fullName);
    const version = (name: string) => (path === null ? tagVersion(name) : (tagFor(name, path) ?? tagVersion(name)));
    const tags = (path === null ? all : all.filter((t) => tagFor(t.name, path) !== null)).sort((a, b) =>
      compareVersions(version(b.name), version(a.name)),
    );
    return { repository: repo.fullName, path, tags, others: all.length - tags.length };
  } catch (e) {
    console.error('[store] tags:', e instanceof Error ? e.message : e);
    throw createError({
      statusCode: e instanceof GithubError && e.status === 404 ? 404 : 502,
      statusMessage: 'GitHub did not list the tags',
    });
  }
}
