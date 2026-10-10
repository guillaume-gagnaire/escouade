// An agent's test launch as launch commands: the steps of its recipe's preparation and its
// processes, or `isola up` when isola runs its worktree's services, each with an id
// `test:<agent>:<prep|run|isola>:<n>` among the launches.

import type { Agent, RunCommand } from './types';

export type TestKind = 'prep' | 'run' | 'isola';

export const testId = (agentId: string, kind: TestKind, index: number) => `test:${agentId}:${kind}:${index}`;

export function parseTestId(id: string): { agentId: string; kind: TestKind; index: number } | null {
  const m = /^test:(.+):(prep|run|isola):(\d+)$/.exec(id);
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

/** The test launch of an agent whose services isola runs: `isola up` in its worktree. */
export function isolaCommand(agent: Agent, shell: string): RunCommand {
  return { id: testId(agent.id, 'isola', 0), name: 'isola up', command: 'isola up', shell, cwd: '' };
}

/** The launch command of a test launch's id, while its agent's recipe still has that step (or isola its services). */
export function testCommand(id: string, agents: Record<string, Agent>, shell: string): RunCommand | null {
  const t = parseTestId(id);
  const a = t ? agents[t.agentId] : undefined;
  if (!t || !a) return null;
  if (t.kind === 'isola') return a.isola && t.index === 0 ? isolaCommand(a, shell) : null;
  const c = recipeCommands(a, shell);
  return (t.kind === 'prep' ? c.prepare : c.processes)[t.index] ?? null;
}

/** The address the browser opens: the one showing the feature, else the first process's. */
export function openAddress(agent: Agent): string | null {
  const r = agent.recipe;
  if (!r) return null;
  return r.open.trim() || r.processes.map((p) => p.url.trim()).find(Boolean) || null;
}

/**
 * The user read this recipe and let it run: the content they approved is the one the agent holds now. Anything that
 * differs (a command, a variable, a folder, the address to open) is a recipe to read again. The backend decides
 * the same way, from the same two fields; this only spares a launch it would refuse.
 */
export function recipeApproved(agent: Agent): boolean {
  return !!agent.recipe && JSON.stringify(agent.recipe) === JSON.stringify(agent.approvedRecipe);
}

/** Characters that change how text looks without being seen: controls (but not line breaks and tabs), zero-width and direction marks. */
function hidden(code: number): boolean {
  return (
    (code < 0x20 && code !== 0x09 && code !== 0x0a) ||
    (code >= 0x7f && code <= 0x9f) ||
    (code >= 0x200b && code <= 0x200f) ||
    (code >= 0x2028 && code <= 0x202e) ||
    (code >= 0x2060 && code <= 0x2069) ||
    code === 0xfeff
  );
}

/**
 * A command, a folder or an address of a recipe as the user is shown it: the characters that would hide part of it (a
 * carriage return that overwrites the line, an escape sequence, a zero-width space, a right-to-left override) are spelled
 * out, so that what is read is what runs.
 */
export function revealHidden(text: string): string {
  return Array.from(text, (ch) => {
    const code = ch.codePointAt(0) ?? 0;
    return hidden(code) ? `⟨U+${code.toString(16).toUpperCase().padStart(4, '0')}⟩` : ch;
  }).join('');
}
