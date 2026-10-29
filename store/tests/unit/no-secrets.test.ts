import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// No secret goes in the repository (docs/STORE.md, §1.3 and §4.5): they live in Coolify, and the signing key on the
// user's machine only. What git tracks under store/ (or would add: untracked files that are not ignored) is read.
const store = fileURLToPath(new URL('../..', import.meta.url));

function files(): string[] {
  try {
    return execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z'], { cwd: store, encoding: 'utf8' })
      .split('\0')
      .filter(Boolean);
  } catch {
    // The Docker build has no .git: the CI runs this test on the checkout.
    return [];
  }
}

const SECRET = [
  /-----BEGIN [A-Z ]*PRIVATE KEY-----/,
  // A minisign secret key file.
  /untrusted comment: minisign encrypted secret key/,
  /gh[pousr]_[A-Za-z0-9]{36}/,
  /github_pat_[A-Za-z0-9_]{40,}/,
  /AKIA[0-9A-Z]{16}/,
  // An address with a password in it, other than the local ones of the examples.
  /postgres(?:ql)?:\/\/[^:\s/]+:(?!store@127\.0\.0\.1|postgres@localhost)[^@\s]+@/,
];

describe('the repository', () => {
  const list = files();

  it('has no .env file, only its example', () => {
    expect(list.filter((f) => /(^|\/)\.env(\.|$)/.test(f) && !f.endsWith('.env.example'))).toEqual([]);
  });

  it('has no secret key in store/', () => {
    const found = list.flatMap((f) => {
      if (/\.(png|jpe?g|webp|woff2?|ico)$/.test(f) || f.endsWith('no-secrets.test.ts')) return [];
      let body: string;
      try {
        body = readFileSync(`${store}/${f}`, 'utf8');
      } catch {
        return [];
      }
      return SECRET.filter((re) => re.test(body)).map((re) => `${f}: ${re}`);
    });
    expect(found).toEqual([]);
  });

  it('keeps only public keys in store/keys/', () => {
    expect(list.filter((f) => f.startsWith('keys/') && !/\.(pub|md)$/.test(f))).toEqual([]);
  });
});
