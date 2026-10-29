// The types of plugins.mjs: the test plugins, as GitHub's archives.
type Entry = string | Buffer | { symlink: string } | { hardlink: string } | { fifo: true } | null;

export function tarball(files: Record<string, Entry>, options?: { top?: string; commit?: string; raw?: Record<string, Entry> }): Buffer;
export function png(width: number, height: number, options?: { noise?: boolean; text?: string }): Buffer;
export const MIT: string;
export function manifest(repo: string, over?: Record<string, unknown>): Record<string, unknown>;
export function pluginFiles(
  repo: string,
  options?: { manifest?: Record<string, unknown>; files?: Record<string, Entry> },
): Record<string, Entry>;
export function sha256(b: Buffer | string): string;
export function attestation(
  digest: string,
  options: { repo: string; commit: string; workflow?: string },
): { bundle: { dsseEnvelope: { payload: string; payloadType: string } } };
