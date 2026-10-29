import { createHash } from 'node:crypto';

// A version of a plugin on the store (docs/STORE.md, §3.4): one tag, one commit, one archive kept as it came, its
// binaries; and `version.json`, the small canonical file that names them all with their SHA-256. Its own SHA-256 is
// the version's fingerprint: what the app checks and what the review signs (§4.5), so that a signature covers the
// identity, the number, the code and the binaries together.

export const sha256 = (b: Buffer | string) => createHash('sha256').update(b).digest('hex');

/** The version a tag names: `v1.2.0` and `1.2.0` both name 1.2.0. */
export const tagVersion = (tag: string) => tag.replace(/^v(?=\d)/, '');

const SEMVER = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/;

/** Compares two versions as semver does (a pre-release comes before its release; the build is not compared). */
export function compareVersions(a: string, b: string): number {
  const x = SEMVER.exec(a);
  const y = SEMVER.exec(b);
  if (!x || !y) return a.localeCompare(b);
  for (let i = 1; i <= 3; i++) {
    const d = Number(x[i]) - Number(y[i]);
    if (d) return Math.sign(d);
  }
  if (!x[4] || !y[4]) return x[4] ? -1 : y[4] ? 1 : 0;
  const p = x[4].split('.');
  const q = y[4].split('.');
  for (let i = 0; i < Math.max(p.length, q.length); i++) {
    if (p[i] === undefined) return -1;
    if (q[i] === undefined) return 1;
    const [m, n] = [p[i]!, q[i]!];
    const [mn, nn] = [/^\d+$/.test(m), /^\d+$/.test(n)];
    if (mn && nn && Number(m) !== Number(n)) return Math.sign(Number(m) - Number(n));
    if (mn !== nn) return mn ? -1 : 1;
    if (m !== n) return m < n ? -1 : 1;
  }
  return 0;
}

/** JSON with its keys sorted at every level and no spaces: the same value always gives the same bytes. */
export function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value !== null && typeof value === 'object')
    return `{${Object.keys(value)
      .filter((k) => (value as Record<string, unknown>)[k] !== undefined)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canonical((value as Record<string, unknown>)[k])}`)
      .join(',')}}`;
  return JSON.stringify(value);
}

/** A system and an architecture, the key of a binary in `version.json`: `windows-x64`, `macos-arm64`… */
export type Platform = `${'windows' | 'macos' | 'linux'}-${'x64' | 'arm64'}`;

export interface Binary {
  /** The file's name in the release. */
  name: string;
  sha256: string;
  size: number;
  /** Whether GitHub holds a provenance attestation that ties it to this commit and this repository. */
  attested: boolean;
}

export interface VersionFile {
  id: string;
  version: string;
  repository: string;
  tag: string;
  commit: string;
  archive: { sha256: string; size: number };
  binaries: Partial<Record<Platform, Binary>>;
  manifest_sha256: string;
  /** What the plugin speaks and where it was tried (docs/STORE.md §8.1): the app picks the last compatible version. */
  escouade: { api: string; min: string; max?: string };
}

/** `version.json` as the store serves it, and its SHA-256: the version's fingerprint. */
export function versionFile(v: VersionFile): { json: string; sha256: string } {
  const json = canonical({ format: 1, ...v });
  return { json, sha256: sha256(json) };
}
