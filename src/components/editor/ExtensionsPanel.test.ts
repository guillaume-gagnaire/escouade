import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { ExtState, Installed } from '../../lib/vscode/extensions';
import { fakeBackend, resetApp } from '../../test/ipc';
import ExtensionsPanel from './ExtensionsPanel.svelte';

function ext(over: Partial<ExtState> = {}): ExtState {
  return {
    id: 'acme.night',
    key: 'acme.night',
    namespace: 'acme',
    name: 'night',
    version: '1.0.0',
    displayName: 'Acme Night',
    description: 'A theme',
    publisher: 'Acme Inc.',
    kind: 'declarative',
    enabled: true,
    verified: true,
    preRelease: false,
    sha256: 'ab'.repeat(32),
    signed: true,
    installedAt: 0,
    files: 3,
    bytes: 1200,
    contributes: { themes: ['Acme Night'], iconThemes: [], languages: [], grammars: 0, snippets: 0 },
    dir: 'acme.night-1.0.0',
    refused: null,
    waits: null,
    ...over,
  };
}

const none = (over: Partial<Installed> = {}): Installed => ({
  extensions: [],
  theme: null,
  layers: [],
  warnings: [],
  private: null,
  ...over,
});

const card = (over: Record<string, unknown> = {}) => ({
  detail: {
    id: 'acme.night',
    namespace: 'acme',
    name: 'night',
    version: '1.0.0',
    displayName: 'Acme Night',
    description: 'A dark theme',
    verified: false,
    icon: 'https://open-vsx.org/icon.png',
    downloads: 1234,
    deprecated: false,
    publisher: 'Acme Inc.',
    license: 'MIT',
    repository: '',
    homepage: '',
    preRelease: false,
    downloadable: true,
    timestamp: '',
    versions: ['1.0.0'],
    categories: [],
  },
  readme: '# Acme Night\n\n![shot](https://example.com/shot.png)\n',
  refused: null,
  installed: null,
  ...over,
});

const ok = <T>(value: T) => ({ value, refusal: null });
const no = (code: string, message: string) => ({ value: null, refusal: { code, message } });

describe('ExtensionsPanel', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
  });

  function mount(handlers: Record<string, (a: any) => unknown>) {
    const b = fakeBackend({ vsx_installed: () => none(), ...handlers });
    const onlaid = vi.fn();
    render(ExtensionsPanel, { projects: ['p1'], onclose: () => {}, onlaid });
    return { b, onlaid };
  }

  it('searches through the core, shows a card with its namespace not verified, asks to trust its publisher, then installs', async () => {
    let installed = none();
    let asked = 0;
    const { b, onlaid } = mount({
      vsx_installed: () => installed,
      vsx_search: () => ok({ extensions: [{ ...card().detail }], total: 1 }),
      vsx_card: () => ok(card()),
      vsx_image: () => ok('data:image/png;base64,AAAA'),
      vsx_install: (a) => {
        asked++;
        if (!a.trust) return no('untrusted', 'Faites-vous confiance à l’éditeur « Acme Inc. » ?');
        installed = none({ extensions: [ext()] });
        return ok(ext());
      },
    });
    await waitFor(() => expect(onlaid).toHaveBeenCalledTimes(1));
    await userEvent.type(screen.getByTestId('vscode-ext-search'), 'night{Enter}');
    expect(b.called('vsx_search')[0].args).toEqual({ projects: ['p1'], query: 'night', offset: 0 });
    await userEvent.click(await screen.findByTestId('vscode-ext-result'));
    const c = await screen.findByTestId('vscode-ext-card');
    expect(within(c).getByTestId('vscode-ext-unverified')).toHaveTextContent('Non vérifié');
    expect(within(c).getByText('par Acme Inc.')).toBeTruthy();
    // The icon by the core; the README's remote image not loaded without a click.
    await waitFor(() => expect(c.querySelector('.logo img')).toHaveAttribute('src', 'data:image/png;base64,AAAA'));
    expect(b.called('vsx_image')[0].args.url).toBe('https://open-vsx.org/icon.png');
    expect(within(c).getByTestId('vscode-ext-readme').querySelector('img')).toBeNull();
    expect(within(c).getByTestId('vscode-ext-readme').querySelector('[data-remote-src="https://example.com/shot.png"]')).toBeTruthy();
    await userEvent.click(within(c).getByTestId('vscode-ext-install'));
    const trust = await screen.findByTestId('vscode-ext-trust');
    expect(trust).toHaveTextContent('Faites-vous confiance à l’éditeur « Acme Inc. » ?');
    expect(trust).toHaveTextContent('Open VSX n’a pas vérifié son espace de noms « acme »');
    await userEvent.click(within(trust).getByTestId('vscode-ext-trust-yes'));
    await waitFor(() => expect(screen.queryByTestId('vscode-ext-trust')).toBeNull());
    expect(b.called('vsx_install').map((c) => c.args.trust)).toEqual([false, true]);
    expect(asked).toBe(2);
    // Laid in the view: the panel tells it the new list.
    await waitFor(() => expect(onlaid).toHaveBeenLastCalledWith(installed));
  });

  it('says the refusals of the core: outside the allowed list (install off), a false signature', async () => {
    mount({
      vsx_search: () => ok({ extensions: [{ ...card().detail }], total: 1 }),
      vsx_card: () => ok(card({ refused: 'L’extension « acme.night » n’est pas permise par la stratégie de ce poste.' })),
    });
    await userEvent.type(screen.getByTestId('vscode-ext-search'), 'night{Enter}');
    await userEvent.click(await screen.findByTestId('vscode-ext-result'));
    expect(await screen.findByTestId('vscode-ext-card-refused')).toHaveTextContent('n’est pas permise par la stratégie de ce poste');
    expect(screen.getByTestId('vscode-ext-install')).toBeDisabled();
  });

  it('says a false signature as the core says it', async () => {
    mount({
      vsx_search: () => ok({ extensions: [{ ...card().detail, verified: true }], total: 1 }),
      vsx_card: () => ok(card({ detail: { ...card().detail, verified: true } })),
      vsx_install: () => no('badSignature', 'La signature d’Open VSX ne correspond pas à l’archive : elle n’est pas installée.'),
    });
    await userEvent.type(screen.getByTestId('vscode-ext-search'), 'night{Enter}');
    await userEvent.click(await screen.findByTestId('vscode-ext-result'));
    expect(await screen.findByTestId('vscode-ext-verified')).toHaveTextContent('Vérifié');
    await userEvent.click(screen.getByTestId('vscode-ext-install'));
    const r = await screen.findByTestId('vscode-ext-refusal');
    expect(r).toHaveAttribute('data-code', 'badSignature');
    expect(r).toHaveTextContent('La signature d’Open VSX ne correspond pas');
  });

  it('shows what each installed extension is: on, off, waiting for its host, needing Node, refused', async () => {
    mount({
      vsx_installed: () =>
        none({
          extensions: [
            ext(),
            ext({ id: 'a.off', key: 'a.off', displayName: 'Off', enabled: false }),
            ext({
              id: 'a.vim',
              key: 'a.vim',
              displayName: 'Vim',
              kind: 'web',
              waits: 'En attente de l’hôte d’extensions : son code ne tourne pas encore dans Escouade.',
            }),
            ext({
              id: 'a.lint',
              key: 'a.lint',
              displayName: 'Lint',
              kind: 'node',
              waits: 'Demande l’hôte Node, pas encore dans Escouade.',
            }),
            ext({
              id: 'a.no',
              key: 'a.no',
              displayName: 'No',
              refused: 'L’extension « a.no » n’est pas permise par le .escouade.yml de demo-api.',
            }),
          ],
          layers: [{ by: 'par l’équipe Acme', imposed: true, rules: {} }],
        }),
    });
    await waitFor(() => expect(screen.getAllByTestId('vscode-ext-item')).toHaveLength(5));
    const statuses = screen
      .getAllByTestId('vscode-ext-item')
      .map((i) => [i.dataset.id, i.dataset.status, i.querySelector('.chip')?.textContent]);
    expect(statuses).toEqual([
      ['acme.night', 'active', 'Active'],
      ['a.off', 'off', 'Coupée'],
      ['a.vim', 'waitsHost', 'En attente de l’hôte'],
      ['a.lint', 'needsNode', 'Demande Node'],
      ['a.no', 'refused', 'Non permise'],
    ]);
    expect(screen.getByText(/n’est pas permise par le .escouade.yml de demo-api/)).toBeTruthy();
    expect(screen.getByTestId('vscode-ext-imposed')).toHaveTextContent('Liste permise imposée par l’équipe Acme.');
    // Only a theme of an extension that runs can be chosen.
    expect(screen.getAllByTestId('vscode-ext-use-theme')).toHaveLength(1);
  });

  it('chooses a theme of an extension on purpose, and goes back to Escouade’s in one gesture', async () => {
    let theme: string | null = null;
    const { b, onlaid } = mount({
      vsx_installed: () => none({ extensions: [ext()], theme }),
      vsx_set_theme: (a) => void (theme = a.theme),
    });
    await userEvent.click(await screen.findByTestId('vscode-ext-use-theme'));
    expect(b.called('vsx_set_theme')[0].args).toEqual({ theme: 'acme.night/Acme Night' });
    expect(await screen.findByTestId('vscode-ext-theme-on')).toHaveTextContent('La vue a le thème « Acme Night ».');
    expect(onlaid).toHaveBeenLastCalledWith(expect.objectContaining({ theme: 'acme.night/Acme Night' }));
    await userEvent.click(screen.getByTestId('vscode-ext-theme-off'));
    expect(b.called('vsx_set_theme')[1].args).toEqual({ theme: null });
    await waitFor(() => expect(screen.queryByTestId('vscode-ext-theme-on')).toBeNull());
  });

  it('proposes updates, never installs them by itself, turns off and removes', async () => {
    let list = [ext()];
    const { b } = mount({
      vsx_installed: () => none({ extensions: list }),
      vsx_updates: () => ok([{ id: 'acme.night', from: '1.0.0', to: '1.2.0', refused: null }]),
      vsx_install: () => {
        list = [ext({ version: '1.2.0' })];
        return ok(list[0]);
      },
      vsx_set_enabled: (a) => void (list = [ext({ enabled: a.enabled })]),
      vsx_remove: () => void (list = []),
    });
    await userEvent.click(await screen.findByTestId('vscode-ext-check'));
    const up = await screen.findByTestId('vscode-ext-update-item');
    expect(up).toHaveTextContent('1.0.0 → 1.2.0');
    expect(b.called('vsx_install')).toHaveLength(0);
    await userEvent.click(within(up).getByTestId('vscode-ext-update-one'));
    expect(b.called('vsx_install')[0].args).toMatchObject({ id: 'acme.night', version: '1.2.0', trust: false });
    await waitFor(() => expect(screen.queryByTestId('vscode-ext-update-item')).toBeNull());
    await userEvent.click(screen.getByTestId('vscode-ext-toggle'));
    expect(b.called('vsx_set_enabled')[0].args).toEqual({ id: 'acme.night', enabled: false });
    await waitFor(() => expect(screen.getByTestId('vscode-ext-item')).toHaveAttribute('data-status', 'off'));
    await userEvent.click(screen.getByTestId('vscode-ext-remove'));
    expect(b.called('vsx_remove')[0].args).toEqual({ id: 'acme.night' });
    expect(await screen.findByText('Aucune extension installée.')).toBeTruthy();
  });

  it('asks nothing of Open VSX from a private project', async () => {
    const { b } = mount({ vsx_installed: () => none({ private: 'demo-api', extensions: [ext()] }) });
    expect(await screen.findByTestId('vscode-ext-private')).toHaveTextContent('« demo-api » est un projet privé');
    expect(screen.queryByTestId('vscode-ext-search')).toBeNull();
    expect(screen.queryByTestId('vscode-ext-check')).toBeNull();
    expect(b.called('vsx_search')).toHaveLength(0);
    expect(b.called('vsx_image')).toHaveLength(0);
  });
});
