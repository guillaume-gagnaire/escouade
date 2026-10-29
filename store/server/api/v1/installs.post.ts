import { createError, getHeader, readBody, setResponseHeader, setResponseStatus } from 'h3';
import { requireDb } from '../../utils/app-api';
import { countInstall, readInstall } from '../../utils/installs';
import { clientAddress, limiter } from '../../utils/limits';

// POST /api/v1/installs { id, version, event }: one installation or update, sent by the app only if its user
// accepted the statistics, without any id (docs/STORE.md §5.4). JSON only (a form of another site cannot send it);
// the same version from the same address counts once a day, and 60 an hour at most (server/utils/limits.ts).
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  if (!/^application\/json\b/.test(getHeader(event, 'content-type') ?? ''))
    throw createError({ statusCode: 415, statusMessage: 'JSON only' });
  const what = readInstall(await readBody(event).catch(() => null));
  if (!what) throw createError({ statusCode: 400, statusMessage: 'Expected { id, version, event: "install" | "update" }' });
  const db = requireDb();
  const address = clientAddress(getHeader(event, 'x-forwarded-for'), event.node.req.socket?.remoteAddress);
  // Said the same whether it counted or not: the answer tells nothing of who sent what before.
  if (limiter.firstToday(address, `${what.id}@${what.version}:${what.event}`) && !(await countInstall(db, what)))
    throw createError({ statusCode: 404, statusMessage: 'No such version' });
  setResponseStatus(event, 204);
  return null;
});
