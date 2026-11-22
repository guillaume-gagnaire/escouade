// The types of minisign.mjs: a key made for a test, and its signer.
export function testKey(): {
  pub: string;
  id: string;
  secret: string;
  /** The secret in a file shaped as minisign's, locked by a password. */
  locked: string;
  /** The secret in a file shaped as minisign's, without a password. */
  open: string;
  sign(data: Buffer | string, trusted: string, options?: { legacy?: boolean }): string;
};
export function signWithSecret(secret: string, data: Buffer | string, trusted: string): string;
