import { createError, getQuery, setResponseHeader } from 'h3';
import { normalizePath } from '../../../shared/folder';
import { cachedFolders, folderList, judgeFolder, treeRead } from '../../utils/folder';
import { GithubError } from '../../utils/github';
import { requireAccount } from '../../utils/session';
import { TooBig } from '../../utils/source';
import { submissionsReady } from '../../utils/submit-route';
import { pickRepo } from '../../utils/submissions';
import { ARCHIVE_MAX, readArchive } from '../../utils/tar';
import { compareVersions, tagVersion } from '../../utils/versions';

// The plugin's folder, picked before its version (ESC-313, docs/STORE.md §3.6): the folders of the repository that
// hold a manifest, read in the tree of a tag's commit (the newest, or `at`; its archive only past the tree's bound, kept
// for the last commits read), and the folder typed by the author judged: absolute, climbing, through a link, without
// a manifest. The checks judge it again on the tag submitted.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  const deps = submissionsReady();
  const q = getQuery(event);
  const repo = await pickRepo(db, account.id, q.repo);
  if ('error' in repo) throw createError({ statusCode: repo.error === 'not-yours' ? 404 : 409, statusMessage: repo.error });
  const typed = normalizePath(q.path ?? '');
  const path = 'path' in typed ? typed.path : null;
  try {
    const source = deps.source(repo.installationId);
    const tags = (await source.tags(repo.fullName)).sort((a, b) => compareVersions(tagVersion(b.name), tagVersion(a.name)));
    const at = (typeof q.at === 'string' && tags.find((t) => t.name === q.at)) || tags[0];
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
});
