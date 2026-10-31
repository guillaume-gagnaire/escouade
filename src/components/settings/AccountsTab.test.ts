import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The settings' other tabs have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));
// Links open in the user's browser: noted instead.
const opened = vi.hoisted(() => [] as string[]);
vi.mock('../../lib/opener', () => ({ openUrl: vi.fn(async (u: string) => void opened.push(u)), openPath: vi.fn() }));

import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { Account, AccountStatus, ClosedProject } from '../../lib/types';
import {
  agent,
  AGY_ENGINE,
  BARE_ENGINE,
  CURSOR_ENGINE,
  CLAUDE_ENGINE,
  GEMINI_ENGINE,
  VIBE_ENGINE,
  fakeBackend,
  project,
  resetApp,
  SETTINGS,
  usageOf,
} from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\Users\\ada\\.escouade\\claude\\pro', claudePath: '', active: true };

const status = (over: Record<string, Partial<AccountStatus>> = {}) => {
  const all: Record<string, AccountStatus> = {
    principal: { connected: true, email: 'ada@atlas.dev', dir: 'C:\\Users\\ada\\.claude', stamp: 'principal-sign-in', ...over.principal },
    pro: { connected: false, email: null, dir: PRO.configDir, stamp: null, ...over.pro },
  };
  return (a: { id: string }) => all[a.id];
};

const backend = (over: Record<string, (a: any) => unknown> = {}) =>
  fakeBackend({ account_status: status(), save_settings: () => [], ...over });

/** The row of the account shown as `name`. */
const row = (name: string) => screen.getByRole('listitem', { name });
const names = () => screen.getAllByRole('listitem').map((li) => li.getAttribute('aria-label'));

describe('AccountsTab', () => {
  beforeEach(() => {
    resetApp();
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, PRO] };
    app.modal = { kind: 'settings', tab: 'accounts' };
  });

  it('sets « Projection des quotas » at once, past 80 % by default, with the app’s other settings as they are, the draft left as it is', async () => {
    const b = backend();
    render(SettingsModal, { tab: 'accounts' });
    expect(screen.getByRole('heading', { name: 'Quotas' })).toBeInTheDocument();
    const projection = screen.getByRole('group', { name: 'Projection des quotas' });
    const choice = (name: string) => within(projection).getByRole('button', { name });
    expect(
      within(projection)
        .getAllByRole('button')
        .map((c) => c.textContent?.trim()),
    ).toEqual(['Au-delà de 80 %', 'Toujours', 'Jamais']);
    expect(choice('Au-delà de 80 %')).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(choice('Toujours'));
    expect(choice('Toujours')).toHaveAttribute('aria-pressed', 'true');
    expect(app.settings.quotaProjection).toBe('always');
    expect(b.called('save_settings')).toHaveLength(1);
    expect(b.called('save_settings')[0].args.settings).toMatchObject({ quotaProjection: 'always', accounts: [PRINCIPAL, PRO] });
    await userEvent.click(choice('Jamais'));
    expect(b.called('save_settings')[1].args.settings).toMatchObject({ quotaProjection: 'off' });
    expect(screen.getByRole('tab', { name: 'Comptes' })).not.toHaveClass('changed');
  });

  it('reads « Projection des quotas » saved before 2.0.2 as a switch: off is « Jamais »', () => {
    backend();
    (app.settings as { quotaProjection: unknown }).quotaProjection = false;
    render(SettingsModal, { tab: 'accounts' });
    const projection = screen.getByRole('group', { name: 'Projection des quotas' });
    expect(within(projection).getByRole('button', { name: 'Jamais' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('sets « Changer de compte à » at once, « À la limite » by default, and says what it changes', async () => {
    const b = backend();
    render(SettingsModal, { tab: 'accounts' });
    const at = screen.getByRole('group', { name: 'Changer de compte à' });
    expect(
      within(at)
        .getAllByRole('button')
        .map((x) => x.textContent),
    ).toEqual(['80 %', '90 %', '95 %', 'À la limite']);
    // Settings saved before it had none: the limit, as before.
    expect(within(at).getByRole('button', { name: 'À la limite' })).toHaveAttribute('aria-pressed', 'true');
    expect(
      screen.getByText(/^Quand la fenêtre de 5 h ou la fenêtre hebdomadaire de son compte atteint ce seuil, un agent passe/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/« Reprendre sur un autre compte… », ci-dessous, décide\. « Pause au-delà du quota » \(Kanban\)/),
    ).toBeInTheDocument();
    await userEvent.click(within(at).getByRole('button', { name: '95 %' }));
    expect(within(at).getByRole('button', { name: '95 %' })).toHaveAttribute('aria-pressed', 'true');
    expect(app.settings.switchAt).toBe(95);
    expect(b.called('save_settings')).toHaveLength(1);
    expect(b.called('save_settings')[0].args.settings).toMatchObject({ switchAt: 95, accounts: [PRINCIPAL, PRO] });
    // Not part of the draft: nothing to save, nothing to cancel.
    expect(screen.getByRole('tab', { name: 'Comptes' })).not.toHaveClass('changed');
  });

  it('says « Changer de compte à » in English', () => {
    backend();
    setLang('en');
    app.settings = { ...app.settings, switchAt: 90 };
    render(SettingsModal, { tab: 'accounts' });
    const at = screen.getByRole('group', { name: 'Switch accounts at' });
    expect(within(at).getByRole('button', { name: '90%' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(at).getByRole('button', { name: 'At the limit' })).toBeInTheDocument();
    setLang('fr');
  });

  it('comes after « Claude Code » and shows each account, its folder and whether it is signed in', async () => {
    const b = backend();
    render(SettingsModal, { tab: 'accounts' });
    const tabs = screen.getAllByRole('tab');
    const at = tabs.indexOf(screen.getByRole('tab', { name: 'Comptes' }));
    expect(tabs[at - 1]).toBe(screen.getByRole('tab', { name: 'Harness' }));
    expect(tabs[at]).toHaveAttribute('aria-selected', 'true');
    expect(names()).toEqual(['Principal', 'Pro']);
    expect(await within(row('Principal')).findByText('Connecté · ada@atlas.dev')).toBeInTheDocument();
    expect(within(row('Principal')).getByText('~/.claude')).toBeInTheDocument();
    expect(await within(row('Pro')).findByText('Pas connecté')).toBeInTheDocument();
    expect(within(row('Pro')).getByText('~/.escouade/claude/pro')).toBeInTheDocument();
    expect(b.called('account_status').map((c) => c.args.id)).toEqual(['principal', 'pro']);
    // Its own claude, else the settings' one.
    expect(within(row('Pro')).getByRole('textbox', { name: 'Exécutable' })).toHaveAttribute('placeholder', 'Celui des réglages');
    // Not signed in: « Se connecter… »; signed in, « Se reconnecter… ».
    expect(within(row('Pro')).getByRole('button', { name: 'Se connecter…' })).toBeInTheDocument();
    expect(within(row('Principal')).queryByRole('button', { name: 'Se connecter…' })).not.toBeInTheDocument();
    expect(within(row('Principal')).getByRole('button', { name: 'Se reconnecter…' })).toBeInTheDocument();
    expect(within(row('Pro')).queryByRole('button', { name: 'Se reconnecter…' })).not.toBeInTheDocument();
    // The machine's account is told, and goes like any other (ESC-128).
    expect(within(row('Principal')).getByText('Compte de la machine')).toBeInTheDocument();
    expect(within(row('Pro')).queryByText('Compte de la machine')).not.toBeInTheDocument();
    expect(within(row('Principal')).getByRole('button', { name: 'Supprimer' })).toBeInTheDocument();
    expect(within(row('Pro')).getByRole('button', { name: 'Supprimer' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'Les nouveaux agents partent sur le premier compte actif de leur harness sous les seuils « Pause au-delà du quota » et « Changer de compte à », dans cet ordre.',
      ),
    ).toBeInTheDocument();
    // Saved at once: nothing left for « Enregistrer ».
    expect(screen.getByRole('tab', { name: 'Comptes' })).not.toHaveClass('changed');
  });

  it('says what an account at rest, Principal on an API key or a sign-in out of date is', async () => {
    backend({ account_status: status({ principal: { connected: false, email: null }, pro: { connected: true, email: 'ada@pro.dev' } }) });
    app.settings.accounts = [PRINCIPAL, { ...PRO, active: false }];
    app.usage = {
      ...app.usage,
      accounts: [
        usageOf('principal', { connected: false, reason: 'Pas connecté', updatedAt: 0 }),
        usageOf('pro', { reason: 'Connexion expirée : relance Claude Code pour ce compte.', updatedAt: 0 }),
      ],
    };
    render(SettingsModal, { tab: 'accounts' });
    expect(await within(row('Principal')).findByText('Pas connecté par un compte claude.ai (clé d’API ?)')).toBeInTheDocument();
    expect(within(row('Principal')).getByRole('button', { name: 'Se connecter…' })).toBeInTheDocument();
    expect(await within(row('Pro')).findByText('Inactif')).toBeInTheDocument();
    expect(within(row('Pro')).getByText('Connexion expirée : relance Claude Code pour ce compte.')).toBeInTheDocument();
    // Expired: a sign-in again gets it a new token.
    expect(within(row('Pro')).getByRole('button', { name: 'Se connecter…' })).toBeInTheDocument();
  });

  it('renames an account and gives it its own claude, as the backend saves them', async () => {
    const b = backend({
      account_update: (a: { account: Account }) => [PRINCIPAL, { ...a.account, name: a.account.name.trim() }],
    });
    render(SettingsModal, { tab: 'accounts' });
    const name = within(row('Pro')).getByRole('textbox', { name: 'Nom' });
    await userEvent.clear(name);
    await userEvent.type(name, ' Boulot {Enter}');
    await waitFor(() => expect(b.called('account_update')).toHaveLength(1));
    expect(b.called('account_update')[0].args.account).toEqual({ ...PRO, name: ' Boulot ' });
    await waitFor(() => expect(names()).toEqual(['Principal', 'Boulot']));
    expect(app.settings.accounts[1].name).toBe('Boulot');
    const exe = within(row('Boulot')).getByRole('textbox', { name: 'Exécutable' });
    await userEvent.type(exe, 'C:\\outils\\claude.exe');
    await userEvent.tab();
    await waitFor(() => expect(b.called('account_update')).toHaveLength(2));
    expect(b.called('account_update')[1].args.account).toEqual({ ...PRO, name: 'Boulot', claudePath: 'C:\\outils\\claude.exe' });
    // Left as it was: nothing sent.
    await userEvent.click(within(row('Boulot')).getByRole('textbox', { name: 'Nom' }));
    await userEvent.tab();
    expect(b.called('account_update')).toHaveLength(2);
  });

  it('gives an account of a harness without a quota a budget of the day, saved as the backend has it (ESC-45)', async () => {
    const OC: Account = { id: 'oc', name: 'Local', engine: 'codex', configDir: 'C:\\oc', claudePath: '', active: true };
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, OC] };
    const b = backend({
      account_status: (a: { id: string }) => (a.id === 'oc' ? { connected: true, email: null, dir: 'C:\\oc', stamp: null } : status()(a)),
      account_update: (a: { account: Account }) => [PRINCIPAL, a.account],
    });
    render(SettingsModal, { tab: 'accounts' });
    // Claude Code has its quota: no budget.
    expect(within(row('Principal')).queryByRole('textbox', { name: 'Budget par jour ($)' })).not.toBeInTheDocument();
    const field = within(row('Local')).getByRole('textbox', { name: 'Budget par jour ($)' });
    expect(field).toHaveAttribute('placeholder', 'Aucun');
    expect(field).toHaveAccessibleDescription(/pilote auto se met en pause jusqu’à minuit/);
    await userEvent.type(field, '2,5');
    await userEvent.tab();
    await waitFor(() => expect(b.called('account_update')).toHaveLength(1));
    expect(b.called('account_update')[0].args.account).toEqual({ ...OC, dailyBudget: 2.5 });
    await waitFor(() => expect(app.settings.accounts[1].dailyBudget).toBe(2.5));
    // Not an amount: refused, as it was.
    const now = within(row('Local')).getByRole('textbox', { name: 'Budget par jour ($)' });
    await userEvent.clear(now);
    await userEvent.type(now, '-3');
    await userEvent.tab();
    expect(await within(row('Local')).findByRole('alert')).toHaveTextContent('Le budget par jour est un montant en dollars, positif.');
    expect(now).toHaveValue('2.5');
    expect(b.called('account_update')).toHaveLength(1);
    // Emptied: no budget.
    await userEvent.clear(now);
    await userEvent.tab();
    await waitFor(() => expect(b.called('account_update')).toHaveLength(2));
    expect(b.called('account_update')[1].args.account.dailyBudget).toBeNull();
  });

  it('links a Cursor account to its dashboard, with neither cost nor budget, and gives its key again (ESC-43)', async () => {
    const CURSOR_MAIN: Account = {
      id: 'principal-cursor',
      name: 'Principal',
      engine: 'cursor',
      configDir: '',
      claudePath: '',
      active: true,
    };
    const TEAM: Account = { id: 'equipe', name: 'Équipe', engine: 'cursor', configDir: 'C:\equipe', claudePath: '', active: true };
    app.engines = [CLAUDE_ENGINE, CURSOR_ENGINE];
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, CURSOR_MAIN, TEAM] };
    opened.length = 0;
    backend({
      account_status: (a: { id: string }) =>
        a.id === 'equipe'
          ? { connected: true, email: 'clé …abcd', dir: 'C:\equipe', stamp: null }
          : a.id === 'principal-cursor'
            ? { connected: true, email: null, dir: 'C:\Users\ada\.cursor', stamp: null }
            : status()(a),
    });
    render(SettingsModal, { tab: 'accounts' });
    const team = screen.getAllByRole('listitem', { name: 'Équipe' })[0];
    expect(await within(team).findByText('Connecté · clé …abcd')).toBeInTheDocument();
    // Neither its cost of the day nor a budget: Cursor tells none.
    expect(within(team).queryByText(/Aujourd’hui/)).not.toBeInTheDocument();
    expect(within(team).queryByRole('textbox', { name: 'Budget par jour ($)' })).not.toBeInTheDocument();
    await userEvent.click(within(team).getByRole('button', { name: 'Usage sur cursor.com' }));
    expect(opened).toEqual(['https://cursor.com/dashboard?tab=usage']);
    // Its key, given again; the machine's signs in by agent login.
    await userEvent.click(within(team).getByRole('button', { name: 'Changer la clé…' }));
    expect(app.modal).toEqual({ kind: 'account', accountId: 'equipe' });
  });

  it('shows what a Vibe account spent today with its budget, links Mistral’s console and gives its key again (ESC-267)', async () => {
    const TEAM: Account = { id: 'vibe-equipe', name: 'Équipe', engine: 'vibe', configDir: 'C:\\equipe', claudePath: '', active: true };
    app.engines = [CLAUDE_ENGINE, VIBE_ENGINE];
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, TEAM] };
    app.usage = { ...app.usage, accounts: [{ ...usageOf('vibe-equipe', {}), todayCost: 0.37 }] };
    opened.length = 0;
    backend({
      account_status: (a: { id: string }) =>
        a.id === 'vibe-equipe' ? { connected: true, email: 'clé …wxyz', dir: 'C:\\equipe', stamp: null } : status()(a),
    });
    render(SettingsModal, { tab: 'accounts' });
    const team = screen.getAllByRole('listitem', { name: 'Équipe' })[0];
    expect(await within(team).findByText('Connecté · clé …wxyz')).toBeInTheDocument();
    // Its cost told, no quota: the day's cost and a budget stand in, and its usage is on Mistral's console.
    expect(within(team).getByText('0,37 $')).toBeInTheDocument();
    expect(within(team).getByRole('textbox', { name: 'Budget par jour ($)' })).toBeInTheDocument();
    await userEvent.click(within(team).getByRole('button', { name: 'Usage sur admin.mistral.ai' }));
    expect(opened).toEqual(['https://admin.mistral.ai/organization/usage']);
    await userEvent.click(within(team).getByRole('button', { name: 'Changer la clé…' }));
    expect(app.modal).toEqual({ kind: 'account', accountId: 'vibe-equipe' });
  });

  it('links an Antigravity account to its plans, says its quota is in its /usage, and gives its key again (ESC-271)', async () => {
    const AGY_MAIN: Account = { id: 'principal-agy', name: 'Principal', engine: 'agy', configDir: '', claudePath: '', active: true };
    const TEAM: Account = { id: 'agy-equipe', name: 'Équipe', engine: 'agy', configDir: 'C:\\equipe', claudePath: '', active: true };
    app.engines = [CLAUDE_ENGINE, AGY_ENGINE];
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, AGY_MAIN, TEAM] };
    opened.length = 0;
    backend({
      account_status: (a: { id: string }) =>
        a.id === 'agy-equipe'
          ? { connected: true, email: 'clé …wxyz', dir: 'C:\\equipe', stamp: null }
          : a.id === 'principal-agy'
            ? { connected: false, email: null, dir: 'C:\\Users\\ada\\.gemini', stamp: null }
            : status()(a),
    });
    render(SettingsModal, { tab: 'accounts' });
    const team = screen.getAllByRole('listitem', { name: 'Équipe' })[0];
    expect(await within(team).findByText('Connecté · clé …wxyz')).toBeInTheDocument();
    // Neither cost nor budget: Antigravity tells tokens alone.
    expect(within(team).queryByRole('textbox', { name: 'Budget par jour ($)' })).not.toBeInTheDocument();
    const usage = within(team).getByRole('button', { name: 'Usage sur antigravity.google' });
    await userEvent.hover(usage);
    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      /son quota \(par groupe de modèles, hebdomadaire\) se lit par \/usage dans son interface/,
    );
    await userEvent.unhover(usage);
    await userEvent.click(usage);
    expect(opened).toEqual(['https://antigravity.google/docs/plans']);
    await userEvent.click(within(team).getByRole('button', { name: 'Changer la clé…' }));
    expect(app.modal).toEqual({ kind: 'account', accountId: 'agy-equipe' });
    // The machine's, not signed in: by agy in a terminal.
    const main = screen.getAllByRole('listitem', { name: 'Principal' }).find((l) => within(l).queryByText('Usage sur antigravity.google'))!;
    await userEvent.click(within(main).getByRole('button', { name: 'Se connecter…' }));
    expect(app.modal).toEqual({ kind: 'account', accountId: 'principal-agy' });
  });

  it('links a Gemini account to its usage, signs one in by Google and gives another its key again (ESC-275)', async () => {
    const MAIN: Account = { id: 'principal-gemini', name: 'Principal', engine: 'gemini', configDir: '', claudePath: '', active: true };
    const PERSO: Account = { id: 'gemini-perso', name: 'Perso', engine: 'gemini', configDir: 'C:\\perso', claudePath: '', active: true };
    const CLE: Account = { ...PERSO, id: 'gemini-cle', name: 'Clé', configDir: 'C:\\cle', apiKey: true };
    app.engines = [CLAUDE_ENGINE, GEMINI_ENGINE];
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, MAIN, PERSO, CLE] };
    opened.length = 0;
    backend({
      account_status: (a: { id: string }) =>
        a.id === 'gemini-cle'
          ? { connected: true, email: 'clé …wxyz', dir: 'C:\\cle', stamp: null }
          : a.id === 'gemini-perso'
            ? { connected: true, email: 'moi@exemple.fr', dir: 'C:\\perso', stamp: 't' }
            : a.id === 'principal-gemini'
              ? { connected: true, email: null, dir: 'C:\\Users\\ada\\.gemini', stamp: null }
              : status()(a),
    });
    render(SettingsModal, { tab: 'accounts' });
    const cle = screen.getAllByRole('listitem', { name: 'Clé' })[0];
    expect(await within(cle).findByText('Connecté · clé …wxyz')).toBeInTheDocument();
    expect(within(cle).queryByRole('textbox', { name: 'Budget par jour ($)' })).not.toBeInTheDocument();
    const usage = within(cle).getByRole('button', { name: 'Usage sur aistudio.google.com' });
    await userEvent.hover(usage);
    expect(await screen.findByRole('tooltip')).toHaveTextContent(/son usage se lit par \/stats dans son interface/);
    await userEvent.unhover(usage);
    await userEvent.click(usage);
    expect(opened).toEqual(['https://aistudio.google.com/usage']);
    await userEvent.click(within(cle).getByRole('button', { name: 'Changer la clé…' }));
    expect(app.modal).toEqual({ kind: 'account', accountId: 'gemini-cle' });
    // Signed in to Google by gemini in its folder: signed in again there.
    const perso = screen.getAllByRole('listitem', { name: 'Perso' })[0];
    expect(await within(perso).findByText('Connecté · moi@exemple.fr')).toBeInTheDocument();
    await userEvent.click(within(perso).getByRole('button', { name: 'Se reconnecter…' }));
    expect(app.modal).toEqual({ kind: 'account', accountId: 'gemini-perso' });
    // The machine's: a key of the app's, nothing to sign in here (~/.gemini is never touched).
    const main = screen
      .getAllByRole('listitem', { name: 'Principal' })
      .find((l) => within(l).queryByText('Usage sur aistudio.google.com'))!;
    expect(within(main).queryByRole('button', { name: /Se connecter|Se reconnecter|Changer la clé/ })).not.toBeInTheDocument();
  });

  it('shows what an account of a harness without a quota spent today, in the place of the gauges', async () => {
    const OC: Account = { id: 'oc', name: 'Local', engine: 'codex', configDir: 'C:\\oc', claudePath: '', active: true };
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, OC] };
    app.usage = { ...app.usage, accounts: [usageOf('principal'), usageOf('oc', { engine: 'codex', meters: [], todayCost: 0.42 })] };
    backend();
    render(SettingsModal, { tab: 'accounts' });
    expect(within(row('Local')).getByText('0,42 $')).toBeInTheDocument();
    expect(within(row('Local')).getByText('0,42 $').parentElement).toHaveTextContent('Aujourd’hui 0,42 $');
    expect(within(row('Principal')).queryByText(/Aujourd’hui/)).not.toBeInTheDocument();
  });

  it('puts the name back when the backend refuses it', async () => {
    backend({ account_update: () => Promise.reject('Un compte s’appelle déjà « Principal ».') });
    render(SettingsModal, { tab: 'accounts' });
    const name = within(row('Pro')).getByRole('textbox', { name: 'Nom' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Principal{Enter}');
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['Un compte s’appelle déjà « Principal ».']));
    expect(name).toHaveValue('Pro');
  });

  it('puts the accounts in another order', async () => {
    const b = backend({ account_reorder: (a: { ids: string[] }) => a.ids.map((id) => (id === 'pro' ? PRO : PRINCIPAL)) });
    render(SettingsModal, { tab: 'accounts' });
    expect(within(row('Principal')).getByRole('button', { name: 'Monter Principal' })).toBeDisabled();
    expect(within(row('Pro')).getByRole('button', { name: 'Descendre Pro' })).toBeDisabled();
    await userEvent.click(within(row('Principal')).getByRole('button', { name: 'Descendre Principal' }));
    expect(b.called('account_reorder')[0].args.ids).toEqual(['pro', 'principal']);
    await waitFor(() => expect(names()).toEqual(['Pro', 'Principal']));
    await userEvent.click(within(row('Principal')).getByRole('button', { name: 'Monter Principal' }));
    expect(b.called('account_reorder')[1].args.ids).toEqual(['principal', 'pro']);
  });

  it('puts an account at rest, never the last one active', async () => {
    const b = backend({ account_update: (a: { account: Account }) => [PRINCIPAL, a.account] });
    render(SettingsModal, { tab: 'accounts' });
    const principal = within(row('Principal')).getByRole('switch', { name: 'Actif' });
    expect(principal).toBeEnabled();
    await userEvent.click(within(row('Pro')).getByRole('switch', { name: 'Actif' }));
    expect(b.called('account_update')[0].args.account).toEqual({ ...PRO, active: false });
    await waitFor(() => expect(within(row('Pro')).getByRole('switch', { name: 'Actif' })).toHaveAttribute('aria-checked', 'false'));
    // Principal is then the only one active.
    expect(principal).toBeDisabled();
    expect(principal).toHaveAccessibleDescription('Il faut au moins un compte actif par harness.');
  });

  it('keeps Principal to projects too, and says which projects no active account serves then', async () => {
    app.projects = [project({ id: 'p1', name: 'demo' }), project({ id: 'p2', name: 'infra' }), project({ id: 'p3', name: 'web' })];
    const pro = { ...PRO, active: false };
    app.settings.accounts = [PRINCIPAL, pro];
    const b = backend({ account_update: (a: { account: Account }) => [a.account, pro] });
    render(SettingsModal, { tab: 'accounts' });
    expect(screen.getByRole('status').textContent).toBe('');
    await userEvent.click(within(row('Principal')).getByRole('checkbox', { name: 'demo' }));
    expect(b.called('account_update')[0].args.account).toEqual({ ...PRINCIPAL, projects: ['p1'] });
    // Pro switched off: no account serves the others any more, and the tab says so.
    const status = screen.getByRole('status');
    await waitFor(() =>
      expect(status).toHaveTextContent(
        'Aucun compte actif de leur harness par défaut ne sert « infra » et « web » : leurs agents et leurs tickets ne démarrent pas.',
      ),
    );
    await userEvent.click(within(row('Principal')).getByRole('checkbox', { name: 'web' }));
    await waitFor(() =>
      expect(status).toHaveTextContent(
        'Aucun compte actif de son harness par défaut ne sert « infra » : ses agents et ses tickets ne démarrent pas.',
      ),
    );
  });

  it('does not remove an account agents still run on', async () => {
    const b = backend();
    resetApp({
      agents: [
        agent({ id: 'a1', account: 'pro' }),
        agent({ id: 'a2', account: 'pro' }),
        agent({ id: 'a3', account: 'pro', archived: true }),
      ],
    });
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, PRO] };
    app.modal = { kind: 'settings', tab: 'accounts' };
    render(SettingsModal, { tab: 'accounts' });
    await userEvent.click(within(row('Pro')).getByRole('button', { name: 'Supprimer' }));
    expect(within(row('Pro')).getByRole('alert')).toHaveTextContent('Le compte sert encore à 2 agents.');
    expect(app.modal?.kind).toBe('settings');
    delete app.agents.a2;
    await userEvent.click(within(row('Pro')).getByRole('button', { name: 'Supprimer' }));
    expect(within(row('Pro')).getByRole('alert')).toHaveTextContent('Le compte sert encore à 1 agent.');
    expect(b.called('account_remove')).toHaveLength(0);
  });

  it('removes an account once confirmed, saying its folder stays, then comes back to the tab', async () => {
    const b = backend({ account_remove: () => [PRINCIPAL] });
    render(SettingsModal, { tab: 'accounts' });
    await userEvent.click(within(row('Pro')).getByRole('button', { name: 'Supprimer' }));
    const modal = app.modal;
    if (modal?.kind !== 'confirm') throw new Error(`${modal?.kind}`);
    expect(modal.title).toBe('Supprimer le compte « Pro » ?');
    expect(modal.body).toBe(
      'Les nouveaux agents ne partiront plus dessus. Son dossier reste sur le disque, avec sa connexion et ses conversations : ~/.escouade/claude/pro',
    );
    expect(modal.danger).toBe(true);
    await modal.onConfirm(false);
    expect(b.called('account_remove')[0].args.id).toBe('pro');
    expect(app.settings.accounts).toEqual([PRINCIPAL]);
    expect(app.modal).toEqual({ kind: 'settings', tab: 'accounts', resume: true });
    // Cancelled: back to the tab, nothing removed.
    modal.onCancel?.();
    expect(app.modal).toEqual({ kind: 'settings', tab: 'accounts', resume: true });
  });

  it('removes the machine’s account once confirmed, saying the machine stays signed in (ESC-128)', async () => {
    const b = backend({ account_remove: () => [PRO] });
    render(SettingsModal, { tab: 'accounts' });
    await within(row('Principal')).findByText('Connecté · ada@atlas.dev');
    await userEvent.click(within(row('Principal')).getByRole('button', { name: 'Supprimer' }));
    const modal = app.modal;
    if (modal?.kind !== 'confirm') throw new Error(`${modal?.kind}`);
    expect(modal.title).toBe('Retirer le compte de la machine « Principal » ?');
    expect(modal.body).toBe(
      'Les nouveaux agents ne partiront plus dessus. Claude Code reste connecté sur cette machine : le retirer ne déconnecte rien (~/.claude).',
    );
    expect(modal.confirm).toBe('Retirer');
    await modal.onConfirm(false);
    expect(b.called('account_remove')[0].args.id).toBe('principal');
    expect(app.settings.accounts).toEqual([PRO]);
  });

  it('opens the window that adds an account, or signs one in', async () => {
    backend();
    render(SettingsModal, { tab: 'accounts' });
    // Each harness's group has its own (2.0 design): the window opens on that harness.
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter un compte Claude Code…' }));
    expect(app.modal).toEqual({ kind: 'account', engine: 'claude' });
    // (Rendered here by itself, the tab is still there.)
    await userEvent.click(await within(row('Pro')).findByRole('button', { name: 'Se connecter…' }));
    expect(app.modal).toEqual({ kind: 'account', accountId: 'pro' });
  });

  describe('and the projects closed with their history kept', () => {
    /** A closed project, as `closed_projects` lists it (or an entry it cannot read, with `unreadable`). */
    const closed = (id: string, name: string, over: Partial<ClosedProject> = {}): ClosedProject => ({
      id,
      name,
      color: 'oklch(0.72 0.12 300)',
      path: `C:\\code\\${name}`,
      worktreePerAgent: false,
      agents: 1,
      tickets: 1,
      lastActivity: 1,
      closedAt: 1,
      size: 1024,
      unreadable: false,
      ...over,
    });
    const checkboxes = (name: string) => within(row(name)).queryAllByRole('checkbox');

    beforeEach(() => {
      app.projects = [project({ id: 'p1', name: 'demo' })];
    });

    it('keeps one an account is reserved to in its list, ticked and marked closed, and unticks it like any other', async () => {
      app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['p1', 'c1'] }];
      const b = backend({
        closed_projects: () => [closed('c1', 'ancien'), closed('c2', 'vieux')],
        account_update: (a: { account: Account }) => [PRINCIPAL, a.account],
      });
      render(SettingsModal, { tab: 'accounts' });
      const old = await within(row('Pro')).findByRole('checkbox', { name: 'ancien (fermé)' });
      expect(old).toBeChecked();
      expect(within(row('Pro')).getByRole('checkbox', { name: 'demo' })).toBeChecked();
      // Only the closed projects an account is reserved to: not every closed project, and not on the other accounts.
      expect(within(row('Pro')).queryByRole('checkbox', { name: 'vieux (fermé)' })).not.toBeInTheDocument();
      expect(within(row('Principal')).queryByRole('checkbox', { name: 'ancien (fermé)' })).not.toBeInTheDocument();
      expect(b.called('closed_projects')).toHaveLength(1);
      // The same save as for a project that is open, the closed one gone from the list.
      await userEvent.click(old);
      expect(b.called('account_update')[0].args.account).toEqual({ ...PRO, projects: ['p1'] });
      await waitFor(() => expect(within(row('Pro')).queryByRole('checkbox', { name: 'ancien (fermé)' })).not.toBeInTheDocument());
    });

    it('lights the square of a chip as its box is ticked, open or closed project, and puts it out when unticked', async () => {
      app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['p1', 'c1'] }];
      backend({
        closed_projects: () => [closed('c1', 'ancien')],
        account_update: (a: { account: Account }) => [PRINCIPAL, a.account],
      });
      render(SettingsModal, { tab: 'accounts' });
      const dot = (box: HTMLElement) => box.closest('label')!.querySelector('i')!;
      const old = await within(row('Pro')).findByRole('checkbox', { name: 'ancien (fermé)' });
      const open = within(row('Pro')).getByRole('checkbox', { name: 'demo' });
      expect(dot(open)).toHaveClass('lit');
      expect(dot(old)).toHaveClass('lit');
      expect(dot(open)).toHaveAttribute('aria-hidden', 'true');
      expect(dot(old)).toHaveAttribute('aria-hidden', 'true');
      await userEvent.click(open);
      await waitFor(() => expect(dot(open)).not.toHaveClass('lit'));
      expect(dot(old)).toHaveClass('lit');
      await userEvent.click(open);
      await waitFor(() => expect(dot(open)).toHaveClass('lit'));
      await userEvent.click(old);
      await waitFor(() => expect(within(row('Pro')).queryByRole('checkbox', { name: 'ancien (fermé)' })).not.toBeInTheDocument());
    });

    it('lights every square while nothing is reserved (all the projects), then only those of the projects reserved', async () => {
      app.projects = [project({ id: 'p1', name: 'demo' }), project({ id: 'p2', name: 'autre' })];
      app.settings.accounts = [PRINCIPAL, PRO];
      backend({ account_update: (a: { account: Account }) => [PRINCIPAL, a.account] });
      render(SettingsModal, { tab: 'accounts' });
      const dot = (name: string) => within(row('Pro')).getByRole('checkbox', { name }).closest('label')!.querySelector('i')!;
      expect(dot('demo')).toHaveClass('lit');
      expect(dot('autre')).toHaveClass('lit');
      await userEvent.click(within(row('Pro')).getByRole('checkbox', { name: 'demo' }));
      await waitFor(() => expect(dot('autre')).not.toHaveClass('lit'));
      expect(dot('demo')).toHaveClass('lit');
    });

    it('shows the list of an account reserved to a closed project alone, though no project is open', async () => {
      app.projects = [];
      app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['c1'] }];
      backend({ closed_projects: () => [closed('c1', 'ancien')] });
      render(SettingsModal, { tab: 'accounts' });
      expect(await within(row('Pro')).findByRole('checkbox', { name: 'ancien (fermé)' })).toBeChecked();
      expect(within(row('Pro')).getByRole('group', { name: 'Réservé aux projets' })).toBeInTheDocument();
      expect(within(row('Pro')).queryByText(/Tous les projets/)).not.toBeInTheDocument();
      // Principal has nothing to be reserved to: no list, as before.
      expect(within(row('Principal')).queryByRole('group', { name: 'Réservé aux projets' })).not.toBeInTheDocument();
    });

    it('says « Tous les projets » only when nothing at all is reserved, a closed project included', async () => {
      app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['c1'] }];
      backend({ closed_projects: () => [closed('c1', 'ancien')] });
      render(SettingsModal, { tab: 'accounts' });
      await within(row('Pro')).findByRole('checkbox', { name: 'ancien (fermé)' });
      expect(within(row('Pro')).queryByText(/Tous les projets/)).not.toBeInTheDocument();
      expect(within(row('Principal')).getByText('Tous les projets')).toBeInTheDocument();
      // Unticked, the last one: the account is open to every project again.
      app.settings.accounts = [PRINCIPAL, PRO];
      await waitFor(() => expect(within(row('Pro')).getByText('Tous les projets')).toBeInTheDocument());
    });

    it('shows nothing of a project it has no name for: gone from both lists, or a file the app cannot read', async () => {
      app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['p1', 'gone', 'bad'] }];
      const b = backend({ closed_projects: () => [closed('bad', '', { unreadable: true, path: '', color: '' })] });
      render(SettingsModal, { tab: 'accounts' });
      await waitFor(() => expect(b.called('closed_projects')).toHaveLength(1));
      await new Promise((r) => setTimeout(r));
      expect(
        within(row('Pro'))
          .getAllByRole('checkbox')
          .map((c) => c.closest('label')?.textContent?.trim()),
      ).toEqual(['demo']);
      expect(checkboxes('Pro')).toHaveLength(1);
    });

    it('is the same list as before when the closed projects cannot be read', async () => {
      app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['p1', 'c1'] }];
      backend({
        closed_projects: () => {
          throw new Error('disque illisible');
        },
      });
      render(SettingsModal, { tab: 'accounts' });
      expect(await within(row('Pro')).findByRole('checkbox', { name: 'demo' })).toBeChecked();
      await new Promise((r) => setTimeout(r));
      expect(checkboxes('Pro')).toHaveLength(1);
      expect(app.toasts).toEqual([]);
    });

    it('is written in English', async () => {
      setLang('en');
      app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['c1'] }];
      backend({ closed_projects: () => [closed('c1', 'ancien')] });
      render(SettingsModal, { tab: 'accounts' });
      expect(await within(row('Pro')).findByRole('checkbox', { name: 'ancien (closed)' })).toBeChecked();
    });
  });

  it('reads in English, Principal by its English name until it is renamed', async () => {
    setLang('en');
    backend();
    render(SettingsModal, { tab: 'accounts' });
    expect(screen.getByRole('tab', { name: 'Accounts' })).toBeInTheDocument();
    expect(names()).toEqual(['Main', 'Pro']);
    expect(await within(row('Main')).findByText('Signed in · ada@atlas.dev')).toBeInTheDocument();
    expect(await within(row('Pro')).findByText('Not signed in')).toBeInTheDocument();
    expect(within(row('Pro')).getByRole('textbox', { name: 'Executable' })).toHaveAttribute('placeholder', 'The one in the settings');
    expect(screen.getByRole('button', { name: 'Add a Claude Code account…' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'New agents start on the first active account of their harness under the “Pause above quota” and “Switch accounts at” thresholds, in this order.',
      ),
    ).toBeInTheDocument();
    expect(within(row('Main')).getByText('This machine’s account')).toBeInTheDocument();
    // What its new agents go with: the choices are read at each render, in the language of the interface.
    const defaults = within(within(row('Pro')).getByRole('group', { name: 'New agents' }));
    const options = (name: string) =>
      within(defaults.getByRole('combobox', { name }))
        .getAllByRole('option')
        .map((o) => o.textContent);
    expect(options('Effort')).toEqual(['The harness’s', 'Low', 'Medium', 'High', 'Very high', 'Max']);
    expect(options('Mode')).toEqual(['The harness’s', 'Auto', 'Ask', 'Plan', 'Accept edits', 'Bypass']);
    expect(options('Model')[0]).toBe('The harness’s (Sonnet)');
    app.settings.accounts = [{ ...PRINCIPAL, name: 'Perso' }, PRO];
    await waitFor(() => expect(names()).toEqual(['Perso', 'Pro']));
  });
});

describe('AccountsTab with two harnesses (ESC-30)', () => {
  const CODEX_MAIN: Account = { id: 'principal-codex', name: 'Principal', engine: 'codex', configDir: '', claudePath: '', active: true };
  const WORK: Account = { id: 'work', name: 'Travail', engine: 'codex', configDir: 'C:\codex\work', claudePath: '', active: true };
  const statuses = () => ({ connected: true, email: null, dir: '', stamp: null });

  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, PRO, CODEX_MAIN, WORK], engineOrder: ['claude', 'codex'] };
    app.modal = { kind: 'settings', tab: 'accounts' };
  });

  const list = (name: string) => screen.getByRole('list', { name });
  const items = (name: string) =>
    within(list(name))
      .getAllByRole('listitem')
      .map((li) => li.getAttribute('aria-label') ?? li.textContent?.trim());

  it('groups the accounts by harness, each kept to its own harness', async () => {
    const b = fakeBackend({
      account_status: statuses,
      save_settings: () => [],
      account_reorder: (a: { ids: string[] }) => a.ids.map((id) => app.settings.accounts.find((x) => x.id === id)),
    });
    render(SettingsModal, { tab: 'accounts' });
    expect(items('Comptes Claude Code')).toEqual(['Principal', 'Pro']);
    expect(items('Comptes Harness nu')).toEqual(['Principal', 'Travail']);
    // The machine's account of each harness goes like the others (ESC-128).
    const codex = list('Comptes Harness nu');
    expect(
      within(within(codex).getByRole('listitem', { name: 'Principal' })).getByRole('button', { name: 'Supprimer' }),
    ).toBeInTheDocument();
    expect(within(within(codex).getByRole('listitem', { name: 'Travail' })).getByRole('button', { name: 'Supprimer' })).toBeInTheDocument();
    // An account goes up among its harness's: never past another harness's.
    expect(within(codex).getByRole('button', { name: 'Monter Principal' })).toBeDisabled();
    await userEvent.click(within(codex).getByRole('button', { name: 'Monter Travail' }));
    await waitFor(() => expect(b.called('account_reorder')).toHaveLength(1));
    expect(b.called('account_reorder')[0].args.ids).toEqual(['principal', 'pro', 'work', 'principal-codex']);
  });

  it('shows each harness installed, one without an account with « Aucun compte » and an add button on it (ESC-128)', async () => {
    app.enginesFound = ['claude', 'codex'];
    app.settings.accounts = [PRO];
    fakeBackend({ account_status: statuses, save_settings: () => [] });
    render(SettingsModal, { tab: 'accounts' });
    expect(items('Comptes Claude Code')).toEqual(['Pro']);
    const empty = screen.getByRole('group', { name: 'Comptes Harness nu' });
    expect(within(empty).getByText('Aucun compte')).toBeInTheDocument();
    expect(within(empty).getByText('Aucun agent ni ticket ne démarre sur Harness nu avant qu’un compte soit ajouté.')).toBeInTheDocument();
    await userEvent.click(
      within(screen.getByRole('region', { name: 'Comptes Harness nu' })).getByRole('button', { name: 'Ajouter un compte Harness nu…' }),
    );
    expect(app.modal).toEqual({ kind: 'account', engine: 'codex' });
  });

  it('shows « Aucun compte » for every harness installed on a first installation, and nothing of one not installed', () => {
    app.enginesFound = ['claude'];
    app.settings.accounts = [];
    app.settings.engineOrder = [];
    fakeBackend({ account_status: statuses, save_settings: () => [] });
    render(SettingsModal, { tab: 'accounts' });
    expect(within(screen.getByRole('group', { name: 'Comptes Claude Code' })).getByText('Aucun compte')).toBeInTheDocument();
    expect(screen.queryByRole('group', { name: 'Comptes Harness nu' })).not.toBeInTheDocument();
    expect(screen.queryByRole('list', { name: 'Comptes Claude Code' })).not.toBeInTheDocument();
  });

  it('sets what the new agents of an account go with, among what its own harness has (ESC-128)', async () => {
    const b = fakeBackend({
      account_status: statuses,
      save_settings: () => [],
      account_update: (a: { account: Account }) => app.settings.accounts.map((x) => (x.id === a.account.id ? a.account : x)),
    });
    render(SettingsModal, { tab: 'accounts' });
    const work = within(list('Comptes Harness nu')).getByRole('listitem', { name: 'Travail' });
    const defaults = within(within(work).getByRole('group', { name: 'Nouveaux agents' }));
    // The models of its harness, nothing of Claude Code's; its modes; no effort, which it takes none of.
    const model = defaults.getByRole('combobox', { name: 'Modèle' });
    expect(
      within(model)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual([
      // No model of its own by default: the one its harness runs, the first it declares here.
      'Celui du harness (GPT-5.5)',
      'GPT-5.5',
      'GPT-5.5 mini',
    ]);
    expect(defaults.queryByRole('combobox', { name: 'Effort' })).not.toBeInTheDocument();
    const mode = defaults.getByRole('combobox', { name: 'Mode' });
    expect(
      within(mode)
        .getAllByRole('option')
        .map((o) => (o as HTMLOptionElement).value),
    ).toEqual(['', 'default', 'plan']);
    await userEvent.selectOptions(model, 'gpt-5.5-mini');
    await waitFor(() => expect(b.called('account_update')).toHaveLength(1));
    expect(b.called('account_update')[0].args.account).toMatchObject({ id: 'work', defaultModel: 'gpt-5.5-mini' });
    await userEvent.selectOptions(mode, 'plan');
    await waitFor(() => expect(b.called('account_update')).toHaveLength(2));
    expect(b.called('account_update')[1].args.account).toMatchObject({ id: 'work', defaultModel: 'gpt-5.5-mini', defaultMode: 'plan' });
    // Claude Code's account: Claude Code's models, efforts and modes.
    const pro = within(list('Comptes Claude Code')).getByRole('listitem', { name: 'Pro' });
    const claude = within(within(pro).getByRole('group', { name: 'Nouveaux agents' }));
    expect(within(claude.getByRole('combobox', { name: 'Modèle' })).getByRole('option', { name: 'Sonnet' })).toBeInTheDocument();
    expect(claude.getByRole('combobox', { name: 'Effort' })).toBeInTheDocument();
    expect(within(claude.getByRole('combobox', { name: 'Mode' })).getByRole('option', { name: 'Auto' })).toBeInTheDocument();
  });

  it('keeps one account of each harness active, whatever the other harness has', () => {
    app.settings.accounts = [PRINCIPAL, PRO, CODEX_MAIN, { ...WORK, active: false }];
    fakeBackend({ account_status: statuses, save_settings: () => [] });
    render(SettingsModal, { tab: 'accounts' });
    const codex = list('Comptes Harness nu');
    expect(within(within(codex).getByRole('listitem', { name: 'Principal' })).getByRole('switch', { name: 'Actif' })).toBeDisabled();
    const claude = list('Comptes Claude Code');
    expect(within(within(claude).getByRole('listitem', { name: 'Pro' })).getByRole('switch', { name: 'Actif' })).toBeEnabled();
  });

  it('sets the order of the harnesses, which gives a project its fallback harnesses', async () => {
    const b = fakeBackend({ account_status: statuses, save_settings: () => [], engine_order_set: (a: { order: string[] }) => a.order });
    render(SettingsModal, { tab: 'accounts' });
    expect(screen.getByText(/Après son harness par défaut, un projet passe aux suivants dans cet ordre/)).toBeInTheDocument();
    const order = list('Ordre des harness');
    expect(
      within(order)
        .getAllByRole('listitem')
        .map((li) => li.textContent?.replace(/[↑↓\s]+/g, ' ').trim()),
    ).toEqual(['1 Claude Code', '2 Harness nu']);
    expect(within(order).getByRole('button', { name: 'Monter Claude Code' })).toBeDisabled();
    await userEvent.click(within(order).getByRole('button', { name: 'Monter Harness nu' }));
    await waitFor(() => expect(b.called('engine_order_set')).toHaveLength(1));
    expect(b.called('engine_order_set')[0].args.order).toEqual(['codex', 'claude']);
    expect(app.settings.engineOrder).toEqual(['codex', 'claude']);
    // The groups follow it.
    await waitFor(() =>
      expect(
        screen
          .getAllByRole('list')
          .map((l) => l.getAttribute('aria-label'))
          .slice(0, 2),
      ).toEqual(['Comptes Harness nu', 'Comptes Claude Code']),
    );
  });

  it('shows no order with a single harness', () => {
    app.engines = [CLAUDE_ENGINE];
    app.settings.accounts = [PRINCIPAL, PRO];
    fakeBackend({ account_status: statuses, save_settings: () => [] });
    render(SettingsModal, { tab: 'accounts' });
    expect(screen.queryByRole('list', { name: 'Ordre des harness' })).not.toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Comptes Claude Code' })).toBeInTheDocument();
  });

  it('is written in English', () => {
    setLang('en');
    fakeBackend({ account_status: statuses, save_settings: () => [] });
    render(SettingsModal, { tab: 'accounts' });
    expect(screen.getByRole('list', { name: 'Claude Code accounts' })).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Harness order' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Move Harness nu up' })).toBeInTheDocument();
  });
});

describe('AccountsTab: an account a private project shares (ESC-141)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project({ id: 'p1', name: 'client', private: true }), project({ id: 'p2', name: 'site' })] });
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, PRO] };
    app.modal = { kind: 'settings', tab: 'accounts' };
  });

  it('says a private project still goes to it, and reserves it to that project alone', async () => {
    const b = backend({ account_update: ({ account }) => [PRINCIPAL, account] });
    render(SettingsModal, { tab: 'accounts' });
    const said = 'Le projet privé « client » part encore sur ce compte, partagé avec d’autres projets.';
    expect(within(row('Principal')).getByText(said)).toBeVisible();
    expect(within(row('Pro')).getByText(said)).toBeVisible();
    await userEvent.click(within(row('Pro')).getByRole('button', { name: 'Réserver à « client »' }));
    expect(b.called('account_update')[0].args.account).toEqual({ ...PRO, projects: ['p1'] });
    // Its own now: neither account says it any more (it goes to Pro alone).
    await waitFor(() => expect(screen.queryByText(said)).not.toBeInTheDocument());
  });

  it('says nothing of an account reserved to the private project alone, nor of a project that is not private', () => {
    backend();
    app.settings.accounts = [PRINCIPAL, { ...PRO, projects: ['p1'] }];
    app.projects[0].sharedOnLimit = false;
    render(SettingsModal, { tab: 'accounts' });
    expect(screen.queryByText(/part encore sur ce compte/)).not.toBeInTheDocument();
  });
});
