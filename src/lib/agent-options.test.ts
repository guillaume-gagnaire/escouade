import { describe, expect, it } from 'vitest';
import { agent, BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, resetApp } from '../test/ipc';
import { effortChoicesOf, modeChoicesOf, setAgentOption } from './agent-options';
import { app } from './state.svelte';

describe('agent options', () => {
  it('greys a mode the harness lacks, with why', () => {
    resetApp({ agents: [agent({ id: 'a1', engine: 'codex', mode: 'bypassPermissions' })] });
    // A harness of one mode alone, as Composer.test's (ESC-489).
    app.engines = [
      CLAUDE_ENGINE,
      {
        ...BARE_ENGINE,
        name: 'Unharnessed',
        caps: { ...BARE_ENGINE.caps, modes: [{ id: 'bypassPermissions', meaning: 'Ne demande jamais.' }] },
      },
    ];
    const plan = modeChoicesOf(app.agents.a1).find((c) => c.value === 'plan');
    expect(plan).toMatchObject({ disabled: true });
    expect(plan?.reason).toBeTruthy();
  });

  it('offers no effort for a model that takes none', () => {
    resetApp();
    expect(effortChoicesOf(agent({ model: 'haiku' }))).toEqual([]);
    expect(effortChoicesOf(agent({ model: 'opus' })).length).toBeGreaterThan(0);
  });

  it('sets an option on the agent and tells the core', () => {
    const backend = fakeBackend();
    resetApp({ agents: [agent({ id: 'a1' })] });
    setAgentOption('a1', { effort: 'high' });
    expect(app.agents.a1.effort).toBe('high');
    expect(backend.called('set_agent_options')[0].args).toMatchObject({ id: 'a1', effort: 'high' });
  });
});
