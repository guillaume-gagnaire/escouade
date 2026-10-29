import { createError } from 'h3';
import { service } from './service';
import type { Deps } from './submissions';

/** What the submission's routes need: the App's reading of repositories and the queue; a 503 when either is off. */
export function submissionsReady(): Deps & { boss: NonNullable<typeof service.boss> } {
  if (!service.deps || !service.boss) throw createError({ statusCode: 503, statusMessage: 'Submissions are off on this store' });
  return { ...service.deps, boss: service.boss };
}

/** The id of a route's `[id]`, or a 404. */
export function versionId(raw: string | undefined): number {
  const id = Number(raw);
  if (!Number.isSafeInteger(id) || id <= 0 || id > 2 ** 31 - 1) throw createError({ statusCode: 404, statusMessage: 'No such version' });
  return id;
}

/** The submission's page in the form's language. */
export const submitPage = (lang: unknown) => (lang === 'en' ? '/en/submit' : '/submit');
