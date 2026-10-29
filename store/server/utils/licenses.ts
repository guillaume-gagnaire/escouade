// The free licenses a plugin of the store may take (docs/STORE.md §2.4, §3.2): approved by the OSI or held free by
// the FSF, by their SPDX identifier. The manifest's `license` is compared to what GitHub detects in the commit's
// LICENSE or COPYING file (`GET /repos/{o}/{r}/license?ref=<sha>`), which names the GPL family without `-only` or
// `-or-later` (the file's text is the same): both sides are compared without those suffixes.

const FREE = [
  '0BSD',
  'AFL-3.0',
  'AGPL-3.0',
  'Apache-2.0',
  'APSL-2.0',
  'Artistic-2.0',
  'BlueOak-1.0.0',
  'BSD-2-Clause',
  'BSD-2-Clause-Patent',
  'BSD-3-Clause',
  'BSD-3-Clause-Clear',
  'BSL-1.0',
  'CC0-1.0',
  'CC-BY-4.0',
  'CC-BY-SA-4.0',
  'CDDL-1.0',
  'CECILL-2.1',
  'ECL-2.0',
  'EPL-1.0',
  'EPL-2.0',
  'EUPL-1.1',
  'EUPL-1.2',
  'GPL-2.0',
  'GPL-3.0',
  'ISC',
  'LGPL-2.1',
  'LGPL-3.0',
  'LPPL-1.3c',
  'MIT',
  'MIT-0',
  'MPL-1.1',
  'MPL-2.0',
  'MS-PL',
  'MS-RL',
  'MulanPSL-2.0',
  'NCSA',
  'OFL-1.1',
  'OSL-3.0',
  'PostgreSQL',
  'Python-2.0',
  'Unicode-DFS-2016',
  'Unlicense',
  'UPL-1.0',
  'Vim',
  'WTFPL',
  'Zlib',
  'ZPL-2.1',
];

const FREE_SET = new Set(FREE.map((l) => l.toLowerCase()));

/** An identifier without the GPL family's `-only`, `-or-later` or `+`, in lower case, for comparing. */
export const spdxKey = (id: string) =>
  id
    .trim()
    .replace(/(-only|-or-later|\+)$/i, '')
    .toLowerCase();

/** The identifiers of an SPDX expression (`MIT OR Apache-2.0`, `(MIT)`), or null if it is no plain one (`WITH`, `AND`). */
export function spdxParts(expression: string): string[] | null {
  const s = expression.trim().replace(/^\((.*)\)$/, '$1');
  if (!s || /\b(AND|WITH)\b/.test(s)) return null;
  const parts = s.split(/\s+OR\s+/).map((p) => p.trim());
  return parts.every((p) => /^[A-Za-z0-9.+-]+$/.test(p)) ? parts : null;
}

/** Whether an identifier names a free license. */
export const isFree = (id: string) => FREE_SET.has(spdxKey(id));

export type LicenseVerdict =
  | { ok: true }
  | {
      ok: false;
      code: 'missing' | 'expression' | 'not-free' | 'no-file' | 'undetected' | 'different';
      manifest?: string;
      detected?: string;
    };

/**
 * The manifest's `license` against what GitHub detects in the commit (`detected`: its SPDX id, `NOASSERTION` for a
 * file it does not recognize, null when there is no license file). `hasFile`: a LICENSE or COPYING in the archive.
 */
export function judgeLicense(manifest: unknown, detected: string | null, hasFile: boolean): LicenseVerdict {
  if (typeof manifest !== 'string' || !manifest.trim()) return { ok: false, code: 'missing' };
  const parts = spdxParts(manifest);
  if (!parts) return { ok: false, code: 'expression', manifest };
  const unfree = parts.find((p) => !isFree(p));
  if (unfree) return { ok: false, code: 'not-free', manifest: unfree };
  if (!hasFile || detected === null) return { ok: false, code: 'no-file', manifest };
  if (detected === 'NOASSERTION' || detected === 'other') return { ok: false, code: 'undetected', manifest };
  if (!parts.some((p) => spdxKey(p) === spdxKey(detected))) return { ok: false, code: 'different', manifest, detected };
  return { ok: true };
}
