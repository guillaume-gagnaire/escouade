import { readBody, sendRedirect } from 'h3';
import { pluginPage } from '../utils/listing';
import { sendMail } from '../utils/mail';
import { fileReport, reportMail, type Reason } from '../utils/reports';
import { service } from '../utils/service';

// The report form of a plugin's page (« Signaler », docs/STORE.md §4.6), sent by anyone with an address: the
// report is kept for the review queue, its sender gets an acknowledgement, and the form says how it went.
export default defineEventHandler(async (event) => {
  const body = ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>;
  const lang = body.lang === 'en' ? 'en' : 'fr';
  const id = typeof body.plugin === 'string' ? body.plugin : '';
  const page = await pluginPage(service.db, service.archives, id);
  const base = `${lang === 'en' ? '/en' : ''}/plugins/${encodeURIComponent(id)}/report`;
  if (!page || !service.db) return sendRedirect(event, `${base}?notice=off`, 303);
  const version =
    typeof body.version === 'string' && page.versions.some((v) => v.version === body.version) ? body.version : page.entry.version;
  const filed = await fileReport(service.db, body, { pluginId: id, version, lang });
  if ('error' in filed) return sendRedirect(event, `${base}?notice=${filed.error}`, 303);
  await sendMail(
    process.env,
    reportMail(lang, filed.email, { id: filed.id, plugin: page.entry.name[lang], reason: body.reason as Reason }),
  );
  return sendRedirect(event, `${base}?notice=sent&n=${filed.id}`, 303);
});
