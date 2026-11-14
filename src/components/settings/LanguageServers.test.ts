import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import type { LspInfo, LspStatus } from '../../lib/types';
import { fakeBackend, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

// « Serveurs de langage » (Général › Avancé, ESC-446): where each server is, its settings in the draft, the trust a
// server of the project waits for, and the log of those running.

const panel = () => document.getElementById('settings-panel')!;

const status = (over: Partial<LspStatus>): LspStatus => ({
  id: 'rust',
  name: 'Rust (rust-analyzer)',
  languages: ['rust'],
  enabled: true,
  enabledBy: 'default',
  found: null,
  missing: null,
  install: 'rustup component add rust-analyzer',
  args: [],
  ofProject: false,
  runsProjectCode: false,
  needsTrust: false,
  trusted: false,
  ...over,
});

const SERVERS: LspStatus[] = [
  status({ found: { program: '/usr/bin/rust-analyzer', origin: 'path' } }),
  status({
    id: 'typescript',
    name: 'TypeScript, JavaScript (typescript-language-server)',
    found: { program: '/p/node_modules/.bin/typescript-language-server', origin: 'nodeModules' },
    args: ['--stdio'],
    ofProject: true,
    needsTrust: true,
  }),
  status({ id: 'go', name: 'Go (gopls)', missing: { kind: 'absent', install: 'go install golang.org/x/tools/gopls@latest' } }),
];

const RUNNING: LspInfo[] = [
  {
    id: 7,
    project: 'p',
    server: 'rust',
    root: '/p',
    program: '/usr/bin/rust-analyzer',
    state: 'failed',
    run: 6,
    restarts: 5,
    error: 'exit code 101',
    retryInMs: null,
  },
];

async function open() {
  await userEvent.click(within(panel()).getByRole('button', { name: /^Avancé/ }));
  await vi.waitFor(() => expect(panel()).toHaveTextContent('Rust (rust-analyzer)'));
}

describe('the language servers in the settings', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'settings', page: 'general' };
  });

  it('says where each server is, or how to install it, and sets them in the draft', async () => {
    const backend = fakeBackend({ lsp_servers: () => SERVERS, lsp_running: () => [] });
    render(SettingsModal, { page: 'general' });
    await open();
    expect(panel()).toHaveTextContent('Trouvé : /usr/bin/rust-analyzer (PATH)');
    expect(panel()).toHaveTextContent('Introuvable. Pour l’installer : go install golang.org/x/tools/gopls@latest');
    await userEvent.click(screen.getByRole('switch', { name: 'Go (gopls) · Allumé' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Rust (rust-analyzer) · Chemin' }), '/opt/ra');
    await userEvent.type(screen.getByRole('textbox', { name: 'Rust (rust-analyzer) · Arguments' }), '--log  x');
    expect(settingsForm.settings.languageServers).toEqual({
      go: { enabled: false },
      rust: { path: '/opt/ra', args: ['--log', 'x'] },
    });
    expect(settingsForm.changed('general')).toBe(true);
    expect(backend.called('lsp_servers')[0].args).toMatchObject({ agentId: null });
  });

  it('asks to trust the project before its own server runs', async () => {
    let trusted = false;
    const backend = fakeBackend({
      lsp_servers: () => SERVERS.map((s) => (s.ofProject ? { ...s, trusted } : s)),
      lsp_running: () => [],
      lsp_trust: (a: { trusted: boolean }) => {
        trusted = a.trusted;
        return null;
      },
    });
    app.projects = [{ ...(app.projects[0] ?? {}), id: 'p', name: 'p', path: '/p' } as (typeof app.projects)[number]];
    settingsForm.projectId = 'p';
    render(SettingsModal, { page: 'general' });
    await open();
    expect(panel()).toHaveTextContent('Trouvé : /p/node_modules/.bin/typescript-language-server (node_modules du projet)');
    expect(panel()).toHaveTextContent('Ce serveur vient du projet');
    await userEvent.click(within(panel()).getByRole('button', { name: 'Faire confiance au projet' }));
    await vi.waitFor(() => expect(panel()).toHaveTextContent('Tu fais confiance à ce projet pour ses serveurs.'));
    expect(backend.called('lsp_trust').map((c) => c.args)).toEqual([{ projectId: 'p', trusted: true }]);
  });

  it('says that rust-analyzer runs code of the project, and asks the trust for it', async () => {
    const rust = status({ found: { program: '/usr/bin/rust-analyzer', origin: 'path' }, runsProjectCode: true, needsTrust: true });
    const json = status({ id: 'json', name: 'JSON (vscode-json-language-server)', found: { program: '/usr/bin/x', origin: 'path' } });
    const backend = fakeBackend({ lsp_servers: () => [rust, json], lsp_running: () => [], lsp_trust: () => null });
    app.projects = [{ ...(app.projects[0] ?? {}), id: 'p', name: 'p', path: '/p' } as (typeof app.projects)[number]];
    settingsForm.projectId = 'p';
    render(SettingsModal, { page: 'general' });
    await open();
    expect(panel()).toHaveTextContent(
      'Ce serveur exécute du code du projet (build.rs, macros) : il ne se lance dans un projet qu’une fois que tu fais confiance au projet.',
    );
    // Not « vient du projet »: it is the machine's own; the JSON server asks nothing.
    expect(panel()).not.toHaveTextContent('Ce serveur vient du projet');
    expect(within(panel()).getAllByRole('button', { name: 'Faire confiance au projet' })).toHaveLength(1);
    await userEvent.click(within(panel()).getByRole('button', { name: 'Faire confiance au projet' }));
    expect(backend.called('lsp_trust').map((c) => c.args)).toEqual([{ projectId: 'p', trusted: true }]);
  });

  it('shows the log of a server that failed, and restarts it', async () => {
    const backend = fakeBackend({
      lsp_servers: () => SERVERS,
      lsp_running: () => RUNNING,
      lsp_log: () => [{ at: 0, source: 'stderr', text: 'thread main panicked' }],
      lsp_restart: () => null,
    });
    render(SettingsModal, { page: 'general' });
    await open();
    expect(panel()).toHaveTextContent('Arrêté après trop de plantages · exit code 101');
    await userEvent.click(within(panel()).getByRole('button', { name: 'Journal' }));
    await vi.waitFor(() => expect(screen.getByRole('log')).toHaveTextContent('stderr thread main panicked'));
    await userEvent.click(within(panel()).getByRole('button', { name: 'Relancer' }));
    expect(backend.called('lsp_restart').map((c) => c.args)).toEqual([{ id: 7 }]);
  });
});
