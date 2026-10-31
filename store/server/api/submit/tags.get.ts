import { createError, getQuery, setResponseHeader } from 'h3';
import { normalizePath, tagFor } from '../../../shared/folder';
import { GithubError } from '../../utils/github';
import { requireAccount } from '../../utils/session';
import { submissionsReady } from '../../utils/submit-route';
import { pickRepo } from '../../utils/submissions';
import { compareVersions, tagVersion } from '../../utils/versions';

// The second step: the tags and releases of a repository the author picked, the newest version first, each with the
// commit it names now (docs/STORE.md §3.1: "v1.2.0 → 3f9c2e1"). With a folder (`path`, ESC-313), that folder's tags
// alone (`v1.2.3` at the root; `<folder>/v1.2.3`, `<name>@1.2.3` or `<name>-v1.2.3` in a folder), and how many
// others were left out.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  const deps = submissionsReady();
  const q = getQuery(event);
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
});
