import { randomBytes } from 'node:crypto';
import { setResponseHeader } from 'h3';
import { pagePolicy, withNonce } from '../utils/headers';

// Each page rendered gets a fresh nonce, set on Nuxt's inline scripts and styles and named in its policy: no other
// inline script runs. In development (Vite's own scripts) the policy is left to the static one of the API.
export default defineNitroPlugin((nitro) => {
  if (import.meta.dev) return;
  nitro.hooks.hook('render:html', (html, { event }) => {
    const nonce = randomBytes(16).toString('base64');
    for (const part of ['head', 'bodyPrepend', 'body', 'bodyAppend'] as const) html[part] = html[part].map((c) => withNonce(c, nonce));
    setResponseHeader(event, 'content-security-policy', pagePolicy(nonce));
  });
});
