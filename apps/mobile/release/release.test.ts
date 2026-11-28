// @vitest-environment node
// The publication of the phone's app (MO-8): the secrets read by their names, the same in the workflow, the Fastfile,
// release.mjs and the guide; a workflow launched by hand alone, each of its jobs under the environment mobile-release;
// the listings within the stores' limits, in French and in English.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
// @ts-expect-error a module of plain JavaScript, run by node in the workflow
import { APP_ID, checkSecrets, ENVIRONMENT, PLAY_TRACK, PROFILE_NAME, SECRETS, versions } from './release.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const read = (p: string) => fs.readFileSync(path.join(ROOT, p), 'utf8');
const WORKFLOW = read('.github/workflows/mobile-release.yml');
const FASTFILE = read('apps/mobile/fastlane/Fastfile');
type Secret = { name: string; upload?: boolean; optional?: boolean };
const ALL: Secret[] = [...SECRETS.android, ...SECRETS.ios];
const NAMES = ALL.map((s) => s.name);

/** The guide's contract (ESC-576, the guide published for the user): these names, exactly. */
const CONTRACT = [
  'ANDROID_UPLOAD_KEYSTORE_BASE64',
  'ANDROID_UPLOAD_KEYSTORE_PASSWORD',
  'ANDROID_UPLOAD_KEY_ALIAS',
  'ANDROID_UPLOAD_KEY_PASSWORD',
  'GOOGLE_PLAY_SERVICE_ACCOUNT_JSON',
  'GOOGLE_SERVICES_JSON',
  'APPLE_TEAM_ID',
  'APP_STORE_CONNECT_API_KEY_ID',
  'APP_STORE_CONNECT_API_ISSUER_ID',
  'APP_STORE_CONNECT_API_KEY_P8',
  'IOS_DISTRIBUTION_CERT_P12_BASE64',
  'IOS_DISTRIBUTION_CERT_PASSWORD',
  'IOS_PROVISIONING_PROFILE_BASE64',
];

/** The jobs of the workflow, each with its own lines (read as text: the repository has no YAML parser at its root). */
function jobs(text: string): Map<string, string> {
  const body = text.slice(text.indexOf('\njobs:\n') + 7);
  const found = new Map<string, string>();
  for (const part of body.split(/^(?= {2}[a-z0-9_-]+:\s*$)/m)) {
    const name = /^ {2}([a-z0-9_-]+):/.exec(part)?.[1];
    if (name) found.set(name, part);
  }
  return found;
}

describe('the secrets, by their names', () => {
  it('are the guide’s, exactly, and nothing else', () => {
    expect([...NAMES].sort()).toEqual([...CONTRACT].sort());
    expect(ENVIRONMENT).toBe('mobile-release');
    expect(APP_ID).toBe('dev.escouade.mobile');
    expect(PLAY_TRACK).toBe('alpha');
    expect(PROFILE_NAME).toBe('Escouade App Store');
  });

  it('are each read in the workflow from the secrets of its environment, never from anywhere else', () => {
    for (const name of NAMES) expect(WORKFLOW, name).toContain(`secrets.${name}`);
    const read = [...WORKFLOW.matchAll(/secrets\.([A-Z0-9_]+)/g)].map((m) => m[1]);
    for (const name of read) expect(NAMES, `the workflow reads ${name}`).toContain(name);
    expect(WORKFLOW).not.toMatch(/GITHUB_TOKEN|vars\./);
  });

  it('are the Fastfile’s, its build and its upload apart', () => {
    for (const s of ALL.filter((x) => !x.optional)) expect(FASTFILE, s.name).toContain(s.name);
    expect(FASTFILE).toContain('GOOGLE_SERVICES_JSON');
  });

  it('missing, are each named; build only does not need the keys of the upload', () => {
    expect(checkSecrets('android', { env: {} }).missing).toEqual([
      'ANDROID_UPLOAD_KEYSTORE_BASE64',
      'ANDROID_UPLOAD_KEYSTORE_PASSWORD',
      'ANDROID_UPLOAD_KEY_ALIAS',
      'ANDROID_UPLOAD_KEY_PASSWORD',
      'GOOGLE_PLAY_SERVICE_ACCOUNT_JSON',
    ]);
    expect(checkSecrets('android', { env: {}, buildOnly: true }).missing).not.toContain('GOOGLE_PLAY_SERVICE_ACCOUNT_JSON');
    expect(checkSecrets('ios', { env: {}, buildOnly: true }).missing).toEqual([
      'APPLE_TEAM_ID',
      'IOS_DISTRIBUTION_CERT_P12_BASE64',
      'IOS_DISTRIBUTION_CERT_PASSWORD',
      'IOS_PROVISIONING_PROFILE_BASE64',
    ]);
    expect(checkSecrets('ios', { env: {} }).missing).toHaveLength(7);
    // An empty secret (GitHub gives '' for one that is not set) is a missing one.
    expect(checkSecrets('ios', { env: { APPLE_TEAM_ID: '  ' }, buildOnly: true }).missing).toContain('APPLE_TEAM_ID');
  });

  it('google-services.json may be missing: said, never a failure', () => {
    const env = Object.fromEntries(SECRETS.android.map((s: Secret) => [s.name, 'x']));
    delete env.GOOGLE_SERVICES_JSON;
    expect(checkSecrets('android', { env })).toEqual({ missing: [], optionalMissing: ['GOOGLE_SERVICES_JSON'] });
  });

  it('the check stops with each name, and never prints a value', () => {
    const run = (env: Record<string, string>, ...args: string[]) => {
      try {
        return {
          code: 0,
          out: execFileSync(process.execPath, ['apps/mobile/release/release.mjs', 'check', ...args], {
            cwd: ROOT,
            env: { PATH: process.env.PATH ?? '', ...env },
            encoding: 'utf8',
          }),
        };
      } catch (e) {
        const err = e as { status: number; stdout: string };
        return { code: err.status, out: err.stdout };
      }
    };
    const none = run({}, 'android');
    expect(none.code).toBe(1);
    expect(none.out).toContain('::error::Missing secret ANDROID_UPLOAD_KEYSTORE_BASE64 in the environment mobile-release');
    expect(none.out).toContain('::notice::GOOGLE_SERVICES_JSON is not set');
    const value = 'une-valeur-qui-ne-doit-pas-sortir';
    const all = run(Object.fromEntries(SECRETS.ios.map((s: Secret) => [s.name, value])), 'ios');
    expect(all.code).toBe(0);
    expect(all.out).not.toContain(value);
  });
});

describe('the versions given to the stores', () => {
  it('a build number that grows, three numbers for App Store Connect', () => {
    expect(versions('2.3.0-beta.2', '41')).toEqual({ name: '2.3.0-beta.2', marketing: '2.3.0', code: '41' });
    expect(() => versions('2.3', '1')).toThrow();
    expect(() => versions('2.3.0', '0')).toThrow();
  });
});

describe('the workflow Mobile release', () => {
  it('is launched by hand alone, with the platform and « build only »', () => {
    const on = WORKFLOW.slice(WORKFLOW.indexOf('\non:\n'), WORKFLOW.indexOf('\npermissions:'));
    expect(on).toMatch(/^ {2}workflow_dispatch:/m);
    expect(on).not.toMatch(/^ {2}(push|pull_request|pull_request_target|schedule|workflow_run|release):/m);
    expect(on).toMatch(/platform:[\s\S]*options: \[android, ios, both\]/);
    expect(on).toMatch(/build_only:[\s\S]*type: boolean[\s\S]*default: true/);
    expect(WORKFLOW).toMatch(/^permissions:\n {2}contents: read$/m);
  });

  it('runs each job under the environment mobile-release, in the origin repository alone', () => {
    const all = jobs(WORKFLOW);
    expect([...all.keys()]).toEqual(['android', 'ios']);
    for (const [name, job] of all) {
      expect(job, name).toMatch(/^ {4}environment: mobile-release$/m);
      expect(job, name).toContain("github.repository == 'guillaume-gagnaire/escouade'");
      expect(job, name).toContain('persist-credentials: false');
      // The secrets checked before anything is built.
      expect(job.indexOf('release.mjs check'), name).toBeLessThan(job.indexOf('npm ci'));
      // The files of the secrets removed, even in failure.
      expect(job, name).toMatch(/if: always\(\)\n\s+run: \|[\s\S]*rm -rf "\$RUNNER_TEMP\/escouade-secrets"/);
    }
    expect(all.get('ios')).toMatch(/security delete-keychain escouade-release\.keychain-db/);
  });

  it('sends to the closed test and to TestFlight alone: no review, no production', () => {
    expect(FASTFILE).toContain('track: PLAY_TRACK');
    expect(FASTFILE).toContain('skip_waiting_for_build_processing: true');
    expect(FASTFILE).toContain('skip_submission: true');
    expect(FASTFILE).toContain('distribute_external: false');
    expect(FASTFILE).not.toMatch(/upload_to_app_store|deliver\(|submit_for_review|track: "production"|track_promote_to/);
    // The pins of the third parties' actions, by their commit.
    for (const m of WORKFLOW.matchAll(/uses: ([^\s]+)/g)) expect(m[1], m[1]).toMatch(/@[0-9a-f]{40}$/);
  });
});

describe('the listings', () => {
  const LIMITS: Record<string, Record<string, number>> = {
    android: { 'title.txt': 30, 'short_description.txt': 80, 'full_description.txt': 4000, 'release_notes.txt': 500 },
    ios: {
      'name.txt': 30,
      'subtitle.txt': 30,
      'keywords.txt': 100,
      'promotional_text.txt': 170,
      'description.txt': 4000,
      'release_notes.txt': 4000,
      'privacy_url.txt': 255,
      'support_url.txt': 255,
      'marketing_url.txt': 255,
    },
  };
  const text = (store: string, locale: string, file: string) =>
    read(`apps/mobile/store/listing/${store}/${locale}/${file}`).replace(/\n$/, '');

  for (const store of ['android', 'ios'])
    for (const locale of ['fr-FR', 'en-US'])
      it(`${store} ${locale}: every text, within the store’s limits, without a seller’s name`, () => {
        for (const [file, max] of Object.entries(LIMITS[store])) {
          const t = text(store, locale, file);
          expect(t.length, file).toBeGreaterThan(0);
          expect([...t].length, `${file}: ${[...t].length} > ${max}`).toBeLessThanOrEqual(max);
          expect(t, file).not.toMatch(/GAGNAIRE|SARL|SAS\b|Guillaume/i);
        }
      });

  it('the addresses are the site’s pages, in the listing’s language', () => {
    expect(text('ios', 'fr-FR', 'privacy_url.txt')).toBe('https://escouade.dev/mobile/privacy/');
    expect(text('ios', 'en-US', 'privacy_url.txt')).toBe('https://escouade.dev/en/mobile/privacy/');
    expect(text('ios', 'fr-FR', 'support_url.txt')).toBe('https://escouade.dev/docs/mobile-app/');
    expect(text('ios', 'en-US', 'support_url.txt')).toBe('https://escouade.dev/en/docs/mobile-app/');
  });
});
