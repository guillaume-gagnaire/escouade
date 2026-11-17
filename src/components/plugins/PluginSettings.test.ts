import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { PluginView, SecretState, SecretsFile } from '../../lib/plugins';
import { installedPlugin } from '../../test/plugins';
import { fakeBackend, resetApp } from '../../test/ipc';
import PluginSettings from './PluginSettings.svelte';

// A plugin's secrets (issue #37, ESC-499): « Garder » that kept nothing says why beside its field, never in silence; where the
// keychain refused, the field offers the file readable by the user alone, kept once chosen; a secret in the file, once a keychain
// is there, offers to move to it.

const PATH = '/home/ada/.escouade/plugin-secrets.json';
const REFUSED = { message: 'Pas de trousseau système sur ce poste : le secret n’a pas été gardé.', keychainRefused: true };

function vpn(): PluginView {
  return {
    ...installedPlugin({
      id: 'dev.test.vpn',
      version: '1.0.0',
      escouade: { api: '1', min: '2.2.0' },
      permissions: { secrets: ['password'] },
      contributes: { secrets: [{ key: 'password', title: { fr: 'Mot de passe', en: 'Password' } }] },
    }),
    name: 'VPN',
    secretKeys: ['password'],
  };
}

/** The backend of a machine without a keychain (`keychain`: one there), the secret kept where the commands put it. */
function backend(keychain = false, start: Partial<SecretState> = {}) {
  let state: SecretState = { key: 'password', kept: false, inFile: false, ...start };
  const file = (): SecretsFile => ({
    chosen: state.inFile,
    path: PATH,
    keychain,
    secrets: state.inFile ? [{ plugin: 'dev.test.vpn', pluginName: 'VPN', key: 'password' }] : [],
  });
  return fakeBackend({
    plugin_secrets: () => [state],
    secrets_file: file,
    plugin_set_secret: (a: { value: string; inFile: boolean }) => {
      if (!a.value) state = { ...state, kept: false, inFile: false };
      else if (a.inFile) state = { ...state, kept: true, inFile: true };
      else if (!keychain) throw REFUSED;
      else state = { ...state, kept: true };
      return null;
    },
    plugin_secret_move: () => {
      state = { ...state, inFile: false };
      return null;
    },
  });
}

describe('a plugin’s secrets', () => {
  beforeEach(() => {
    resetApp();
    setLang('fr');
  });

  it('says beside its field why « Garder » kept nothing, and offers the file, kept once chosen', async () => {
    const ipc = backend();
    render(PluginSettings, { plugin: vpn() });
    const row = await screen.findByRole('group', { name: 'Mot de passe' });
    // The note says the truth of this machine: no keychain.
    expect(await screen.findByText(/^Ce poste n’a pas de trousseau système/)).toBeInTheDocument();
    await userEvent.type(within(row).getByLabelText('Mot de passe'), 'hunter2');
    await userEvent.click(within(row).getByRole('button', { name: 'Garder' }));

    // Beside the field, with its cause; the line still says none is kept.
    expect(await within(row).findByRole('alert')).toHaveTextContent(REFUSED.message);
    expect(row).toHaveTextContent('aucun');
    // The file offered, with what it means; nothing taken by itself.
    const offer = within(row).getByRole('button', { name: 'Garder dans un fichier lisible de toi seul' });
    expect(row).toHaveTextContent(`en clair sur le disque, dans ${PATH}, avec les droits de ton seul compte`);
    expect(ipc.called('plugin_set_secret').map((c) => c.args.inFile)).toEqual([false]);

    // Chosen: kept in the file, said so, the error gone.
    await userEvent.click(offer);
    expect(ipc.called('plugin_set_secret').at(-1)?.args).toMatchObject({
      id: 'dev.test.vpn',
      key: 'password',
      value: 'hunter2',
      inFile: true,
    });
    expect(await within(row).findByText('gardé dans un fichier lisible de toi seul')).toBeInTheDocument();
    expect(within(row).queryByRole('alert')).toBeNull();
    expect(within(row).queryByRole('button', { name: 'Garder dans un fichier lisible de toi seul' })).toBeNull();
    // No keychain to move it to: not offered.
    expect(within(row).queryByRole('button', { name: 'Déplacer vers le trousseau' })).toBeNull();
  });

  it('keeps in the keychain where there is one, the file never offered', async () => {
    const ipc = backend(true);
    render(PluginSettings, { plugin: vpn() });
    const row = await screen.findByRole('group', { name: 'Mot de passe' });
    await userEvent.type(within(row).getByLabelText('Mot de passe'), 'hunter2');
    await userEvent.click(within(row).getByRole('button', { name: 'Garder' }));
    expect(await within(row).findByText('gardé dans le trousseau')).toBeInTheDocument();
    expect(within(row).queryByRole('alert')).toBeNull();
    expect(within(row).queryByRole('button', { name: 'Garder dans un fichier lisible de toi seul' })).toBeNull();
    expect(ipc.called('plugin_set_secret')).toHaveLength(1);
  });

  it('offers to move a secret of the file to a keychain that came since, and moves nothing by itself', async () => {
    const ipc = backend(true, { kept: true, inFile: true });
    render(PluginSettings, { plugin: vpn() });
    const row = await screen.findByRole('group', { name: 'Mot de passe' });
    expect(await within(row).findByText('gardé dans un fichier lisible de toi seul')).toBeInTheDocument();
    expect(ipc.called('plugin_secret_move')).toHaveLength(0);
    await userEvent.click(within(row).getByRole('button', { name: 'Déplacer vers le trousseau' }));
    expect(ipc.called('plugin_secret_move')[0].args).toEqual({ id: 'dev.test.vpn', key: 'password' });
    expect(await within(row).findByText('gardé dans le trousseau')).toBeInTheDocument();
  });

  it('says the cause in English too', async () => {
    setLang('en');
    backend();
    render(PluginSettings, { plugin: vpn() });
    const row = await screen.findByRole('group', { name: 'Password' });
    await userEvent.type(within(row).getByLabelText('Password'), 'hunter2');
    await userEvent.click(within(row).getByRole('button', { name: 'Keep' }));
    expect(await within(row).findByRole('alert')).toBeInTheDocument();
    expect(within(row).getByRole('button', { name: 'Keep in a file only you can read' })).toBeInTheDocument();
    expect(row).toHaveTextContent(`in clear on the disk, in ${PATH}`);
  });
});
