import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// The sign-in's terminal is xterm.js: none in jsdom.
const term = vi.hoisted(() => ({
  opened: [] as string[],
  mounted: [] as (string | null)[],
  disposed: [] as string[],
  next: 1,
}));
vi.mock('../../lib/terminals', () => ({
  openAccountLogin: vi.fn(async (accountId: string) => {
    term.opened.push(accountId);
    return { id: `term-${term.next++}`, projectId: '', name: accountId, shell: 'claude' };
  }),
  mountTerminal: vi.fn((id: string, el: HTMLElement | null) => term.mounted.push(el ? id : null)),
  disposeTerminal: vi.fn((id: string) => term.disposed.push(id)),
}));

import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { Account, AccountStatus } from '../../lib/types';
import {
  AGY_ENGINE,
  BARE_ENGINE,
  CURSOR_ENGINE,
  CLAUDE_ENGINE,
  GEMINI_ENGINE,
  VIBE_ENGINE,
  fakeBackend,
  resetApp,
  SETTINGS,
} from '../../test/ipc';
import AccountModal from './AccountModal.svelte';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\Users\\ada\\.escouade\\claude\\pro', claudePath: '', active: true };
const SHAREABLE = ['settings.json', 'CLAUDE.md', 'skills', 'agents'];
const OUT: AccountStatus = { connected: false, email: null, dir: PRO.configDir, stamp: null };
const IN: AccountStatus = { connected: true, email: 'ada@pro.dev', dir: PRO.configDir, stamp: 'new-sign-in' };
/** Signed in before the user does it again (the token out of date, or not): another stamp than IN's. */
const BEFORE: AccountStatus = { connected: true, email: 'ada@pro.dev', dir: PRO.configDir, stamp: 'old-sign-in' };

/** A backend whose `account_status` answers `answers` in turn (the last one again after). */
function backend(answers: AccountStatus[] = [OUT], over: Record<string, (a: any) => unknown> = {}) {
  let n = 0;
  return fakeBackend({
    account_shareable: () => SHAREABLE,
    account_create: () => PRO,
    account_status: () => answers[Math.min(n++, answers.length - 1)],
    refresh_usage: () => undefined,
    ...over,
  });
}

const HINT = 'Connecte-toi dans le terminal ci-dessous (commande /login si Claude Code ne la propose pas).';

describe('AccountModal', () => {
  beforeEach(() => {
    resetApp();
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL] };
    app.modal = { kind: 'account' };
    term.opened.length = 0;
    term.mounted.length = 0;
    term.disposed.length = 0;
    term.next = 1;
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });
  afterEach(() => vi.useRealTimers());

  it('offers what Principal has to share, all of it checked and linked; on Windows, its files are copied', async () => {
    backend();
    render(AccountModal, {});
    const dialog = screen.getByRole('dialog', { name: 'Ajouter un compte Claude Code' });
    const share = await within(dialog).findByRole('group', { name: 'Partager avec Principal' });
    const boxes = within(share).getAllByRole('checkbox');
    expect(boxes.map((b) => b.getAttribute('name'))).toEqual(SHAREABLE);
    expect(boxes.every((b) => (b as HTMLInputElement).checked)).toBe(true);
    // Only what Principal has.
    expect(within(share).queryByRole('checkbox', { name: 'plugins' })).not.toBeInTheDocument();
    expect(within(share).getByRole('radio', { name: 'Lier (un changement vaut pour les deux comptes)' })).toBeChecked();
    expect(within(share).getByRole('radio', { name: 'Copier' })).not.toBeChecked();
    expect(within(share).getByText('copiés (un lien de fichier demande des droits d’administrateur)')).toBeInTheDocument();
    // Copied anyway: nothing to say.
    await userEvent.click(within(share).getByRole('radio', { name: 'Copier' }));
    expect(within(share).queryByText('copiés (un lien de fichier demande des droits d’administrateur)')).not.toBeInTheDocument();
    expect(
      within(dialog).getByText('La connexion (.credentials.json) et .claude.json ne sont jamais partagés : chaque compte a les siens.'),
    ).toBeInTheDocument();
    // A name first.
    const create = within(dialog).getByRole('button', { name: 'Créer et se connecter' });
    expect(create).toBeDisabled();
    await userEvent.type(within(dialog).getByRole('textbox', { name: 'Nom' }), '  ');
    expect(create).toBeDisabled();
  });

  it('says when Principal has nothing to share', async () => {
    backend([OUT], { account_shareable: () => [] });
    app.settings.accounts = [{ ...PRINCIPAL, name: 'Perso' }];
    render(AccountModal, {});
    expect(
      await screen.findByText(
        'Perso n’a ni réglages, ni CLAUDE.md, ni skills, agents, commandes, plugins, hooks ou styles de sortie à partager.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole('group')).not.toBeInTheDocument();
  });

  it('adds an account of Codex, picked among the harnesses found, with what its Principal shares and never its sign-in', async () => {
    app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, name: 'Codex' }];
    app.enginesFound = ['claude', 'codex'];
    const WORK: Account = {
      ...PRO,
      id: 'travail',
      name: 'Travail',
      engine: 'codex',
      configDir: 'C:\\Users\\ada\\.escouade\\codex\\travail',
    };
    const b = backend([OUT], {
      account_shareable: (a) => (a.engine === 'codex' ? ['config.toml', 'AGENTS.md', 'skills'] : SHAREABLE),
      account_create: () => WORK,
    });
    render(AccountModal, {});
    expect(await screen.findByRole('dialog', { name: 'Ajouter un compte Claude Code' })).toBeInTheDocument();
    // The harnesses in segments, as the preview draws them.
    const harness = screen.getByRole('radiogroup', { name: 'Harness' });
    expect(
      within(harness)
        .getAllByRole('radio')
        .map((r) => r.textContent),
    ).toEqual(['Claude Code', 'Codex']);
    await userEvent.click(within(harness).getByRole('radio', { name: 'Codex' }));
    expect(within(harness).getByRole('radio', { name: 'Codex' })).toBeChecked();
    expect(await screen.findByRole('dialog', { name: 'Ajouter un compte Codex' })).toBeInTheDocument();
    await waitFor(() => expect(b.called('account_shareable').at(-1)?.args).toEqual({ engine: 'codex' }));
    const group = await screen.findByRole('group', { name: 'Partager avec le compte de la machine' });
    // config.toml and AGENTS.md are files, skills a folder.
    expect(within(group).getByRole('checkbox', { name: 'config.toml' })).toBeChecked();
    expect(within(group).getByRole('checkbox', { name: 'skills' })).toBeChecked();
    expect(screen.getByText(/La connexion \(auth\.json\) n’est jamais copiée/)).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Travail');
    await userEvent.click(screen.getByRole('button', { name: 'Créer et se connecter' }));
    await waitFor(() => expect(b.called('account_create')).toHaveLength(1));
    expect(b.called('account_create')[0].args).toEqual({
      name: 'Travail',
      share: ['config.toml', 'AGENTS.md', 'skills'],
      mode: 'link',
      engine: 'codex',
    });
    // Its `codex login` in a terminal, said so.
    await screen.findByRole('dialog', { name: 'Se connecter au compte « Travail »' });
    expect(term.opened).toEqual(['travail']);
    expect(screen.getByRole('status')).toHaveTextContent('codex login ouvre ton navigateur');
  });

  it('adds an account of Copilot CLI, sharing its MCP servers, instructions, agents and skills, signed in by copilot login', async () => {
    app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, id: 'copilot', name: 'Copilot CLI' }];
    app.enginesFound = ['claude', 'copilot'];
    const WORK: Account = {
      ...PRO,
      id: 'travail',
      name: 'Travail',
      engine: 'copilot',
      configDir: 'C:\Users\ada\.escouade\copilot\travail',
    };
    const SHARED = ['mcp-config.json', 'copilot-instructions.md', 'agents', 'skills'];
    const b = backend([OUT], {
      account_shareable: (a) => (a.engine === 'copilot' ? SHARED : SHAREABLE),
      account_create: () => WORK,
    });
    render(AccountModal, { engine: 'copilot' });
    expect(await screen.findByRole('dialog', { name: 'Ajouter un compte Copilot CLI' })).toBeInTheDocument();
    await waitFor(() => expect(b.called('account_shareable').at(-1)?.args).toEqual({ engine: 'copilot' }));
    const group = await screen.findByRole('group', { name: 'Partager avec le compte de la machine' });
    for (const name of SHARED) expect(within(group).getByRole('checkbox', { name })).toBeChecked();
    expect(screen.getByText(/La connexion n’est jamais copiée : chaque compte se connecte par copilot login/)).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Travail');
    await userEvent.click(screen.getByRole('button', { name: 'Créer et se connecter' }));
    await waitFor(() => expect(b.called('account_create')).toHaveLength(1));
    expect(b.called('account_create')[0].args).toEqual({ name: 'Travail', share: SHARED, mode: 'link', engine: 'copilot' });
    // Its `copilot login` in a terminal of its folder, said so.
    await screen.findByRole('dialog', { name: 'Se connecter au compte « Travail »' });
    expect(term.opened).toEqual(['travail']);
    expect(screen.getByRole('status')).toHaveTextContent('copilot login donne un code');
  });

  describe('a Cursor account by its API key (ESC-43)', () => {
    const KEYED: Account = {
      ...PRO,
      id: 'equipe',
      name: 'Équipe',
      engine: 'cursor',
      configDir: 'C:\\Users\\ada\\.escouade\\cursor\\equipe',
    };
    const KEPT: AccountStatus = { connected: true, email: 'clé …abcd', dir: KEYED.configDir, stamp: null };

    beforeEach(() => {
      app.engines = [CLAUDE_ENGINE, CURSOR_ENGINE];
      app.enginesFound = ['claude', 'cursor'];
    });

    it('adds a new one by its name and key, kept in the keychain: no terminal, no sign-in', async () => {
      const b = backend([OUT], {
        account_shareable: () => [],
        account_machine: () => ({ found: true, added: true, connected: true, signsIn: true, email: null, dir: '' }),
        account_create: () => KEYED,
        account_key: () => KEPT,
      });
      render(AccountModal, { engine: 'cursor' });
      expect(await screen.findByRole('dialog', { name: 'Ajouter un compte Cursor CLI' })).toBeInTheDocument();
      expect(screen.getByText('La clé d’API d’un autre compte Cursor, gardée dans le trousseau du système.')).toBeInTheDocument();
      // A key, hidden as it is typed; nothing to share.
      const key = screen.getByLabelText('Clé d’API');
      expect(key).toHaveAttribute('type', 'password');
      expect(screen.queryByRole('group', { name: /Partager/ })).not.toBeInTheDocument();
      expect(screen.getByText(/jamais écrite dans un fichier ni sur une ligne de commande/)).toBeInTheDocument();
      await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Équipe');
      const add = screen.getByRole('button', { name: 'Ajouter le compte' });
      expect(add).toBeDisabled();
      await userEvent.type(key, 'cle-essai-0000000000abcd');
      await userEvent.click(add);
      await waitFor(() => expect(b.called('account_key')).toHaveLength(1));
      expect(b.called('account_create')[0].args).toEqual({ name: 'Équipe', share: [], mode: 'link', engine: 'cursor' });
      expect(b.called('account_key')[0].args).toEqual({ id: 'equipe', key: 'cle-essai-0000000000abcd' });
      expect(term.opened).toEqual([]);
      expect(app.modal).toEqual({ kind: 'settings', page: 'accounts', resume: true });
    });

    it('gives the key again by « Changer la clé… », and says why one is refused', async () => {
      app.settings.accounts = [PRINCIPAL, KEYED];
      let n = 0;
      const b = backend([KEPT], {
        account_key: () => (n++ === 0 ? Promise.reject('Ce n’est pas une clé d’API de Cursor') : KEPT),
      });
      render(AccountModal, { accountId: 'equipe' });
      expect(await screen.findByRole('dialog', { name: 'Clé d’API du compte « Équipe »' })).toBeInTheDocument();
      expect(term.opened).toEqual([]);
      await userEvent.type(screen.getByLabelText('Clé d’API'), 'courte');
      await userEvent.click(screen.getByRole('button', { name: 'Enregistrer la clé' }));
      expect(await screen.findByRole('alert')).toHaveTextContent('Ce n’est pas une clé d’API de Cursor');
      expect(app.modal).toEqual({ kind: 'account' });
      await userEvent.clear(screen.getByLabelText('Clé d’API'));
      await userEvent.type(screen.getByLabelText('Clé d’API'), 'cle-essai-0000000000abcd{Enter}');
      await waitFor(() => expect(b.called('account_key')).toHaveLength(2));
      expect(app.modal).toEqual({ kind: 'settings', page: 'accounts', resume: true });
    });
  });

  describe('a Mistral Vibe account by its API key (ESC-267)', () => {
    const KEYED: Account = {
      ...PRO,
      id: 'vibe-equipe',
      name: 'Équipe',
      engine: 'vibe',
      configDir: 'C:\\Users\\ada\\.escouade\\vibe\\equipe',
    };
    const KEPT: AccountStatus = { connected: true, email: 'clé …wxyz', dir: KEYED.configDir, stamp: null };

    beforeEach(() => {
      app.engines = [CLAUDE_ENGINE, VIBE_ENGINE];
      app.enginesFound = ['claude', 'vibe'];
    });

    it('adds a new one by its name and its Mistral key, kept in the keychain: no terminal', async () => {
      const b = backend([OUT], {
        account_shareable: () => [],
        account_machine: () => ({ found: true, added: true, connected: true, signsIn: true, email: null, dir: '' }),
        account_create: () => KEYED,
        account_key: () => KEPT,
      });
      render(AccountModal, { engine: 'vibe' });
      expect(await screen.findByRole('dialog', { name: 'Ajouter un compte Mistral Vibe' })).toBeInTheDocument();
      expect(screen.getByText('La clé d’API d’un autre compte Mistral, gardée dans le trousseau du système.')).toBeInTheDocument();
      const key = screen.getByLabelText('Clé d’API');
      expect(key).toHaveAttribute('type', 'password');
      expect(key).toHaveAttribute('placeholder', 'Sur console.mistral.ai, API Keys');
      expect(screen.getByText(/donnée aux agents de ce compte par MISTRAL_API_KEY/)).toBeInTheDocument();
      await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Équipe');
      await userEvent.type(key, 'cle-mistral-000000000wxyz');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter le compte' }));
      await waitFor(() => expect(b.called('account_key')).toHaveLength(1));
      expect(b.called('account_create')[0].args).toEqual({ name: 'Équipe', share: [], mode: 'link', engine: 'vibe' });
      expect(b.called('account_key')[0].args).toEqual({ id: 'vibe-equipe', key: 'cle-mistral-000000000wxyz' });
      expect(term.opened).toEqual([]);
      expect(app.modal).toEqual({ kind: 'settings', page: 'accounts', resume: true });
    });

    it('may leave its key empty when the Mistral account has one (ESC-327)', async () => {
      const b = backend([OUT], {
        account_shareable: () => [],
        account_machine: () => ({ found: true, added: true, connected: true, signsIn: true, email: null, dir: '' }),
        account_create: () => KEYED,
        voice_mistral_key: () => ({ own: 'clé …wxyz', borrowed: null }),
      });
      render(AccountModal, { engine: 'vibe' });
      const key = await screen.findByLabelText('Clé d’API');
      await waitFor(() => expect(key).toHaveAttribute('placeholder', 'Vide : la clé Mistral de Réglages › Voix (clé …wxyz)'));
      await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Équipe');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter le compte' }));
      await waitFor(() => expect(b.called('account_create')).toHaveLength(1));
      expect(b.called('account_key')).toHaveLength(0);
      await waitFor(() => expect(app.modal).toEqual({ kind: 'settings', page: 'accounts', resume: true }));
    });

    it('asks its key while the Mistral account has none', async () => {
      backend([OUT], {
        account_shareable: () => [],
        account_machine: () => ({ found: true, added: true, connected: true, signsIn: true, email: null, dir: '' }),
        voice_mistral_key: () => ({ own: null, borrowed: null }),
      });
      render(AccountModal, { engine: 'vibe' });
      await userEvent.type(await screen.findByRole('textbox', { name: 'Nom' }), 'Équipe');
      expect(screen.getByRole('button', { name: 'Ajouter le compte' })).toBeDisabled();
    });
  });

  describe('a Gemini CLI account (ESC-275)', () => {
    const GOOGLE: Account = {
      ...PRO,
      id: 'gemini-perso',
      name: 'Perso',
      engine: 'gemini',
      configDir: 'C:\\Users\\ada\\.escouade\\gemini\\perso',
    };
    const KEYED: Account = { ...PRO, id: 'gemini-cle', name: 'Clé', engine: 'gemini', configDir: 'C:\\Users\\ada\\.escouade\\gemini\\cle' };
    const apart = {
      found: true,
      added: false,
      connected: false,
      signsIn: true,
      signsInLater: false,
      apart: true,
      email: null,
      dir: 'C:\\Users\\ada\\.gemini',
    };
    const SIGNED: AccountStatus = { connected: true, email: 'moi@exemple.fr', dir: GOOGLE.configDir, stamp: 't1' };

    beforeEach(() => {
      app.engines = [CLAUDE_ENGINE, GEMINI_ENGINE];
      app.enginesFound = ['claude', 'gemini'];
      app.settings.accounts = [];
    });

    it('says why the machine’s account is not offered, and signs a new one in to Google by gemini in a terminal', async () => {
      const b = backend([OUT, OUT, SIGNED], {
        account_machine: () => apart,
        account_shareable: () => [],
        account_create: () => GOOGLE,
      });
      render(AccountModal, { engine: 'gemini' });
      expect(await screen.findByText(/Gemini CLI garde sa connexion Google dans ~\/.gemini/)).toBeInTheDocument();
      expect(screen.getByRole('radio', { name: /Le compte de la machine/ })).toBeDisabled();
      await waitFor(() => expect(screen.getByRole('radio', { name: /Un nouveau compte/ })).toBeChecked());
      expect(screen.getByRole('radio', { name: /Compte Google/ })).toBeChecked();
      expect(screen.queryByLabelText('Clé d’API')).not.toBeInTheDocument();
      await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Perso');
      await userEvent.click(screen.getByRole('button', { name: 'Créer et se connecter' }));
      await waitFor(() => expect(term.opened).toEqual(['gemini-perso']));
      expect(b.called('account_key')).toHaveLength(0);
      const dialog = await screen.findByRole('dialog', { name: 'Se connecter au compte « Perso »' });
      const said = within(dialog).getByRole('status');
      expect(said).toHaveTextContent('gemini ouvre ton navigateur sur ton compte Google. Escouade ne voit jamais tes identifiants.');
      await vi.advanceTimersByTimeAsync(4000);
      await waitFor(() => expect(said).toHaveTextContent('Connecté au compte moi@exemple.fr'));
    });

    it('adds a new one by its Gemini API key, which it goes by from then on', async () => {
      const b = backend([OUT], {
        account_machine: () => apart,
        account_shareable: () => [],
        account_create: () => KEYED,
        account_key: () => ({ connected: true, email: 'clé …wxyz', dir: KEYED.configDir, stamp: null }),
      });
      render(AccountModal, { engine: 'gemini' });
      await userEvent.click(await screen.findByRole('radio', { name: /Clé d’API/ }));
      const key = screen.getByLabelText('Clé d’API', { selector: 'input' });
      expect(key).toHaveAttribute('placeholder', 'Sur aistudio.google.com, Get API key');
      await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Clé');
      await userEvent.type(key, 'AIzaCleGemini000000wxyz');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter le compte' }));
      await waitFor(() => expect(b.called('account_key')).toHaveLength(1));
      expect(b.called('account_create')[0].args).toEqual({ name: 'Clé', share: [], mode: 'link', engine: 'gemini' });
      expect(b.called('account_key')[0].args).toEqual({ id: 'gemini-cle', key: 'AIzaCleGemini000000wxyz' });
      expect(term.opened).toEqual([]);
    });
  });

  describe('an Antigravity CLI account (ESC-271)', () => {
    const AGY_MAIN: Account = { id: 'principal-agy', name: 'Principal', engine: 'agy', configDir: '', claudePath: '', active: true };
    const KEYED: Account = { ...PRO, id: 'agy-equipe', name: 'Équipe', engine: 'agy', configDir: 'C:\\Users\\ada\\.escouade\\agy\\equipe' };
    const machine = (over: Record<string, unknown> = {}) => ({
      found: true,
      added: false,
      connected: false,
      signsIn: true,
      signsInLater: true,
      email: null,
      dir: 'C:\\Users\\ada\\.gemini',
      ...over,
    });
    const SIGNED: AccountStatus = { connected: true, email: null, dir: 'C:\\Users\\ada\\.gemini', stamp: 'keychain' };

    beforeEach(() => {
      app.engines = [CLAUDE_ENGINE, AGY_ENGINE];
      app.enginesFound = ['claude', 'agy'];
      app.settings.accounts = [];
    });

    it('adds the machine’s account not signed in yet, then signs it in by agy in the terminal until its keychain has it', async () => {
      const b = backend([OUT, OUT, SIGNED], { account_machine: () => machine(), account_add_machine: () => AGY_MAIN });
      render(AccountModal, { engine: 'agy' });
      const offered = await screen.findByRole('radio', { name: /Le compte de la machine/ });
      await waitFor(() => expect(offered).toBeChecked());
      expect(offered).toBeEnabled();
      expect(
        screen.getByText('Antigravity CLI n’est pas encore connecté sur cette machine : tu t’y connectes juste après, dans un terminal'),
      ).toBeInTheDocument();
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter ce compte' }));
      await waitFor(() => expect(b.called('account_add_machine')).toHaveLength(1));
      expect(app.settings.accounts).toEqual([AGY_MAIN]);
      // Its sign-in, by agy itself: Escouade never sees the credentials.
      const dialog = await screen.findByRole('dialog', { name: 'Se connecter au compte « Principal »' });
      expect(term.opened).toEqual(['principal-agy']);
      const said = within(dialog).getByRole('status');
      expect(said).toHaveTextContent('agy ouvre ton navigateur sur ton compte Google. Escouade ne voit jamais tes identifiants.');
      await vi.advanceTimersByTimeAsync(4000);
      // No email to wait for: its keychain's entry alone says it.
      await vi.advanceTimersByTimeAsync(2000);
      await waitFor(() => expect(said).toHaveTextContent('Connecté'));
      await vi.advanceTimersByTimeAsync(1500);
      expect(term.disposed).toEqual(['term-1']);
    });

    it('adds the machine’s account signed in already as it is, and a new one by its Gemini API key', async () => {
      const b = backend([OUT], {
        account_machine: () => machine({ connected: true }),
        account_add_machine: () => AGY_MAIN,
        account_shareable: () => [],
        account_create: () => KEYED,
        account_key: () => ({ connected: true, email: 'clé …wxyz', dir: KEYED.configDir, stamp: null }),
      });
      const { unmount } = render(AccountModal, { engine: 'agy' });
      await screen.findByText('Antigravity CLI est connecté sur cette machine');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter ce compte' }));
      await waitFor(() => expect(b.called('account_add_machine')).toHaveLength(1));
      expect(term.opened).toEqual([]);
      unmount();
      app.modal = { kind: 'account' };
      render(AccountModal, { engine: 'agy' });
      await userEvent.click(await screen.findByRole('radio', { name: /Un nouveau compte/ }));
      expect(screen.getByText('La clé d’API Gemini d’un autre compte Google, gardée dans le trousseau du système.')).toBeInTheDocument();
      const key = screen.getByLabelText('Clé d’API');
      expect(key).toHaveAttribute('placeholder', 'Sur aistudio.google.com, Get API key');
      expect(screen.getByText(/donnée aux agents de ce compte par GEMINI_API_KEY/)).toBeInTheDocument();
      await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Équipe');
      await userEvent.type(key, 'AIzaCleGemini000000wxyz');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter le compte' }));
      await waitFor(() => expect(b.called('account_key')).toHaveLength(1));
      expect(b.called('account_create')[0].args).toEqual({ name: 'Équipe', share: [], mode: 'link', engine: 'agy' });
      expect(b.called('account_key')[0].args).toEqual({ id: 'agy-equipe', key: 'AIzaCleGemini000000wxyz' });
      expect(term.opened).toEqual([]);
    });
  });

  describe('the machine’s account or a new one (ESC-128)', () => {
    const machine = (over: Record<string, unknown> = {}) => ({
      found: true,
      added: false,
      connected: true,
      signsIn: true,
      email: 'ada@atlas.dev',
      dir: 'C:\\Users\\ada\\.codex',
      ...over,
    });
    const CODEX_MAIN: Account = { id: 'principal-codex', name: 'Principal', engine: 'codex', configDir: '', claudePath: '', active: true };

    beforeEach(() => {
      app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, name: 'Codex' }];
      app.enginesFound = ['claude', 'codex'];
      app.settings.accounts = [];
      app.modal = { kind: 'account', engine: 'codex' };
    });

    it('offers the account signed in on the machine first, and adds it as it is', async () => {
      const b = backend([OUT], { account_machine: () => machine(), account_add_machine: () => CODEX_MAIN });
      render(AccountModal, { engine: 'codex' });
      const offered = await screen.findByRole('radio', { name: /Le compte de la machine/ });
      await waitFor(() => expect(offered).toBeChecked());
      expect(b.called('account_machine')[0].args).toEqual({ engine: 'codex' });
      expect(screen.getByText('Codex est connecté sur cette machine · ada@atlas.dev')).toBeInTheDocument();
      expect(screen.getByText(/Son dossier habituel, tel quel/)).toHaveTextContent('(~/.codex)');
      // Nothing to name nor share: it is the machine's own.
      expect(screen.queryByRole('textbox', { name: 'Nom' })).not.toBeInTheDocument();
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter ce compte' }));
      await waitFor(() => expect(b.called('account_add_machine')).toHaveLength(1));
      expect(b.called('account_add_machine')[0].args).toEqual({ engine: 'codex' });
      expect(app.settings.accounts).toEqual([CODEX_MAIN]);
      // No sign-in to make: back to the accounts.
      expect(term.opened).toEqual([]);
      expect(app.modal).toEqual({ kind: 'settings', page: 'accounts', resume: true });
    });

    it('offers a new account instead while the harness is not signed in on the machine, or its account is added already', async () => {
      backend([OUT], { account_machine: () => machine({ connected: false, email: null }) });
      const { unmount } = render(AccountModal, { engine: 'codex' });
      expect(await screen.findByText('Codex n’est pas connecté sur cette machine')).toBeInTheDocument();
      expect(screen.getByRole('radio', { name: /Le compte de la machine/ })).toBeDisabled();
      expect(screen.getByRole('radio', { name: /Un nouveau compte/ })).toBeChecked();
      expect(screen.getByRole('textbox', { name: 'Nom' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Créer et se connecter' })).toBeInTheDocument();
      unmount();
      backend([OUT], { account_machine: () => machine({ added: true }) });
      render(AccountModal, { engine: 'codex' });
      expect(await screen.findByText('Déjà ajouté')).toBeInTheDocument();
      expect(screen.getByRole('radio', { name: /Le compte de la machine/ })).toBeDisabled();
    });

    it('says OpenCode is installed, which signs in to no account', async () => {
      backend([OUT], { account_machine: () => machine({ signsIn: false, email: null }) });
      render(AccountModal, { engine: 'codex' });
      expect(await screen.findByText('Codex est installé sur cette machine')).toBeInTheDocument();
    });

    it('opened over another window, goes back to it when it closes', async () => {
      backend([OUT], { account_machine: () => machine() });
      const onclose = vi.fn();
      app.modal = { kind: 'newProject' };
      render(AccountModal, { engine: 'codex', onclose });
      await screen.findByRole('button', { name: 'Ajouter ce compte' });
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      expect(onclose).toHaveBeenCalledOnce();
      expect(app.modal).toEqual({ kind: 'newProject' });
    });
  });

  it('offers no harness to pick while only one is found', async () => {
    backend();
    render(AccountModal, {});
    await screen.findByRole('group', { name: 'Partager avec Principal' });
    expect(screen.queryByRole('radiogroup', { name: 'Harness' })).not.toBeInTheDocument();
  });

  it('makes the account with what is checked, then runs its claude until the sign-in is there', async () => {
    const b = backend([OUT, OUT, IN]);
    render(AccountModal, {});
    await screen.findByRole('group', { name: 'Partager avec Principal' });
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Pro');
    await userEvent.click(screen.getByRole('checkbox', { name: 'agents' }));
    await userEvent.click(screen.getByRole('radio', { name: 'Copier' }));
    await userEvent.click(screen.getByRole('button', { name: 'Créer et se connecter' }));
    await waitFor(() => expect(b.called('account_create')).toHaveLength(1));
    expect(b.called('account_create')[0].args).toEqual({
      name: 'Pro',
      share: ['settings.json', 'CLAUDE.md', 'skills'],
      mode: 'copy',
      engine: 'claude',
    });
    // Saved: the settings know it, last.
    expect(app.settings.accounts).toEqual([PRINCIPAL, PRO]);
    // Its claude in a terminal, in the window.
    const dialog = await screen.findByRole('dialog', { name: 'Se connecter au compte « Pro »' });
    // What it says is read out as it changes.
    const said = within(dialog).getByRole('status');
    expect(said).toHaveTextContent(HINT);
    expect(term.opened).toEqual(['pro']);
    await waitFor(() => expect(term.mounted).toContain('term-1'));
    // Where it stands before the terminal opens, then looked at every 2 s.
    expect(b.called('account_status').map((c) => c.args.id)).toEqual(['pro']);
    await vi.advanceTimersByTimeAsync(2000);
    expect(b.called('account_status').map((c) => c.args.id)).toEqual(['pro', 'pro']);
    expect(said).toHaveTextContent(HINT);
    await vi.advanceTimersByTimeAsync(2000);
    await waitFor(() => expect(said).toHaveTextContent('Connecté au compte ada@pro.dev'));
    expect(within(dialog).queryByText(HINT)).not.toBeInTheDocument();
    // Seen for a moment, then the terminal goes and the account's quota is read.
    expect(term.disposed).toEqual([]);
    await vi.advanceTimersByTimeAsync(1500);
    expect(term.disposed).toEqual(['term-1']);
    expect(b.called('refresh_usage')).toHaveLength(1);
    // No more looking.
    await vi.advanceTimersByTimeAsync(6000);
    expect(b.called('account_status')).toHaveLength(3);
    await userEvent.click(within(dialog).getByRole('button', { name: 'Terminé' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'accounts', resume: true });
  });

  it('asked by a private project, makes a new account reserved to it alone and goes back to its settings (ESC-141)', async () => {
    app.projects = [{ ...app.projects[0], id: 'p1', name: 'client', private: true }];
    const reservedPro = { ...PRO, projects: ['p1'] };
    const b = backend([OUT, IN], {
      account_machine: () => ({
        found: true,
        added: false,
        connected: true,
        signsIn: true,
        email: 'ada@atlas.dev',
        dir: 'C:\\Users\\ada\\.claude',
      }),
      account_update: () => [PRINCIPAL, reservedPro],
    });
    render(AccountModal, { engine: 'claude', reserveFor: 'p1' });
    expect(
      screen.getByText(
        'Réservé au projet « client » : ses sessions, sa mémoire et son historique ne seront partagés avec aucun autre projet.',
      ),
    ).toBeInTheDocument();
    // The machine's account may be added, but a new one is offered first: its folder is the harness's outside Escouade.
    await screen.findByText(/Claude Code est connecté sur cette machine/);
    expect(screen.getByRole('radio', { name: /Un nouveau compte/ })).toBeChecked();
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Client');
    await userEvent.click(screen.getByRole('button', { name: 'Créer et se connecter' }));
    await waitFor(() => expect(b.called('account_update')).toHaveLength(1));
    expect(b.called('account_update')[0].args.account).toMatchObject({ id: 'pro', projects: ['p1'] });
    expect(app.settings.accounts).toEqual([PRINCIPAL, reservedPro]);
    await vi.advanceTimersByTimeAsync(2000);
    await userEvent.click(await screen.findByRole('button', { name: 'Terminé' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'privacy', projectId: 'p1', resume: true });
  });

  it('waits a moment more for the email Claude Code writes after the sign-in', async () => {
    backend([OUT, { ...IN, email: null }, { ...IN, email: null }]);
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await vi.advanceTimersByTimeAsync(2000);
    expect(screen.getByRole('status')).toHaveTextContent(HINT);
    await vi.advanceTimersByTimeAsync(2000);
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent(/^Connecté$/));
  });

  it('does not take the sign-in an account had, out of date, for the one the user is about to make', async () => {
    // « Se connecter… » on an account whose sign-in expired: connected, but not signed in just now.
    const b = backend([BEFORE, BEFORE, BEFORE, BEFORE, IN]);
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await waitFor(() => expect(term.opened).toEqual(['pro']));
    const said = screen.getByRole('status');
    for (let i = 0; i < 3; i++) await vi.advanceTimersByTimeAsync(2000);
    expect(b.called('account_status')).toHaveLength(4);
    expect(said).toHaveTextContent(HINT);
    expect(term.disposed).toEqual([]);
    expect(screen.queryByRole('button', { name: 'Terminé' })).not.toBeInTheDocument();
    // The user signs in: another sign-in, which is told, then the terminal goes.
    await vi.advanceTimersByTimeAsync(2000);
    await waitFor(() => expect(said).toHaveTextContent('Connecté au compte ada@pro.dev'));
    await vi.advanceTimersByTimeAsync(1500);
    expect(term.disposed).toEqual(['term-1']);
    expect(b.called('refresh_usage')).toHaveLength(1);
  });

  it('signs an account that is signed in already in again, the same way', async () => {
    backend([BEFORE, BEFORE, IN]);
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await vi.advanceTimersByTimeAsync(2000);
    expect(screen.getByRole('status')).toHaveTextContent(HINT);
    await vi.advanceTimersByTimeAsync(2000);
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Connecté au compte ada@pro.dev'));
  });

  it('keeps the account as it stood when it could not be read before the terminal opened, even asked twice', async () => {
    // Nothing known of its sign-in: any sign-in found then is the user’s.
    let n = 0;
    const b = fakeBackend({
      account_status: () => (n++ < 2 ? Promise.reject('refusé') : IN),
      refresh_usage: () => undefined,
    });
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    // The second look comes after a short wait; the poll starts once the terminal is open.
    await vi.advanceTimersByTimeAsync(2500);
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Connecté au compte ada@pro.dev'));
    // Asked once more, not more: the third look is the first of the poll.
    expect(b.called('account_status').length).toBeGreaterThanOrEqual(3);
  });

  it('asks once more for the sign-in it had when the first look failed (the keychain busy, a file being written)', async () => {
    // A look that fails is not an account with no sign-in: read again, so that the out-of-date sign-in it has is not
    // taken, a moment later, for the one the user has just made.
    let n = 0;
    let signedIn = false;
    const b = fakeBackend({
      account_status: () => (n++ === 0 ? Promise.reject('trousseau occupé') : signedIn ? IN : BEFORE),
      refresh_usage: () => undefined,
    });
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await waitFor(() => expect(term.opened).toEqual(['pro']));
    const said = screen.getByRole('status');
    for (let i = 0; i < 2; i++) await vi.advanceTimersByTimeAsync(2000);
    // The retry and the looks of the poll: all the same out-of-date sign-in, none the user’s.
    expect(b.called('account_status').length).toBeGreaterThanOrEqual(3);
    expect(said).toHaveTextContent(HINT);
    expect(term.disposed).toEqual([]);
    // The user signs in: another sign-in.
    signedIn = true;
    await vi.advanceTimersByTimeAsync(2000);
    await waitFor(() => expect(said).toHaveTextContent('Connecté au compte ada@pro.dev'));
  });

  it('waits a moment before it looks again at the sign-in it had, which the first look may have failed on for a moment only', async () => {
    let n = 0;
    const b = fakeBackend({
      account_status: () => (n++ === 0 ? Promise.reject('trousseau occupé') : BEFORE),
      refresh_usage: () => undefined,
    });
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await vi.advanceTimersByTimeAsync(100);
    expect(b.called('account_status')).toHaveLength(1);
    expect(term.opened).toEqual([]);
    await vi.advanceTimersByTimeAsync(300);
    expect(b.called('account_status')).toHaveLength(2);
    await waitFor(() => expect(term.opened).toEqual(['pro']));
  });

  it('does not ask again while the last look is still going', async () => {
    let release: (s: AccountStatus) => void = () => {};
    let n = 0;
    const b = fakeBackend({
      account_status: () => (n++ === 0 ? OUT : new Promise((r) => (release = r))),
      refresh_usage: () => undefined,
    });
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await waitFor(() => expect(term.opened).toEqual(['pro']));
    // The first look (the keychain, on macOS, may be waiting for an answer) goes on for a while.
    for (let i = 0; i < 4; i++) await vi.advanceTimersByTimeAsync(2000);
    expect(b.called('account_status')).toHaveLength(2);
    release(OUT);
    await vi.advanceTimersByTimeAsync(2000);
    expect(b.called('account_status').length).toBeGreaterThan(2);
  });

  it('forgets the exit of the terminal it killed, which comes after', async () => {
    backend();
    const drop = vi.spyOn(app, 'dropExit');
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await waitFor(() => expect(term.opened).toEqual(['pro']));
    await userEvent.click(screen.getByRole('button', { name: 'Fermer' }));
    expect(term.disposed).toEqual(['term-1']);
    expect(drop).toHaveBeenCalledWith('term-1');
  });

  it('closed before the sign-in, the account stays and its terminal goes', async () => {
    const b = backend();
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    // « Se connecter… »: straight to its terminal.
    expect(await screen.findByText(HINT)).toBeInTheDocument();
    expect(b.called('account_create')).toHaveLength(0);
    await waitFor(() => expect(term.opened).toEqual(['pro']));
    await vi.advanceTimersByTimeAsync(2000);
    await userEvent.click(screen.getByRole('button', { name: 'Fermer' }));
    expect(term.disposed).toEqual(['term-1']);
    expect(app.modal).toEqual({ kind: 'settings', page: 'accounts', resume: true });
    expect(app.settings.accounts).toEqual([PRINCIPAL, PRO]);
    const asked = b.called('account_status').length;
    await vi.advanceTimersByTimeAsync(6000);
    expect(b.called('account_status')).toHaveLength(asked);
  });

  it('says when Claude Code stops before the sign-in, and starts it again', async () => {
    backend();
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await waitFor(() => expect(term.opened).toEqual(['pro']));
    app.exitedTerms['term-1'] = 0;
    expect(await screen.findByText('Claude Code s’est arrêté avant la connexion.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Relancer' }));
    await waitFor(() => expect(term.opened).toEqual(['pro', 'pro']));
    expect(term.disposed).toEqual(['term-1']);
    await waitFor(() => expect(screen.queryByText('Claude Code s’est arrêté avant la connexion.')).not.toBeInTheDocument());
  });

  it('says that Claude Code is gone, or that its installation was cut, rather than that it stopped (ESC-133)', async () => {
    const gone =
      'Claude Code est introuvable : réinstalle-le (`irm https://claude.ai/install.ps1 | iex`, ou `npm install -g @anthropic-ai/claude-code`), ou indique son chemin dans Réglages › Comptes et harness.';
    const cut =
      'L’installation de Claude Code a été interrompue (il en reste /opt/homebrew/bin/.claude-Xa3k9) : relance `npm install -g @anthropic-ai/claude-code` dans un terminal.';
    let why: string | null = gone;
    const b = backend([OUT], { account_sign_in_ended: () => why });
    app.settings.accounts = [PRINCIPAL, PRO];
    render(AccountModal, { accountId: 'pro' });
    await waitFor(() => expect(term.opened).toEqual(['pro']));
    app.exitedTerms['term-1'] = 1;
    expect(await screen.findByText(gone)).toBeInTheDocument();
    expect(screen.queryByText('Claude Code s’est arrêté avant la connexion.')).not.toBeInTheDocument();
    expect(b.called('account_sign_in_ended').at(-1)!.args).toEqual({ id: 'pro' });
    // Started again, it stops again: asked again.
    why = cut;
    await userEvent.click(screen.getByRole('button', { name: 'Relancer' }));
    await waitFor(() => expect(term.opened).toEqual(['pro', 'pro']));
    await waitFor(() => expect(screen.queryByText(gone)).not.toBeInTheDocument());
    app.exitedTerms['term-2'] = 1;
    expect(await screen.findByText(cut)).toBeInTheDocument();
  });

  it('says why the account was not made, and stays on the form', async () => {
    backend([OUT], { account_create: () => Promise.reject('Un compte s’appelle déjà « Pro ».') });
    render(AccountModal, {});
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Pro');
    await userEvent.click(screen.getByRole('button', { name: 'Créer et se connecter' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Un compte s’appelle déjà « Pro ».');
    expect(screen.getByRole('dialog', { name: 'Ajouter un compte Claude Code' })).toBeInTheDocument();
    expect(term.opened).toEqual([]);
  });

  it('reads in English', async () => {
    setLang('en');
    backend([OUT, IN]);
    render(AccountModal, {});
    expect(screen.getByRole('dialog', { name: 'Add a Claude Code account' })).toBeInTheDocument();
    const share = await screen.findByRole('group', { name: 'Share with Main' });
    expect(within(share).getByRole('radio', { name: 'Link (a change applies to both accounts)' })).toBeChecked();
    expect(within(share).getByText('copied (linking a file needs administrator rights)')).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'Pro');
    await userEvent.click(screen.getByRole('button', { name: 'Create and sign in' }));
    expect(await screen.findByRole('dialog', { name: 'Sign in to the account “Pro”' })).toBeInTheDocument();
    expect(screen.getByText('Sign in in the terminal below (use the /login command if Claude Code doesn’t offer it).')).toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(2000);
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Signed in as ada@pro.dev'));
  });
});
