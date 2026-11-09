import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { BARE_ENGINE, CLAUDE_ENGINE, branchInfo, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import { afterGesture } from '../../test/settings';
import type { Manifest, PluginView } from '../../lib/plugins';
import { plugins } from '../../lib/plugins-view.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

describe('PrivacyPage', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.modal = { kind: 'settings' };
  });

  describe('remote control', () => {
    const LABEL = 'Remote control pour les agents de ce projet';
    const REASON = 'Activé pour tous les projets dans Réglages › Agents';
    const tab = (name: string) => within(screen.getByRole('navigation')).getByRole('button', { name });
    const headings = () => screen.getAllByRole('heading').map((h) => h.textContent);
    /** The project, then its page « Confidentialité », from the bar. */
    const toPrivacy = async (privacy = 'Confidentialité') => {
      await userEvent.click(tab('demo-api'));
      await userEvent.click(tab(privacy));
    };
    /** The modal on « Agents », the setting for all agents switched on or off, then on the project’s « Confidentialité ». */
    const toProjects = async (global: boolean, [allLabel, privacy] = ['Remote control pour tous les agents', 'Confidentialité']) => {
      render(SettingsModal, { page: 'agents' });
      const all = screen.getByRole('switch', { name: allLabel });
      if (global !== (all.getAttribute('aria-checked') === 'true')) await userEvent.click(all);
      await toPrivacy(privacy);
    };

    it('is hidden while no harness has Remote Control', () => {
      fakeBackend();
      app.engines = [BARE_ENGINE];
      render(SettingsModal, { page: 'privacy', projectId: 'p1' });
      expect(screen.queryByRole('switch', { name: LABEL })).toBeNull();
      expect(headings()).not.toContain('Remote control');
    });

    it('has its own group after « Projet privé », before the plugins, off by default, and says what it does', () => {
      fakeBackend();
      render(SettingsModal, { page: 'privacy', projectId: 'p1' });
      const names = headings();
      expect(names.slice(names.indexOf('Confidentialité'))).toEqual(['Confidentialité', 'Remote control', 'Plugins dans ce projet']);
      const remote = screen.getByRole('switch', { name: LABEL });
      expect(remote).toHaveAttribute('aria-checked', 'false');
      expect(remote).toBeEnabled();
      expect(screen.getByText(/^Chaque agent de ce projet, existant ou nouveau/)).toHaveTextContent(
        'Chaque agent de ce projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l’app Claude. Escouade lance tout de suite le process claude de chacun, qui reste lancé. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu’à son prochain lancement. Pour mettre un agent à part : clic droit, « Désactiver le remote control ».',
      );
      expect(screen.queryByText(REASON)).not.toBeInTheDocument();
    });

    it('is saved with the project when switched on, and shows what the project has', async () => {
      const backend = fakeBackend();
      resetApp({ projects: [project({ remoteControl: true }), project({ id: 'p2', name: 'site' })] });
      app.modal = { kind: 'settings' };
      render(SettingsModal, { page: 'privacy', projectId: 'p2' });
      expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'false');
      await userEvent.click(screen.getByRole('switch', { name: LABEL }));
      expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'true');
      await afterGesture();
      expect(backend.called('update_project')).toHaveLength(1);
      expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p2', remoteControl: true });
      expect(app.projects[1].remoteControl).toBe(true);
      // The other project kept its own value.
      expect(app.projects[0].remoteControl).toBe(true);
    });

    it('is switched on by a project that has it, and saved at each switch', async () => {
      const backend = fakeBackend();
      resetApp({ projects: [project({ remoteControl: true })] });
      app.modal = { kind: 'settings' };
      render(SettingsModal, { page: 'privacy', projectId: 'p1' });
      const remote = screen.getByRole('switch', { name: LABEL });
      expect(remote).toHaveAttribute('aria-checked', 'true');
      await userEvent.click(remote);
      await afterGesture();
      expect(backend.called('update_project').map((c) => c.args.project.remoteControl)).toEqual([false]);
      await userEvent.click(remote);
      await afterGesture();
      expect(backend.called('update_project').map((c) => c.args.project.remoteControl)).toEqual([false, true]);
      expect(app.projects[0].remoteControl).toBe(true);
    });

    it('is shown on and disabled while the setting for all agents is on in the modal, with the reason in view', async () => {
      const backend = fakeBackend({ save_settings: () => [] });
      await toProjects(true);
      const remote = screen.getByRole('switch', { name: LABEL });
      expect(remote).toHaveAttribute('aria-checked', 'true');
      expect(remote).toBeDisabled();
      expect(screen.getByText(REASON)).toBeVisible();
      expect(remote).toHaveAccessibleDescription(REASON);
      // A click changes nothing, and nothing of the project is saved.
      await userEvent.click(remote);
      expect(remote).toHaveAttribute('aria-checked', 'true');
      await afterGesture();
      expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ remoteControlAll: true });
      expect(backend.called('update_project')).toHaveLength(0);
      expect(app.projects[0].remoteControl).toBe(false);
    });

    it('finds its own value and is active again once the setting for all agents is switched off', async () => {
      fakeBackend();
      await toProjects(true);
      expect(screen.getByRole('switch', { name: LABEL })).toBeDisabled();
      await userEvent.click(tab('Agents'));
      await userEvent.click(screen.getByRole('switch', { name: 'Remote control pour tous les agents' }));
      await toPrivacy();
      const remote = screen.getByRole('switch', { name: LABEL });
      expect(remote).toHaveAttribute('aria-checked', 'false');
      expect(remote).toBeEnabled();
      expect(screen.queryByText(REASON)).not.toBeInTheDocument();
      expect(remote).not.toHaveAccessibleDescription(REASON);
    });

    it('keeps the project’s own value under the setting for all agents, on until the project turns it off', async () => {
      const backend = fakeBackend({ save_settings: () => [] });
      resetApp({ projects: [project({ remoteControl: true })] });
      app.modal = { kind: 'settings' };
      await toProjects(true);
      expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'true');
      expect(screen.getByRole('switch', { name: LABEL })).toBeDisabled();
      await userEvent.click(tab('Agents'));
      await userEvent.click(screen.getByRole('switch', { name: 'Remote control pour tous les agents' }));
      await toPrivacy();
      expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'true');
      expect(screen.getByRole('switch', { name: LABEL })).toBeEnabled();
      expect(backend.called('update_project')).toHaveLength(0);
    });

    it('says it in English', async () => {
      setLang('en');
      fakeBackend();
      await toProjects(true, ['Remote control for all agents', 'Privacy']);
      const remote = screen.getByRole('switch', { name: 'Remote control for this project’s agents' });
      expect(remote).toBeDisabled();
      expect(remote).toHaveAttribute('aria-checked', 'true');
      expect(screen.getByText('On for all projects in Settings › Agents')).toBeVisible();
      expect(screen.getByText(/^Every agent of this project/)).toHaveTextContent(
        'Every agent of this project, existing or new (ticket agents included), is reachable from claude.ai and the Claude app. Escouade starts each one’s claude process right away, and it stays running. The session appears on claude.ai as “project · agent” (for a ticket agent, the name comes from the ticket). In Claude Code view, an interface that is already open only picks it up at its next launch. To set an agent apart: right-click, “Turn off remote control”.',
      );
      expect(screen.getAllByRole('heading').map((h) => h.textContent)).toContain('Remote control');
    });
  });
});

describe('PrivacyPage: « Projet privé » (ESC-139)', () => {
  const LABEL = 'Projet privé';
  const DEFAULT = 'Nouveaux projets privés';
  const tab = (name: string) => within(screen.getByRole('navigation')).getByRole('button', { name });

  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site', private: true })] });
    app.modal = { kind: 'settings' };
  });

  it('is off by default, says what it changes, and is saved with the project', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    const own = screen.getByRole('switch', { name: LABEL });
    expect(own).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByRole('heading', { name: 'Confidentialité' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'Les agents des autres projets ne le voient pas par le serveur MCP d’Escouade : ni dans les listes, ni par son nom, ses tickets ou ses agents. Les siens ne voient que lui. Toi, Claude hors Escouade et les vues de l’app voyez tout.',
      ),
    ).toBeVisible();
    await userEvent.click(own);
    await afterGesture();
    expect(backend.called('update_project')).toHaveLength(1);
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', private: true });
    expect(app.projects[0].private).toBe(true);
    expect(backend.called('save_settings')).toHaveLength(0);
  });

  it('says, harness by harness, whether its files are kept by a guard or a barrier, for a private project only (ESC-140, choice 28)', async () => {
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(screen.queryByRole('list', { name: 'Ses fichiers' })).toBeNull();
    await userEvent.click(screen.getByRole('switch', { name: 'Projet privé' }));
    const list = screen.getByRole('list', { name: 'Ses fichiers' });
    const rows = within(list).getAllByRole('listitem');
    expect(rows).toHaveLength(8);
    const kind = (name: string) => rows.find((r) => r.textContent?.startsWith(name))!.querySelector('.kind')!.textContent;
    expect(kind('Claude Code')).toBe('Garde-fou');
    expect(kind('Codex')).toBe('Barrière');
    expect(kind('OpenCode')).toBe('Garde-fou');
    expect(kind('Copilot CLI')).toBe('Garde-fou');
    expect(kind('Cursor CLI')).toBe('Garde-fou');
    expect(kind('Mistral Vibe')).toBe('Garde-fou');
    expect(kind('Antigravity CLI')).toBe('Garde-fou');
    expect(kind('Gemini CLI')).toBe('Garde-fou');
    // What each cannot guarantee, said.
    expect(within(list).getByText(/sauf en mode Bypass, qui n’en a pas/)).toBeVisible();
    expect(within(list).getByText(/Rien n’est garanti en mode Bypass, ni en vue Copilot CLI/)).toBeVisible();
    expect(within(list).getByText(/pas à un script qui ouvre un fichier lui-même/)).toBeVisible();
    expect(within(list).getByText(/Ses lectures et ses écritures ne demandent rien, même hors du dossier de l’agent/)).toBeVisible();
    expect(within(list).getByText(/Vibe reste dans son agent ask pour que ces refus tiennent/)).toBeVisible();
    expect(within(list).getByText(/que son mode Bypass ne lève pas/)).toBeVisible();
    expect(within(list).getByText(/que son mode Bypass \(yolo\) ne lève pas/)).toBeVisible();
    expect(screen.getByText(/Une barrière pour tous les harness demanderait un compte système par projet/)).toBeVisible();
  });

  it('shows a private project as it is, its lock among the projects of the settings', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p2' });
    expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'true');
    const lock = 'Projet privé : invisible pour les agents des autres projets';
    expect(screen.getByRole('button', { name: /site/ })).toContainElement(screen.getByRole('img', { name: lock }));
    expect(screen.getAllByRole('img', { name: lock })).toHaveLength(1);
    // Each switch saved at once: put back, it is private again.
    await userEvent.click(screen.getByRole('switch', { name: LABEL }));
    await userEvent.click(screen.getByRole('switch', { name: LABEL }));
    await afterGesture();
    expect(backend.called('update_project').map((c) => c.args.project.private)).toEqual([false, true]);
  });

  it('sets the default of the new projects, off until switched on, also with no project open', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'allProjects' });
    expect(screen.getByRole('heading', { name: 'Nouveaux projets' })).toBeInTheDocument();
    const byDefault = screen.getByRole('switch', { name: DEFAULT });
    expect(byDefault).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText('Chaque projet ajouté est privé dès le départ. Ceux qui existent gardent leur réglage.')).toBeVisible();
    await userEvent.click(byDefault);
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ privateProjects: true });
    expect(app.settings.privateProjects).toBe(true);
    // The projects that exist keep their own.
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('offers the default with no project open', () => {
    fakeBackend();
    resetApp({ projects: [] });
    render(SettingsModal, { page: 'allProjects' });
    expect(screen.getByRole('switch', { name: DEFAULT })).toBeInTheDocument();
  });

  it('says it in English', () => {
    setLang('en');
    fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(screen.getByRole('switch', { name: 'Private project' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Private project: unseen by the agents of the other projects' })).toBeInTheDocument();
    cleanup();
    render(SettingsModal, { page: 'allProjects' });
    expect(screen.getByRole('switch', { name: 'New projects are private' })).toBeInTheDocument();
  });
});

describe('PrivacyPage: a private project’s accounts (ESC-141)', () => {
  const PRIVATE = 'Projet privé';
  const LIMIT = 'Comptes partagés à la limite';
  const pro = { id: 'pro', name: 'Pro', configDir: 'C:/acc/pro', claudePath: '', active: true };
  const principal = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };

  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.settings.accounts = [principal, pro];
    app.modal = { kind: 'settings' };
  });

  it('says nothing of the accounts of a project that is not private', () => {
    fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(screen.queryByRole('switch', { name: LIMIT })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Ajouter un compte réservé…' })).toBeNull();
  });

  it('made private, says which shared accounts it still goes to and offers to reserve one', async () => {
    const backend = fakeBackend({
      account_update: ({ account }) => [principal, account],
    });
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    await userEvent.click(screen.getByRole('switch', { name: PRIVATE }));
    expect(
      screen.getByText(
        'Ses agents peuvent partir sur « Principal » et « Pro », des comptes partagés avec d’autres projets : sessions, mémoire, historique et réglages globaux de Claude Code y sont communs.',
      ),
    ).toBeVisible();
    // Pro reserved to it alone, at once: what it shares is now Principal only, at the limit too unless allowed.
    await userEvent.click(screen.getByRole('button', { name: 'Réserver Pro à ce projet' }));
    expect(backend.called('account_update')[0].args.account).toMatchObject({ id: 'pro', projects: ['p1'] });
    expect(
      await screen.findByText(
        'Compte réservé à ce projet : « Pro ». Sessions, mémoire et historique de Claude Code n’y sont partagés avec aucun autre projet.',
      ),
    ).toBeVisible();
    expect(screen.queryByText(/^Ses agents peuvent partir sur/)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Ajouter un compte réservé…' })).toBeNull();
    // Allowed to go on on a shared account at the limit: Principal is shared again, and said so.
    const limit = screen.getByRole('switch', { name: LIMIT });
    expect(limit).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(limit);
    expect(screen.getByText(/Ses agents peuvent partir sur « Principal », un compte partagé/)).toBeVisible();
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', private: true, sharedOnLimit: true });
  });

  it('adds an account reserved to it, back to its settings as drafted', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    await userEvent.click(screen.getByRole('switch', { name: PRIVATE }));
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter un compte réservé…' }));
    expect(app.modal).toEqual({ kind: 'account', engine: 'claude', reserveFor: 'p1' });
  });

  it('says the machine’s account reserved to it is still the harness’s outside Escouade', () => {
    fakeBackend();
    app.projects[0].private = true;
    app.settings.accounts = [
      { ...principal, projects: ['p1'] },
      { ...pro, projects: ['p2'] },
    ];
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(
      screen.getByText(
        'Compte réservé à ce projet : « Principal ». Sessions, mémoire et historique de Claude Code n’y sont partagés avec aucun autre projet. Le compte de la machine reste celui de Claude Code hors d’Escouade.',
      ),
    ).toBeVisible();
  });

  it('says it in English', async () => {
    setLang('en');
    fakeBackend();
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    await userEvent.click(screen.getByRole('switch', { name: 'Private project' }));
    expect(screen.getByRole('switch', { name: 'Shared accounts at the limit' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add a reserved account…' })).toBeInTheDocument();
    expect(screen.getByText(/Its agents may go to “Main” and “Pro”, accounts shared with other projects/)).toBeVisible();
  });
});

describe('PrivacyPage: the plugins in this project', () => {
  const plugin = (
    id: string,
    name: string,
    over: Partial<PluginView> = {},
    contributes: Manifest['contributes'] = { ticketSources: [{ id, name }] },
  ) =>
    ({
      id,
      name,
      version: '1.0.0',
      description: name,
      origin: { kind: 'store', version: '1.0.0', sha256: '', verified: true, repository: '', tag: '', commit: '', seed: true },
      fingerprint: 'store:abcd',
      folder: '',
      declarative: false,
      state: { kind: 'ready' },
      ignored: [],
      manifest: {
        id,
        name,
        version: '1.0.0',
        escouade: { api: 1, min: '2.2.0' },
        activation: [],
        permissions: { events: [], network: [], secrets: [], commands: [], decide: false },
        contributes,
      },
      installedAt: '2026-11-08T10:00:00Z',
      verified: true,
      withdrawn: null,
      inStore: null,
      choices: { enabled: true, projects: {}, revoked: [], settings: {} },
      asks: [],
      secretKeys: [],
      update: null,
      commands: [],
      commandsApproved: true,
      ...over,
    }) as PluginView;
  const JIRA = plugin('dev.escouade.jira', 'Jira');
  const LINEAR = plugin('dev.escouade.linear', 'Linear', { choices: { enabled: false, projects: {}, revoked: [], settings: {} } });
  const THEME = plugin('dev.escouade.nord', 'Nord', {}, { themes: [{ id: 'nord', name: 'Nord' }] });

  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site', private: true })] });
    app.modal = { kind: 'settings' };
    plugins.list = [];
    plugins.loaded = false;
  });

  it('lists each plugin on project by project with its switch, set at once, a theme apart', async () => {
    const backend = fakeBackend({
      plugins_list: () => [JIRA, LINEAR, THEME],
      plugin_seed_offers: () => [],
      plugin_set_project: ({ id, project: p, on }: { id: string; project: string; on: boolean }) => ({
        ...JIRA,
        id,
        choices: { ...JIRA.choices, projects: { [p]: on } },
      }),
    });
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    const group = await screen.findByRole('region', { name: 'Plugins dans ce projet' });
    const jira = await within(group).findByRole('switch', { name: 'Jira dans ce projet' });
    expect(jira).toHaveAttribute('aria-checked', 'true');
    // Off for the whole app: shown off, with why, and not switched here.
    const linear = within(group).getByRole('switch', { name: 'Linear dans ce projet' });
    expect(linear).toHaveAttribute('aria-checked', 'false');
    expect(linear).toBeDisabled();
    expect(within(group).getByText(/désactivé pour toute l’app/)).toBeInTheDocument();
    // A theme reaches the whole app: no switch of a project.
    expect(within(group).queryByRole('switch', { name: 'Nord dans ce projet' })).toBeNull();
    await userEvent.click(jira);
    expect(backend.called('plugin_set_project').at(-1)!.args).toEqual({ id: 'dev.escouade.jira', project: 'p1', on: false });
    await vi.waitFor(() =>
      expect(within(group).getByRole('switch', { name: 'Jira dans ce projet' })).toHaveAttribute('aria-checked', 'false'),
    );
  });

  it('shows a private project off by default, and says nothing reaches it', async () => {
    fakeBackend({ plugins_list: () => [JIRA], plugin_seed_offers: () => [] });
    render(SettingsModal, { page: 'privacy', projectId: 'p2' });
    const jira = await screen.findByRole('switch', { name: 'Jira dans ce projet' });
    expect(jira).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText(/projet privé : coupé d’office, rien ne lui parvient/)).toBeInTheDocument();
  });

  it('says when no plugin is on project by project, in English too', async () => {
    fakeBackend({ plugins_list: () => [THEME], plugin_seed_offers: () => [] });
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(await screen.findByText('Aucun plugin installé qui s’active projet par projet.')).toBeInTheDocument();
    cleanup();
    setLang('en');
    render(SettingsModal, { page: 'privacy', projectId: 'p1' });
    expect(await screen.findByRole('region', { name: 'Plugins in this project' })).toBeInTheDocument();
  });
});
