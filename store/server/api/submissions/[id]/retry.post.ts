import { getRouterParam, readBody, sendRedirect } from 'h3';
import { CHECK } from '../../../utils/queue';
import { requireAccount } from '../../../utils/session';
import { submissionsReady, submitPage, versionId } from '../../../utils/submit-route';
import { retrySubmission } from '../../../utils/submissions';

// Runs the checks again when the store (or GitHub) failed, not the plugin: a plugin's fault asks for a new tag.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const deps = submissionsReady();
  const id = versionId(getRouterParam(event, 'id'));
  const body = (await readBody(event).catch(() => ({}))) ?? {};
  if (await retrySubmission(db, account, id)) await deps.boss.send(CHECK, { id });
  return sendRedirect(event, `${submitPage(body.lang)}?version=${id}`, 303);
});
