import { getRouterParam, readBody, sendRedirect } from 'h3';
import { requireAccount, siteUrl } from '../../../utils/session';
import { submitPage, versionId } from '../../../utils/submit-route';
import { publishVersion } from '../../../utils/submissions';

// Publishes a version that passed its checks, from the last step's form: unverified, in the review queue if asked.
// A box left unchecked sends nothing: its absence is "off".
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const id = versionId(getRouterParam(event, 'id'));
  const body = (await readBody(event).catch(() => ({}))) ?? {};
  const done = await publishVersion(
    db,
    account,
    id,
    {
      categories: body.categories,
      keywords: body.keywords,
      review: body.review ?? 'off',
      autoPublish: body.autoPublish ?? 'off',
    },
    new Date(),
    { env: process.env, site: siteUrl() },
  );
  const notice = 'error' in done ? done.error : 'published';
  return sendRedirect(event, `${submitPage(body.lang)}?version=${id}&notice=${notice}`, 303);
});
