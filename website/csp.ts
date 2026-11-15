// The content security policy of the site's pages (SEC-16 of the 2.2.0 audit), written in each page at its generation
// (`nuxt.config.ts`, Nitro's `prerender:generate`). The site is static: its scripts are its own (`/_nuxt/`), and the
// few Nuxt writes in a page (its import map, its configuration, the redirection to the visitor's language) are allowed
// by their SHA-256 alone, page by page; any other inline script is refused. Styles stay allowed inline: Vue writes a
// `<style>` per component and `style` attributes, which no hash can name. Nothing is loaded from elsewhere: no
// measure of the audience, no font, no image nor video of another host.
//
// What a `<meta>` cannot carry (`frame-ancestors`) is in the headers of `nginx.conf`, with the rest of them.
import { createHash } from 'node:crypto';

/** The inline scripts of `html` that a browser runs: neither a `src` nor data (`application/json`). */
export function inlineScripts(html: string): string[] {
  return [...html.matchAll(/<script(\s[^>]*)?>([\s\S]*?)<\/script>/g)]
    .filter((m) => !/\ssrc=/.test(m[1] ?? '') && !/\stype="application\/(ld\+)?json"/.test(m[1] ?? ''))
    .map((m) => m[2]!);
}

/** A script's source as `script-src` names it. */
export const hashOf = (script: string) => `'sha256-${createHash('sha256').update(script, 'utf8').digest('base64')}'`;

/** The policy of a page whose inline scripts are `scripts`. */
export function policyOf(scripts: string[]): string {
  const hashes = [...new Set(scripts.map(hashOf))];
  return [
    "default-src 'self'",
    `script-src 'self' ${hashes.join(' ')}`.trim(),
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data:",
    "font-src 'self' data:",
    "media-src 'self'",
    "connect-src 'self'",
    "object-src 'none'",
    "base-uri 'self'",
    "form-action 'self'",
  ].join('; ');
}

/** `html` with its policy, first in its `<head>`: before any script it names. */
export function withPolicy(html: string): string {
  const meta = `<meta http-equiv="Content-Security-Policy" content="${policyOf(inlineScripts(html))}">`;
  const charset = html.match(/<meta charset="[^"]*">/);
  if (charset) return html.replace(charset[0], `${charset[0]}${meta}`);
  return html.replace(/<head(\s[^>]*)?>/, (head) => `${head}${meta}`);
}
