#!/usr/bin/env node
// Signs the Windows build with Azure Artifact Signing (Authenticode), for the Release workflow: SmartScreen then weighs
// the reputation of the publisher rather than that of each new file.
//   node scripts/windows-signing.mjs setup <dir>
//     With the AZURE_* secrets (an app registration that has the Certificate Profile Signer role on the profile),
//     fetches Microsoft's dlib for signtool (its SHA-256 checked) into <dir>, writes its metadata.json and a Tauri
//     configuration whose signCommand signs through it, and gives `args=--config <that file>` to the step's outputs
//     (GITHUB_OUTPUT). Tauri then signs escouade.exe, escouade-hook.exe, escouade-plugin.exe, the NSIS plugins, the uninstaller and the
//     installer, before the updater's signature. The dlib authenticates with the AZURE_* variables of the build step
//     (EnvironmentCredential). Without the secrets, a warning and no output: the build is not signed.
//   node scripts/windows-signing.mjs verify <setup.exe> [<dir>]
//     Checks that the installer and the app's binaries in it (extracted with 7-Zip into <dir>) are signed by one
//     publisher and timestamped.
// SIGNTOOL_PATH overrides the signtool found in the newest Windows SDK.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// The account and profile made in Azure; the endpoint is that of their region.
export const ACCOUNT = {
  endpoint: 'https://neu.codesigning.azure.net',
  account: 'escouadesigning',
  profile: 'escouade',
};

// Microsoft.ArtifactSigning.Client: the dlib signtool loads, with the .NET and C++ libraries it needs.
export const DLIB = {
  version: '1.0.146',
  sha256: '7761e5d8f31e54b764da21d33cd8b0a2940e1b05526c7dfcb5e94b0431426f70',
  get url() {
    return `https://www.nuget.org/api/v2/package/Microsoft.ArtifactSigning.Client/${this.version}`;
  },
};

export const SECRETS = ['AZURE_CLIENT_ID', 'AZURE_CLIENT_SECRET', 'AZURE_TENANT_ID'];

// Every credential DefaultAzureCredential tries but EnvironmentCredential (the AZURE_* variables): the runner has the
// Azure CLI, and a browser login would wait for nobody.
const EXCLUDED_CREDENTIALS = [
  'ManagedIdentityCredential',
  'WorkloadIdentityCredential',
  'SharedTokenCacheCredential',
  'VisualStudioCredential',
  'VisualStudioCodeCredential',
  'AzureCliCredential',
  'AzurePowerShellCredential',
  'AzureDeveloperCliCredential',
  'InteractiveBrowserCredential',
];

// Artifact Signing's certificates last three days: the timestamp keeps a signature valid after.
const TIMESTAMP_URL = 'http://timestamp.acs.microsoft.com';

// The binaries of the app in the installer, beside it.
const INSIDE = ['escouade.exe', 'escouade-hook.exe', 'escouade-plugin.exe'];

/** The AZURE_* secrets missing from `env`, or blank. */
export function missingSecrets(env) {
  return SECRETS.filter((name) => !String(env[name] ?? '').trim());
}

const parts = (version) => version.split('.').map(Number);

function compareVersions(a, b) {
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < x.length; i++) if (x[i] !== y[i]) return x[i] - y[i];
  return 0;
}

/** The newest of the Windows SDK versions `names` (folders of `Windows Kits\10\bin`) whose signtool the dlib accepts: 10.0.22621 or later (it refuses 20348). Null when none. */
export function pickSdk(names) {
  const versions = names.filter((name) => /^10\.0\.\d+\.\d+$/.test(name) && parts(name)[2] >= 22621);
  return versions.sort(compareVersions).at(-1) ?? null;
}

/** Whether `dotnet --list-runtimes` printed a .NET runtime 8 or later, which the dlib runs on (it rolls forward). */
export function dotnetReady(list) {
  return String(list)
    .split(/\r?\n/)
    .some((line) => {
      const found = /^Microsoft\.NETCore\.App (\d+)\./.exec(line.trim());
      return found !== null && Number(found[1]) >= 8;
    });
}

/** Throws unless `data` has the SHA-256 `sha256`. */
export function checkDigest(data, sha256) {
  const actual = createHash('sha256').update(data).digest('hex');
  if (actual !== sha256.toLowerCase()) {
    throw new Error(`the SHA-256 of the download is ${actual}, not the pinned ${sha256}`);
  }
}

/** The metadata.json of the dlib. */
export function metadata({ endpoint, account, profile, correlationId }) {
  return {
    Endpoint: endpoint,
    CodeSigningAccountName: account,
    CertificateProfileName: profile,
    CorrelationId: correlationId,
    ExcludeCredentials: EXCLUDED_CREDENTIALS,
  };
}

/** The Tauri configuration that signs each file through the dlib. The object form of signCommand: the path of signtool has spaces, which the string form would split. */
export function signConfig({ signtool, dlib, metadata: metadataFile }) {
  return {
    bundle: {
      windows: {
        signCommand: {
          cmd: signtool,
          args: [
            'sign',
            '/v',
            '/fd',
            'SHA256',
            '/tr',
            TIMESTAMP_URL,
            '/td',
            'SHA256',
            '/d',
            'Escouade',
            '/dlib',
            dlib,
            '/dmdf',
            metadataFile,
            '%1',
          ],
        },
      },
    },
  };
}

/** What is wrong with the signatures `results` (`{ file, status, subject, timestamper }`, as PowerShell reads them); empty when every file is validly signed by the first one's publisher, and timestamped. */
export function signatureProblems(results) {
  if (!results.length) return ['no file to check'];
  const found = [];
  const first = results.find((r) => r.status === 'Valid');
  for (const r of results) {
    if (r.status !== 'Valid') found.push(`${r.file}: ${r.status || 'signature not read'}, not a valid signature`);
    else if (r.subject !== first.subject) found.push(`${r.file}: signed by ${r.subject}, not by ${first.subject} like ${first.file}`);
    else if (!r.timestamper) found.push(`${r.file}: not timestamped`);
  }
  return found;
}

/** `env` for Windows PowerShell, without PSModulePath (in any case): started from pwsh, the shell of the runner's steps, powershell.exe would inherit the modules' path of PowerShell 7, and the module of Get-AuthenticodeSignature would not load. */
export function powershellEnv(env) {
  return Object.fromEntries(Object.entries(env).filter(([name]) => name.toLowerCase() !== 'psmodulepath'));
}

function signtool() {
  if (process.env.SIGNTOOL_PATH) {
    if (!fs.existsSync(process.env.SIGNTOOL_PATH)) throw new Error(`signtool ${process.env.SIGNTOOL_PATH} not found`);
    return process.env.SIGNTOOL_PATH;
  }
  const bin = path.join(process.env['ProgramFiles(x86)'] || 'C:\\Program Files (x86)', 'Windows Kits', '10', 'bin');
  const names = fs.existsSync(bin) ? fs.readdirSync(bin).filter((name) => fs.existsSync(path.join(bin, name, 'x64', 'signtool.exe'))) : [];
  const sdk = pickSdk(names);
  if (!sdk) throw new Error(`no signtool of a Windows SDK 10.0.22621 or later in ${bin}`);
  return path.join(bin, sdk, 'x64', 'signtool.exe');
}

async function setup(dir) {
  const missing = missingSecrets(process.env);
  if (missing.length) {
    console.log(`::warning::The Windows build is not signed: ${missing.join(', ')} missing from the secrets`);
    return;
  }
  const tool = signtool();
  let runtimes = '';
  try {
    runtimes = execFileSync('dotnet', ['--list-runtimes'], { encoding: 'utf8' });
  } catch {
    // Said below.
  }
  if (!dotnetReady(runtimes))
    throw new Error('the dlib of signtool needs a .NET runtime 8 or later, which `dotnet --list-runtimes` does not list');

  fs.mkdirSync(dir, { recursive: true });
  const response = await fetch(DLIB.url);
  if (!response.ok) throw new Error(`${DLIB.url}: HTTP ${response.status}`);
  const data = Buffer.from(await response.arrayBuffer());
  checkDigest(data, DLIB.sha256);
  const nupkg = path.join(dir, `Microsoft.ArtifactSigning.Client.${DLIB.version}.nupkg`);
  fs.writeFileSync(nupkg, data);
  // The tar of Windows (bsdtar) reads a zip; Git Bash's, first in some PATHs, does not.
  const tar = path.join(process.env.SystemRoot || 'C:\\Windows', 'System32', 'tar.exe');
  const lib = path.join(dir, 'dlib');
  fs.mkdirSync(lib, { recursive: true });
  execFileSync(tar, ['-xf', nupkg, '-C', lib, 'bin/x64'], { stdio: 'inherit' });
  const dlib = path.join(lib, 'bin', 'x64', 'Azure.CodeSigning.Dlib.dll');
  if (!fs.existsSync(dlib)) throw new Error(`no bin/x64/Azure.CodeSigning.Dlib.dll in ${nupkg}`);

  const run = process.env.GITHUB_RUN_ID ? `${process.env.GITHUB_RUN_ID}-${process.env.GITHUB_RUN_ATTEMPT || 1}` : 'local';
  const metadataFile = path.join(dir, 'metadata.json');
  fs.writeFileSync(metadataFile, JSON.stringify(metadata({ ...ACCOUNT, correlationId: `escouade-${run}` }), null, 2));
  const config = path.join(dir, 'tauri.signing.conf.json');
  fs.writeFileSync(config, JSON.stringify(signConfig({ signtool: tool, dlib, metadata: metadataFile }), null, 2));

  const args = `--config ${config}`;
  if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `args=${args}\n`);
  console.log(`Windows build signed by ${ACCOUNT.account}/${ACCOUNT.profile} (${ACCOUNT.endpoint})`);
  console.log(`  signtool: ${tool}`);
  console.log(`  dlib: Microsoft.ArtifactSigning.Client ${DLIB.version}`);
  console.log(`  build arguments: ${args}`);
}

function sevenZip() {
  const installed = path.join(process.env.ProgramFiles || 'C:\\Program Files', '7-Zip', '7z.exe');
  return fs.existsSync(installed) ? installed : '7z';
}

function verify(installer, dir) {
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  execFileSync(sevenZip(), ['e', '-y', `-o${dir}`, installer, ...INSIDE], { stdio: 'ignore' });
  const files = [installer, ...INSIDE.map((name) => path.join(dir, name))];
  const absent = files.filter((file) => !fs.existsSync(file));
  if (absent.length) return absent.map((file) => `${file}: not found (not in the installer?)`);
  // One PowerShell for every file: the list goes in by a variable, the results out as JSON. An error stops it, with
  // its exit code: a file it could not read is not taken for an unsigned one.
  const script = `
    $ErrorActionPreference = 'Stop'
    $results = foreach ($file in ($env:ESCOUADE_SIGNED_FILES | ConvertFrom-Json)) {
      $s = Get-AuthenticodeSignature -LiteralPath $file
      [pscustomobject]@{ file = $file; status = "$($s.Status)"; subject = "$($s.SignerCertificate.Subject)"; timestamper = "$($s.TimeStamperCertificate.Subject)" }
    }
    ConvertTo-Json -InputObject @($results) -Compress`;
  const json = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], {
    encoding: 'utf8',
    env: { ...powershellEnv(process.env), ESCOUADE_SIGNED_FILES: JSON.stringify(files) },
  });
  const results = JSON.parse(json);
  const found = signatureProblems(results);
  if (!found.length) {
    for (const r of results) console.log(`${r.file}: ${r.subject}, timestamped by ${r.timestamper}`);
  }
  return found;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  // A workflow command stops at the end of the line.
  const oneLine = (text) => String(text).replace(/\s*\r?\n\s*/g, ' ');
  const fail = (message) => {
    console.log(`::error::${oneLine(message)}`);
    process.exitCode = 1;
  };
  const [command, target, dir] = process.argv.slice(2);
  try {
    if (command === 'setup' && target) await setup(target);
    else if (command === 'verify' && target) {
      for (const problem of verify(target, dir || path.join(os.tmpdir(), 'escouade-signatures'))) fail(problem);
    } else fail('usage: node scripts/windows-signing.mjs setup <dir> | verify <setup.exe> [<dir>]');
  } catch (error) {
    fail(`Windows signing: ${error.message}`);
  }
}
