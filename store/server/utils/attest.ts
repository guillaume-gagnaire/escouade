import type { FetchedBinary } from './checks';

// What GitHub's provenance attestations say of a binary of a release (`actions/attest-build-provenance`,
// `GET /repos/{o}/{r}/attestations/sha256:<digest>`): each is a Sigstore bundle whose DSSE payload is an in-toto
// statement of SLSA provenance, naming the file's digest, the repository and the commit it was built from, and its
// workflow. The store reads the statement to tie the binary to the commit it froze (docs/STORE.md §3.2, check 10);
// the signature of the bundle is checked by the review on the user's machine (`gh attestation verify`, §4.2), which
// alone gives the badge. A binary without an attestation is published unverified and cannot be verified (§4.4).

interface Statement {
  subject?: { digest?: { sha256?: string } }[];
  predicateType?: string;
  predicate?: {
    buildDefinition?: {
      externalParameters?: { workflow?: { repository?: string; path?: string } };
      resolvedDependencies?: { uri?: string; digest?: { gitCommit?: string } }[];
    };
  };
}

function statementOf(a: unknown): Statement | null {
  const env = (a as { bundle?: { dsseEnvelope?: { payload?: string; payloadType?: string } } })?.bundle?.dsseEnvelope;
  if (!env?.payload || env.payloadType !== 'application/vnd.in-toto+json') return null;
  try {
    return JSON.parse(Buffer.from(env.payload, 'base64').toString('utf8')) as Statement;
  } catch {
    return null;
  }
}

const repoOf = (url: string | undefined) =>
  (url ?? '')
    .replace(/^git\+/, '')
    .replace(/^https:\/\/github\.com\//i, '')
    .replace(/@.*$/, '')
    .replace(/\.git$/, '')
    .toLowerCase();

/** The verdict on a binary from the attestations GitHub lists for its digest. */
export function judgeAttestations(
  list: unknown[],
  want: { sha256: string; repository: string; commit: string },
): FetchedBinary['attestation'] {
  let seen: { commit?: string; repository?: string } | null = null;
  for (const a of list) {
    const s = statementOf(a);
    if (!s || s.predicateType !== 'https://slsa.dev/provenance/v1') continue;
    if (!s.subject?.some((x) => x.digest?.sha256?.toLowerCase() === want.sha256)) continue;
    const build = s.predicate?.buildDefinition;
    const repository = repoOf(build?.externalParameters?.workflow?.repository);
    const commit = build?.resolvedDependencies?.find((d) => d.digest?.gitCommit)?.digest?.gitCommit;
    if (repository === want.repository.toLowerCase() && commit === want.commit)
      return { state: 'attested', workflow: build?.externalParameters?.workflow?.path ?? '?' };
    seen = { commit, repository };
  }
  return seen ? { state: 'mismatch', ...seen } : { state: 'none' };
}
