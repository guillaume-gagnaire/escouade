import { createError, getRouterParam, readBody, sendRedirect } from 'h3';
import { dismissVersion, yankVersion } from '../../../../utils/author';
import { requireAccount, siteUrl } from '../../../../utils/session';
import { versionId } from '../../../../utils/submit-route';
import { publishVersion, type Version } from '../../../../utils/submissions';
import { versions } from '../../../../db/schema';
import { eq } from 'drizzle-orm';

// What an author does of a version from their account: publish one found on a new tag that waits for their click
// (the plugin's categories stay as they are), set it aside, or take a published one back (« yank »).
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const id = versionId(getRouterParam(event, 'id'));
  const action = getRouterParam(event, 'action') ?? '';
  const body = (await readBody(event).catch(() => ({}))) ?? {};
  const how = { env: process.env, site: siteUrl() };
  const [v] = (await db.select({ pluginId: versions.pluginId }).from(versions).where(eq(versions.id, id))) as Pick<Version, 'pluginId'>[];
  let done;
  if (action === 'publish') done = await publishVersion(db, account, id, {}, new Date(), how);
  else if (action === 'dismiss') done = await dismissVersion(db, account, id);
  else if (action === 'yank') done = await yankVersion(db, account, id, how);
  else throw createError({ statusCode: 404, statusMessage: 'No such action' });
  const notice = 'error' in done ? `version-${done.error}` : `version-${action}`;
  const page = body.lang === 'en' ? '/en/account' : '/account';
  const plugin = v?.pluginId ? `plugin=${encodeURIComponent(v.pluginId)}&` : '';
  return sendRedirect(event, `${page}?${plugin}notice=${notice}`, 303);
});
