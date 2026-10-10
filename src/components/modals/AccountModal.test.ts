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
import { fakeBackend, resetApp, SETTINGS } from '../../test/ipc';
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
    const dialog = screen.getByRole('dialog', { name: 'Ajouter un compte Claude' });
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

  it('makes the account with what is checked, then runs its claude until the sign-in is there', async () => {
    const b = backend([OUT, OUT, IN]);
    render(AccountModal, {});
    await screen.findByRole('group', { name: 'Partager avec Principal' });
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Pro');
    await userEvent.click(screen.getByRole('checkbox', { name: 'agents' }));
    await userEvent.click(screen.getByRole('radio', { name: 'Copier' }));
    await userEvent.click(screen.getByRole('button', { name: 'Créer et se connecter' }));
    await waitFor(() => expect(b.called('account_create')).toHaveLength(1));
    expect(b.called('account_create')[0].args).toEqual({ name: 'Pro', share: ['settings.json', 'CLAUDE.md', 'skills'], mode: 'copy' });
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
    expect(app.modal).toEqual({ kind: 'settings', tab: 'accounts', resume: true });
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
    expect(app.modal).toEqual({ kind: 'settings', tab: 'accounts', resume: true });
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

  it('says why the account was not made, and stays on the form', async () => {
    backend([OUT], { account_create: () => Promise.reject('Un compte s’appelle déjà « Pro ».') });
    render(AccountModal, {});
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom' }), 'Pro');
    await userEvent.click(screen.getByRole('button', { name: 'Créer et se connecter' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Un compte s’appelle déjà « Pro ».');
    expect(screen.getByRole('dialog', { name: 'Ajouter un compte Claude' })).toBeInTheDocument();
    expect(term.opened).toEqual([]);
  });

  it('reads in English', async () => {
    setLang('en');
    backend([OUT, IN]);
    render(AccountModal, {});
    expect(screen.getByRole('dialog', { name: 'Add a Claude account' })).toBeInTheDocument();
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
