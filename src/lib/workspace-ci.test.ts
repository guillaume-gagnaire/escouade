import { describe, expect, it, vi } from 'vitest';
import { agent, CLAUDE_ENGINE, fakeBackend, project, pullRequest, SETTINGS } from '../test/ipc';
import type { InitialState, UiEvent } from './types';
import { check, prDetail, projectEnvs } from '../test/workspace';
import { app } from './state.svelte';
import { followEnvironments, prDetails, readPrDetail } from './workspace-ci.svelte';
import { landingOf } from './workspace-landing';

const INITIAL: InitialState = {
  projects: [project()],
  agents: [],
  ui: { activeProject: 'p1', view: 'project', selectedAgent: {} },
  settings: SETTINGS,
  usage: { todayCost: 0, accounts: [], current: { claude: 'principal' } },
  git: {},
  shells: [],
  terminals: [],
  tickets: [],
  claudeFound: true,
  version: '0.1.0',
  models: [],
  engines: [CLAUDE_ENGINE],
};

describe('what the canvas reads of the CI', () => {
  it("reads the detail of the agent's open pull request", async () => {
    const backend = fakeBackend({ agent_pr_detail: () => ({ data: { ...prDetail(), checks: [check('ci', 'success')] } }) });
    await readPrDetail(agent({ id: 'a1', prs: [pullRequest({ number: 7, state: 'open' })] }));
    expect(backend.called('agent_pr_detail')[0].args).toMatchObject({ id: 'a1', number: 7 });
    expect(prDetails.a1.checks[0].name).toBe('ci');
  });

  it("keeps the pull request's state when its detail cannot be read", async () => {
    fakeBackend({ agent_pr_detail: () => ({ failure: { kind: 'unreachable', message: 'x' } }) });
    const a = agent({ id: 'a2', prs: [pullRequest({ state: 'open', checks: 'failure' })] });
    await readPrDetail(a);
    expect(prDetails.a2).toBeUndefined();
    expect(landingOf(a, { pr: null, environments: null })[0]).toMatchObject({ kind: 'ci', state: 'failure', steps: null });
  });

  it('follows the environments of the project while it is open', async () => {
    vi.useFakeTimers();
    const backend = fakeBackend({ project_environments: () => ({ ...projectEnvs(), projectId: 'p1', readAt: Date.now() }) });
    const stop = followEnvironments('p1');
    await vi.advanceTimersByTimeAsync(5 * 60 * 1000);
    expect(backend.called('project_environments')).toHaveLength(2);
    stop();
    await vi.advanceTimersByTimeAsync(5 * 60 * 1000);
    expect(backend.called('project_environments')).toHaveLength(2);
    vi.useRealTimers();
  });

  it('reads the same pull request once a minute, whoever asks', async () => {
    const backend = fakeBackend({ agent_pr_detail: () => ({ data: prDetail() }) });
    const a = agent({ id: 'a3', prs: [pullRequest({ number: 7, state: 'open', checks: 'pending' })] });
    await readPrDetail(a);
    await readPrDetail(a);
    expect(backend.called('agent_pr_detail')).toHaveLength(1);
    // Its checks changed: read again.
    await readPrDetail(agent({ id: 'a3', prs: [pullRequest({ number: 7, state: 'open', checks: 'success' })] }));
    expect(backend.called('agent_pr_detail')).toHaveLength(2);
  });

  it('drops an answer that comes after its pull request was closed or changed', async () => {
    const answers: ((v: unknown) => void)[] = [];
    fakeBackend({ agent_pr_detail: () => new Promise((r) => answers.push(r)) });
    const open = agent({ id: 'a4', prs: [pullRequest({ number: 7, state: 'open' })] });
    const reading = readPrDetail(open);
    await readPrDetail(agent({ id: 'a4', prs: [pullRequest({ number: 7, state: 'merged' })] }));
    answers[0]({ data: prDetail() });
    await reading;
    expect(prDetails.a4).toBeUndefined();

    // An old answer after a newer one: the newer stays.
    const old = readPrDetail(agent({ id: 'a4', prs: [pullRequest({ number: 8, state: 'open', checks: 'pending' })] }));
    const fresh = readPrDetail(agent({ id: 'a4', prs: [pullRequest({ number: 8, state: 'open', checks: 'failure' })] }));
    answers[2]({ data: { ...prDetail(), checks: [check('new', 'failure')] } });
    await fresh;
    answers[1]({ data: { ...prDetail(), checks: [check('old', 'pending')] } });
    await old;
    expect(prDetails.a4.checks[0].name).toBe('new');
  });

  it('lands no deployment of a pull request that is no longer open', () => {
    const a = agent({ id: 'a5', prs: [pullRequest({ state: 'merged' })] });
    const pr = { ...prDetail(), deployments: [{ environment: 'prod', state: 'success', url: null } as never] };
    expect(landingOf(a, { pr, environments: null }).filter((l) => l.kind === 'deploy')).toEqual([]);
  });

  it('reads the environments in silence, and never those of a host without deployments', async () => {
    app.toasts = [];
    const backend = fakeBackend({
      project_environments: () => ({ ...projectEnvs(), projectId: 'p2', readAt: null }),
      project_environments_read: () => {
        throw { kind: 'unreachable', message: 'offline' };
      },
    });
    const stop = followEnvironments('p2');
    await vi.waitFor(() => expect(backend.called('project_environments_read')).toHaveLength(1));
    await new Promise((r) => setTimeout(r, 0));
    expect(app.toasts).toEqual([]);
    stop();

    const b2 = fakeBackend({ project_environments: () => ({ ...projectEnvs(), projectId: 'p3', unsupported: true, readAt: null }) });
    followEnvironments('p3')();
    await new Promise((r) => setTimeout(r, 0));
    expect(b2.called('project_environments')).toHaveLength(1);
    expect(b2.called('project_environments_read')).toHaveLength(0);
  });

  it('forgets the detail of an agent removed', async () => {
    let channel: { onmessage: (e: UiEvent) => void } | null = null;
    const a = agent({ id: 'gone', prs: [pullRequest({ number: 9, state: 'open' })] });
    fakeBackend({
      subscribe: (args: any) => {
        channel = args.channel;
        return { ...INITIAL, agents: [a] };
      },
      agent_pr_detail: () => ({ data: prDetail() }),
    });
    await app.init();
    await readPrDetail(a);
    expect(prDetails.gone).toBeTruthy();
    channel!.onmessage({ type: 'agentRemoved', id: 'gone', projectId: 'p1' });
    expect(prDetails.gone).toBeUndefined();
  });
});
