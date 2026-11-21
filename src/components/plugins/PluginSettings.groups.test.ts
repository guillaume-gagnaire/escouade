import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { PluginView, SecretState } from '../../lib/plugins';
import { installedPlugin } from '../../test/plugins';
import { fakeBackend, resetApp } from '../../test/ipc';
import PluginSettings from './PluginSettings.svelte';

// Issue #40 (ESC-526): the groups of the manifest, settings and secrets mixed in the order of `fields`, a folded group; a
// secret's description; a field on several lines, numbered; a secret on several lines kept never read back (its lines and
// size, « Remplacer… », « Oublier »); « Importer un fichier… » that reads a file once, its kinds, UTF-8, 64 KiB, its path
// never kept.

const CONF = 'client\ndev tun\nproto udp\nremote vpn.acme.example 1194\n';

function tunnel(extra: Record<string, unknown> = {}): PluginView {
  return {
    ...installedPlugin({
      id: 'dev.test.tunnel',
      version: '1.0.0',
      escouade: { api: '1', min: '2.2.0' },
      permissions: { secrets: ['password', 'ovpn'] },
      contributes: {
        groups: [
          {
            id: 'connection',
            title: { fr: 'Connexion', en: 'Connection' },
            description: { fr: 'Pour tous les projets.', en: 'For every project.' },
            fields: ['server', 'autoStart'],
          },
          { id: 'credentials', title: { fr: 'Identifiants', en: 'Credentials' }, fields: ['login', 'password', 'ovpn'] },
          { id: 'advanced', title: { fr: 'Avancé', en: 'Advanced' }, folded: true, fields: ['mtu', 'notes'] },
        ],
        settings: [
          { key: 'server', type: 'string', title: 'Serveur', description: 'Le nom du serveur.' },
          { key: 'autoStart', type: 'boolean', title: 'Au lancement' },
          { key: 'login', type: 'string', title: 'Identifiant' },
          { key: 'mtu', type: 'number', title: 'MTU' },
          { key: 'notes', type: 'string', title: 'Notes', multiline: true, lines: 4, import: ['.txt'] },
          { key: 'loose', type: 'string', title: 'Hors groupe' },
        ],
        secrets: [
          { key: 'password', title: { fr: 'Mot de passe', en: 'Password' }, description: { fr: 'Celui du VPN.', en: 'The VPN’s.' } },
          {
            key: 'ovpn',
            title: { fr: 'Configuration OpenVPN', en: 'OpenVPN configuration' },
            description: 'Le fichier .ovpn.',
            multiline: true,
            import: ['.ovpn', '.conf'],
          },
        ],
        ...extra,
      },
    }),
    name: 'Tunnel',
    secretKeys: ['password', 'ovpn'],
  };
}

function backend(start: Record<string, Partial<SecretState>> = {}) {
  const states: Record<string, SecretState> = {
    password: { key: 'password', kept: false, inFile: false, ...start.password },
    ovpn: { key: 'ovpn', kept: false, inFile: false, ...start.ovpn },
  };
  return fakeBackend({
    plugin_secrets: () => Object.values(states),
    secrets_file: () => ({ chosen: false, path: '', keychain: true, secrets: [] }),
    plugin_set_secret: (a: { key: string; value: string }) => {
      const lines = a.value.replace(/\n$/, '').split('\n').length;
      states[a.key] = a.value
        ? { key: a.key, kept: true, inFile: false, size: { lines, bytes: new TextEncoder().encode(a.value).length } }
        : { key: a.key, kept: false, inFile: false };
      return null;
    },
  });
}

/** The labels of the rows and groups, in the order of the page. */
const order = (root: HTMLElement) =>
  [...root.querySelectorAll('section > h4, section > .fold .ft, .row .lab b')].map((e) => e.textContent?.trim());

/** Chooses `file` in the field's file input, as the system's dialog does. */
async function choose(container: HTMLElement, key: string, file: File) {
  const input = container.querySelector<HTMLInputElement>(`input.import-file[data-field="${key}"]`)!;
  expect(input, `no file input for ${key}`).toBeTruthy();
  Object.defineProperty(input, 'files', { value: [file], configurable: true });
  await fireEvent.change(input);
  return input;
}

describe('the groups of a plugin’s page', () => {
  beforeEach(() => {
    resetApp();
    setLang('fr');
  });

  it('shows the groups in order, settings and secrets mixed as `fields` says, then what is in no group', async () => {
    backend();
    const { container } = render(PluginSettings, { plugin: tunnel() });
    await screen.findByRole('group', { name: 'Mot de passe' });
    expect(order(container)).toEqual([
      'Connexion',
      'Serveur',
      'Au lancement',
      'Identifiants',
      'Identifiant',
      'Mot de passe',
      'Configuration OpenVPN',
      'Avancé',
      'Réglages',
      'Hors groupe',
    ]);
    const connection = screen.getByRole('region', { name: 'Connexion' });
    expect(connection).toHaveTextContent('Pour tous les projets.');
    // A secret's description, under its title.
    expect(screen.getByRole('group', { name: 'Mot de passe' })).toHaveTextContent('Celui du VPN.');
    // Every secret is in a group: no « Secrets » section, the note still said.
    expect(container.querySelectorAll('section > h4')).toHaveLength(3);
    expect(screen.getByText(/^Gardé dans le trousseau du système/)).toBeInTheDocument();
  });

  it('folds a `folded` group, opened and closed by the keyboard, and names how many fields it hides', async () => {
    backend();
    render(PluginSettings, { plugin: tunnel() });
    const fold = await screen.findByRole('button', { name: /^Avancé/ });
    expect(fold).toHaveAttribute('aria-expanded', 'false');
    expect(fold).toHaveTextContent('Avancé· 2 réglages');
    expect(screen.queryByLabelText('MTU')).toBeNull();
    fold.focus();
    await userEvent.keyboard('{Enter}');
    expect(fold).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByLabelText('MTU')).toBeInTheDocument();
    await userEvent.keyboard(' ');
    expect(fold).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByLabelText('MTU')).toBeNull();
  });

  it('renders a plugin without groups as before: « Réglages » then « Secrets »', async () => {
    backend();
    const plugin = tunnel({ groups: undefined });
    const { container } = render(PluginSettings, { plugin });
    await screen.findByRole('group', { name: 'Mot de passe' });
    expect(order(container)).toEqual([
      'Réglages',
      'Serveur',
      'Au lancement',
      'Identifiant',
      'MTU',
      'Notes',
      'Hors groupe',
      'Secrets',
      'Mot de passe',
      'Configuration OpenVPN',
    ]);
    expect(container.querySelector('.grp, .fold')).toBeNull();
  });

  it('a field on several lines is a numbered text of `lines` lines in the font of fixed width', async () => {
    backend();
    render(PluginSettings, { plugin: tunnel() });
    const row = await screen.findByRole('group', { name: 'Configuration OpenVPN' });
    const area = within(row).getByRole('textbox', { name: 'Configuration OpenVPN' });
    expect(area.tagName).toBe('TEXTAREA');
    expect(area).toHaveAttribute('rows', '6');
    await userEvent.type(area, 'client{Enter}dev tun{Enter}proto udp');
    // The text is seen while typed; the gutter counts at least the lines shown.
    expect(area).toHaveValue('client\ndev tun\nproto udp');
    const gutter = row.querySelector('.gutter')!;
    expect(gutter.getAttribute('aria-hidden')).toBe('true');
    expect(gutter.textContent).toBe('123456');
    // `lines: 4` on a setting.
    await userEvent.click(screen.getByRole('button', { name: /^Avancé/ }));
    expect(screen.getByRole('textbox', { name: 'Notes' })).toHaveAttribute('rows', '4');
  });

  it('keeps a secret on several lines, then never reads it back: its lines and size, « Remplacer… », « Oublier »', async () => {
    const ipc = backend();
    render(PluginSettings, { plugin: tunnel() });
    const row = await screen.findByRole('group', { name: 'Configuration OpenVPN' });
    const area = within(row).getByRole('textbox', { name: 'Configuration OpenVPN' });
    await fireEvent.input(area, { target: { value: CONF } });
    await userEvent.click(within(row).getByRole('button', { name: 'Garder' }));
    expect(ipc.called('plugin_set_secret').at(-1)?.args).toMatchObject({ key: 'ovpn', value: CONF });
    expect(await within(row).findByText('Gardé · 4 lignes · 0,1 Ko')).toBeInTheDocument();
    expect(within(row).queryByRole('textbox')).toBeNull();
    expect(row).not.toHaveTextContent('remote');

    // « Remplacer… »: an empty text, never the one kept; « Annuler » goes back.
    await userEvent.click(within(row).getByRole('button', { name: 'Remplacer…' }));
    expect(within(row).getByRole('textbox', { name: 'Configuration OpenVPN' })).toHaveValue('');
    await userEvent.click(within(row).getByRole('button', { name: 'Annuler' }));
    expect(within(row).getByText('Gardé · 4 lignes · 0,1 Ko')).toBeInTheDocument();

    // « Oublier ».
    await userEvent.click(within(row).getByRole('button', { name: 'Oublier' }));
    expect(ipc.called('plugin_set_secret').at(-1)?.args).toMatchObject({ key: 'ovpn', value: '' });
    expect(await within(row).findByRole('textbox', { name: 'Configuration OpenVPN' })).toBeInTheDocument();
  });

  it('says the size of a kept secret in kilobytes with one decimal', async () => {
    backend({ ovpn: { kept: true, size: { lines: 84, bytes: 5222 } } });
    render(PluginSettings, { plugin: tunnel() });
    expect(await screen.findByText('Gardé · 84 lignes · 5,1 Ko')).toBeInTheDocument();
  });

  it('refuses a text over 64 KiB with a message, nothing kept nor cut', async () => {
    const ipc = backend();
    render(PluginSettings, { plugin: tunnel() });
    const row = await screen.findByRole('group', { name: 'Configuration OpenVPN' });
    const area = within(row).getByRole('textbox', { name: 'Configuration OpenVPN' });
    const big = 'x'.repeat(64 * 1024 + 1);
    await fireEvent.input(area, { target: { value: big } });
    await userEvent.click(within(row).getByRole('button', { name: 'Garder' }));
    expect(await within(row).findByRole('alert')).toHaveTextContent('Plus de 64 Kio : rien n’a été pris, rien n’est coupé.');
    expect(ipc.called('plugin_set_secret')).toHaveLength(0);
    expect(area).toHaveValue(big);
  });
});

describe('« Importer un fichier… »', () => {
  beforeEach(() => {
    resetApp();
    setLang('fr');
  });

  it('opens the dialog filtered on its kinds, reads the file once, fills the field, and keeps nothing of where it was', async () => {
    const ipc = backend();
    const { container } = render(PluginSettings, { plugin: tunnel() });
    const row = await screen.findByRole('group', { name: 'Configuration OpenVPN' });
    const input = container.querySelector<HTMLInputElement>('input.import-file[data-field="ovpn"]')!;
    expect(input).toHaveAttribute('accept', '.ovpn,.conf');
    let opened = 0;
    input.addEventListener('click', (e) => {
      opened++;
      e.preventDefault();
    });
    await userEvent.click(within(row).getByRole('button', { name: 'Importer un fichier…' }));
    expect(opened).toBe(1);

    let reads = 0;
    const file = new File([CONF], 'client-secret-name.ovpn', { type: 'application/x-openvpn-profile' });
    file.arrayBuffer = () => {
      reads++;
      return new Promise((resolve) => {
        const r = new FileReader();
        r.onload = () => resolve(r.result as ArrayBuffer);
        r.readAsArrayBuffer(file);
      });
    };
    const chosen = await choose(container, 'ovpn', file);
    const area = await within(row).findByRole('textbox', { name: 'Configuration OpenVPN' });
    await expect.poll(() => (area as HTMLTextAreaElement).value).toBe(CONF);
    expect(reads).toBe(1);
    // The input is emptied at once: it holds no file any more.
    expect(chosen.value).toBe('');
    // Nothing of the file's name anywhere in the page, nor sent.
    expect(container.innerHTML).not.toContain('client-secret-name');
    await userEvent.click(within(row).getByRole('button', { name: 'Garder' }));
    const sent = ipc.calls.map((c) => JSON.stringify(c.args)).join('\n');
    expect(sent).toContain('remote vpn.acme.example 1194');
    expect(sent).not.toContain('client-secret-name');
    expect(sent).not.toContain('.ovpn"');
    // No command beyond keeping the secret: the plugin gains no right on the files.
    expect(new Set(ipc.calls.map((c) => c.cmd))).toEqual(new Set(['plugin_secrets', 'secrets_file', 'plugin_set_secret']));
  });

  it('refuses a file of another kind, one over 64 KiB, and one that is not UTF-8 text, each with its message', async () => {
    backend();
    const { container } = render(PluginSettings, { plugin: tunnel() });
    const row = await screen.findByRole('group', { name: 'Configuration OpenVPN' });
    const area = within(row).getByRole('textbox', { name: 'Configuration OpenVPN' });

    await choose(container, 'ovpn', new File(['x'], 'cle.pem'));
    expect(await within(row).findByRole('alert')).toHaveTextContent('Ce fichier n’est pas d’un genre attendu (.ovpn, .conf)');

    await choose(container, 'ovpn', new File(['x'.repeat(64 * 1024 + 1)], 'gros.ovpn'));
    await expect.poll(() => within(row).getByRole('alert').textContent).toContain('Plus de 64 Kio');

    await choose(container, 'ovpn', new File([new Uint8Array([0xff, 0xfe, 0x00, 0x41])], 'binaire.conf'));
    await expect.poll(() => within(row).getByRole('alert').textContent).toContain('n’est pas du texte en UTF-8');

    await choose(container, 'ovpn', new File(['a\u0000b'], 'nul.conf'));
    await expect.poll(() => within(row).getByRole('alert').textContent).toContain('n’est pas du texte en UTF-8');
    expect(area).toHaveValue('');

    // Exactly 64 KiB, and an extension in capitals: taken.
    await choose(container, 'ovpn', new File(['y'.repeat(64 * 1024)], 'PROFIL.OVPN'));
    await expect.poll(() => (area as HTMLTextAreaElement).value.length).toBe(64 * 1024);
    expect(within(row).queryByRole('alert')).toBeNull();
  });

  it('imports into a setting on several lines, kept as the setting', async () => {
    const ipc = backend();
    const { container } = render(PluginSettings, { plugin: tunnel() });
    await userEvent.click(await screen.findByRole('button', { name: /^Avancé/ }));
    await choose(container, 'notes', new File(['une\ndeux'], 'notes.txt'));
    await expect
      .poll(() => ipc.called('plugin_set_setting').at(-1)?.args)
      .toEqual({
        id: 'dev.test.tunnel',
        key: 'notes',
        value: 'une\ndeux',
      });
  });

  it('speaks English too', async () => {
    setLang('en');
    backend({ ovpn: { kept: true, size: { lines: 84, bytes: 5222 } } });
    render(PluginSettings, { plugin: tunnel() });
    expect(await screen.findByText('Kept · 84 lines · 5.1 KB')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Replace…' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Credentials' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^Advanced/ })).toHaveTextContent('· 2 settings');
    await userEvent.click(screen.getByRole('button', { name: 'Replace…' }));
    expect(screen.getByRole('button', { name: 'Import a file…' })).toBeInTheDocument();
  });
});
