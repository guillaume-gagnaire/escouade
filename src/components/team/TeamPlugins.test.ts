import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { teams } from '../../lib/teams.svelte';
import { namespaceOk, rowAction, type TeamPluginRow, type TeamPluginsView } from '../../lib/team-plugins';
import { fakeBackend, resetApp } from '../../test/ipc';
import TeamPlugins from './TeamPlugins.svelte';
import TeamStore from '../plugins/TeamStore.svelte';

// The team's private store (TS-14): the key and its namespace made by an administrator, a plugin proposed after its
// checks, signed after the same checks again, withdrawn; in the view Plugins, « Acme · privé » and « Installer », which
// opens the window of its permissions (the member's gesture).

const KEY = { namespace: 'fr.acme.*', ids: ['AAAA1111BBBB2222', 'CCCC3333DDDD4444'], revoked: [], by: '11', at: 1, here: true };

function row(over: Partial<TeamPluginRow> = {}): TeamPluginRow {
  return {
    id: 'fr.acme.rules',
    name: 'Règles d’Acme',
    description: 'Les règles de l’équipe',
    version: '1.0.0',
    by: '11',
    signedAt: 1_800_000_000,
    digest: 'ab'.repeat(32),
    installed: null,
    elsewhere: false,
    withdrawn: null,
    ...over,
  };
}

function view(over: Partial<TeamPluginsView> = {}): TeamPluginsView {
  return {
    team: 'acme',
    name: 'ACME',
    admin: true,
    key: KEY,
    conflict: null,
    plugins: [],
    proposed: [],
    serial: null,
    refused: 0,
    alarm: false,
    ...over,
  };
}

const CHECKS = ['archive', 'manifest', 'version', 'permissions', 'binaries', 'compile'].map((key) => ({ key, ok: true, detail: 'ok' }));

describe('the namespace and the lines of the team’s store', () => {
  it('takes a reversed domain of two parts, never Escouade’s own', () => {
    expect(namespaceOk('fr.acme.*')).toBe('ok');
    expect(namespaceOk('com.acme.tools.*')).toBe('ok');
    expect(namespaceOk('fr.acme')).toBe('form');
    expect(namespaceOk('acme.*')).toBe('form');
    expect(namespaceOk('fr..acme.*')).toBe('form');
    expect(namespaceOk('dev.escouade.*')).toBe('firstParty');
    expect(namespaceOk('dev.*')).toBe('firstParty');
  });

  it('offers to install, to update, or nothing', () => {
    expect(rowAction(row())).toBe('install');
    expect(rowAction(row({ installed: '0.9.0' }))).toBe('update');
    expect(rowAction(row({ installed: '1.0.0' }))).toBe('installed');
    expect(rowAction(row({ withdrawn: 'Fuite' }))).toBe('withdrawn');
    expect(rowAction(row({ elsewhere: true }))).toBe('elsewhere');
  });
});

describe('« Plugins de l’équipe » (TS-14)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    teams.list = [];
    app.modal = { kind: 'team', step: 'plugins', teamId: 'acme' };
  });

  it('lets an administrator make the key once the namespace and both passwords hold', async () => {
    const be = fakeBackend({
      team_plugins: () => view({ key: null }),
      team_plugin_key_create: () => ({ view: view(), backup: 'E:\\usb\\escouade-team-X-backup.key' }),
    });
    render(TeamPlugins, { teamId: 'acme', onback: () => {} });
    const form = await screen.findByTestId('team-plugin-key-form');
    const create = within(form).getByRole('button', { name: 'Créer la clé et déclarer l’espace' });
    expect(create).toBeDisabled();
    await userEvent.type(within(form).getByLabelText('Espace de noms'), 'dev.escouade.*');
    expect(within(form).getByRole('alert')).toHaveTextContent('dev.escouade.* est à Escouade');
    await userEvent.clear(within(form).getByLabelText('Espace de noms'));
    await userEvent.type(within(form).getByLabelText('Espace de noms'), 'fr.acme.*');
    await userEvent.type(within(form).getByLabelText('Mot de passe'), 'un mot de passe long');
    await userEvent.type(within(form).getByLabelText('Le même, encore'), 'un autre mot de passe');
    expect(within(form).getByText('Les deux mots de passe diffèrent.')).toBeInTheDocument();
    expect(create).toBeDisabled();
    await userEvent.clear(within(form).getByLabelText('Le même, encore'));
    await userEvent.type(within(form).getByLabelText('Le même, encore'), 'un mot de passe long');
    await userEvent.type(within(form).getByLabelText('Dossier de la clé de secours'), 'E:\\usb');
    expect(create).toBeEnabled();
    await userEvent.click(create);
    await waitFor(() =>
      expect(be.called('team_plugin_key_create')[0]?.args).toEqual({
        team: 'acme',
        namespace: 'fr.acme.*',
        password: 'un mot de passe long',
        backupDir: 'E:\\usb',
      }),
    );
    expect(await screen.findByText(/La clé de secours est dans E:\\usb/)).toBeInTheDocument();
    expect(screen.getByTestId('team-plugin-key')).toHaveTextContent('Espace fr.acme.*');
  });

  it('tells a member the team has no key, and never shows him the form', async () => {
    fakeBackend({ team_plugins: () => view({ admin: false, key: null }) });
    render(TeamPlugins, { teamId: 'acme', onback: () => {} });
    expect(await screen.findByText('L’équipe n’a pas encore de clé : un administrateur la crée sur son poste.')).toBeInTheDocument();
    expect(screen.queryByTestId('team-plugin-key-form')).toBeNull();
  });

  it('proposes a plugin after its checks, and says the one that failed', async () => {
    const failed = CHECKS.map((c) => (c.key === 'version' ? { ...c, ok: false, detail: 'l’étiquette v2 n’est pas la version 1.0.0' } : c));
    let answer: unknown = {
      report: { id: 'fr.acme.rules', version: '1.0.0', digest: 'cd'.repeat(32), checks: failed, ok: false },
      proposal: null,
      view: null,
    };
    const be = fakeBackend({ team_plugins: () => view(), team_plugin_submit: () => answer });
    render(TeamPlugins, { teamId: 'acme', onback: () => {} });
    await userEvent.type(await screen.findByLabelText('Dossier du plugin'), 'C:\\src\\rules');
    await userEvent.type(screen.getByPlaceholderText('Étiquette'), 'v2');
    await userEvent.click(screen.getByRole('button', { name: 'Contrôler et proposer' }));
    expect(await screen.findByText('Un contrôle échoue : rien n’est envoyé.')).toBeInTheDocument();
    const bad = screen.getAllByTestId('team-plugin-check').filter((li) => li.dataset.ok === 'false');
    expect(bad).toHaveLength(1);
    expect(bad[0]).toHaveTextContent('Version');
    answer = {
      report: { id: 'fr.acme.rules', version: '1.0.0', digest: 'cd'.repeat(32), checks: CHECKS, ok: true },
      proposal: 'ee'.repeat(16),
      view: view({
        proposed: [
          {
            id: 'ee'.repeat(16),
            plugin: 'fr.acme.rules',
            name: 'Règles',
            version: '1.0.0',
            tag: 'v1.0.0',
            by: '22',
            mine: true,
            at: 1,
            digest: 'cd'.repeat(32),
            checks: CHECKS,
          },
        ],
      }),
    };
    await userEvent.click(screen.getByRole('button', { name: 'Contrôler et proposer' }));
    expect(await screen.findByText(/Proposé : fr.acme.rules v1.0.0, empreinte cdcdcdcdcdcd/)).toBeInTheDocument();
    expect(be.called('team_plugin_submit')[1].args).toEqual({ team: 'acme', folder: 'C:\\src\\rules', tag: 'v2' });
    expect(screen.getByTestId('team-plugin-proposed')).toHaveTextContent('proposé par toi');
  });

  it('signs a proposal with the key’s password, after the checks played again here', async () => {
    const proposal = {
      id: 'ee'.repeat(16),
      plugin: 'fr.acme.rules',
      name: 'Règles',
      version: '1.0.0',
      tag: 'v1.0.0',
      by: '22',
      mine: false,
      at: 1,
      digest: 'cd'.repeat(32),
      checks: CHECKS,
    };
    const be = fakeBackend({
      team_plugins: () => view({ proposed: [proposal] }),
      team_plugin_sign: () => ({ report: { digest: 'cd'.repeat(32), checks: CHECKS, ok: true }, view: view({ plugins: [row()] }) }),
    });
    render(TeamPlugins, { teamId: 'acme', onback: () => {} });
    const item = await screen.findByTestId('team-plugin-proposed');
    await userEvent.click(within(item).getByRole('button', { name: 'Contrôler à nouveau et signer' }));
    await userEvent.type(within(item).getByLabelText('Mot de passe'), 'un mot de passe long');
    await userEvent.click(within(item).getByRole('button', { name: 'Contrôler à nouveau et signer' }));
    await waitFor(() =>
      expect(be.called('team_plugin_sign')[0]?.args).toEqual({ team: 'acme', proposal: proposal.id, password: 'un mot de passe long' }),
    );
    expect(await screen.findByText(/Signé : les deux empreintes sont les mêmes/)).toBeInTheDocument();
    expect(screen.getByTestId('team-plugin-published')).toHaveTextContent('ACME · privé');
  });

  it('withdraws a version by the team’s list, its reason and the key’s password', async () => {
    const be = fakeBackend({
      team_plugins: () => view({ plugins: [row()] }),
      team_plugin_revoke: () => view({ plugins: [row({ withdrawn: 'Fuite' })], serial: 1 }),
    });
    render(TeamPlugins, { teamId: 'acme', onback: () => {} });
    const item = await screen.findByTestId('team-plugin-published');
    await userEvent.click(within(item).getByRole('button', { name: 'Retirer…' }));
    await userEvent.type(within(item).getByLabelText('Raison'), 'Fuite');
    await userEvent.type(within(item).getByLabelText('Mot de passe'), 'un mot de passe long');
    await userEvent.click(within(item).getByRole('button', { name: 'Retirer cette version' }));
    await waitFor(() =>
      expect(be.called('team_plugin_revoke')[0]?.args).toEqual({
        team: 'acme',
        plugin: 'fr.acme.rules',
        versions: ['1.0.0'],
        reason: 'Fuite',
        password: 'un mot de passe long',
        revokeKey: null,
      }),
    );
    expect(await screen.findByText('Retirée : Fuite')).toBeInTheDocument();
    expect(screen.getByText('Liste de révocation de l’équipe : n° 1')).toBeInTheDocument();
  });
});

describe('« Équipe » in the view Plugins (TS-14)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
  });

  it('shows « ACME · privé » and opens the window of its permissions on « Installer »', async () => {
    const be = fakeBackend({
      team_plugins_all: () => [view({ admin: false, plugins: [row(), row({ id: 'fr.acme.lint', name: 'Lint', withdrawn: 'Retirée' })] })],
      team_plugin_inspect: () => {
        throw { message: 'refusée pour le test', problems: [] };
      },
    });
    render(TeamStore);
    const store = await screen.findByTestId('team-store');
    expect(within(store).getByRole('heading', { name: 'ACME · privé' })).toBeInTheDocument();
    const lines = within(store).getAllByTestId('team-store-plugin');
    expect(lines[1]).toHaveTextContent('Retiré par l’équipe : Retirée');
    expect(within(lines[1]).queryByRole('button', { name: 'Installer' })).toBeNull();
    await userEvent.click(within(lines[0]).getByRole('button', { name: 'Installer' }));
    await waitFor(() => expect(be.called('team_plugin_inspect')[0]?.args).toEqual({ team: 'acme', id: 'fr.acme.rules', version: '1.0.0' }));
    expect(be.called('plugin_install')).toHaveLength(0);
  });
});
