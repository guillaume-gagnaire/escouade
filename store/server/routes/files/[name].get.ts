import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { createError, getRouterParam, setResponseHeaders } from 'h3';
import { service } from '../../utils/service';

// The images the store copied (the screenshots and a README's images, re-encoded at the check): served from the
// store, never from a third party, under their SHA-256, which never changes (docs/STORE.md §7). Images only: the
// archives and the binaries are STORE-6's.
const TYPES: Record<string, string> = { png: 'image/png', jpg: 'image/jpeg', webp: 'image/webp' };

export default defineEventHandler(async (event) => {
  const name = getRouterParam(event, 'name') ?? '';
  const m = /^([a-f0-9]{64})\.(png|jpg|webp)$/.exec(name);
  if (!m) throw createError({ statusCode: 404, statusMessage: 'No such file' });
  const bytes = await readFile(join(service.archives, 'sha256', m[1]!.slice(0, 2), name)).catch(() => null);
  if (!bytes) throw createError({ statusCode: 404, statusMessage: 'No such file' });
  setResponseHeaders(event, {
    'content-type': TYPES[m[2]!]!,
    'cache-control': 'public, max-age=31536000, immutable',
    'content-security-policy': "default-src 'none'; sandbox",
  });
  return bytes;
});
