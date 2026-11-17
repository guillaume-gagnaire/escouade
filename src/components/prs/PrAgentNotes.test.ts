import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { PrWriting } from '../../lib/prthreads';
import { app } from '../../lib/state.svelte';
import type { AgentReview, PrContents, PrContentsFile, PrThread, ReviewNote } from '../../lib/types';
import { fakeBackend, project, pullRequest as pr, resetApp } from '../../test/ipc';
import PrView from './PrView.svelte';

// The reviewer agent's remarks in the view of a pull request (ESC-490, PR-5): threads of the agent under their line in
// the tab « Fichiers », marked « agent, pas publié », with their severity (counted, filtered, sorted), their suggestion as
// a diff, the summary in the Conversation with a link to each; « Écarter » resolves one here, « Publier » sends it as the
// user's comment on a click (one, or those kept after a confirmation that lists them), « Relire à nouveau » after a push
// says the stale ones apart; the agent's text is a third party's, nothing remote loads.

const PR = pr({ number: 214, title: 'Connexion par lien magique', head: 'ticket/atl-42', base: 'main', author: 'bob', state: 'open' });

const LINK: PrContentsFile = {
  path: 'src/auth/magic-link.ts',
  status: 'modified',
  additions: 2,
  deletions: 1,
  lines: [
    { kind: '@', text: '@@ -1,3 +1,4 @@' },
    { kind: ' ', old: 1, new: 1, text: 'export const ttl = 10;' },
    { kind: '-', old: 2, text: 'return mailer.send(email);' },
    { kind: '+', new: 2, text: 'if (given === stored) {' },
    { kind: '+', new: 3, text: 'return { sent: true };' },
    { kind: ' ', old: 3, new: 4, text: '}' },
  ],
  binary: false,
  generated: false,
  cut: false,
};

const ALL: PrWriting = { comment: true, lineComment: true, reply: true, resolve: true, reopen: true };

const HOST_THREAD: PrThread = {
  id: 'T1',
  path: LINK.path,
  line: 3,
  side: 'right',
  state: 'open',
  comments: [{ id: '11', author: 'marta', body: 'Et si le lien a expiré ?' }],
};

const note = (over: Partial<ReviewNote>): ReviewNote => ({
  id: 1,
  path: LINK.path,
  line: 2,
  severity: 'detail',
  body: 'x',
  proposed: 'x',
  choice: 'open',
  excerpt: [],
  outside: false,
  code: [],
  stale: false,
  ...over,
});

const SECURITY = note({
  id: 1,
  line: 2,
  severity: 'security',
  body: 'Compare en **temps constant**. ![traceur](https://evil.example/p.png)',
  proposed: 'Compare en **temps constant**. ![traceur](https://evil.example/p.png)',
  code: ['if (given === stored) {'],
  suggestion: 'if (timingSafeEqual(given, stored)) {',
});
const DETAIL = note({ id: 2, line: 4, severity: 'detail', body: 'Accolade seule.', proposed: 'Accolade seule.', code: ['}'] });
const SPEC = note({ id: 3, path: '', line: 0, severity: 'spec', body: 'La spec dit 10 min.', proposed: 'La spec dit 10 min.' });
const BUG = note({
  id: 4,
  line: 3,
  severity: 'bug',
  body: 'Le retour manque.',
  proposed: 'Le retour manque.',
  code: ['return { sent: true };'],
});

const review = (notes: ReviewNote[], over: Partial<AgentReview> = {}): AgentReview => ({
  projectId: 'p1',
  number: 214,
  word: 'PR',
  host: 'GitHub',
  kind: 'github',
  author: 'bob',
  title: PR.title,
  head: PR.head,
  base: PR.base,
  url: '',
  headSha: 'a1b2c3d4e5f6',
  phase: 'ready',
  summary: 'Le flux est clair. Deux points de **sécurité**.',
  notes,
  leftOut: ['package-lock.json'],
  by: 'Claude Sonnet',
  at: 1,
  readAt: Date.now() - 60_000,
  ...over,
});

const contents = (threads: PrThread[] = []): PrContents => ({
  thirdParty: true,
  word: 'PR',
  pr: PR,
  commits: [{ sha: 'a1b2c3d4e5', message: 'feat: lien magique', author: 'bob' }],
  files: [LINK],
  threads,
  reading: { description: true, commits: true, files: true, threads: true },
  writing: ALL,
  diff: true,
  tip: 'a1b2c3d4e5',
  readAt: 0,
  checkedAt: 0,
});

/** A backend that keeps the review and answers its commands as the core does. */
function backend(notes: ReviewNote[], over: Record<string, (a: any) => unknown> = {}) {
  let r = review(notes);
  const sent: { id: number; body?: string; onLine: boolean }[][] = [];
  const be = fakeBackend({
    pr_contents: () => ({ data: contents([HOST_THREAD]) }),
    pr_detail: () => ({ data: null }),
    pr_reviews: () => [r],
    pr_review_note: (a: { id: number; choice: ReviewNote['choice'] }) => {
      r = { ...r, notes: r.notes.map((n) => (n.id === a.id ? { ...n, choice: a.choice } : n)) };
      return r;
    },
    pr_review_send: (a: { notes: { id: number; body?: string; onLine: boolean }[] }) => {
      sent.push(a.notes);
      const ids = a.notes.map((n) => n.id);
      r = {
        ...r,
        notes: r.notes.map((n) => (ids.includes(n.id) ? { ...n, choice: 'published', publishedAt: 1 } : n)),
      };
      return { review: r, sent: ids };
    },
    pr_review_start: () => {
      r = { ...r, phase: 'reading' };
      return r;
    },
    ...over,
  });
  return { be, sent, set: (next: AgentReview) => (r = next) };
}

async function openFiles() {
  const main = await screen.findByRole('main', { name: 'La PR #214' });
  await userEvent.click(await within(main).findByRole('tab', { name: /^Fichiers/ }));
  return main;
}

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [project()] });
  app.codeHosts = { p1: { kind: 'github', name: 'GitHub', word: 'PR', pullRequests: true, deployments: true } };
});

describe('the reviewer agent’s remarks (ESC-490)', () => {
  it('are threads under their line, marked « agent, pas publié », with their severity and their suggestion as a diff', async () => {
    backend([SECURITY, DETAIL, SPEC]);
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const link = await within(main).findByRole('region', { name: LINK.path });
    const thread = await within(link).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:2` });
    // Under the row of its line, after the line, distinct from the host's thread (its tag says it without the colour).
    const row = link.querySelector('.row[data-new="2"]')!;
    expect(row.nextElementSibling?.contains(thread)).toBe(true);
    expect(row).toHaveClass('marked');
    expect(thread).toHaveTextContent('agent, pas publié');
    expect(thread).toHaveTextContent('Agent relecteur');
    expect(within(thread).getByLabelText('Sévérité : Sécurité')).toBeInTheDocument();
    expect(thread).toHaveClass('agent');
    const host = within(link).getByRole('article', { name: 'Fil sur src/auth/magic-link.ts:3' });
    expect(host).not.toHaveTextContent('agent, pas publié');
    // The agent's text is a third party's Markdown: rendered, nothing remote loaded without a click.
    expect(within(thread).getByText('temps constant').tagName).toBe('STRONG');
    expect(thread.querySelector('img')).toBeNull();
    // The suggestion: its lines removed, then what replaces them, never run; copied by a click.
    const sugg = within(thread).getByRole('group', { name: 'Suggestion' });
    const lines = [...sugg.querySelectorAll('.dl')].map((l) => l.textContent?.trim());
    expect(lines).toEqual(['-if (given === stored) {', '+if (timingSafeEqual(given, stored)) {']);
    expect(within(sugg).getByRole('button', { name: 'Copier' })).toBeInTheDocument();
    // The remark on the whole pull request, above the files.
    expect(within(main).getByRole('group', { name: '1 remarque sur la PR entière' })).toHaveTextContent('La spec dit 10 min.');
  });

  it('counts them by severity in the head of the tab, and filters them', async () => {
    backend([SECURITY, DETAIL, SPEC, BUG]);
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const bar = await within(main).findByRole('region', { name: 'Relecture de l’agent' });
    const chips = within(bar).getByRole('group', { name: 'Remarques de l’agent par sévérité' });
    expect(within(chips).getByRole('button', { name: /^Sécurité/ })).toHaveTextContent('1');
    expect(within(chips).getByRole('button', { name: /^Bogue/ })).toHaveTextContent('1');
    expect(within(chips).getByRole('button', { name: /^Spec/ })).toHaveTextContent('1');
    expect(within(chips).getByRole('button', { name: /^Détail/ })).toHaveTextContent('1');
    expect(main).toHaveTextContent('4 remarques');
    await within(main).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:4` });
    await userEvent.click(within(chips).getByRole('button', { name: /^Sécurité/ }));
    expect(within(chips).getByRole('button', { name: /^Sécurité/ })).toHaveAttribute('aria-pressed', 'true');
    expect(within(main).queryByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:4` })).toBeNull();
    expect(within(main).getByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:2` })).toBeInTheDocument();
    await userEvent.click(within(chips).getByRole('button', { name: /^Sécurité/ }));
    expect(within(main).getByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:4` })).toBeInTheDocument();
  });

  it('sums the review up in the Conversation, sorted, with a link to each thread', async () => {
    backend([DETAIL, SECURITY, SPEC]);
    render(PrView, { project: project(), number: 214 });
    const main = await screen.findByRole('main', { name: 'La PR #214' });
    const sum = await within(main).findByRole('region', { name: 'Résumé de la relecture de l’agent' });
    expect(sum).toHaveTextContent('a relu la PR au commit a1b2c3d');
    expect(sum).toHaveTextContent('3 remarques · 1 Sécurité · 1 Spec · 1 Détail');
    expect(within(sum).getByText('sécurité').tagName).toBe('STRONG');
    expect(sum).toHaveTextContent('1 fichier n’a pas été lu');
    // By severity first: the security one leads.
    const items = () => [...sum.querySelectorAll('.sum-list li')].map((li) => li.querySelector('.mono')?.textContent);
    expect(items()).toEqual([`${LINK.path}:2`, 'Sur la PR', `${LINK.path}:4`]);
    await userEvent.selectOptions(within(sum).getByRole('combobox', { name: 'Trier les remarques' }), 'place');
    expect(items()).toEqual(['Sur la PR', `${LINK.path}:2`, `${LINK.path}:4`]);
    // A link opens the tab « Fichiers » on its thread.
    await userEvent.click(within(sum).getByRole('button', { name: `Voir la remarque sur ${LINK.path}:4` }));
    expect(within(main).getByRole('tab', { name: /^Fichiers/ })).toHaveAttribute('aria-selected', 'true');
    const thread = await within(main).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:4` });
    await waitFor(() => expect(thread).toHaveClass('current'));
  });

  it('« Écarter » resolves the agent’s thread here: folded, kept, nothing published', async () => {
    const { be } = backend([SECURITY]);
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const thread = await within(main).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:2` });
    await userEvent.click(within(thread).getByRole('button', { name: /^Écarter/ }));
    await waitFor(() => expect(be.called('pr_review_note')[0].args).toMatchObject({ id: 1, choice: 'dropped' }));
    const folded = await waitFor(() => {
      const el = main.querySelector('[data-thread-key="agent:1"]')!;
      expect(el).toHaveClass('folded');
      return el as HTMLElement;
    });
    expect(folded).toHaveTextContent('Écartée');
    expect(be.called('pr_review_send')).toHaveLength(0);
    expect(be.called('pr_write')).toHaveLength(0);
    await userEvent.click(within(folded).getByRole('button', { name: 'Reprendre' }));
    await waitFor(() => expect(be.called('pr_review_note')[1].args).toMatchObject({ id: 1, choice: 'open' }));
  });

  it('« Publier » sends one remark, its text read and corrected first, on its line, on a click', async () => {
    const { be, sent } = backend([SECURITY]);
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const thread = await within(main).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:2` });
    await userEvent.click(within(thread).getByRole('button', { name: /^Publier/ }));
    // Nothing yet: the text is shown to be read and corrected.
    expect(be.called('pr_review_send')).toHaveLength(0);
    const box = within(thread).getByRole('textbox', { name: 'Texte publié sur GitHub' });
    expect(box).toHaveValue(SECURITY.body);
    expect(thread).toHaveTextContent(`comme ton commentaire sur ${LINK.path}:2`);
    await userEvent.clear(box);
    await userEvent.type(box, 'Compare en temps constant.');
    const reads = be.called('pr_contents').length;
    await userEvent.click(within(thread).getByRole('button', { name: /Publier sur GitHub/ }));
    await waitFor(() => expect(sent).toEqual([[{ id: 1, body: 'Compare en temps constant.', onLine: true }]]));
    // Published: read again, the thread is the host's now (no double).
    await waitFor(() => expect(be.called('pr_contents').length).toBeGreaterThan(reads));
    await waitFor(() => expect(main.querySelector('[data-thread-key="agent:1"]')).toBeNull());
  });

  it('« Publier les n remarques gardées » lists them in a confirmation and sends them on its click alone', async () => {
    const { be, sent } = backend([SECURITY, DETAIL, BUG]);
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    for (const line of [2, 4]) {
      const th = await within(main).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:${line}` });
      await userEvent.click(within(th).getByRole('button', { name: /^Garder/ }));
      await waitFor(() => expect(th).toHaveTextContent('Gardée'));
    }
    await userEvent.click(within(main).getByRole('button', { name: 'Publier les 2 remarques gardées' }));
    const dialog = within(main).getByRole('dialog', { name: 'Publier les 2 remarques gardées ?' });
    expect(dialog).toHaveTextContent('sous ton compte');
    expect([...dialog.querySelectorAll('li .mono')].map((x) => x.textContent)).toEqual([`${LINK.path}:2`, `${LINK.path}:4`]);
    expect(be.called('pr_review_send')).toHaveLength(0);
    await userEvent.click(within(dialog).getByRole('button', { name: 'Publier les 2' }));
    await waitFor(() =>
      expect(sent).toEqual([
        [
          { id: 1, onLine: true },
          { id: 2, onLine: true },
        ],
      ]),
    );
    await waitFor(() => expect(within(main).queryByRole('dialog')).toBeNull());
    // The bug, not kept, stays the agent's.
    expect(within(main).getByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:3` })).toBeInTheDocument();
  });

  it('a refused publication is said in the thread, the remark kept to publish', async () => {
    backend([SECURITY], {
      pr_review_send: () => ({
        review: review([SECURITY]),
        sent: [],
        failure: { kind: 'forbidden', message: 'Resource not accessible' },
      }),
    });
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const thread = await within(main).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:2` });
    await userEvent.click(within(thread).getByRole('button', { name: /^Publier/ }));
    await userEvent.click(within(thread).getByRole('button', { name: /Publier sur GitHub/ }));
    expect(await within(thread).findByRole('alert')).toHaveTextContent('GitHub');
    expect(within(thread).getByRole('textbox', { name: 'Texte publié sur GitHub' })).toBeInTheDocument();
  });

  it('read again after a push: « Relire à nouveau », the stale ones apart, what was dropped stays so', async () => {
    const stale = { ...BUG, stale: true, code: ['return { ok: true };'] };
    const dropped = { ...DETAIL, choice: 'dropped' as const };
    const { be } = backend([SECURITY, stale, dropped]);
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const apart = await within(main).findByRole('group', { name: '1 remarque de l’agent hors du diff : sa ligne n’y est plus' });
    expect(within(apart).getByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:3` })).toHaveTextContent('périmée');
    expect(main.querySelector('[data-thread-key="agent:2"]')).toHaveClass('folded');
    await userEvent.click(within(main).getByRole('button', { name: /Relire à nouveau/ }));
    await waitFor(() => expect(be.called('pr_review_start')).toHaveLength(1));
    expect(be.called('pr_review_send')).toHaveLength(0);
  });

  it('a remark whose code is no longer at its line is apart, not under another line', async () => {
    const moved = { ...SECURITY, code: ['if (a === b) {'] };
    backend([moved]);
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const apart = await within(main).findByRole('group', { name: /hors du diff/ });
    expect(within(apart).getByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:2` })).toBeInTheDocument();
  });

  it('« Suivante » goes through the host’s threads and the agent’s remarks, as chosen', async () => {
    backend([SECURITY, DETAIL]);
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    await within(main).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:2` });
    const current = () => main.querySelector('.current')?.getAttribute('data-thread-key');
    const next = () => userEvent.click(within(main).getByRole('button', { name: /^Suivante/ }));
    await next();
    await waitFor(() => expect(current()).toBe('agent:1'));
    await next();
    await waitFor(() => expect(current()).toBe('T1'));
    await next();
    await waitFor(() => expect(current()).toBe('agent:2'));
    // The host's alone.
    await userEvent.selectOptions(within(main).getByRole('combobox', { name: 'Ce que « Suivante » parcourt' }), 'host');
    await next();
    await waitFor(() => expect(current()).toBe('T1'));
    await next();
    await waitFor(() => expect(current()).toBe('T1'));
    // The agent's alone.
    await userEvent.selectOptions(within(main).getByRole('combobox', { name: 'Ce que « Suivante » parcourt' }), 'agent');
    await next();
    await waitFor(() => expect(current()).toBe('agent:1'));
  });

  it('a remark published stays the host’s thread when a read of before answers after it (ESC-488)', async () => {
    const PUBLISHED: PrThread = {
      id: 'PRRT_pub',
      path: LINK.path,
      line: 2,
      side: 'right',
      state: 'open',
      comments: [{ id: '701', author: 'ada', body: 'Compare en temps constant.' }],
    };
    let published = false;
    let calls = 0;
    let release: () => void = () => {};
    const gate = new Promise<void>((r) => (release = r));
    const { sent } = backend([SECURITY], {
      pr_contents: async () => {
        calls += 1;
        const answer = { data: contents(published ? [HOST_THREAD, PUBLISHED] : [HOST_THREAD]) };
        // « Relire la PR », before the publication: answers late, without the published thread.
        if (calls === 2) await gate;
        return answer;
      },
    });
    render(PrView, { project: project(), number: 214 });
    const main = await openFiles();
    const thread = await within(main).findByRole('article', { name: `Remarque de l’agent sur ${LINK.path}:2` });
    await userEvent.click(within(main).getByRole('button', { name: 'Relire la PR' }));
    await waitFor(() => expect(calls).toBe(2));
    published = true;
    await userEvent.click(within(thread).getByRole('button', { name: /^Publier/ }));
    await userEvent.click(within(thread).getByRole('button', { name: /Publier sur GitHub/ }));
    await waitFor(() => expect(sent).toHaveLength(1));
    await waitFor(() => expect(main.querySelector('[data-thread-key="PRRT_pub"]')).not.toBeNull());
    release();
    await new Promise((r) => setTimeout(r, 50));
    expect(main.querySelector('[data-thread-key="PRRT_pub"]')).not.toBeNull();
  });
});
