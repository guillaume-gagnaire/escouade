// The types of minisign.mjs: a key made for a test, and its signer.
export function testKey(): {
  pub: string;
  id: string;
  sign(data: Buffer | string, trusted: string, options?: { legacy?: boolean }): string;
};
