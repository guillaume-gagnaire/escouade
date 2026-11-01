import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { ForgeAccountView, ForgeProvider, ForgesView } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import ForgesSection from './ForgesSection.svelte';

const opened = vi.hoisted(() => [] as string[]);
vi.mock('../../lib/opener', () => ({ openUrl: vi.fn(async (u: string) => void opened.push(u)), openPath: vi.fn() }));

const CAPS = {
  draft: true,
  autoMerge: true,
  checks: true,
  jobLogs: true,
  reviews: true,
  reviewThreads: true,
  deployments: true,
  mergeStrategies: ['merge', 'squash'] as ('merge' | 'squash' | 'rebase')[],
};

const PROVIDERS: ForgeProvider[] = [
  {
    kind: 'github',
    name: 'GitHub',
    requestWord: 'PR',
    defaultHost: 'github.com',
    needsEmail: false,
    cli: 'gh',
    scopes: ['repo'],
    capabilities: CAPS,
  },
  {
    kind: 'gitlab',
    name: 'GitLab',
    requestWord: 'MR',
    defaultHost: 'gitlab.com',
    needsEmail: false,
    cli: 'glab',
    scopes: ['api'],
    capabilities: CAPS,
  },
  {
    kind: 'bitbucket',
    name: 'Bitbucket',
    requestWord: 'PR',
    defaultHost: 'bitbucket.org',
    needsEmail: true,
    cli: null,
    scopes: ['read:pullrequest:bitbucket'],
    capabilities: CAPS,
  },
  {
    kind: 'gitea',
    name: 'Gitea / Forgejo',
    requestWord: 'PR',
    defaultHost: null,
    needsEmail: false,
    cli: null,
    scopes: ['write:repository'],
    capabilities: CAPS,
  },
];

const account = (over: Partial<ForgeAccountView>): ForgeAccountView => ({
  kind: 'github',
  name: 'GitHub',
  host: 'github.com',
  url: 'https://github.com',
  label: '@ada',
  viaCli: false,
  inFile: false,
  issues: false,
  status: { health: 'connected', missing: [], error: null },
  projects: [],
  ...over,
});

const VIEW: ForgesView = {
  providers: PROVIDERS,
  accounts: [
    account({ viaCli: true, issues: true, projects: ['demo', 'api'] }),
    account({
      kind: 'gitlab',
      name: 'GitLab',
      host: 'gitlab.acme.dev',
      url: 'https://gitlab.acme.dev',
      status: { health: 'missingScopes', missing: ['api'], error: null },
      projects: ['web'],
      issues: true,
    }),
    account({
      kind: 'gitea',
      name: 'Gitea / Forgejo',
      host: 'codeberg.org',
      url: 'https://codeberg.org',
      label: '@ada-cb',
      status: { health: 'expired', missing: [], error: 'Gitea refuse ces identifiants (401)' },
    }),
  ],
  unknown: [{ host: 'git.example.org', url: 'https://git.example.org:8443', kind: null, projects: ['notes'] }],
  issues: [{ service: 'github', connected: true, label: '@ada · via gh' }],
};

const row = (name: string) => screen.getByRole('group', { name });

describe('ForgesSection', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    opened.length = 0;
  });

  it('shows how each account stands, the rights it lacks and the projects on it', async () => {
    const be = fakeBackend({ forges: () => VIEW, forge_check: () => VIEW });
    app.accounts = [];
    render(ForgesSection);
    const gh = await waitFor(() => row('github.com'));
    expect(gh).toHaveTextContent('Connecté · @ada');
    expect(gh).toHaveTextContent('Jeton de gh');
    expect(gh).toHaveTextContent('Sert aussi à GitHub Issues');
    expect(gh).toHaveTextContent('Projets : demo et api');
    const gl = row('gitlab.acme.dev');
    expect(gl).toHaveTextContent('Droits manquants : api');
    expect(gl).toHaveTextContent('Projet : web');
    // GitLab Issues takes it too; a Gitea is no source of tickets, and says so.
    expect(gl).toHaveTextContent('Sert aussi à GitLab Issues');
    const cb = row('codeberg.org');
    expect(cb).toHaveTextContent('Pas de tickets : PR et CI seulement');
    expect(gh).not.toHaveTextContent('Pas de tickets');
    expect(cb).toHaveTextContent('Jeton expiré ou révoqué · @ada-cb');
    expect(cb).toHaveTextContent('Gitea refuse ces identifiants (401)');
    expect(row('git.example.org')).toHaveTextContent('Aucun compte, pour le projet notes');
    // Checked as the tab opens; Issues' accounts follow (and the list is read again, github.com's being theirs).
    expect(be.called('forge_check').length).toBeGreaterThanOrEqual(1);
    expect(app.accounts).toEqual(VIEW.issues);
  });

  it('says under its accounts that the plugin the app gives for a host is switched off, or stopped and why', async () => {
    const off = {
      ...VIEW,
      providers: PROVIDERS.map((p) =>
        p.kind === 'github' ? { ...p, servedBy: { plugin: 'dev.escouade.github', name: 'GitHub', on: false, stopped: null } } : p,
      ),
    };
    fakeBackend({ forges: () => off, forge_check: () => off });
    const { unmount } = render(ForgesSection);
    const gh = await waitFor(() => row('github.com'));
    expect(gh).toHaveTextContent('Le plugin GitHub est coupé (Réglages › Plugins)');
    // Another host is not concerned.
    expect(row('gitlab.acme.dev')).not.toHaveTextContent('plugin');
    unmount();
    const stopped = {
      ...VIEW,
      providers: PROVIDERS.map((p) =>
        p.kind === 'github'
          ? {
              ...p,
              servedBy: { plugin: 'dev.escouade.github', name: 'GitHub', on: true, stopped: 'stopped after 4 crashes within 5 minutes' },
            }
          : p,
      ),
    };
    fakeBackend({ forges: () => stopped, forge_check: () => stopped });
    render(ForgesSection);
    await waitFor(() =>
      expect(row('github.com')).toHaveTextContent('Le plugin GitHub s’est arrêté (stopped after 4 crashes within 5 minutes)'),
    );
  });

  it('reads the list again when the account of GitHub Issues changes, and only then', async () => {
    const be = fakeBackend({ forges: () => VIEW, forge_check: () => VIEW });
    app.accounts = VIEW.issues;
    render(ForgesSection);
    await waitFor(() => row('github.com'));
    await waitFor(() => expect(be.called('forge_check')).toHaveLength(1));
    app.accounts = [{ service: 'github', connected: false, label: '' }];
    await waitFor(() => expect(be.called('forge_check')).toHaveLength(2));
  });

  it('connects an instance with its token, and says why one is refused', async () => {
    let refuse = true;
    const be = fakeBackend({
      forges: () => ({ ...VIEW, accounts: [], unknown: [] }),
      forge_check: () => ({ ...VIEW, accounts: [], unknown: [] }),
      forge_connect: () => {
        if (refuse) throw 'GitLab refuse ces identifiants (401)';
        return VIEW;
      },
      forge_token_page: (a: any) => `https://${a.url}/-/user_settings/personal_access_tokens`,
    });
    render(ForgesSection);
    await userEvent.click(await screen.findByRole('button', { name: 'Connecter un hébergeur…' }));
    const form = screen.getByRole('form', { name: 'Connexion à un hébergeur' });
    await userEvent.selectOptions(within(form).getByLabelText('Hébergeur'), 'gitlab');
    expect(within(form).getByText('Droits à donner au jeton : api')).toBeInTheDocument();
    expect(within(form).getByText('(vide : celui de glab)')).toBeInTheDocument();
    await userEvent.type(within(form).getByLabelText('Adresse'), 'gitlab.acme.dev');
    await userEvent.click(within(form).getByRole('button', { name: 'Créer un jeton' }));
    await waitFor(() => expect(opened).toEqual(['https://gitlab.acme.dev/-/user_settings/personal_access_tokens']));
    await userEvent.type(within(form).getByLabelText(/^Jeton/), 'glpat-x');
    await userEvent.click(within(form).getByRole('button', { name: 'Connecter' }));
    expect(await within(form).findByRole('alert')).toHaveTextContent('GitLab refuse ces identifiants (401)');
    refuse = false;
    await userEvent.click(within(form).getByRole('button', { name: 'Connecter' }));
    await waitFor(() => expect(screen.queryByRole('form')).toBeNull());
    expect(be.called('forge_connect').at(-1)!.args.form).toEqual({ kind: 'gitlab', url: 'gitlab.acme.dev', email: '', token: 'glpat-x' });
    expect(row('gitlab.acme.dev')).toHaveTextContent('Droits manquants : api');
  });

  it('offers the plugins that serve a kind of account, and says which one serves an account (INT-6)', async () => {
    const servers = [
      { plugin: 'dev.escouade.gitlab', name: 'Escouade', firstHand: true },
      { plugin: 'acme.gitlab', name: "GitLab d'entreprise", firstHand: false },
    ];
    const view = {
      ...VIEW,
      providers: PROVIDERS.map((p) => (p.kind === 'gitlab' ? { ...p, servers } : { ...p, servers: [servers[0]] })),
      accounts: [
        account({ kind: 'gitlab', name: 'GitLab', host: 'gitlab.com', url: 'https://gitlab.com', issues: true, plugin: 'acme.gitlab' }),
      ],
      unknown: [],
    };
    const be = fakeBackend({ forges: () => view, forge_check: () => view, forge_connect: () => view });
    render(ForgesSection);
    expect(await waitFor(() => row('gitlab.com'))).toHaveTextContent('Servi par le plugin acme.gitlab');
    await userEvent.click(screen.getByRole('button', { name: 'Connecter un hébergeur…' }));
    const form = screen.getByRole('form', { name: 'Connexion à un hébergeur' });
    // One plugin only for GitHub: nothing to choose.
    expect(within(form).queryByLabelText('Servi par')).toBeNull();
    await userEvent.selectOptions(within(form).getByLabelText('Hébergeur'), 'gitlab');
    const by = within(form).getByLabelText('Servi par');
    expect(
      within(by)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['Escouade', "GitLab d'entreprise"]);
    await userEvent.selectOptions(by, 'acme.gitlab');
    await userEvent.type(within(form).getByLabelText(/^Jeton/), 'glpat-x');
    await userEvent.click(within(form).getByRole('button', { name: 'Connecter' }));
    await waitFor(() => expect(be.called('forge_connect')).toHaveLength(1));
    expect(be.called('forge_connect')[0].args.form).toEqual({
      kind: 'gitlab',
      url: '',
      email: '',
      token: 'glpat-x',
      plugin: 'acme.gitlab',
    });
  });

  it('asks Bitbucket for its e-mail', async () => {
    fakeBackend({ forges: () => VIEW, forge_check: () => VIEW });
    render(ForgesSection);
    await userEvent.click(await screen.findByRole('button', { name: 'Connecter un hébergeur…' }));
    const form = screen.getByRole('form', { name: 'Connexion à un hébergeur' });
    expect(within(form).queryByLabelText('E-mail')).toBeNull();
    await userEvent.selectOptions(within(form).getByLabelText('Hébergeur'), 'bitbucket');
    expect(within(form).getByLabelText('E-mail')).toBeInTheDocument();
    expect(within(form).getByLabelText('Adresse')).toHaveAttribute('placeholder', 'bitbucket.org');
  });

  it('connects an unknown host of a remote, its kind asked of it', async () => {
    const be = fakeBackend({ forges: () => VIEW, forge_check: () => VIEW, forge_guess: () => 'gitea' });
    render(ForgesSection);
    await userEvent.click(await screen.findByRole('button', { name: 'Connecter git.example.org' }));
    const form = screen.getByRole('form', { name: 'Connexion à un hébergeur' });
    await waitFor(() => expect(within(form).getByLabelText('Hébergeur')).toHaveValue('gitea'));
    expect(within(form).getByLabelText('Adresse')).toHaveValue('https://git.example.org:8443');
    expect(be.called('forge_guess')[0].args.url).toBe('https://git.example.org:8443');
  });

  it('connects an Azure DevOps organization with a PAT or az, and says who it is', async () => {
    const AZURE: ForgeProvider = {
      kind: 'azure',
      name: 'Azure DevOps',
      requestWord: 'PR',
      defaultHost: 'dev.azure.com',
      addressHint: 'dev.azure.com/acme',
      needsEmail: false,
      cli: 'az',
      cliLogin: 'az login',
      scopes: ['vso.code_write', 'vso.build_execute', 'vso.work_write'],
      capabilities: { ...CAPS, deployments: false, mergeStrategies: ['merge', 'squash', 'rebase', 'semiLinear'] },
    };
    const connected: ForgesView = {
      ...VIEW,
      providers: [...PROVIDERS, AZURE],
      accounts: [
        account({
          kind: 'azure',
          name: 'Azure DevOps',
          host: 'dev.azure.com',
          url: 'https://dev.azure.com/acme',
          label: 'ada@acme.dev',
          projects: ['demo'],
        }),
      ],
      unknown: [],
    };
    const be = fakeBackend({
      forges: () => ({ ...connected, accounts: [] }),
      forge_check: () => ({ ...connected, accounts: [] }),
      forge_connect: () => connected,
      forge_token_page: () => 'https://dev.azure.com/acme/_usersSettings/tokens',
    });
    render(ForgesSection);
    await userEvent.click(await screen.findByRole('button', { name: 'Connecter un hébergeur…' }));
    const form = screen.getByRole('form', { name: 'Connexion à un hébergeur' });
    await userEvent.selectOptions(within(form).getByLabelText('Hébergeur'), 'azure');
    // Its address is an organization's; its token a PAT, else the az CLI's.
    expect(within(form).getByLabelText('Adresse')).toHaveAttribute('placeholder', 'dev.azure.com/acme');
    expect(within(form).getByLabelText(/^Jeton/)).toHaveAttribute('placeholder', 'az login');
    expect(within(form).getByText('(vide : celui de az)')).toBeInTheDocument();
    expect(within(form).getByText('Droits à donner au jeton : vso.code_write, vso.build_execute, vso.work_write')).toBeInTheDocument();
    await userEvent.type(within(form).getByLabelText('Adresse'), 'dev.azure.com/acme');
    await userEvent.click(within(form).getByRole('button', { name: 'Créer un jeton' }));
    await waitFor(() => expect(opened).toEqual(['https://dev.azure.com/acme/_usersSettings/tokens']));
    await userEvent.type(within(form).getByLabelText(/^Jeton/), 'pat-azure');
    await userEvent.click(within(form).getByRole('button', { name: 'Connecter' }));
    await waitFor(() => expect(screen.queryByRole('form')).toBeNull());
    expect(be.called('forge_connect').at(-1)!.args.form).toEqual({
      kind: 'azure',
      url: 'dev.azure.com/acme',
      email: '',
      token: 'pat-azure',
    });
    // Its row: the organization, who the token is, and no tickets yet.
    const az = row('dev.azure.com');
    expect(az).toHaveTextContent('Azure DevOps · dev.azure.com/acme');
    expect(az).toHaveTextContent('Connecté · ada@acme.dev');
    expect(az).toHaveTextContent('Projet : demo');
    expect(az).toHaveTextContent('Pas de tickets : PR et CI seulement');
    expect(az.querySelector('.badge')).toHaveTextContent('AZ');
  });

  it('disconnects an instance', async () => {
    const be = fakeBackend({
      forges: () => VIEW,
      forge_check: () => VIEW,
      forge_disconnect: () => ({ ...VIEW, accounts: VIEW.accounts.slice(0, 2) }),
    });
    render(ForgesSection);
    await userEvent.click(await screen.findByRole('button', { name: 'Déconnecter codeberg.org' }));
    await waitFor(() => expect(screen.queryByRole('group', { name: 'codeberg.org' })).toBeNull());
    expect(be.called('forge_disconnect')[0].args.host).toBe('codeberg.org');
  });

  it('speaks English', async () => {
    setLang('en');
    try {
      fakeBackend({ forges: () => VIEW, forge_check: () => VIEW });
      render(ForgesSection);
      expect(await screen.findByText('Connected instances')).toBeInTheDocument();
      expect(row('gitlab.acme.dev')).toHaveTextContent('Also used for GitLab Issues');
      expect(row('codeberg.org')).toHaveTextContent('No tickets: PRs and CI only');
      expect(row('gitlab.acme.dev')).toHaveTextContent('Missing permissions: api');
      expect(row('github.com')).toHaveTextContent('Also used for GitHub Issues');
      expect(row('github.com')).toHaveTextContent('Projects: demo and api');
      expect(row('codeberg.org')).toHaveTextContent('Token expired or revoked · @ada-cb');
      expect(screen.getByRole('button', { name: 'Connect git.example.org' })).toBeInTheDocument();
    } finally {
      setLang('fr');
    }
  });
});
