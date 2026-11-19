// What changes machines in server mode (ESC-538), in the window's components: « Nouveau projet » on a server (clone
// first, the folders of the server, their repositories and projects, its disk), « Cloner un dépôt », the files an agent
// sent (downloaded, a script only downloaded, « Voir sur le serveur »), the sign-in of an account (its page opened here at
// a click, its return relayed or the code to paste), the secrets kept in a file on the server, the voice of this
// computer, « Reconnecté à Travail · 12 événements rattrapés ». On this computer, none of it shows.

import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { connection, type ServerInfo } from '../../lib/connection.svelte';
import { setLang } from '../../lib/i18n';
import { received } from '../../lib/server-moves.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import ConnectionBand from '../ConnectionBand.svelte';
import SentFiles from '../conv/SentFiles.svelte';
import CloneRepo from './CloneRepo.svelte';
import SecretsOnServer from './SecretsOnServer.svelte';
import ServerFolders from './ServerFolders.svelte';
import ServerSignIn from './ServerSignIn.svelte';

const TRAVAIL: ServerInfo = {
  id: 's1',
  name: 'Travail',
  ssh: 'acme-escouade',
  host: 'escouade.acme.internal',
  remotePort: 46480,
  device: 'DESKTOP · Windows',
  active: true,
};

const LISTING = {
  path: '/home/escouade/projets',
  parent: '/home/escouade',
  home: '/home/escouade',
  repo: false,
  entries: [
    { name: 'facturation', path: '/home/escouade/projets/facturation', repo: true, branch: 'main', project: 'facturation' },
    { name: 'outils-internes', path: '/home/escouade/projets/outils-internes', repo: true, branch: 'main' },
    { name: 'scripts', path: '/home/escouade/projets/scripts', repo: false },
  ],
  more: 0,
  free: 41 * 1024 ** 3,
  total: 80 * 1024 ** 3,
};

beforeEach(() => {
  setLang('fr');
  resetApp();
  connection.server = TRAVAIL;
  connection.secretsInFile = true;
  connection.missed = null;
  for (const k of Object.keys(received)) delete received[k];
});
afterEach(() => {
  cleanup();
  connection.server = null;
  connection.secretsInFile = false;
  connection.state = 'local';
});

describe('« Un dossier du serveur »', () => {
  it('lists the folders of the server, their repository and branch, a project greyed, its disk; a click chooses, a double click goes in', async () => {
    const backend = fakeBackend({
      fs_browse: (a) =>
        a.path === '/home/escouade/projets/scripts'
          ? { ...LISTING, path: a.path, parent: LISTING.path, entries: [], free: undefined, total: undefined }
          : LISTING,
    });
    let picked = '';
    render(ServerFolders, { props: { onpick: (p: string) => (picked = p) } });
    const list = await screen.findByRole('list', { name: 'Dossiers de /home/escouade/projets' });
    const items = within(list).getAllByRole('button', { pressed: false });
    expect(items.map((b) => b.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'facturation Déjà un projet git · main',
      'outils-internes git · main',
      'scripts dossier',
    ]);
    expect(screen.getByText(/Le disque du serveur : 41.*libres sur 80/)).toBeTruthy();
    // A project already is not chosen again.
    await userEvent.click(items[0]);
    expect(picked).toBe('');
    await userEvent.click(items[1]);
    expect(picked).toBe('/home/escouade/projets/outils-internes');
    // Into a folder, then « Choisir ce dossier ».
    await userEvent.dblClick(items[2]);
    expect(await screen.findByText('Aucun dossier ici.')).toBeTruthy();
    expect(backend.called('fs_browse').at(-1)?.args).toEqual({ path: '/home/escouade/projets/scripts' });
    await userEvent.click(screen.getByRole('button', { name: 'Choisir ce dossier' }));
    expect(picked).toBe('/home/escouade/projets/scripts');
    // The parent folder.
    await userEvent.click(screen.getByRole('button', { name: 'Dossier parent' }));
    expect(backend.called('fs_browse').at(-1)?.args).toEqual({ path: '/home/escouade/projets' });
  });

  it('says why a folder cannot be read', async () => {
    fakeBackend({
      fs_browse: () => {
        throw 'Impossible de lire « /root » : accès refusé';
      },
    });
    render(ServerFolders, { props: { onpick: () => {} } });
    expect((await screen.findByRole('alert')).textContent).toContain('accès refusé');
  });
});

describe('« Cloner un dépôt »', () => {
  it('clones on the server into the folder shown, named as git would name it', async () => {
    const backend = fakeBackend({ clone_repo: (a) => `${a.parent}/${a.folder}` });
    let cloned = '';
    render(CloneRepo, { props: { parent: '/home/escouade/projets', oncloned: (p: string) => (cloned = p) } });
    await userEvent.type(screen.getByPlaceholderText(/git@github.com/), 'git@github.com:acme/facturation.git');
    expect(screen.getByText('/home/escouade/projets/facturation')).toBeTruthy();
    expect(screen.getByText(/clone avec ses propres clés/)).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: 'Cloner sur Travail' }));
    expect(backend.called('clone_repo')[0].args).toEqual({
      url: 'git@github.com:acme/facturation.git',
      parent: '/home/escouade/projets',
      folder: 'facturation',
    });
    expect(cloned).toBe('/home/escouade/projets/facturation');
    expect(screen.getByRole('status').textContent).toContain('Cloné dans /home/escouade/projets/facturation');
  });
});

describe('the files an agent sent, on a server (S12)', () => {
  const item = { id: 'i1', kind: 'tool', name: 'SendUserFile', status: 'done', input: { files: ['out/rapport.pdf', 'rejouer.sh'] } } as any;
  const views = [
    { name: 'rapport.pdf', kind: 'file', state: 'ok', size: 248_000 },
    { name: 'rejouer.sh', kind: 'file', state: 'ok', size: 3000, runnable: true },
  ];

  it('downloads a file to this computer when it is opened, a script only downloaded, and says where the copy went', async () => {
    const backend = fakeBackend({
      sent_files: () => views,
      sent_file_bytes: () => new Uint8Array([1, 2]).buffer,
      received_save: () => ({
        path: 'C:/Dl/Escouade/Travail/rapport.pdf',
        folder: 'C:/Dl/Escouade/Travail',
        name: 'rapport.pdf',
        kind: 'pdf',
        runnable: false,
        opened: true,
      }),
    });
    app.agents.a1 = { id: 'a1', projectId: 'p1', cwd: '/home/escouade/projets/facturation' } as any;
    render(SentFiles, { props: { item, agentId: 'a1' } });
    await screen.findByText(/un script ne s’ouvre pas d’Escouade/);
    // The script: no « Ouvrir », only downloaded; the file: « Ouvrir ».
    const rows = screen.getAllByTestId('sent-acts');
    expect(within(rows[1]).queryByRole('button', { name: /Ouvrir/ })).toBeNull();
    expect(within(rows[1]).getByRole('button', { name: /Télécharger sur ce poste/ })).toBeTruthy();
    await userEvent.click(within(rows[0]).getByRole('button', { name: 'Ouvrir rapport.pdf' }));
    expect(backend.called('sent_file_bytes')[0].args).toEqual({ agentId: 'a1', itemId: 'i1', index: 0 });
    expect(backend.called('received_save')).toHaveLength(1);
    // The copy said: where it came from, where it went; never opened from the server.
    expect((await screen.findByTestId('sent-copied')).textContent).toContain(
      'envoyé depuis Travail · copié sur ce poste dans C:/Dl/Escouade/Travail',
    );
    expect(screen.getByText('Téléchargé')).toBeTruthy();
    expect(backend.called('sent_file_open')).toHaveLength(0);
    // Opened again: the copy, not a new download.
    await userEvent.click(within(screen.getAllByTestId('sent-acts')[0]).getByRole('button', { name: 'Ouvrir rapport.pdf' }));
    expect(backend.called('received_show').at(-1)?.args).toEqual({ path: 'C:/Dl/Escouade/Travail/rapport.pdf', reveal: false });
    expect(backend.called('sent_file_bytes')).toHaveLength(1);
  });

  it('« Voir sur le serveur » shows the file in the panel of the files', async () => {
    fakeBackend({ sent_files: () => views, fs_tree: () => ({ root: '/home/escouade/projets/facturation', entries: [] }) });
    app.agents.a1 = { id: 'a1', projectId: 'p1', cwd: '/home/escouade/projets/facturation' } as any;
    const asked: unknown[] = [];
    const openEditor = app.openEditor.bind(app);
    app.openEditor = (req) => {
      asked.push(req);
      return Promise.resolve();
    };
    try {
      render(SentFiles, { props: { item, agentId: 'a1' } });
      await screen.findAllByTestId('sent-acts');
      await userEvent.click(screen.getAllByRole('button', { name: /Voir sur le serveur/ })[0]);
      expect(asked).toEqual([{ projectId: 'p1', source: 'a1', abs: '/home/escouade/projets/facturation/out/rapport.pdf' }]);
    } finally {
      app.openEditor = openEditor;
    }
  });

  it('on this computer, opens and shows the file where it is, as before', async () => {
    connection.server = null;
    const backend = fakeBackend({ sent_files: () => views });
    app.agents.a1 = { id: 'a1', projectId: 'p1', cwd: 'C:/dev/facturation' } as any;
    render(SentFiles, { props: { item, agentId: 'a1' } });
    await userEvent.click(await screen.findByRole('button', { name: 'Ouvrir rapport.pdf' }));
    expect(backend.called('sent_file_open')).toHaveLength(1);
    expect(screen.queryByText(/Voir sur le serveur/)).toBeNull();
  });
});

describe('the sign-in of an account on a server (S13)', () => {
  const CODEX =
    'Starting local login server on http://localhost:1455.\r\nhttps://auth.openai.com/oauth/authorize?redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback\r\n';

  it('opens the page on this computer at a click, its return on localhost relayed to the server on the same port', async () => {
    const backend = fakeBackend({ server_relay: (a) => ({ outcome: 'ok', value: { local: a.port, remote: a.port } }) });
    render(ServerSignIn, { props: { text: CODEX } });
    expect(screen.getByText(/La page de connexion est prête/)).toBeTruthy();
    // Nothing opened, nothing listened to, before the click.
    expect(backend.called('open_url')).toHaveLength(0);
    expect(backend.called('server_relay')).toHaveLength(0);
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir la page sur ce poste' }));
    expect(backend.called('server_relay')[0].args).toEqual({ port: 1455, same: true });
    expect(backend.called('open_url')[0].args).toEqual({ url: expect.stringMatching(/^https:\/\/auth\.openai\.com/) });
    expect(screen.getByText(/s’est ouverte sur ce poste/)).toBeTruthy();
    expect(screen.getByText(/Le retour par localhost:1455 est relayé à Travail/)).toBeTruthy();
    expect(screen.getByText(/Le compte sera gardé sur Travail/)).toBeTruthy();
  });

  it('says to paste the code when the port is taken here, or when the page gives a code', async () => {
    fakeBackend({ server_relay: () => ({ outcome: 'problem', problem: { kind: 'refused', code: 'relay-busy', message: 'pris' } }) });
    const { unmount } = render(ServerSignIn, { props: { text: CODEX } });
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir la page sur ce poste' }));
    expect(screen.getByText(/Le port 1455 est pris sur ce poste/)).toBeTruthy();
    unmount();
    render(ServerSignIn, { props: { text: 'https://claude.ai/oauth/authorize?code=true' } });
    expect(screen.getByText(/colle-le dans le terminal/)).toBeTruthy();
  });
});

describe('what else the window says on a server', () => {
  it('says the secrets are kept in a file on the server, nothing on this computer', () => {
    const { unmount } = render(SecretsOnServer, { props: { one: true } });
    expect(screen.getByTestId('secrets-in-file').textContent).toMatch(
      /Gardée dans un fichier sur Travail.*Pas dans le trousseau de ce poste/,
    );
    unmount();
    connection.server = null;
    render(SecretsOnServer);
    expect(screen.queryByTestId('secrets-in-file')).toBeNull();
  });

  it('« Reconnecté à Travail · 12 événements rattrapés »', () => {
    fakeBackend();
    connection.state = 'lost';
    connection.lostAt = Date.now();
    render(ConnectionBand);
    connection.missed = 12;
    connection.state = 'open';
    flushSync();
    expect(app.toasts.map((t) => t.text)).toContain('Reconnecté à Travail · 12 événements rattrapés');
  });
});
