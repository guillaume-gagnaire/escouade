// The security headers of every answer of the store (asked by the chief of the squad on 2026-10-29, once the store
// was online without any): HSTS, nosniff, the referrer cut to the origin, the browser's features closed, no frame.
// The pages get a strict Content-Security-Policy whose nonce marks Nuxt's own scripts and styles; the API's answers
// a closed one.

export const STATIC_HEADERS: Record<string, string> = {
  'strict-transport-security': 'max-age=31536000; includeSubDomains',
  'x-content-type-options': 'nosniff',
  'referrer-policy': 'strict-origin-when-cross-origin',
  'permissions-policy':
    'accelerometer=(), autoplay=(), camera=(), display-capture=(), encrypted-media=(), fullscreen=(), geolocation=(), gyroscope=(), hid=(), idle-detection=(), magnetometer=(), microphone=(), midi=(), payment=(), picture-in-picture=(), publickey-credentials-get=(), screen-wake-lock=(), serial=(), usb=(), xr-spatial-tracking=(), browsing-topics=()',
  'x-frame-options': 'DENY',
  'cross-origin-opener-policy': 'same-origin',
  'cross-origin-resource-policy': 'same-origin',
  // What answers that are not pages get: nothing runs, nothing frames them.
  'content-security-policy': "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
};

/** A page's policy: its own files, and the inline scripts and styles of this answer only (its nonce). */
export function pagePolicy(nonce: string): string {
  return [
    "default-src 'self'",
    `script-src 'self' 'nonce-${nonce}'`,
    `style-src 'self' 'nonce-${nonce}'`,
    "img-src 'self'",
    "font-src 'self'",
    "connect-src 'self'",
    "object-src 'none'",
    "base-uri 'self'",
    // The sign-in leaves for GitHub by a link, not a form; GitHub is allowed all the same, should a form lead there.
    "form-action 'self' https://github.com",
    "frame-ancestors 'none'",
  ].join('; ');
}

/** Puts `nonce` on every <script> and <style> of a page's HTML chunk (Nuxt's payload, its config, its styles). */
export const withNonce = (html: string, nonce: string) => html.replace(/<(script|style)(?=[\s>])/g, `<$1 nonce="${nonce}"`);
