#!/usr/bin/env node
// The publication of the phone's app (MO-8, docs/MOBILE.md § 8, docs/MOBILE-PUBLISH.md): what the workflow
// `Mobile release` (.github/workflows/mobile-release.yml) reads, by name, from the secrets of its environment
// `mobile-release`, and the versions it gives the stores. Never a value of a secret: only whether it is there.
//
//   node apps/mobile/release/release.mjs check <android|ios> [--build-only]   # stops, naming each secret missing
//   node apps/mobile/release/release.mjs version                              # the versions, as KEY=value lines
//
// fastlane (apps/mobile/fastlane/Fastfile) checks the same names again before it uses them.

import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

/** The GitHub environment that holds the secrets: required reviewer, `main` and `release/*` alone. */
export const ENVIRONMENT = 'mobile-release';

/**
 * Every secret the workflow reads, by platform, with what it is for. `upload`: needed only to send to the store
 * (not by the « build only » mode). `optional`: said when missing, never a failure.
 */
export const SECRETS = {
  android: [
    { name: 'ANDROID_UPLOAD_KEYSTORE_BASE64', what: 'the upload key (.jks, PKCS12), in base64' },
    { name: 'ANDROID_UPLOAD_KEYSTORE_PASSWORD', what: 'its password' },
    { name: 'ANDROID_UPLOAD_KEY_ALIAS', what: 'its alias (escouade-upload)' },
    { name: 'ANDROID_UPLOAD_KEY_PASSWORD', what: 'the password of the key (the same: PKCS12)' },
    { name: 'GOOGLE_PLAY_SERVICE_ACCOUNT_JSON', what: 'the JSON key of the play-ci service account', upload: true },
    {
      name: 'GOOGLE_SERVICES_JSON',
      what: 'google-services.json of the Firebase project reduced to FCM (read by MO-6)',
      optional: true,
    },
  ],
  ios: [
    { name: 'APPLE_TEAM_ID', what: 'the team ID (Membership details)' },
    { name: 'IOS_DISTRIBUTION_CERT_P12_BASE64', what: 'the Apple Distribution certificate (.p12, -legacy), in base64' },
    { name: 'IOS_DISTRIBUTION_CERT_PASSWORD', what: 'its password' },
    { name: 'IOS_PROVISIONING_PROFILE_BASE64', what: 'the App Store Connect profile « Escouade App Store », in base64' },
    { name: 'APP_STORE_CONNECT_API_KEY_ID', what: 'the ID of the App Store Connect API key', upload: true },
    { name: 'APP_STORE_CONNECT_API_ISSUER_ID', what: 'the ID of its issuer', upload: true },
    { name: 'APP_STORE_CONNECT_API_KEY_P8', what: 'the content of its .p8', upload: true },
  ],
};

/** The identifier of the app on both stores. */
export const APP_ID = 'dev.escouade.mobile';
/** The track of Google Play the workflow sends to: the closed test. */
export const PLAY_TRACK = 'alpha';
/** The name of the provisioning profile, as the guide makes it. */
export const PROFILE_NAME = 'Escouade App Store';

/**
 * What a run needs and what it lacks, from `env` (process.env): never a value, only the names.
 * @param {'android' | 'ios'} platform
 * @param {{ buildOnly?: boolean, env?: Record<string, string | undefined> }} [opts]
 */
export function checkSecrets(platform, { buildOnly = false, env = process.env } = {}) {
  const list = SECRETS[platform];
  if (!list) throw new Error(`unknown platform: ${platform} (android or ios)`);
  const present = (name) => typeof env[name] === 'string' && env[name].trim() !== '';
  const needed = list.filter((s) => !s.optional && !(buildOnly && s.upload));
  return {
    missing: needed.filter((s) => !present(s.name)).map((s) => s.name),
    optionalMissing: list.filter((s) => s.optional && !present(s.name)).map((s) => s.name),
  };
}

/**
 * The versions the stores are given. `run` is the workflow's run number: Google Play wants a versionCode that grows,
 * App Store Connect a build number that grows; the marketing version is the app's (package.json), without its
 * pre-release (`2.3.0-beta.2` → `2.3.0`: App Store Connect takes three numbers alone).
 * @param {string} appVersion
 * @param {string | number} run
 */
export function versions(appVersion, run) {
  const m = /^(\d+)\.(\d+)\.(\d+)/.exec(appVersion);
  if (!m) throw new Error(`not a version: ${appVersion}`);
  const code = Number(run);
  if (!Number.isInteger(code) || code < 1) throw new Error(`not a run number: ${run}`);
  return { name: appVersion, marketing: `${m[1]}.${m[2]}.${m[3]}`, code: String(code) };
}

function main(argv) {
  const [cmd, platform, ...rest] = argv;
  if (cmd === 'check') {
    const buildOnly = rest.includes('--build-only');
    const { missing, optionalMissing } = checkSecrets(platform, { buildOnly });
    for (const name of optionalMissing) {
      const s = SECRETS[platform].find((x) => x.name === name);
      console.log(`::notice::${name} is not set in the environment ${ENVIRONMENT} (${s.what}): built without it.`);
    }
    if (missing.length) {
      for (const name of missing) {
        const s = SECRETS[platform].find((x) => x.name === name);
        console.log(`::error::Missing secret ${name} in the environment ${ENVIRONMENT}: ${s.what}.`);
      }
      console.log(`Stopped before building: ${missing.length} secret(s) missing (${missing.join(', ')}). See docs/MOBILE-PUBLISH.md.`);
      return 1;
    }
    console.log(`${platform}: every secret is there${buildOnly ? ' (build only: nothing is sent)' : ''}.`);
    return 0;
  }
  if (cmd === 'version') {
    const pkg = JSON.parse(readFileSync(new URL('../../../package.json', import.meta.url), 'utf8'));
    const v = versions(pkg.version, process.env.GITHUB_RUN_NUMBER ?? '1');
    console.log(`ESCOUADE_VERSION_NAME=${v.name}\nESCOUADE_MARKETING_VERSION=${v.marketing}\nESCOUADE_VERSION_CODE=${v.code}`);
    return 0;
  }
  console.error('usage: release.mjs check <android|ios> [--build-only] | version');
  return 2;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv.slice(2));
}
