// @vitest-environment node
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import {
  ACCOUNT,
  DLIB,
  checkDigest,
  dotnetReady,
  metadata,
  missingSecrets,
  pickSdk,
  powershellEnv,
  signConfig,
  signatureProblems,
} from './windows-signing.mjs';

const SCRIPT = path.join(path.dirname(fileURLToPath(import.meta.url)), 'windows-signing.mjs');
const SECRETS = { AZURE_CLIENT_ID: 'client', AZURE_CLIENT_SECRET: 'secret', AZURE_TENANT_ID: 'tenant' };

describe('Windows signing', () => {
  it('signs with the account made in Azure (North Europe)', () => {
    expect(ACCOUNT).toEqual({
      endpoint: 'https://neu.codesigning.azure.net',
      account: 'escouadesigning',
      profile: 'escouade',
    });
  });

  it('pins the dlib package by version and SHA-256', () => {
    expect(DLIB.url).toBe(`https://www.nuget.org/api/v2/package/Microsoft.ArtifactSigning.Client/${DLIB.version}`);
    expect(DLIB.sha256).toMatch(/^[0-9a-f]{64}$/);
  });

  it('names the secrets that are missing or blank', () => {
    expect(missingSecrets({})).toEqual(['AZURE_CLIENT_ID', 'AZURE_CLIENT_SECRET', 'AZURE_TENANT_ID']);
    expect(missingSecrets({ ...SECRETS, AZURE_CLIENT_SECRET: '  ' })).toEqual(['AZURE_CLIENT_SECRET']);
    expect(missingSecrets(SECRETS)).toEqual([]);
  });

  it('takes the newest Windows SDK the dlib accepts (22621 or later)', () => {
    expect(pickSdk(['10.0.14393.0', '10.0.22621.0', '10.0.26100.0', 'x64', 'arm64'])).toBe('10.0.26100.0');
    // Compared as numbers, not as text.
    expect(pickSdk(['10.0.100000.0', '10.0.26100.0'])).toBe('10.0.100000.0');
    expect(pickSdk(['10.0.22621.0'])).toBe('10.0.22621.0');
    // 20348 is refused by the dlib, older ones too.
    expect(pickSdk(['10.0.20348.0', '10.0.19041.0', 'x86'])).toBeNull();
    expect(pickSdk([])).toBeNull();
  });

  it('wants a .NET runtime 8 or later (the dlib rolls forward to newer ones)', () => {
    const line = (name: string, version: string) => `${name} ${version} [C:\\Program Files\\dotnet\\shared\\${name}]`;
    expect(dotnetReady(line('Microsoft.NETCore.App', '8.0.32'))).toBe(true);
    expect(dotnetReady([line('Microsoft.NETCore.App', '6.0.36'), line('Microsoft.NETCore.App', '10.0.1')].join('\r\n'))).toBe(true);
    expect(dotnetReady(line('Microsoft.NETCore.App', '6.0.36'))).toBe(false);
    // The ASP.NET or desktop runtimes alone do not host the dlib.
    expect(dotnetReady(line('Microsoft.AspNetCore.App', '8.0.32'))).toBe(false);
    expect(dotnetReady('')).toBe(false);
  });

  it('refuses a download whose SHA-256 is not the pinned one', () => {
    const data = Buffer.from('dlib');
    const sha256 = createHash('sha256').update(data).digest('hex');
    expect(() => checkDigest(data, sha256)).not.toThrow();
    expect(() => checkDigest(data, sha256.toUpperCase())).not.toThrow();
    expect(() => checkDigest(Buffer.from('other'), sha256)).toThrow(/SHA-256/);
  });

  it('authenticates with the AZURE_* variables only', () => {
    const data = metadata({ ...ACCOUNT, correlationId: 'escouade-1-1' });
    expect(data).toMatchObject({
      Endpoint: 'https://neu.codesigning.azure.net',
      CodeSigningAccountName: 'escouadesigning',
      CertificateProfileName: 'escouade',
      CorrelationId: 'escouade-1-1',
    });
    // Every credential DefaultAzureCredential would try but EnvironmentCredential: the runner has
    // the Azure CLI, and a browser login would wait for nobody.
    expect(data.ExcludeCredentials).toEqual([
      'ManagedIdentityCredential',
      'WorkloadIdentityCredential',
      'SharedTokenCacheCredential',
      'VisualStudioCredential',
      'VisualStudioCodeCredential',
      'AzureCliCredential',
      'AzurePowerShellCredential',
      'AzureDeveloperCliCredential',
      'InteractiveBrowserCredential',
    ]);
  });

  it('gives Tauri a signtool command through the dlib, timestamped, the file last', () => {
    const signtool = 'C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.26100.0\\x64\\signtool.exe';
    const config = signConfig({ signtool, dlib: 'D:\\t\\Azure.CodeSigning.Dlib.dll', metadata: 'D:\\t\\metadata.json' });
    // The object form: the path of signtool has spaces, which the string form would split.
    expect(config).toEqual({
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
              'http://timestamp.acs.microsoft.com',
              '/td',
              'SHA256',
              '/d',
              'Escouade',
              '/dlib',
              'D:\\t\\Azure.CodeSigning.Dlib.dll',
              '/dmdf',
              'D:\\t\\metadata.json',
              '%1',
            ],
          },
        },
      },
    });
  });

  it('finds nothing to say about files signed by one publisher and timestamped', () => {
    const signed = (file: string) => ({ file, status: 'Valid', subject: 'CN=Publisher, O=Publisher', timestamper: 'CN=TSA' });
    expect(signatureProblems([signed('setup.exe'), signed('escouade.exe')])).toEqual([]);
  });

  it('says which file is not signed, not timestamped, or signed by someone else', () => {
    const results = [
      { file: 'setup.exe', status: 'Valid', subject: 'CN=Publisher, O=Publisher', timestamper: 'CN=TSA' },
      { file: 'escouade.exe', status: 'NotSigned', subject: '', timestamper: '' },
      { file: 'escouade-hook.exe', status: 'Valid', subject: 'CN=Publisher, O=Publisher', timestamper: '' },
      { file: 'other.exe', status: 'Valid', subject: 'CN=Other', timestamper: 'CN=TSA' },
    ];
    expect(signatureProblems(results)).toEqual([
      'escouade.exe: NotSigned, not a valid signature',
      // Artifact Signing's certificates last three days: without a timestamp, the signature would
      // stop being valid with them.
      'escouade-hook.exe: not timestamped',
      'other.exe: signed by CN=Other, not by CN=Publisher, O=Publisher like setup.exe',
    ]);
    expect(signatureProblems([])).toEqual(['no file to check']);
    // A status PowerShell could not read is not taken for a signature.
    expect(signatureProblems([{ file: 'setup.exe', status: '', subject: '', timestamper: '' }])).toEqual([
      'setup.exe: signature not read, not a valid signature',
    ]);
  });

  it('starts Windows PowerShell without the PSModulePath of PowerShell 7', () => {
    // Started from pwsh (the shell of the runner's steps), powershell.exe inherits its modules'
    // path, and the module of Get-AuthenticodeSignature no longer loads (the 1.8.14 Release).
    const env = { Path: 'C:\\Windows', PSModulePath: 'C:\\Program Files\\PowerShell\\7\\Modules', OTHER: '1' };
    expect(powershellEnv(env)).toEqual({ Path: 'C:\\Windows', OTHER: '1' });
    expect(powershellEnv({ PSMODULEPATH: 'x', psmodulepath: 'y', Path: 'C:\\Windows' })).toEqual({ Path: 'C:\\Windows' });
    expect(env.PSModulePath).toBe('C:\\Program Files\\PowerShell\\7\\Modules');
  });

  describe('setup', () => {
    const run = (env: Record<string, string>) => {
      const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'windows-signing-'));
      const output = path.join(dir, 'github-output');
      fs.writeFileSync(output, '');
      const clean = Object.fromEntries(
        Object.entries(process.env).filter(([name]) => !name.startsWith('AZURE_') && name !== 'SIGNTOOL_PATH'),
      );
      const result = spawnSync(process.execPath, [SCRIPT, 'setup', path.join(dir, 'signing')], {
        env: { ...clean, GITHUB_OUTPUT: output, ...env },
        encoding: 'utf8',
      });
      return { ...result, output: fs.readFileSync(output, 'utf8') };
    };

    it('leaves the build unsigned, with a warning, without the secrets', () => {
      const result = run({});
      expect(result.status).toBe(0);
      expect(result.stdout).toContain(
        '::warning::The Windows build is not signed: AZURE_CLIENT_ID, AZURE_CLIENT_SECRET, AZURE_TENANT_ID missing from the secrets',
      );
      expect(result.output).toBe('');
    });

    it('fails, rather than leave the build unsigned, when the secrets are there but signtool is not', () => {
      const result = run({ ...SECRETS, SIGNTOOL_PATH: path.join(os.tmpdir(), 'no-such-dir', 'signtool.exe') });
      expect(result.status).toBe(1);
      expect(result.stdout).toMatch(/::error::.*signtool/);
      expect(result.stdout).not.toContain('secret');
      expect(result.output).toBe('');
    });
  });
});
