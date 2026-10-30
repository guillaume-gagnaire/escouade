// The types of minisign.mjs: a key made for a test, and its signer.
export function testKey(): {
  pub: string;
  id: string;
  secret: string;
  sign(data: Buffer | string, trusted: string, options?: { legacy?: boolean }): string;
};
export function signWithSecret(secret: string, data: Buffer | string, trusted: string): string;
