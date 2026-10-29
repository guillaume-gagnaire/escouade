// The types of review-store.mjs: the fake store and GitHub of the local review.
type Entry = string | Buffer | { symlink: string } | { hardlink: string } | { fifo: true } | null;

export function exampleFiles(name: string): Record<string, Entry>;
export const TRAPPED: Record<string, Entry>;
export function startReviewStore(port?: number): Promise<{ url: string; githubApi: string; close: () => Promise<void> }>;
