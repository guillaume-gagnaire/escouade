import { createError, getRouterParam } from 'h3';
import { authorPage } from '../../../utils/listing';
import { service } from '../../../utils/service';

// An author's page: their GitHub login and their published plugins; `escouade` lists Escouade's own.
export default defineEventHandler(async (event) => {
  const page = await authorPage(service.db, getRouterParam(event, 'login') ?? '');
  if (!page) throw createError({ statusCode: 404, statusMessage: 'No such author' });
  return page;
});
