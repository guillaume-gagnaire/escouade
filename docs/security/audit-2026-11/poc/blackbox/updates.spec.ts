// Audit 2026-11, black box: the update channel against a server of the test's (`ESCOUADE_UPDATE_ENDPOINT`, which only a
// debug build reads; a release reads GitHub's alone). The server plays who holds the channel: it serves the latest.json
// it wants and an « installer » that is a text file. What must hold: nothing installs unless it is signed by the key the
// app embeds, and no version older than the app's is ever offered.
import http from 'node:http';
import type { Page } from '@playwright/test';
import { expect, test } from '../../../../../tests/e2e/fixture';
import { testKey } from '../../../../../store/tests/fake/minisign.mjs';
import { invoke, keep, PORTS } from './helpers';

const ORIGIN = `http://127.0.0.1:${PORTS.updates}`;
/** What stands for an installer: never a program. */
const INSTALLER = Buffer.from('AUDIT-POC: this is not an installer.\n');
/** A key that is not Escouade's: its signatures are well formed, and worth nothing. */
const stranger = testKey();
const b64 = (text: string) => Buffer.from(text, 'utf8').toString('base64');

const served = { body: '{}', status: 200, hits: [] as string[] };

function manifest(version: string, signature: string, url = `${ORIGIN}/setup.exe`) {
  const entry = { signature, url };
  const targets = ['windows-x86_64', 'windows-x86_64-nsis', 'darwin-aarch64', 'darwin-x86_64', 'linux-x86_64'];
  return JSON.stringify({
    version,
    notes: 'audit',
    pub_date: '2026-11-12T00:00:00Z',
    platforms: Object.fromEntries(targets.map((t) => [t, entry])),
  });
}

test.use({
  appEnv: async ({}, use) => {
    const server = http.createServer((req, res) => {
      const name = (req.url ?? '/').split('?')[0];
      served.hits.push(`${req.method} ${name}`);
      if (name === '/latest.json') res.writeHead(served.status, { 'Content-Type': 'application/json' }).end(served.body);
      else if (name === '/setup.exe') res.writeHead(200, { 'Content-Type': 'application/octet-stream' }).end(INSTALLER);
      else res.writeHead(404).end('not found');
    });
    await new Promise<void>((resolve) => server.listen(PORTS.updates, '127.0.0.1', resolve));
    await use({ ESCOUADE_UPDATE_ENDPOINT: `${ORIGIN}/latest.json` });
    await new Promise((resolve) => server.close(resolve));
  },
});

type Checked = { kind: string; update?: { id: number; version: string }; trouble?: unknown };
type Downloaded = { kind: string; [k: string]: unknown };

/** One check with `body` served, then the download of what it found: what the app said of each. */
async function play(page: Page, body: string, status = 200) {
  served.body = body;
  served.status = status;
  const before = served.hits.length;
  const checked = await invoke<Checked>(page, 'update_check', { manual: true });
  let downloaded: Downloaded | null = null;
  if (checked.kind === 'found' && checked.update) {
    downloaded = await invoke<Downloaded>(page, 'update_download', { id: checked.update.id, manual: true });
  }
  // « Redémarrer maintenant » with nothing that holds: refused.
  const restart = await invoke(page, 'update_restart').then(
    () => 'ACCEPTED',
    (e) => `refused: ${String(e).slice(0, 120)}`,
  );
  // The page of the releases the app would send the user to names the publisher's account: not kept in the notes.
  const brief = (v: unknown) =>
    JSON.stringify(v ?? null)
      .replace(/https:\/\/github\.com\/[^"\\\s]*/g, '<page des versions>')
      .slice(0, 420);
  return {
    check: checked.kind,
    offered: checked.update?.version ?? null,
    download: downloaded?.kind ?? null,
    said: brief(downloaded ?? checked.trouble),
    restart,
    asked: served.hits.slice(before),
  };
}

test('an update that is not signed by Escouade never installs, and an older version is never offered', async ({ app }) => {
  const { page } = app;
  const current = await page.evaluate(() =>
    (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string): Promise<string> } }).__TAURI_INTERNALS__.invoke('plugin:app|version'),
  );
  const seen: Record<string, Awaited<ReturnType<typeof play>>> = {};

  // A newer version whose signature is not one.
  seen['newer, signature that is no signature'] = await play(page, manifest('99.0.0', 'test'));
  seen['newer, empty signature'] = await play(page, manifest('99.0.1', ''));
  // A signature well formed, by a key that is not the app's.
  seen['newer, signed by another key'] = await play(
    page,
    manifest('99.0.2', b64(stranger.sign(INSTALLER, 'timestamp:1762905600\tfile:setup.exe'))),
  );
  // The app's own public key given as if it were a signature, and a signature of other bytes.
  seen['newer, signed for other bytes'] = await play(
    page,
    manifest('99.0.3', b64(stranger.sign('something else', 'timestamp:1762905600\tfile:setup.exe'))),
  );
  // A downgrade: an older version, however it is signed, and the same version again.
  seen['older version (downgrade)'] = await play(page, manifest('0.0.1', 'test'));
  seen['older version, one patch below'] = await play(
    page,
    manifest(
      current.replace(/(\d+)$/, (n) => String(Math.max(0, Number(n) - 1))),
      'test',
    ),
  );
  seen['same version'] = await play(page, manifest(current, 'test'));
  // A beta on the stable channel, betas off.
  seen['pre-release without the betas'] = await play(page, manifest('99.0.0-beta.1', 'test'));
  // A latest.json that is none.
  seen['not JSON'] = await play(page, '<html>captive portal</html>');
  seen['no platform'] = await play(page, JSON.stringify({ version: '99.1.0', platforms: {} }));
  seen['a version that is none'] = await play(page, manifest('../../99', 'test'));
  seen['HTTP 500'] = await play(page, '{}', 500);
  // An installer that is a file of the machine, or of another scheme.
  seen['installer by file://'] = await play(page, manifest('99.2.0', 'test', 'file:///C:/Windows/System32/calc.exe'));
  seen['installer by a share'] = await play(page, manifest('99.2.1', 'test', '\\\\\\\\127.0.0.1\\\\share\\\\setup.exe'));

  keep('updates', { appVersion: current, cases: seen });
  for (const [name, r] of Object.entries(seen)) {
    expect(r.restart, `${name}: an install was accepted`).not.toBe('ACCEPTED');
    expect(r.download, `${name}: a download was kept`).not.toBe('ready');
  }
  for (const name of ['older version (downgrade)', 'older version, one patch below', 'same version', 'pre-release without the betas']) {
    expect(seen[name].offered, name).toBeNull();
  }
  // The window is still the app's, the process still there.
  expect(await page.evaluate(() => document.readyState)).toBe('complete');
});
