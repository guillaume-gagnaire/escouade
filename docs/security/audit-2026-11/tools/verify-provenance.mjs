// Checks the build provenance attestation of the binaries of a published release (GitHub, Sigstore) with the GitHub CLI,
// and says what each attests: which workflow built it, from which commit and tag, on which runner.
//
//   node verify-provenance.mjs <folder of the release's files> <owner/repository> [<commit the tag names>]
//
// A binary is every file of the folder but the updater's manifest (.json) and signatures (.sig). With a commit, the
// attested commit must be that one. Needs `gh` signed in (reading is enough). The repository's name and the folders of
// the machine are left out of what is printed: the report this goes to names neither. Exits 1 when a check fails.
import { spawnSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const [folder, repo, commit] = process.argv.slice(2);
if (!folder || !repo) {
  console.error('usage: node verify-provenance.mjs <folder> <owner/repository> [<commit>]');
  process.exit(2);
}
const anonymous = (text) => String(text).split(repo).join('<dépôt>');
const files = fs
  .readdirSync(folder)
  .filter((f) => !f.endsWith('.json') && !f.endsWith('.sig'))
  .sort();
let all = true;
for (const name of files) {
  const file = path.join(folder, name);
  const sha256 = crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
  console.log(`\nfile: ${name} (sha256 ${sha256})`);
  const r = spawnSync('gh', ['attestation', 'verify', file, '--repo', repo, '--format', 'json'], { encoding: 'utf8' });
  if (r.status !== 0) {
    // What `gh` said, without the address it asked nor the path of the file.
    const said = `${r.stdout}\n${r.stderr}`
      .split('\n')
      .map((l) => l.trim())
      .filter((l) => /error|not found|failed/i.test(l))
      .map((l) =>
        anonymous(
          l
            .replace(/\(https?:[^)]*\)/g, '')
            .replace(/file:\/\/\S+/g, '')
            .trim(),
        ),
      )
      .slice(0, 2);
    console.log(`FAIL no attestation verified: ${said.join(' ; ') || `gh exited ${r.status}`}`);
    all = false;
    continue;
  }
  const attestations = JSON.parse(r.stdout);
  for (const a of attestations) {
    const v = a.verificationResult;
    const c = v.signature.certificate;
    const subject = v.statement.subject.find((s) => s.name === name);
    const checks = {
      'the attestation verifies (signature, transparency log)': true,
      'it names this file, by this SHA-256': subject?.digest?.sha256 === sha256,
      'it is a SLSA build provenance': v.statement.predicateType === 'https://slsa.dev/provenance/v1',
      'the builder is a workflow of the repository': String(c.buildSignerURI).startsWith(`https://github.com/${repo}/.github/workflows/`),
      'built on a runner of GitHub': c.runnerEnvironment === 'github-hosted',
      ...(commit ? { 'built from the commit the tag names': c.sourceRepositoryDigest === commit } : {}),
    };
    console.log(`workflow: ${anonymous(c.buildSignerURI).replace('https://github.com/<dépôt>/', '')}`);
    console.log(`source: ${c.sourceRepositoryRef}, commit ${c.sourceRepositoryDigest}`);
    console.log(`trigger: ${c.githubWorkflowTrigger} ; run: ${anonymous(c.runInvocationURI).replace('https://github.com/<dépôt>/', '')}`);
    console.log(`signed through: ${c.issuer} ; certificate of ${c.certificateIssuer}`);
    console.log(`logged: ${(v.verifiedTimestamps ?? []).map((t) => `${t.type} ${new Date(t.timestamp).toISOString()}`).join(', ')}`);
    for (const [what, ok] of Object.entries(checks)) console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`);
    all = Object.values(checks).every(Boolean) && all;
  }
}
console.log(`\n${files.length} files: ${all ? 'every attestation verified' : 'SOME FAILED'}`);
process.exit(all ? 0 : 1);
