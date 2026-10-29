import { readBody, sendRedirect } from 'h3';
import { CHECK } from '../utils/queue';
import { requireAccount } from '../utils/session';
import { submissionsReady, submitPage } from '../utils/submit-route';
import { startSubmission } from '../utils/submissions';

// Submits a tag of a repository, from the form of the second step: the tag resolved into its commit, the version
// written down, its checks sent to the queue; the page then shows them as they run.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const deps = submissionsReady();
  const body = (await readBody(event).catch(() => ({}))) ?? {};
  const page = submitPage(body.lang);
  const started = await startSubmission(db, deps, account, { repo: body.repo, tag: body.tag });
  if ('error' in started) {
    const q = new URLSearchParams({ notice: started.error, ...(typeof body.repo === 'string' ? { repo: body.repo } : {}) });
    return sendRedirect(event, `${page}?${q}`, 303);
  }
  if (!started.again) await deps.boss.send(CHECK, { id: started.id });
  return sendRedirect(event, `${page}?version=${started.id}`, 303);
});
