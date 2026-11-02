import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { CiDetail } from '../../lib/types';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import ContextMenu from '../ContextMenu.svelte';
import CiModal from './CiModal.svelte';

const DETAIL: CiDetail = {
  branch: 'main',
  word: 'PR',
  hostName: 'GitHub',
  can: { checks: true, jobLogs: true, rerun: true, reviews: true, deployments: true },
  checks: [
    { name: 'lint', state: 'success', url: null, job: '8', started: 100, finished: 112, cancelled: false },
    { name: 'test', state: 'failure', url: 'https://github.com/acme/demo/runs/7', job: '7', started: 100, finished: 250, cancelled: false },
    { name: 'ci/jenkins', state: 'failure', url: 'https://ci.example.com/1', job: null, started: null, finished: null, cancelled: false },
  ],
};
const OF = { kind: 'target' as const, id: 'p1' };

describe('the window of a CI (ESC-63)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ agents: [agent({ id: 'a1', name: 'refacto-auth' }), agent({ id: 'a2', name: 'docs', archived: true })] });
  });

  it('lists the jobs of the target, opens a failed one’s log in reading only, and says a third-party CI has none', async () => {
    const backend = fakeBackend({
      ci_detail: () => ({ data: DETAIL }),
      ci_job_log: () => ({ data: 'npm test\n\x1b[31mFAIL\x1b[0m src/app.test.ts\n##[error]Process completed with exit code 1.' }),
    });
    render(CiModal, { of: OF, projectId: 'p1' });
    expect(await screen.findByRole('dialog', { name: 'CI de main' })).toBeInTheDocument();
    const jobs = screen.getByRole('region', { name: 'CI' });
    expect(jobs).toHaveTextContent('2 échecs sur 3');
    expect(within(jobs).getByText('CI tierce : un statut et une adresse, pas de journal')).toBeInTheDocument();
    // A third-party CI has no log to open.
    expect(within(jobs).queryByRole('button', { name: 'ci/jenkins' })).toBeNull();
    await userEvent.click(within(jobs).getByRole('button', { name: 'test' }));
    const log = await screen.findByRole('listitem', { name: 'Journal de test' });
    await waitFor(() => expect(log).toHaveTextContent('FAIL src/app.test.ts'));
    expect(backend.called('ci_job_log').map((c) => c.args)).toEqual([{ of: OF, job: '7' }]);
    const term = log.querySelector('pre.term')!;
    // Coloured as a terminal, an error line in red, and nothing to type in.
    expect(term.querySelector('.fg-1')).toHaveTextContent('FAIL');
    expect(term.querySelector('.ln.error')).toHaveTextContent('Process completed');
    expect(within(log).queryByRole('textbox')).toBeNull();
    // Nothing was written on the host.
    expect(backend.called('ci_rerun')).toHaveLength(0);
    expect(backend.called('ci_log_to_agent')).toHaveLength(0);
  });

  it('re-runs the failed jobs on a click only, and says how many ran', async () => {
    const backend = fakeBackend({ ci_detail: () => ({ data: DETAIL }), ci_rerun: () => 1 });
    render(CiModal, { of: OF, projectId: 'p1' });
    const rerun = await screen.findByRole('button', { name: 'Relancer les jobs en échec' });
    expect(backend.called('ci_rerun')).toHaveLength(0);
    await userEvent.click(rerun);
    await waitFor(() => expect(backend.called('ci_rerun').map((c) => c.args)).toEqual([{ of: OF }]));
    await waitFor(() => expect(app.toasts.map((x) => x.text)).toContain('1 exécution relancée.'));
  });

  it('says where the host cannot re-run them, with no button', async () => {
    fakeBackend({ ci_detail: () => ({ data: { ...DETAIL, hostName: 'Bitbucket', can: { ...DETAIL.can, rerun: false } } }) });
    render(CiModal, { of: OF, projectId: 'p1' });
    expect(await screen.findByText('Bitbucket ne relance pas les jobs depuis Escouade : relance-les sur son site.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Relancer les jobs en échec' })).toBeNull();
  });

  it('sends a failed job and its log to an agent of the project chosen in a menu', async () => {
    const backend = fakeBackend({ ci_detail: () => ({ data: DETAIL }), ci_log_to_agent: () => null });
    render(ContextMenu);
    render(CiModal, { of: OF, projectId: 'p1' });
    await userEvent.click(await screen.findByRole('button', { name: 'test · Envoyer à un agent' }));
    const menu = await screen.findByRole('menu');
    // The archived agent is not offered.
    expect(
      within(menu)
        .getAllByRole('menuitem')
        .map((m) => m.textContent?.trim()),
    ).toEqual(['refacto-auth']);
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'refacto-auth' }));
    await waitFor(() =>
      expect(backend.called('ci_log_to_agent').map((c) => c.args)).toEqual([
        { of: OF, job: '7', check: 'test', url: 'https://github.com/acme/demo/runs/7', agentId: 'a1' },
      ]),
    );
    await waitFor(() => expect(app.toasts.map((x) => x.text)).toContain('Journal de test envoyé à refacto-auth.'));
  });

  it('says why the host could not be read', async () => {
    fakeBackend({ ci_detail: () => ({ failure: { kind: 'unreachable', message: 'GitHub ne répond pas' } }) });
    render(CiModal, { of: OF, projectId: 'p1' });
    expect(await screen.findByRole('alert')).toHaveTextContent('GitHub ne répond pas');
  });

  it('speaks English', async () => {
    setLang('en');
    fakeBackend({ ci_detail: () => ({ data: DETAIL }) });
    render(CiModal, { of: OF, projectId: 'p1' });
    expect(await screen.findByRole('dialog', { name: 'CI of main' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Re-run failed jobs' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'test · Send to an agent' })).toBeInTheDocument();
  });
});
