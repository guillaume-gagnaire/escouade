import { describe, expect, it } from 'vitest';
import { agent, branchCi, pullRequest } from '../test/ipc';
import { check, prDetail, projectEnvs } from '../test/workspace';
import { landed, landingOf, type LandingCtx, type Ribbon } from './workspace-landing';

const ctx = (over: Partial<LandingCtx> = {}): LandingCtx => ({ pr: null, environments: null, ...over });

describe('landingOf', () => {
  it('lands an artifact by its address, and keeps one being published above', () => {
    const a = agent({
      canvas: {
        tools: [],
        subs: [],
        artifacts: [{ url: 'https://claude.ai/artifact/z', title: 'R', versions: 2, at: 5 }],
        publishing: [{ call: 'p', name: 'brouillon.html', since: 6 }],
      },
    });
    const r = landingOf(a, ctx());
    expect(r).toEqual([
      expect.objectContaining({ kind: 'artifact', title: 'R', versions: 2 }),
      expect.objectContaining({ kind: 'artifact', url: null, title: 'brouillon.html' }),
    ]);
    expect(r.map(landed)).toEqual([true, false]);
  });

  it("takes the steps of the pull request, else the branch's summary", () => {
    const pr = pullRequest({ number: 12, head: 'feat/x', checks: 'failure', url: 'https://h/pr/12', state: 'open' });
    const detail = {
      ...prDetail(),
      checks: [check('lint', 'success'), check('test', 'failure')],
      deployments: [{ environment: 'staging', state: 'in_progress', url: 'https://s' }],
    };
    const r = landingOf(agent({ prs: [pr] }), ctx({ pr: detail }));
    expect(r[0]).toMatchObject({
      kind: 'ci',
      number: 12,
      state: 'failure',
      line: 'feat/x',
      where: 'h',
      steps: [
        { name: 'lint', state: 'success' },
        { name: 'test', state: 'failure' },
      ],
    });
    expect(r[1]).toMatchObject({ kind: 'deploy', line: 'feat/x → staging', state: 'running' });
    const b = landingOf(agent({ branchCi: branchCi({ checks: 'failure', checksTotal: 5, checksFailed: 2 }) }), ctx());
    expect(b[0]).toMatchObject({ kind: 'ci', steps: null, total: 5, failed: 2 });
  });

  it("lands a deployment of the agent's branch, the pull request's first, never a commit", () => {
    const dep = (environment: string, sha: string, agents: string[]) => ({
      deployment: { environment, state: 'success', url: 'https://p', sha, log: 'https://h/job/1' },
      agents,
    });
    const environments = {
      ...projectEnvs(),
      branches: [dep('Preview', 'abc1234', ['a1']), dep('staging', 'abc1234', ['a1']), dep('Preview', 'fff', ['a2'])],
    };
    const a = agent({ id: 'a1', worktree: { path: '/w', branch: 'feat/y', baseBranch: 'main' } });
    const r = landingOf(a, ctx({ environments }));
    expect(r.map((l) => (l as Ribbon).line)).toEqual(['feat/y → Preview', 'feat/y → staging']);
    expect(r[0]).toMatchObject({ state: 'success', opens: { kind: 'url', url: 'https://h/job/1' } });
    expect(JSON.stringify(r)).not.toContain('abc1234');
    const withPr = agent({ ...a, prs: [pullRequest({ head: 'feat/y', state: 'open', checks: null })] });
    const fromPr = { ...prDetail(), deployments: [{ environment: 'staging', state: 'success', url: 'https://s' }] };
    expect(
      landingOf(withPr, ctx({ pr: fromPr, environments }))
        .filter((l) => l.kind === 'deploy')
        .map((l) => (l as Ribbon).line),
    ).toEqual(['feat/y → staging', 'feat/y → Preview']);
  });

  it('lands one deployment an environment: the latest, when the pull request was deployed there twice', () => {
    const pr = pullRequest({ head: 'feat/y', state: 'open', checks: null });
    const deployments = [
      { environment: 'Preview', state: 'in_progress', url: 'https://new' },
      { environment: 'Preview', state: 'inactive', url: 'https://old' },
    ];
    const r = landingOf(agent({ prs: [pr] }), ctx({ pr: { ...prDetail(), deployments } }));
    expect(r).toEqual([expect.objectContaining({ kind: 'deploy', id: 'deploy:Preview', state: 'running', where: 'new' })]);
  });

  it('lands nothing for an agent with nothing', () => {
    expect(landingOf(agent(), ctx())).toEqual([]);
  });
});
