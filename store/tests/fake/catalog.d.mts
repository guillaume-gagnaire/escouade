// The types of catalog.mjs: published plugins seeded for the public pages' tests.
export const TRAPPED: string;
export function seedCatalog(options?: {
  url?: string;
  archives?: string;
}): Promise<{ run: string; login: string; ids: { slack: string; ntfy: string; nord: string } }>;
