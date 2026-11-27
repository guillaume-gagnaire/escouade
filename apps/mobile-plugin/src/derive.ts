/**
 * The pairing's derivation in a browser (the e2e, `npm run dev`, ESC-570): what
 * `crates/escouade-mobile/src/tunnel/pairing.rs` derives on the phone, by WebCrypto. Apart from `web.ts`, so that it is
 * tested without Capacitor.
 */

export const b64 = (bytes: ArrayBuffer | Uint8Array): string => btoa(String.fromCharCode(...new Uint8Array(bytes)));

export const b64url = (bytes: ArrayBuffer | Uint8Array): string => b64(bytes).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');

export const unb64 = (text: string): Uint8Array<ArrayBuffer> => {
  const t = text.trim().replace(/-/g, '+').replace(/_/g, '/');
  const bin = atob(t + '='.repeat((4 - (t.length % 4)) % 4));
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
};

/** The device's token and the pairing key, from the secret of the pairing: HKDF-SHA256, no salt. */
export async function derive(shared: ArrayBuffer): Promise<{ token: string; key: string }> {
  const ikm = await crypto.subtle.importKey('raw', shared, 'HKDF', false, ['deriveBits']);
  const bits = (info: string) =>
    crypto.subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt: new Uint8Array(), info: new TextEncoder().encode(info) }, ikm, 256);
  return {
    token: b64url(await bits('escouade-mobile/device-token/v1')),
    key: b64(await bits('escouade-mobile/pairing-key/v1')),
  };
}
