import { createError, getQuery, setResponseHeader } from 'h3';
import { GithubError } from '../../utils/github';
import { requireAccount } from '../../utils/session';
import { submissionsReady } from '../../utils/submit-route';
import { pickRepo } from '../../utils/submissions';
import { compareVersions, tagVersion } from '../../utils/versions';

// The second step: the tags and releases of a repository the author picked, the newest version first, each with the
// commit it names now (docs/STORE.md §3.1: "v1.2.0 → 3f9c2e1").
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  const deps = submissionsReady();
  const repo = await pickRepo(db, account.id, getQuery(event).repo);
  if ('error' in repo) throw createError({ statusCode: repo.error === 'not-yours' ? 404 : 409, statusMessage: repo.error });
  try {
    const tags = await deps.source(repo.installationId).tags(repo.fullName);
    return { repository: repo.fullName, tags: tags.sort((a, b) => compareVersions(tagVersion(b.name), tagVersion(a.name))) };
  } catch (e) {
    console.error('[store] tags:', e instanceof Error ? e.message : e);
    throw createError({
      statusCode: e instanceof GithubError && e.status === 404 ? 404 : 502,
      statusMessage: 'GitHub did not list the tags',
    });
  }
});
