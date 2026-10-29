import { randomBytes } from 'node:crypto';
import { setResponseHeader } from 'h3';
import { nonceChunks, pagePolicy } from '../utils/headers';

// Each page rendered gets a fresh nonce, set on Nuxt's inline scripts and styles (never on the page's body, where a
// README is) and named in its policy: no other inline script runs. In development (Vite's own scripts) the policy is
// left to the static one of the API.
export default defineNitroPlugin((nitro) => {
  if (import.meta.dev) return;
  nitro.hooks.hook('render:html', (html, { event }) => {
    const nonce = randomBytes(16).toString('base64');
    nonceChunks(html, nonce);
    setResponseHeader(event, 'content-security-policy', pagePolicy(nonce));
  });
});
