import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import {
  boxRows,
  controlFailure,
  filterNames,
  labelColor,
  mergeable,
  methodsOf,
  mergedText,
  ordered,
  written,
  type PrControlRead,
} from '../../lib/prcontrol';
import type { PrWriting } from '../../lib/prthreads';
import { app } from '../../lib/state.svelte';
import type { PrContents, PrDetail, PullRequest, TicketPr } from '../../lib/types';
import { agent, fakeBackend, project, pullRequest, resetApp, ticket } from '../../test/ipc';
import PrView from './PrView.svelte';

// The control of a pull request and its merge box (ESC-488, PR-8): each state of the box with the gesture that unblocks
// it, the method and its confirmation, the branch and the worktree, the auto-merge, the ticket « Terminé », the pickers
// of reviewers and labels, what a host does not do left out and said, the refusals said in the box, a third party's
// names kept as text, and nothing sent without a click.

const ALL: PrWriting = {
  comment: true,
  lineComment: true,
  reply: true,
  resolve: true,
  reopen: true,
  draft: true,
  reviewers: true,
  labels: true,
  updateMerge: true,
  updateRebase: true,
  rerunCheck: true,
  cancelAutoMerge: true,
  deleteBranch: true,
  control: true,
};
const ANSWERS: PrWriting = { comment: true, lineComment: false, reply: true, resolve: true, reopen: false };

const PR = (over: Partial<PullRequest> = {}) =>
  pullRequest({ number: 214, title: 'Connexion par lien magique', head: 'ticket/atl-42', base: 'main', author: 'bob', ...over });

const contents = (pr: PullRequest, writing: PrWriting = ALL, labels: string[] = ['auth']): PrContents => ({
  thirdParty: true,
  word: 'PR',
  pr,
  description: { body: 'Le lien.', labels },
  commits: [],
  files: [],
  threads: [],
  reading: { description: true, commits: true, files: true, threads: true },
  writing,
  diff: false,
  readAt: 0,
  checkedAt: 0,
});

const check = (name: string, state: 'success' | 'failure' | 'pending', job: string | null = null) => ({
  name,
  state,
  url: null,
  job,
  started: null,
  finished: null,
  cancelled: false,
});
const detail = (checks = [check('lint', 'success'), check('e2e', 'success', '77')], reviews: PrDetail['reviews'] = []): PrDetail => ({
  checks,
  reviews,
  comments: [],
  deployments: [],
  can: { checks: true, jobLogs: true, rerun: true, reviews: true, deployments: false },
});

const read = (over: Partial<PrControlRead> = {}, control: Partial<NonNullable<PrControlRead['control']>> = {}): PrControlRead => ({
  control: { mergeState: 'clean', behind: false, labels: ['auth'], deletesBranch: false, ...control },
  options: { strategies: ['merge', 'squash', 'rebase'], autoMerge: true },
  preferred: 'squash',
  writing: ALL,
  ...over,
});

const reads = ['pr_contents', 'pr_detail', 'pr_control', 'pr_reviews'];

function backend(o: {
  pr?: PullRequest;
  d?: PrDetail;
  ctl?: PrControlRead | null | (() => PrControlRead);
  writing?: PrWriting;
  write?: unknown;
  merged?: unknown;
}) {
  return fakeBackend({
    pr_contents: () => ({ data: contents(o.pr ?? PR(), o.writing ?? ALL) }),
    pr_detail: () => ({ data: o.d ?? detail() }),
    pr_control: () =>
      o.ctl === null
        ? { failure: { kind: 'other', message: 'non' } }
        : { data: structuredClone(typeof o.ctl === 'function' ? o.ctl() : (o.ctl ?? read())) },
    pr_candidates: () => ({
      data: {
        reviewers: ['ada', 'marta', 'bob'],
        labels: [
          { name: 'bug', color: 'd73a4a' },
          { name: 'auth', color: '0e8a16' },
          { name: '<img src=x onerror=alert(1)>', color: 'red;background:url(x)' },
        ],
      },
    }),
    pr_write: () => o.write ?? { data: true },
    pr_merge: () => o.merged ?? { data: { merged: true, branch: 'ticket/atl-42', ticket: 'ATL-42' } },
  });
}

async function box() {
  const main = await screen.findByRole('main', { name: 'La PR #214' });
  const b = await within(main).findByRole('region', { name: 'Prête à merger ?' });
  return { main, box: b };
}

const writes = (b: ReturnType<typeof fakeBackend>) => b.calls.filter((c) => !reads.includes(c.cmd) && c.cmd !== 'pr_candidates');

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [project()] });
  app.codeHosts = { p1: { kind: 'github', name: 'GitHub', word: 'PR', pullRequests: true, deployments: true } };
});

afterEach(() => {
  Object.defineProperty(navigator, 'onLine', { value: true, configurable: true });
});

describe('the merge box (lib/prcontrol)', () => {
  const input = (pr: PullRequest, over: Partial<Parameters<typeof boxRows>[0]> = {}) => ({
    pr,
    detail: detail(),
    control: read(),
    openThreads: 0,
    people: [],
    host: 'GitHub',
    ticketAgent: false,
    agent: null,
    ...over,
  });

  it('says what is missing, the blocking lines first, each with its gesture', () => {
    const rows = boxRows(
      input(PR({ draft: true, mergeable: false, approvals: 0 }), {
        detail: detail([check('e2e', 'failure', '77'), check('lint', 'success')]),
        control: read({}, { mergeState: 'dirty', reviewDecision: 'reviewRequired' }),
        openThreads: 2,
        people: [{ name: 'marta', state: 'requested' }],
        ticketAgent: true,
      }),
    );
    expect(rows.map((r) => [r.key, r.tone, r.action?.kind])).toEqual([
      ['draft', 'ko', 'ready'],
      ['checks', 'ko', 'rerunOne'],
      ['branch', 'ko', 'ticketAgent'],
      ['reviews', 'wait', undefined],
      ['threads', 'wait', 'threads'],
    ]);
    expect(rows[1].sub).toBe('e2e');
    expect(rows[3].sub).toBe('approbation manquante (marta)');
    expect(mergeable(rows)).toBe(false);
    // All green: nothing blocks, and the box says so.
    const ready = boxRows(input(PR({ approvals: 1 }), { people: [{ name: 'marta', state: 'approved' }] }));
    expect(ready.every((r) => r.tone === 'ok')).toBe(true);
    expect(ready.find((r) => r.key === 'reviews')?.sub).toBe('par marta');
    expect(mergeable(ready)).toBe(true);
  });

  it('says behind its base, the rules of a protected branch, and what the host does not do', () => {
    const behind = boxRows(input(PR(), { control: read({}, { mergeState: 'behind', behind: true }) }));
    expect(behind.find((r) => r.key === 'branch')).toMatchObject({ tone: 'ko', action: { kind: 'update' } });
    const rules = boxRows(input(PR({ approvals: 1 }), { control: read({}, { mergeState: 'blocked' }) }));
    expect(rules[0]).toMatchObject({ key: 'rules', tone: 'ko', action: { kind: 'host' } });
    // A host that does not mark a pull request ready, nor update its branch: no button, the line says where.
    const gitlab = boxRows(
      input(PR({ draft: true }), { control: read({ writing: ANSWERS }, { mergeState: 'behind', behind: true }), host: 'GitLab' }),
    );
    const draft = gitlab.find((r) => r.key === 'draft')!;
    expect(draft.action).toBeUndefined();
    expect(draft.elsewhere).toBe('GitLab ne le fait pas depuis Escouade : fais-le sur GitLab.');
    expect(gitlab.find((r) => r.key === 'branch')?.action).toBeUndefined();
  });

  it('offers the methods the repository allows, the Kanban’s first, and keeps a label’s colour only when it is one', () => {
    expect(methodsOf(read())).toEqual(['squash', 'merge', 'rebase']);
    expect(methodsOf(read({ options: { strategies: ['merge', 'rebase'], autoMerge: false }, preferred: 'rebase' }))).toEqual([
      'rebase',
      'merge',
    ]);
    expect(methodsOf(null)).toEqual([]);
    expect(labelColor('D73A4A')).toBe('#d73a4a');
    expect(labelColor('red;background:url(x)')).toBeNull();
    expect(filterNames(['marta', 'bob', 'Martin'], (x) => x, 'mar')).toEqual(['marta', 'Martin']);
  });

  it('says a refusal and what a merge did in words', () => {
    const f = (kind: Parameters<typeof controlFailure>[0]['kind'], message = 'x') =>
      controlFailure({ kind, message }, 'GitHub', 'main', 'PR');
    expect(f('forbidden')).toContain('n’a pas le droit');
    expect(f('conflict')).toBe('Un conflit est apparu entre-temps avec main : rien n’est mergé. Mets la branche à jour.');
    expect(f('protected', 'At least 1 approving review is required')).toContain('At least 1 approving review is required');
    expect(f('checkRequired', 'Required status check "e2e" is expected.')).toContain('"e2e"');
    expect(f('rateLimit')).toContain('Limite d’API de GitHub');
    expect(controlFailure({ kind: 'unreachable', message: '' }, 'GitHub', 'main', 'PR', false)).toBe('Hors ligne : rien n’est parti.');
    expect(mergedText({ merged: true, branch: 'b', ticket: 'ATL-42' }, { base: 'main', host: 'GitHub', method: 'squash' })).toEqual([
      'Mergée dans main (Squash et merger).',
      'Branche b supprimée.',
      'ATL-42 passé en Terminé.',
    ]);
  });
});

describe('the control of a pull request in its view (ESC-488)', () => {
  it('merges after a confirmation that says the method, the target, the branch, the worktree and the ticket', async () => {
    const tp = { number: 214, namespace: 'acme', name: 'demo' } as TicketPr;
    resetApp({
      projects: [project()],
      tickets: [ticket({ id: 't9', key: 'ATL-42', column: 'pr', agentId: 'a1', pr: tp })],
      agents: [
        agent({ id: 'a1', name: 'Lien magique', worktree: { path: 'C:\\wt\\atl-42', branch: 'ticket/atl-42', baseBranch: 'main' } }),
      ],
    });
    app.codeHosts = { p1: { kind: 'github', name: 'GitHub', word: 'PR', pullRequests: true, deployments: true } };
    const b = backend({ pr: PR({ approvals: 1 }) });
    render(PrView, { project: project(), number: 214 });
    const { box: mb } = await box();
    await waitFor(() => expect(mb).toHaveTextContent('Prête à merger'));
    // Nothing sent by itself: the view only read.
    expect(writes(b)).toEqual([]);

    // The method: the Kanban's first, the others the repository allows.
    await userEvent.click(within(mb).getByRole('button', { name: 'Choisir la méthode de merge' }));
    const menu = within(mb).getByRole('menu', { name: 'Méthode de merge' });
    const items = within(menu).getAllByRole('menuitemradio');
    expect(items.map((x) => x.querySelector('b')?.textContent)).toEqual([
      'Squash et merger',
      'Merger (commit de merge)',
      'Rebase et merger',
    ]);
    expect(items[0]).toHaveTextContent('celle du Kanban');
    expect(items[0]).toHaveTextContent('un seul commit sur main');
    expect(items[0]).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(within(menu).getByRole('menuitemradio', { name: /^Rebase/ }));
    expect(within(mb).getByLabelText('Supprimer la branche ticket/atl-42')).toBeChecked();
    expect(within(mb).getByLabelText('Supprimer le worktree de Lien magique')).toBeChecked();
    await userEvent.click(within(mb).getByLabelText('Supprimer le worktree de Lien magique'));
    expect(mb).toHaveTextContent('ATL-42 passera en Terminé');

    // The confirmation says it all; cancelled, nothing leaves.
    await userEvent.click(within(mb).getByRole('button', { name: 'Rebase et merger' }));
    const confirm = within(mb).getByRole('alertdialog', { name: 'Merger la PR #214 ?' });
    expect(confirm).toHaveTextContent('Rebase et merger dans main : les commits rebasés, sans commit de merge.');
    expect(confirm).toHaveTextContent('La branche ticket/atl-42 est supprimée sur GitHub.');
    expect(confirm).not.toHaveTextContent('worktree');
    expect(confirm).toHaveTextContent('ATL-42 passe en Terminé.');
    await userEvent.click(within(confirm).getByRole('button', { name: 'Annuler' }));
    expect(writes(b)).toEqual([]);

    await userEvent.click(within(mb).getByRole('button', { name: 'Rebase et merger' }));
    await userEvent.click(within(mb).getByRole('button', { name: 'Merger' }));
    await waitFor(() =>
      expect(b.called('pr_merge').map((c) => c.args)).toEqual([
        { projectId: 'p1', number: 214, ask: { strategy: 'rebase', deleteBranch: true, deleteWorktree: false } },
      ]),
    );
    await waitFor(() => expect(mb).toHaveTextContent('Mergée dans main (Rebase et merger).'));
    expect(mb).toHaveTextContent('Branche ticket/atl-42 supprimée.');
    expect(mb).toHaveTextContent('ATL-42 passé en Terminé.');
  });

  it('turns each blocking line’s gesture into one write, on its click', async () => {
    const tp = { number: 214, namespace: 'acme', name: 'demo' } as TicketPr;
    resetApp({ projects: [project()], tickets: [ticket({ id: 't9', key: 'ATL-42', column: 'pr', agentId: 'a1', pr: tp })] });
    app.codeHosts = { p1: { kind: 'github', name: 'GitHub', word: 'PR', pullRequests: true, deployments: true } };
    const b = backend({
      pr: PR({ draft: true, mergeable: false }),
      d: detail([check('e2e', 'failure', '77'), check('lint', 'success')]),
      ctl: read({}, { mergeState: 'dirty' }),
    });
    render(PrView, { project: project(), number: 214 });
    const { box: mb } = await box();
    await waitFor(() => expect(mb).toHaveTextContent('Brouillon'));
    // Blocked: no merge.
    expect(within(mb).getByRole('button', { name: 'Squash et merger' })).toBeDisabled();
    const row = (k: string) => mb.querySelector(`[data-row="${k}"]`) as HTMLElement;
    await userEvent.click(within(row('draft')).getByRole('button', { name: 'Prête à relire' }));
    await userEvent.click(within(row('checks')).getByRole('button', { name: 'Relancer l’échec' }));
    await userEvent.click(within(row('branch')).getByRole('button', { name: 'Confier à l’agent du ticket' }));
    await waitFor(() => expect(b.called('ticket_pr_update_branch')).toHaveLength(1));
    expect(b.called('pr_write').map((c) => c.args.write)).toEqual([
      { kind: 'setDraft', draft: false },
      { kind: 'rerunCheck', job: '77' },
    ]);
    expect(b.called('ticket_pr_update_branch')[0].args).toEqual({ id: 't9' });
  });

  it('updates a branch behind its base by the host’s way chosen, and offers the host’s auto-merge while it waits', async () => {
    // Behind, then up to date once the host updated it.
    let behind = true;
    const b = backend({
      pr: PR({ checks: 'pending' }),
      d: detail([check('e2e', 'pending'), check('lint', 'success')]),
      ctl: () => read({}, behind ? { mergeState: 'behind', behind: true } : { mergeState: 'blocked', behind: false }),
    });
    const updated = () => (behind = false);
    render(PrView, { project: project(), number: 214 });
    const { main, box: mb } = await box();
    await waitFor(() => expect(mb).toHaveTextContent('En retard sur main'));
    const aside = within(main).getByRole('complementary', { name: 'Contrôle de la PR' });
    await userEvent.click(within(aside).getAllByRole('button', { name: 'Mettre à jour' })[0]);
    const menu = within(aside).getByRole('menu', { name: 'Mettre à jour' });
    updated();
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Rebaser la branche sur main' }));
    await waitFor(() => expect(b.called('pr_write').map((c) => c.args.write)).toEqual([{ kind: 'updateBranch', method: 'rebase' }]));
    await waitFor(() => expect(mb).toHaveTextContent('Branche mise à jour par GitHub'));
    await waitFor(() => expect(mb).toHaveTextContent('Pas de conflit'));
    await userEvent.click(await within(mb).findByRole('button', { name: 'Auto-merge quand tout passe' }));
    await waitFor(() => expect(b.called('pr_write').at(-1)?.args.write).toEqual({ kind: 'autoMerge', strategy: 'squash' }));
  });

  it('says the auto-merge on and turns it off on a click', async () => {
    const b = backend({ pr: PR({ autoMerge: true, checks: 'pending' }), ctl: read({}, { autoMerge: 'squash', mergeState: 'blocked' }) });
    render(PrView, { project: project(), number: 214 });
    const { box: mb } = await box();
    await waitFor(() => expect(mb).toHaveTextContent('Auto-merge activé (Squash et merger) : GitHub merge dès que tout passe.'));
    await userEvent.click(within(mb).getByRole('button', { name: 'Désactiver l’auto-merge' }));
    await waitFor(() => expect(b.called('pr_write').map((c) => c.args.write)).toEqual([{ kind: 'cancelAutoMerge' }]));
  });

  it('asks and takes back reviewers and labels among the host’s, its names kept as text', async () => {
    const b = backend({ pr: PR({ reviewers: ['marta'] }) });
    render(PrView, { project: project(), number: 214 });
    const { main } = await box();
    const aside = within(main).getByRole('complementary', { name: 'Contrôle de la PR' });
    await userEvent.click(await within(aside).findByRole('button', { name: 'Demander une relecture' }));
    const people = await within(aside).findByRole('group', { name: 'Relecteurs à demander' });
    // The author is not offered; marta, asked, is ticked.
    expect(within(people).queryByText('bob')).toBeNull();
    expect(within(people).getByLabelText('marta')).toBeChecked();
    await userEvent.click(within(people).getByLabelText('ada'));
    await userEvent.click(within(aside).getByRole('button', { name: 'Retirer la demande à marta' }));

    await userEvent.click(within(aside).getByRole('button', { name: 'Modifier les étiquettes' }));
    const labels = await within(aside).findByRole('group', { name: 'Étiquettes de la PR' });
    const odd = within(labels).getByLabelText('<img src=x onerror=alert(1)>');
    expect(labels.querySelector('img')).toBeNull();
    expect(odd.closest('label')?.querySelector('.dot')).toBeNull();
    await userEvent.click(odd);
    await userEvent.click(within(aside).getByRole('button', { name: 'Retirer l’étiquette auth' }));
    await waitFor(() =>
      expect(b.called('pr_write').map((c) => c.args.write)).toEqual([
        { kind: 'requestReviewers', logins: ['ada'] },
        { kind: 'removeReviewer', login: 'marta' },
        { kind: 'addLabels', labels: ['<img src=x onerror=alert(1)>'] },
        { kind: 'removeLabel', label: 'auth' },
      ]),
    );
  });

  it('marks ready and back to draft from the box’s column', async () => {
    const b = backend({ pr: PR() });
    render(PrView, { project: project(), number: 214 });
    const { main } = await box();
    const aside = within(main).getByRole('complementary', { name: 'Contrôle de la PR' });
    await userEvent.click(within(aside).getByRole('checkbox', { name: 'Repasser la PR en brouillon' }));
    await waitFor(() => expect(b.called('pr_write').map((c) => c.args.write)).toEqual([{ kind: 'setDraft', draft: true }]));
  });

  it('says a refusal in the box, offline too, and nothing changes', async () => {
    backend({
      pr: PR({ approvals: 1 }),
      merged: { failure: { kind: 'checkRequired', message: 'Required status check "e2e" is expected.' } },
      write: { failure: { kind: 'forbidden', message: 'Resource not accessible' } },
    });
    render(PrView, { project: project(), number: 214 });
    const { main, box: mb } = await box();
    await waitFor(() => expect(mb).toHaveTextContent('Prête à merger'));
    await userEvent.click(within(mb).getByRole('button', { name: 'Squash et merger' }));
    await userEvent.click(within(mb).getByRole('button', { name: 'Merger' }));
    await waitFor(() =>
      expect(within(mb).getByRole('alert')).toHaveTextContent(
        'Un contrôle requis manque ou n’est pas passé : Required status check "e2e" is expected.',
      ),
    );
    const aside = within(main).getByRole('complementary', { name: 'Contrôle de la PR' });
    await userEvent.click(within(aside).getByRole('checkbox', { name: 'Repasser la PR en brouillon' }));
    await waitFor(() => expect(within(mb).getByRole('alert')).toHaveTextContent('Ton jeton GitHub n’a pas le droit de faire cela'));
  });

  it('says offline when the host is out of reach and the system says so', async () => {
    Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
    backend({ pr: PR({ approvals: 1 }), merged: { failure: { kind: 'unreachable', message: 'connexion refusée' } } });
    render(PrView, { project: project(), number: 214 });
    const { box: mb } = await box();
    await userEvent.click(await within(mb).findByRole('button', { name: 'Squash et merger' }));
    await userEvent.click(within(mb).getByRole('button', { name: 'Merger' }));
    await waitFor(() => expect(within(mb).getByRole('alert')).toHaveTextContent('Hors ligne : rien n’est parti.'));
  });

  it('shows no button a host does not do, and says it', async () => {
    app.codeHosts = { p1: { kind: 'gitlab', name: 'GitLab', word: 'MR', pullRequests: true, deployments: true } };
    backend({
      pr: PR({ draft: true }),
      writing: ANSWERS,
      ctl: { options: { strategies: [], autoMerge: false }, writing: ANSWERS },
    });
    render(PrView, { project: project(), number: 214 });
    const main = await screen.findByRole('main', { name: /#214/ });
    const mb = await within(main).findByRole('region', { name: 'Prête à merger ?' });
    await waitFor(() => expect(mb).toHaveTextContent('GitLab ne le fait pas depuis Escouade : fais-le sur GitLab.'));
    expect(mb).toHaveTextContent('GitLab ne merge pas cette PR depuis Escouade.');
    const aside = within(main).getByRole('complementary');
    expect(within(aside).queryByRole('button', { name: 'Demander une relecture' })).toBeNull();
    expect(within(aside).queryByRole('button', { name: 'Modifier les étiquettes' })).toBeNull();
    expect(within(aside).queryByRole('checkbox')).toBeNull();
    expect(within(mb).queryByRole('button', { name: /merger/i })).toBeNull();
  });

  it('runs one failed check again from the tab « Contrôles »', async () => {
    const b = backend({ d: detail([check('e2e', 'failure', '77'), check('lint', 'success')]) });
    render(PrView, { project: project(), number: 214 });
    const { main } = await box();
    await userEvent.click(within(main).getByRole('tab', { name: /^Contrôles/ }));
    await userEvent.click(await within(within(main).getByRole('tabpanel')).findByRole('button', { name: 'Relancer e2e seul' }));
    await waitFor(() => expect(b.called('pr_write').map((c) => c.args.write)).toEqual([{ kind: 'rerunCheck', job: '77' }]));
  });

  it('keeps the control open and the view read when the app replaces the project’s object', async () => {
    const b = backend({});
    const view = render(PrView, { project: project(), number: 214 });
    const { main } = await box();
    const toggle = within(main).getByRole('button', { name: 'Contrôle de la PR' });
    await userEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    const reads = b.called('pr_contents').length;
    await view.rerender({ project: { ...project() }, number: 214 });
    expect(within(main).getByRole('button', { name: 'Contrôle de la PR' })).toHaveAttribute('aria-expanded', 'true');
    expect(b.called('pr_contents')).toHaveLength(reads);
  });
});

describe('the reads in order (ESC-488)', () => {
  it('shows only the newest answer, and none set out before a write', () => {
    const o = ordered();
    const a = o.start();
    const b = o.start();
    expect(o.take(b)).toBe(true);
    expect(o.take(a)).toBe(false);
    const c = o.start();
    o.fence();
    expect(o.take(c)).toBe(false);
    expect(o.take(o.start())).toBe(true);
  });

  /** A read of the pull request held until `release`: the answer it was given when asked, older than what comes after. */
  function held() {
    let release: () => void = () => {};
    const gate = new Promise<void>((r) => (release = r));
    return { gate, release: () => release() };
  }

  it('says « Fusionnée » once merged, and a read of before arriving after never brings back « Ouverte »', async () => {
    let calls = 0;
    const slow = held();
    let state: PullRequest['state'] = 'open';
    const b = fakeBackend({
      pr_contents: async () => {
        calls += 1;
        const answer = { data: contents(PR({ approvals: 1, state })) };
        // The second read (« Relire la PR ») is the slow one: it answers « Ouverte » after the merge.
        if (calls === 2) await slow.gate;
        return answer;
      },
      pr_detail: () => ({ data: detail() }),
      pr_control: () => ({ data: read() }),
      pr_merge: () => {
        state = 'merged';
        return { data: { merged: true } };
      },
    });
    render(PrView, { project: project(), number: 214 });
    const { main, box: mb } = await box();
    await waitFor(() => expect(mb).toHaveTextContent('Prête à merger'));
    await userEvent.click(within(main).getByRole('button', { name: 'Relire la PR' }));
    await waitFor(() => expect(calls).toBe(2));
    await userEvent.click(within(mb).getByRole('button', { name: 'Squash et merger' }));
    await userEvent.click(within(mb).getByRole('button', { name: 'Merger' }));
    const pill = () => main.querySelector('.hd .pill-st');
    await waitFor(() => expect(pill()).toHaveTextContent('Fusionnée'));
    expect(b.called('pr_merge')).toHaveLength(1);
    // The read of before answers now: « Ouverte », dropped.
    slow.release();
    await new Promise((r) => setTimeout(r, 50));
    expect(pill()).toHaveTextContent('Fusionnée');
    expect(within(main).getByRole('region', { name: 'Prête à merger ?' })).toHaveTextContent('Fusionnée');
  });

  it('keeps a gesture of the column against a read of before (a draft made ready)', async () => {
    let calls = 0;
    const slow = held();
    let draft = true;
    fakeBackend({
      pr_contents: async () => {
        calls += 1;
        const answer = { data: contents(PR({ draft })) };
        if (calls === 2) await slow.gate;
        return answer;
      },
      pr_detail: () => ({ data: detail() }),
      pr_control: () => ({ data: read() }),
      pr_write: () => {
        draft = false;
        return { data: true };
      },
    });
    render(PrView, { project: project(), number: 214 });
    const { main } = await box();
    const aside = within(main).getByRole('complementary', { name: 'Contrôle de la PR' });
    await waitFor(() => expect(within(aside).getByRole('checkbox', { name: 'Passer la PR prête à relire' })).toBeChecked());
    await userEvent.click(within(main).getByRole('button', { name: 'Relire la PR' }));
    await waitFor(() => expect(calls).toBe(2));
    await userEvent.click(within(aside).getByRole('checkbox', { name: 'Passer la PR prête à relire' }));
    await waitFor(() => expect(within(aside).getByRole('checkbox', { name: 'Repasser la PR en brouillon' })).not.toBeChecked());
    slow.release();
    await new Promise((r) => setTimeout(r, 50));
    expect(within(aside).getByRole('checkbox', { name: 'Repasser la PR en brouillon' })).not.toBeChecked();
  });

  it('reads again on « Relire la PR » while a read is on its way, and shows the newest', async () => {
    let calls = 0;
    const slow = held();
    let approvals = 0;
    fakeBackend({
      pr_contents: async () => {
        calls += 1;
        const answer = { data: contents(PR({ approvals, title: `lecture ${calls}` })) };
        if (calls === 2) await slow.gate;
        return answer;
      },
      pr_detail: () => ({ data: detail() }),
      pr_control: () => ({ data: read() }),
    });
    render(PrView, { project: project(), number: 214 });
    const { main } = await box();
    await userEvent.click(within(main).getByRole('button', { name: 'Relire la PR' }));
    await waitFor(() => expect(calls).toBe(2));
    approvals = 1;
    await userEvent.click(within(main).getByRole('button', { name: 'Relire la PR' }));
    await waitFor(() => expect(calls).toBe(3));
    await waitFor(() => expect(main.querySelector('h1')).toHaveTextContent('lecture 3'));
    slow.release();
    await new Promise((r) => setTimeout(r, 50));
    expect(main.querySelector('h1')).toHaveTextContent('lecture 3');
  });
});

describe('a write shown at once (ESC-488)', () => {
  it('applies what the host accepted to what is shown, before the reread', () => {
    const shown = {
      pr: PR({ draft: true, reviewers: ['marta'] }),
      labels: ['auth'],
      read: read({}, { mergeState: 'draft', behind: true }),
    };
    expect(written({ kind: 'setDraft', draft: false }, shown).pr.draft).toBe(false);
    expect(written({ kind: 'setDraft', draft: false }, shown).read?.control?.mergeState).toBe('unknown');
    expect(written({ kind: 'requestReviewers', logins: ['ada'] }, shown).pr.reviewers).toEqual(['marta', 'ada']);
    expect(written({ kind: 'removeReviewer', login: 'marta' }, shown).pr.reviewers).toEqual([]);
    expect(written({ kind: 'addLabels', labels: ['bug'] }, shown).labels).toEqual(['auth', 'bug']);
    expect(written({ kind: 'removeLabel', label: 'auth' }, shown).read?.control?.labels).toEqual([]);
    expect(written({ kind: 'updateBranch', method: 'merge' }, shown).read?.control?.behind).toBe(false);
    expect(written({ kind: 'autoMerge', strategy: 'squash' }, shown).pr.autoMerge).toBe(true);
    expect(written({ kind: 'cancelAutoMerge' }, shown).read?.control?.autoMerge).toBeUndefined();
  });
});
