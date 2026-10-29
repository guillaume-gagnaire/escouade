import { createError, getRouterParam } from 'h3';
import { pluginPage } from '../../../utils/listing';
import { service } from '../../../utils/service';

// A plugin's page (docs/STORE.md §6): its last version, its README and docs, its permissions in words, its versions,
// where it comes from. A plugin with no published version has no page.
export default defineEventHandler(async (event) => {
  const page = await pluginPage(service.db, service.archives, getRouterParam(event, 'id') ?? '');
  if (!page) throw createError({ statusCode: 404, statusMessage: 'No such plugin' });
  return page;
});
