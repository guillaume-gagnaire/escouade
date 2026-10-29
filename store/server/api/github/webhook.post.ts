import { createError, getHeader, readRawBody, setResponseStatus } from 'h3';
import { applyWebhook } from '../../utils/accounts';
import { githubApp } from '../../utils/github';
import { service } from '../../utils/service';
import { webhookSigned } from '../../utils/tokens';

// The App's webhooks (installation, installation_repositories, repository, create, release): signed by GitHub
// (X-Hub-Signature-256 over the raw body, compared in constant time), handled once per delivery. A body that is not
// signed by the App's secret is refused before it is read.
export default defineEventHandler(async (event) => {
  const app = githubApp(process.env);
  if (!app || !service.db) throw createError({ statusCode: 503, statusMessage: 'Webhooks are off on this store' });
  const body = (await readRawBody(event, false)) ?? Buffer.alloc(0);
  if (!webhookSigned(app.webhookSecret, body, getHeader(event, 'x-hub-signature-256')))
    throw createError({ statusCode: 401, statusMessage: 'Bad signature' });
  const name = getHeader(event, 'x-github-event');
  const delivery = getHeader(event, 'x-github-delivery');
  if (!name || !delivery) throw createError({ statusCode: 400, statusMessage: 'Missing event or delivery' });
  let payload: unknown;
  try {
    payload = JSON.parse(body.toString('utf8'));
  } catch {
    throw createError({ statusCode: 400, statusMessage: 'Not JSON' });
  }
  if (name === 'ping') return { ok: true, handled: 'ignored' };
  const handled = await applyWebhook(service.db, name, delivery, payload as Parameters<typeof applyWebhook>[3]);
  setResponseStatus(event, 202);
  return { ok: true, handled };
});
