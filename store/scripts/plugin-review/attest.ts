import { spawnSync } from 'node:child_process';

// The provenance of a binary, checked on the user's machine (docs/STORE.md §3.3, §4.4): `gh attestation verify` checks the
// Sigstore signature of the attestations GitHub holds for the file, which the store only read. It must tie the file to
// the repository and to the commit the store froze: what is read must be what runs. Without `gh`, or without its
// login, it is not checked, and the report says so.

export type Attested =
  | { state: 'verified'; workflow: string }
  | { state: 'other-commit'; commit: string }
  | { state: 'refused'; why: string }
  | { state: 'unchecked'; why: string };

interface Verified {
  verificationResult?: {
    statement?: {
      predicate?: {
        buildDefinition?: {
          externalParameters?: { workflow?: { path?: string } };
          resolvedDependencies?: { digest?: { gitCommit?: string } }[];
        };
      };
    };
  };
}

export type Runner = (args: string[]) => { status: number | null; stdout: string; stderr: string; error?: Error };

const gh: Runner = (args) => {
  const r = spawnSync('gh', args, { encoding: 'utf8', timeout: 120_000 });
  return { status: r.status, stdout: r.stdout ?? '', stderr: r.stderr ?? '', error: r.error };
};

/** Checks the attestations of the binary at `path` against `repo` (`owner/name`) and `commit`. */
export function verifyAttestation(path: string, repo: string, commit: string, run: Runner = gh): Attested {
  const r = run(['attestation', 'verify', path, '--repo', repo, '--format', 'json']);
  if (r.error) return { state: 'unchecked', why: '`gh` introuvable' };
  if (r.status !== 0) {
    const why = (r.stderr || r.stdout).trim().split('\n').slice(-3).join(' ');
    if (/auth login|not logged|GH_TOKEN|authentication/i.test(why)) return { state: 'unchecked', why: `\`gh\` sans connexion : ${why}` };
    return { state: 'refused', why: why || `code ${r.status}` };
  }
  let list: Verified[];
  try {
    list = JSON.parse(r.stdout) as Verified[];
  } catch {
    return { state: 'refused', why: 'sortie de `gh attestation verify` illisible' };
  }
  let other: string | null = null;
  for (const v of list) {
    const build = v.verificationResult?.statement?.predicate?.buildDefinition;
    const built = build?.resolvedDependencies?.find((d) => d.digest?.gitCommit)?.digest?.gitCommit;
    if (built === commit) return { state: 'verified', workflow: build?.externalParameters?.workflow?.path ?? '?' };
    if (built) other = built;
  }
  return other ? { state: 'other-commit', commit: other } : { state: 'refused', why: 'aucune attestation ne nomme un commit' };
}
