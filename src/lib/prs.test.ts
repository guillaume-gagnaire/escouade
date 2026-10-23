import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from './i18n';
import {
  askOpenPr,
  canFixChecks,
  canFixReviews,
  canOpenPr,
  cardPrs,
  fixPr,
  hasOwnBranch,
  openPrOf,
  ownBranch,
  prGlyph,
  prMenuItem,
  prTip,
  prTone,
  refreshPrs,
} from './prs';
import { app } from './state.svelte';
import { agent, fakeBackend, pullRequest as pr, resetApp } from '../test/ipc';

const { openUrl } = vi.hoisted(() => ({ openUrl: vi.fn(() => Promise.resolve()) }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl }));

const WT = { path: 'C:\\wt\\login', branch: 'escouade/login', baseBranch: 'main', existing: false };

describe('the pull requests of an agent', () => {
  beforeEach(() => resetApp());
  afterEach(() => setLang('fr'));

  it('are an ordinary agent’s with a worktree, on a code host that opens them: never a ticket’s, nor an archived one’s', () => {
    expect(canOpenPr(agent({ worktree: WT, prWord: 'PR' }))).toBe(true);
    expect(canOpenPr(agent({ prWord: 'PR' }))).toBe(false);
    expect(canOpenPr(agent({ worktree: WT, prWord: 'PR', ticketId: 't1' }))).toBe(false);
    expect(canOpenPr(agent({ worktree: WT, prWord: 'PR', archived: true }))).toBe(false);
    // No word from the backend: no remote, a host without a client yet, no account.
    expect(canOpenPr(agent({ worktree: WT }))).toBe(false);
    expect(prMenuItem(agent({ worktree: WT }))).toBeNull();
  });

  it('shows three of them on the card, and the open one is the one to see', () => {
    const prs = [pr({ number: 9 }), pr({ number: 8, state: 'merged' }), pr({ number: 3, state: 'closed' }), pr({ number: 1 })];
    expect(cardPrs(agent({ prs })).map((p) => p.number)).toEqual([9, 8, 3]);
    expect(cardPrs(agent({ prs, ticketId: 't1' }))).toEqual([]);
    expect(openPrOf(agent({ prs }))?.number).toBe(9);
    expect(openPrOf(agent({ prs: [pr({ state: 'merged' })] }))).toBeNull();
  });

  it('offers the corrections of an open one only when there is something to correct', () => {
    expect(canFixChecks(pr({ checks: 'failure' }))).toBe(true);
    expect(canFixChecks(pr({ checks: 'pending' }))).toBe(false);
    expect(canFixChecks(pr({ checks: 'failure', state: 'merged' }))).toBe(false);
    expect(canFixReviews(pr({ changesRequested: true }))).toBe(true);
    expect(canFixReviews(pr({ openThreads: 2 }))).toBe(true);
    expect(canFixReviews(pr())).toBe(false);
    expect(canFixReviews(pr({ openThreads: 2, state: 'closed' }))).toBe(false);
  });

  it('marks each one by what to look at first', () => {
    expect(prTone(pr())).toBe('ok');
    expect(prTone(pr({ checks: 'failure' }))).toBe('fail');
    expect(prTone(pr({ mergeable: false }))).toBe('fail');
    expect(prTone(pr({ changesRequested: true }))).toBe('wait');
    expect(prTone(pr({ checks: 'pending' }))).toBe('wait');
    expect(prTone(pr({ draft: true, checks: 'failure' }))).toBe('dim');
    expect(prTone(pr({ state: 'merged' }))).toBe('merged');
    expect(
      [pr(), pr({ checks: 'failure' }), pr({ checks: 'pending' }), pr({ checks: null }), pr({ state: 'merged' })].map(prGlyph),
    ).toEqual(['✓', '✗', '●', '○', '⇡']);
  });

  it('says its state, its CI and its reviews in words, in French and in English', () => {
    const busy = pr({ checks: 'failure', approvals: 1, changesRequested: true, openThreads: 2, mergeable: false });
    expect(prTip('PR', busy)).toBe(
      'PR #7 · Connexion par jeton\nouverte · CI en échec · 1 approbation · changements demandés · 2 fils ouverts · conflit avec main',
    );
    expect(prTip('MR', pr({ draft: true, checks: null }))).toBe('MR #7 · Connexion par jeton\nbrouillon · pas de CI');
    expect(prTip('PR', pr({ state: 'merged', openThreads: 3 }))).toBe('PR #7 · Connexion par jeton\nmergée · CI verte');
    setLang('en');
    expect(prTip('PR', busy)).toBe(
      'PR #7 · Connexion par jeton\nopen · CI failed · 1 approval · changes requested · 2 open threads · conflicts with main',
    );
  });

  it('opens the window to propose one, or the open one in the browser', () => {
    askOpenPr(agent({ worktree: WT }));
    expect(app.modal).toEqual({ kind: 'pullRequest', agentId: 'a1' });
    app.modal = null;
    askOpenPr(agent({ worktree: WT, prs: [pr()] }));
    expect(app.modal).toBeNull();
    expect(openUrl).toHaveBeenCalledWith('https://github.com/acme/demo/pull/7');
  });

  it('names the menus’ item after the host’s word and the open one', async () => {
    expect(prMenuItem(agent())).toBeNull();
    expect(prMenuItem(agent({ worktree: WT, prWord: 'PR' }))?.label).toBe('Ouvrir une PR…');
    expect(prMenuItem(agent({ worktree: WT, prWord: 'MR' }))?.label).toBe('Ouvrir une MR…');
    expect(prMenuItem(agent({ worktree: WT, prWord: 'PR', prs: [pr({ number: 12 })] }))?.label).toBe('Voir la PR #12');
    setLang('en');
    expect(prMenuItem(agent({ worktree: WT, prWord: 'PR' }))?.label).toBe('Open a PR…');
    // What gets out of its way runs first (the branch picker).
    const order: string[] = [];
    const item = prMenuItem(agent({ worktree: WT, prWord: 'PR' }), () => order.push('released'))!;
    item.onClick!();
    await vi.waitFor(() => expect(app.modal).toEqual({ kind: 'pullRequest', agentId: 'a1' }));
    expect(order).toEqual(['released']);
  });

  it('hands a correction to the agent on the click, and says so', async () => {
    const backend = fakeBackend();
    await fixPr(agent({ worktree: WT }), pr({ checks: 'failure' }), 'checks');
    expect(backend.called('agent_pr_fix').map((c) => c.args)).toEqual([{ id: 'a1', number: 7, fix: 'checks' }]);
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Envoyé à refacto-auth.', kind: 'ok' });
    // Refused (nothing left to correct): the refusal is told, nothing said as sent.
    fakeBackend({
      agent_pr_fix: () => {
        throw 'Aucun retour à traiter sur la PR #7.';
      },
    });
    await fixPr(agent({ worktree: WT }), pr({ openThreads: 1 }), 'reviews');
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Aucun retour à traiter sur la PR #7.', kind: 'error' });
  });

  it('reads them again only for an agent that can have some', () => {
    const backend = fakeBackend();
    refreshPrs(agent({ worktree: WT, ticketId: 't1' }));
    refreshPrs(agent({ worktree: WT, archived: true }));
    refreshPrs(null);
    expect(backend.called('agent_prs_refresh')).toHaveLength(0);
    refreshPrs(agent({ worktree: WT }), true);
    // Without a worktree of its own: it may have gone into one by itself since (ESC-161), which the backend finds.
    refreshPrs(agent({ id: 'a2' }));
    expect(backend.called('agent_prs_refresh').map((c) => c.args)).toEqual([
      { id: 'a1', now: true },
      { id: 'a2', now: false },
    ]);
  });

  it('follows the branch of the worktree an agent went into by itself, never the project’s folder’s (ESC-161)', () => {
    const found = { path: 'C:/p/.claude/worktrees/a', branch: 'feat/a' };
    expect(ownBranch(agent())).toBeNull();
    expect(hasOwnBranch(agent())).toBe(false);
    expect(ownBranch(agent({ foundWorktree: found }))).toBe('feat/a');
    expect(hasOwnBranch(agent({ foundWorktree: found }))).toBe(true);
    expect(canOpenPr(agent({ foundWorktree: found, prWord: 'PR' }))).toBe(true);
    // Its own worktree first.
    expect(ownBranch(agent({ worktree: WT, foundWorktree: found }))).toBe(WT.branch);
  });
});
