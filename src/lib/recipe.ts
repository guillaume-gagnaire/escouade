// An agent's test launch as launch commands: the steps of its recipe's preparation and its
// processes, each with an id `test:<agent>:<prep|run>:<n>` among the launches.

import type { Agent, RunCommand } from './types';

export type TestKind = 'prep' | 'run';

export const testId = (agentId: string, kind: TestKind, index: number) => `test:${agentId}:${kind}:${index}`;

export function parseTestId(id: string): { agentId: string; kind: TestKind; index: number } | null {
  const m = /^test:(.+):(prep|run):(\d+)$/.exec(id);
  return m ? { agentId: m[1], kind: m[2] as TestKind, index: Number(m[3]) } : null;
}

/** The recipe's steps as launch commands, run by `shell` (the system's default one). */
export function recipeCommands(agent: Agent, shell: string): { prepare: RunCommand[]; processes: RunCommand[] } {
  const r = agent.recipe;
  if (!r) return { prepare: [], processes: [] };
  return {
    prepare: r.prepare.map((s, i) => ({
      id: testId(agent.id, 'prep', i),
      name: `Préparation ${i + 1}`,
      command: s.command,
      shell,
      cwd: s.dir,
    })),
    processes: r.processes.map((p, i) => ({
      id: testId(agent.id, 'run', i),
      name: p.name || `processus ${i + 1}`,
      command: p.command,
      shell,
      cwd: p.dir,
    })),
  };
}

/** The launch command of a test launch's id, while its agent's recipe still has that step. */
export function testCommand(id: string, agents: Record<string, Agent>, shell: string): RunCommand | null {
  const t = parseTestId(id);
  const a = t ? agents[t.agentId] : undefined;
  if (!t || !a) return null;
  const c = recipeCommands(a, shell);
  return (t.kind === 'prep' ? c.prepare : c.processes)[t.index] ?? null;
}

/** The address the browser opens: the one showing the feature, else the first process's. */
export function openAddress(agent: Agent): string | null {
  const r = agent.recipe;
  if (!r) return null;
  return r.open.trim() || r.processes.map((p) => p.url.trim()).find(Boolean) || null;
}
