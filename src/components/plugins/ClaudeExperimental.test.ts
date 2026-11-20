import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import manifest from '../../../plugins/claude/escouade-plugin.json';
import { setLang } from '../../lib/i18n';
import type { PluginView } from '../../lib/plugins';
import { plugins } from '../../lib/plugins-view.svelte';
import { installedPlugin } from '../../test/plugins';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import PluginSettings from './PluginSettings.svelte';

// « Fonctionnalités expérimentales » of the plugin Claude Code (ESC-553), as its manifest declares them: Code Mode and the
// Agent Teams with their version and where they work, other names of CLAUDE_CODE_EXPERIMENTAL_*, for every project on the
// plugin's page, then for each project on its page Plugins.

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

function claude(over: Partial<PluginView['choices']> = {}): PluginView {
  const p = installedPlugin(manifest);
  return { ...p, name: 'Claude Code', choices: { ...p.choices, ...over } };
}

beforeEach(() => {
  setLang('fr');
  plugins.list = [];
  plugins.loaded = false;
});

describe('the experimental features of Claude Code', () => {
  it('are a group of the plugin’s page: Code Mode and the Agent Teams with their version and surfaces, and the others', async () => {
    resetApp();
    const view = claude();
    const ipc = fakeBackend({ plugin_set_setting: () => view, plugin_secrets: () => [] });
    render(PluginSettings, { plugin: view });
    const group = screen.getByRole('region', { name: 'Fonctionnalités expérimentales' });
    expect(group).toHaveTextContent('Les drapeaux expérimentaux de Claude Code, pour tous les projets.');
    const code = within(group).getByRole('switch', { name: 'Code Mode' });
    expect(code).toHaveAttribute('aria-checked', 'false');
    expect(group).toHaveTextContent('Claude Code 2.1.323 ou plus (2.1.326 avec les Agent Teams)');
    expect(group).toHaveTextContent('Passé à la vue terminal seulement : jamais à la conversation ni à un agent de ticket.');
    expect(group).toHaveTextContent('ne se reprend plus que dans son terminal');
    expect(group).toHaveTextContent('Claude Code 2.1.32 ou plus.');
    expect(within(group).getByRole('switch', { name: 'Agent Teams' })).toHaveAttribute('aria-checked', 'false');
    expect(group).toHaveTextContent('Des noms de CLAUDE_CODE_EXPERIMENTAL_* seulement');
    await userEvent.click(code);
    expect(ipc.called('plugin_set_setting').at(-1)!.args).toMatchObject({
      id: 'dev.escouade.claude-code',
      key: 'experimental.codeMode',
      value: true,
    });
    const others = within(group).getByLabelText('Autres drapeaux');
    await userEvent.type(others, 'CLAUDE_CODE_EXPERIMENTAL_FOO');
    await userEvent.tab();
    expect(ipc.called('plugin_set_setting').at(-1)!.args).toMatchObject({
      key: 'experimental.others',
      value: 'CLAUDE_CODE_EXPERIMENTAL_FOO',
    });
    // The settings of each project are not on this page.
    expect(screen.queryByRole('combobox')).toBeNull();
  });

  it('says them in English too', () => {
    setLang('en');
    resetApp();
    fakeBackend({ plugin_secrets: () => [] });
    render(PluginSettings, { plugin: claude({ settings: { 'experimental.agentTeams': true } }) });
    const group = screen.getByRole('region', { name: 'Experimental features' });
    expect(within(group).getByRole('switch', { name: 'Agent Teams' })).toHaveAttribute('aria-checked', 'true');
    expect(group).toHaveTextContent('Passed to the terminal view alone: never to the conversation nor to a ticket’s agent.');
  });

  it('are set otherwise in a project, on its page Plugins, never by its .escouade.yml', async () => {
    resetApp({ projects: [project({ id: 'atlas', name: 'atlas-web' })] });
    const view = claude({ projectSettings: { atlas: { 'experimental.project.codeMode': 'off' } } });
    const ipc = fakeBackend({ plugins_list: () => [view], plugin_project_set: () => view });
    render(SettingsModal, { page: 'projectPlugins', projectId: 'atlas' });
    const group = await screen.findByRole('region', { name: 'Claude Code' });
    const code = within(group).getByRole('group', { name: 'Code Mode' });
    expect(code).toHaveTextContent('Dans la vue terminal des agents de ce projet.');
    expect(code).toHaveTextContent('Gardé sur cette machine, jamais dans .escouade.yml.');
    const select = within(code).getByLabelText('Code Mode') as HTMLSelectElement;
    expect(select.value).toBe('off');
    expect([...select.options].map((o) => o.text)).toEqual(['Comme pour tous les projets', 'Activé', 'Désactivé']);
    await userEvent.selectOptions(select, 'on');
    expect(ipc.called('plugin_project_set').at(-1)!.args).toMatchObject({
      id: 'dev.escouade.claude-code',
      project: 'atlas',
      key: 'experimental.project.codeMode',
      value: 'on',
    });
    expect(within(group).getByRole('group', { name: 'Agent Teams' })).toBeInTheDocument();
    expect(within(group).getByRole('group', { name: 'Autres drapeaux' })).toHaveTextContent(
      'vide : ceux de tous les projets ; aucun : aucun',
    );
    expect(within(group).queryByRole('button', { name: 'Écrire dans .escouade.yml' })).toBeNull();
  });
});
