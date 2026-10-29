import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { join } from 'node:path';
import { and, eq } from 'drizzle-orm';
import { createError, getHeader, sendStream, setResponseHeader, setResponseStatus, type H3Event } from 'h3';
import { versions } from '../db/schema';
import type { Db } from './db';
import { parseRevocations, revoked } from './revocation-list';
import { lastRevocations } from './revocations';
import { service } from './service';

// What the routes of the app's API share (server/api/v1/, docs/STORE.md §1.2 and §5.4): simple GETs without cookie
// nor id; what never changes (a version's files) cached for a year, what changes (the catalogue, the revocation
// list) revalidated by its ETag.

/** A year, immutable: a published version's bytes never change (§3.4). */
export const FOREVER = 'public, max-age=31536000, immutable';
/** Kept, but asked again each time: a 304 when nothing changed. */
export const REVALIDATE = 'public, no-cache';

export function requireDb(): Db {
  if (!service.db || service.starting) throw createError({ statusCode: 503, statusMessage: 'The store is starting' });
  return service.db;
}

/** Sets the ETag and says whether the client already has these bytes (then answers 304). */
export function notModified(event: H3Event, etag: string): boolean {
  setResponseHeader(event, 'etag', etag);
  const asked = (getHeader(event, 'if-none-match') ?? '').split(',').map((s) => s.trim().replace(/^W\//, ''));
  if (!asked.includes(etag) && !asked.includes('*')) return false;
  setResponseStatus(event, 304);
  return true;
}

/** A published version, by its plugin's id and its number: a 404 otherwise. */
export async function publishedVersion(db: Db, id: string | undefined, version: string | undefined) {
  if (!id || !version) throw createError({ statusCode: 404, statusMessage: 'No such version' });
  const [v] = await db
    .select()
    .from(versions)
    .where(and(eq(versions.pluginId, id), eq(versions.version, version), eq(versions.state, 'published')))
    .limit(1);
  if (!v || !v.versionJson) throw createError({ statusCode: 404, statusMessage: 'No such version' });
  return v;
}

/** A 410 if the revocation list disables this version: its files are no longer handed out (§4.6). */
export async function refuseDisabled(db: Db, id: string, version: string): Promise<void> {
  const stored = await lastRevocations(db);
  const list = stored ? parseRevocations(stored.json) : null;
  const entry = list && !('error' in list) ? revoked(list, id, version, 'disable') : null;
  if (entry) throw createError({ statusCode: 410, statusMessage: 'Disabled by Escouade', data: { reason: entry.reason } });
}

/** Sends a file kept under its SHA-256 (`sha256/ab/<sha256><ext>`), cached for a year, its ETag its SHA-256. */
export async function sendKept(event: H3Event, sha256: string, ext: string, type: string, filename: string) {
  if (!/^[0-9a-f]{64}$/.test(sha256)) throw createError({ statusCode: 404, statusMessage: 'No such file' });
  const path = join(service.archives, 'sha256', sha256.slice(0, 2), `${sha256}${ext}`);
  const size = await stat(path).then(
    (s) => s.size,
    () => null,
  );
  if (size === null) {
    console.error(`[store] a published file is missing from the volume: ${sha256}${ext}`);
    throw createError({ statusCode: 404, statusMessage: 'No such file' });
  }
  setResponseHeader(event, 'cache-control', FOREVER);
  setResponseHeader(event, 'content-type', type);
  setResponseHeader(event, 'content-disposition', `attachment; filename="${filename.replace(/[^\w.+-]/g, '_')}"`);
  setResponseHeader(event, 'x-content-sha256', sha256);
  if (notModified(event, `"${sha256}"`)) return null;
  setResponseHeader(event, 'content-length', size);
  // HEAD: the headers alone, the size included (a 200, where returning nothing would make a 204).
  if (event.method === 'HEAD') return void event.node.res.end();
  return sendStream(event, createReadStream(path));
}

/** A download answers GET and HEAD (a client that asks the size first), nothing else. */
export function getOrHead(event: H3Event): void {
  if (event.method !== 'GET' && event.method !== 'HEAD') {
    setResponseHeader(event, 'allow', 'GET, HEAD');
    throw createError({ statusCode: 405, statusMessage: 'GET or HEAD' });
  }
}
