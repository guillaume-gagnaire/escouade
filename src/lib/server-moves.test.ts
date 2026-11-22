// What changes machines in server mode (ESC-538, docs/REMOTE.md « Ce qui change de machine », docs/SERVER-UX.md § 4,
// S12, S13), on the window's side: the sign-in's address read from its terminal (a return on localhost, a code of a
// device, a code to paste), an address of the server's loopback relayed to this computer's, the files received kept by
// this computer's core (their space and name said, opened or shown), what the server's host publishes shown here and
// named by its space (never what it asks to open), the voice of this computer, a logo sent as bytes, nothing of this
// changed on this computer.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { connection, type ServerInfo } from './connection.svelte';
import { setLang } from './i18n';
import { cloneFolder, diskText, entryNote, joinServer } from './server-browse';
import {
  bytesOfDataUrl,
  isLoopback,
  loginLink,
  onThisComputer,
  openingOf,
  posteVoice,
  receive,
  received,
  receivedText,
  showHostEvent,
  showReceived,
  type Received,
  withPosteVoice,
} from './server-moves.svelte';
import { openPath, openUrl } from './opener';
import { fakeBackend, resetApp, SETTINGS } from '../test/ipc';
import { fileOrigins } from './file-origin';

const TRAVAIL: ServerInfo = {
  id: 's1',
  name: 'Travail',
  ssh: 'acme-escouade',
  host: 'escouade.acme.internal',
  remotePort: 46480,
  device: 'DESKTOP · Windows',
  active: true,
};

/** Each call with its options (the headers of a raw body), as the IPC would get it. */
function withOptions() {
  const got: { cmd: string; args: unknown; options?: { headers?: Record<string, string> } }[] = [];
  const internals = (window as unknown as { __TAURI_INTERNALS__: { invoke: (...a: unknown[]) => Promise<unknown> } }).__TAURI_INTERNALS__;
  const invoke = internals.invoke;
  internals.invoke = (cmd: unknown, args: unknown, options: unknown) => {
    got.push({ cmd: String(cmd), args, options: options as { headers?: Record<string, string> } });
    return invoke(cmd, args, options);
  };
  return got;
}

beforeEach(() => {
  setLang('fr');
  resetApp();
  connection.server = TRAVAIL;
  connection.secretsInFile = true;
  for (const k of Object.keys(received)) delete received[k];
  posteVoice.value = null;
});
afterEach(() => {
  connection.server = null;
  connection.secretsInFile = false;
});

describe('the sign-in of an account on a server (S13)', () => {
  it('reads the page and the port of localhost its return waits on (Codex: its local server said before)', () => {
    const out =
      'Starting local login server on http://localhost:1455.\r\nIf your browser did not open, navigate to this URL to authenticate:\r\n\r\n' +
      '\x1b[36mhttps://auth.openai.com/oauth/authorize?response_type=code&client_id=app_X&redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback&scope=openid\x1b[0m\r\n';
    const l = loginLink(out)!;
    expect(l.url).toMatch(/^https:\/\/auth\.openai\.com\/oauth\/authorize\?/);
    expect(l.url).not.toContain('\x1b');
    expect(l.port).toBe(1455);
    expect(l.device).toBe(false);
  });

  it('reads a return on 127.0.0.1 in the page itself (Gemini CLI), a code to paste (Claude Code), a code of a device (Copilot)', () => {
    const gemini = loginLink(
      'Code Assist login required.\nhttps://accounts.google.com/o/oauth2/v2/auth?redirect_uri=http%3A%2F%2F127.0.0.1%3A41231%2Foauth2callback&access_type=offline',
    );
    expect(gemini?.port).toBe(41231);
    const claude = loginLink(
      "Browser didn't open? Use the url below to sign in:\n\nhttps://claude.ai/oauth/authorize?code=true&client_id=x&redirect_uri=https%3A%2F%2Fconsole.anthropic.com%2Foauth%2Fcode%2Fcallback\n\nPaste code here if prompted >",
    );
    expect(claude?.port).toBeNull();
    expect(claude?.device).toBe(false);
    const copilot = loginLink('To authenticate, visit https://github.com/login/device and enter code ABCD-1234.');
    expect(copilot).toEqual({ url: 'https://github.com/login/device', port: null, device: true });
    // Nothing yet, or only the server's own loopback: nothing to open.
    expect(loginLink('Starting…')).toBeNull();
    expect(loginLink('Starting local login server on http://localhost:1455.')).toBeNull();
  });
});

describe('the ports of the server', () => {
  it('relays an address of the server’s loopback to this computer, once, and leaves any other as it is', async () => {
    const backend = fakeBackend({ server_relay: (a) => ({ outcome: 'ok', value: { local: 51234, remote: a.port } }) });
    expect(isLoopback('http://localhost:5173/app')).toBe(true);
    expect(isLoopback('http://0.0.0.0:3000')).toBe(true);
    expect(isLoopback('https://escouade.dev')).toBe(false);
    expect(await onThisComputer('http://localhost:5173/app?x=1')).toBe('http://localhost:51234/app?x=1');
    expect(backend.called('server_relay').map((c) => c.args)).toEqual([{ port: 5173, same: false }]);
    // Already this computer's: not relayed again.
    expect(await onThisComputer('http://localhost:51234/app')).toBe('http://localhost:51234/app');
    expect(await onThisComputer('https://escouade.dev/docs')).toBe('https://escouade.dev/docs');
    expect(backend.called('server_relay')).toHaveLength(1);
    // A link opens in this computer's browser, at the address of this computer.
    await openUrl('http://127.0.0.1:8080/');
    expect(backend.called('open_url').at(-1)?.args).toEqual({ url: 'http://127.0.0.1:51234/' });
  });

  it('says why a port is not relayed', async () => {
    fakeBackend({
      server_relay: () => ({ outcome: 'problem', problem: { kind: 'refused', code: 'relay-busy', message: 'Le port 1455 est déjà pris' } }),
    });
    await expect(onThisComputer('http://localhost:1455/')).rejects.toBe('Le port 1455 est déjà pris');
  });

  it('on this computer, relays nothing and opens paths as before', async () => {
    connection.server = null;
    const backend = fakeBackend();
    expect(await onThisComputer('http://localhost:5173/')).toBe('http://localhost:5173/');
    await openPath('C:/dev/api');
    expect(backend.called('open_path')).toHaveLength(1);
    expect(backend.called('server_relay')).toHaveLength(0);
  });

  it('never opens a path of the server on this computer', async () => {
    const backend = fakeBackend();
    await expect(openPath('/home/escouade/projets/api')).rejects.toContain('Travail');
    expect(backend.called('open_path')).toHaveLength(0);
  });
});

describe('the files received (S12)', () => {
  it('are kept by this computer’s core, their space and name said, opened or shown in their folder', async () => {
    const answer = {
      path: 'C:/Users/me/Downloads/Escouade/Travail/rapport.pdf',
      folder: 'C:/Users/me/Downloads/Escouade/Travail',
      name: 'rapport.pdf',
      kind: 'pdf',
      runnable: false,
      opens: 'system',
      opened: true,
    };
    fakeBackend({ received_save: () => answer });
    const calls = withOptions();
    const got = await receive('a1|i1|0', 'rapport relances.pdf', async () => new Uint8Array([37, 80, 68, 70]).buffer, 'open');
    expect(got).toEqual(answer);
    expect(received['a1|i1|0']).toEqual(answer);
    const call = calls.find((c) => c.cmd === 'received_save')!;
    expect(call.args).toBeInstanceOf(Uint8Array);
    expect([...(call.args as Uint8Array)]).toEqual([37, 80, 68, 70]);
    expect(call.options?.headers).toEqual({ 'x-space': 'Travail', 'x-name': 'rapport%20relances.pdf', 'x-open': '1' });
    // Shown in its folder: no « x-open ».
    await receive('a1|i1|1', 'rejouer.sh', async () => new ArrayBuffer(2), 'reveal');
    expect(calls.filter((c) => c.cmd === 'received_save').at(-1)?.options?.headers).toEqual({
      'x-space': 'Travail',
      'x-name': 'rejouer.sh',
      'x-reveal': '1',
    });
  });

  it('open only an image, a PDF or a text of the list; anything else is shown in its folder, with why (ESC-629)', async () => {
    for (const n of ['capture.png', 'photo.JPG', 'photo.jpeg', 'anim.gif', 'image.webp', 'rapport.pdf'])
      expect(openingOf(n), n).toBe('system');
    for (const n of ['notes.txt', 'lisez-moi.md', 'journal.markdown', 'export.csv', 'donnees.json']) expect(openingOf(n), n).toBe('editor');
    for (const n of [
      'rapport.msc',
      'ouvrir.terminal',
      'lien.webloc',
      'app.appinstaller',
      'disque.iso',
      'page.html',
      'image.svg',
      'etude.docx',
      'rejouer.sh',
      'sans-extension',
      'pdf',
    ])
      expect(openingOf(n), n).toBe('folder');
    const copy = (over: Partial<Received>): Received => ({
      path: 'C:/Dl/Escouade/Travail/x',
      folder: 'C:/Dl/Escouade/Travail',
      name: 'x',
      kind: 'file',
      runnable: false,
      opens: 'folder',
      opened: false,
      ...over,
    });
    expect(receivedText(copy({ name: 'rapport.msc' }))).toBe(
      '« rapport.msc » n’est pas d’un type qu’Escouade ouvre (images, PDF, texte, Markdown, CSV, JSON) : téléchargé dans C:/Dl/Escouade/Travail et affiché dans son dossier.',
    );
    expect(receivedText(copy({ name: 'export.csv', opens: 'editor' }))).toContain('se lit dans l’éditeur d’Escouade');
    expect(receivedText(copy({ name: 'rejouer.sh', runnable: true }))).toContain('est un programme ou un script');
    // Opened again: only a copy of the list asks to be opened; any other is shown in its folder.
    const backend = fakeBackend({ received_show: () => null });
    await showReceived(copy({ path: 'C:/Dl/a.msc' }), false);
    await showReceived(copy({ path: 'C:/Dl/a.csv', opens: 'editor' }), false);
    await showReceived(copy({ path: 'C:/Dl/a.pdf', opens: 'system' }), false);
    expect(backend.called('received_show').map((c) => c.args)).toEqual([
      { path: 'C:/Dl/a.msc', reveal: true },
      { path: 'C:/Dl/a.csv', reveal: true },
      { path: 'C:/Dl/a.pdf', reveal: false },
    ]);
  });
});

describe('what the server’s host publishes', () => {
  it('is shown on this computer, the notification named by its space and its click asked of the server', async () => {
    const backend = fakeBackend({ server_alert: () => true, host_notification_click: () => true });
    showHostEvent({ type: 'notify', id: 7, title: 'facturation · relances', body: 'Tour fini' });
    showHostEvent({ type: 'chime' });
    showHostEvent({ type: 'waiting', count: 2 });
    await Promise.resolve();
    const alerts = backend.called('server_alert').map((c) => c.args);
    expect(alerts[0]).toMatchObject({ kind: 'notify', title: 'Travail · facturation · relances', body: 'Tour fini' });
    expect(alerts[1]).toMatchObject({ kind: 'chime' });
    expect(alerts[2]).toMatchObject({ kind: 'waiting', count: 2 });
  });

  it('never opens what the server asks to open', async () => {
    const backend = fakeBackend();
    showHostEvent({ type: 'open', kind: 'url', target: 'https://example.com/pwned' });
    showHostEvent({ type: 'open', kind: 'path', target: '/tmp/x.sh' });
    await Promise.resolve();
    expect(backend.calls.filter((c) => /open|alert/.test(c.cmd))).toHaveLength(0);
  });
});

describe('the voice, the files joined, a logo', () => {
  it('takes this computer’s voice in place of the server’s', () => {
    const settings = { ...SETTINGS };
    expect(withPosteVoice(settings)).toBe(settings);
    posteVoice.value = { ...SETTINGS.voice, model: 'small', device: 'Micro du poste' };
    expect(withPosteVoice(settings).voice.device).toBe('Micro du poste');
    connection.server = null;
    expect(withPosteVoice(settings).voice).toBe(SETTINGS.voice);
  });

  it('copies a file joined on a server, never its origin on this computer', async () => {
    expect(await fileOrigins([new File(['x'], 'a.txt')], 'windows')).toEqual([null]);
  });

  it('reads the bytes of a logo’s preview, to send them', () => {
    expect([...bytesOfDataUrl('data:image/png;base64,iVBORw==')]).toEqual([0x89, 0x50, 0x4e, 0x47]);
  });
});

describe('« Un dossier du serveur » and « Cloner un dépôt »', () => {
  it('says each folder, the disk, the folder of a clone', () => {
    expect(entryNote({ repo: true, branch: 'main' })).toBe('git · main');
    expect(entryNote({ repo: true })).toBe('git');
    expect(entryNote({ repo: false })).toBe('dossier');
    expect(diskText({ free: 41 * 1024 ** 3, total: 80 * 1024 ** 3 })).toMatch(/^Le disque du serveur : 41.*libres sur 80/);
    expect(diskText({})).toBe('');
    expect(cloneFolder('git@github.com:acme/facturation.git')).toBe('facturation');
    expect(cloneFolder('https://git.acme.fr/acme/outils/')).toBe('outils');
    expect(joinServer('/home/escouade/projets', 'api')).toBe('/home/escouade/projets/api');
    expect(joinServer('/', 'srv')).toBe('/srv');
  });
});
