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

/**
 * The isola launch was approved: the .isola.toml the user read is the worktree's now (\`config\`, read again), and the
 * address to open is the one they were shown. The backend decides the same way for the file, when the services start.
 */
export function isolaApproved(agent: Agent, config: string): boolean {
  const approved = agent.approvedIsola;
  return !!approved && approved.config === config && approved.open === (agent.recipe?.open ?? '');
}

/** Code point ranges of the characters that change how text looks, or draw nothing, without being seen. */
const HIDDEN: [number, number][] = [
  [0x00, 0x08], // controls, but not the tab (0x09) and the line break (0x0a)
  [0x0b, 0x1f], // including the carriage return that overwrites a line, and escape
  [0x7f, 0x9f],
  [0xad, 0xad], // soft hyphen
  [0x34f, 0x34f], // combining grapheme joiner
  [0x61c, 0x61c], // Arabic letter mark: reorders what follows it
  [0x115f, 0x1160], // hangul fillers
  [0x17b4, 0x17b5],
  [0x180b, 0x180e], // Mongolian free variation selectors and separator
  [0x200b, 0x200f], // zero-width spaces and joiners, left-to-right and right-to-left marks
  [0x2028, 0x202e], // line and paragraph separators, direction embeddings and overrides
  [0x2060, 0x2069], // word joiner, invisible operators, direction isolates
  [0x2800, 0x2800], // braille blank
  [0x3164, 0x3164], // hangul filler
  [0xfeff, 0xfeff], // zero-width no-break space
  [0xffa0, 0xffa0], // halfwidth hangul filler
  [0xe0000, 0xe007f], // tag characters
];

/** Empty lines in a row from which they are counted instead of shown, and spaces in a row likewise. */
const BLANK_LINES = 2;
const SPACES = 24;

/**
 * Text of a recipe as the user is shown it, so that what is read is what runs: the characters that would hide part of
 * it (a carriage return that overwrites the line, an escape sequence, a zero-width space, a right-to-left override) are
 * spelled out, and so are the runs of empty lines or of spaces that would push the rest of a command out of sight.
 */
export function revealHidden(text: string): string {
  const spelled = Array.from(text.split('\r\n').join('\n'), (ch) => {
    const code = ch.codePointAt(0) ?? 0;
    return HIDDEN.some(([from, to]) => code >= from && code <= to) ? `⟨U+${code.toString(16).toUpperCase().padStart(4, '0')}⟩` : ch;
  }).join('');
  return countBlanks(spelled);
}

/** Runs of empty lines (blank ones too) and of spaces long enough to hide something, as « ⟨12 lignes vides⟩ ». */
function countBlanks(text: string): string {
  // The last line break ends the last line: it does not start an empty one.
  const ends = text.endsWith('\n');
  const lines = (ends ? text.slice(0, -1) : text).split('\n');
  const out: string[] = [];
  let blank: string[] = [];
  const flush = () => {
    out.push(...(blank.length >= BLANK_LINES ? [`⟨${blank.length} lignes vides⟩`] : blank));
    blank = [];
  };
  for (const line of lines) {
    if (line.trim() === '') {
      blank.push(line);
      continue;
    }
    flush();
    out.push(line.replace(/[^\S\n]+/g, (run) => (run.length >= SPACES ? `⟨${run.length} espaces⟩` : run)));
  }
  flush();
  return out.join('\n') + (ends ? '\n' : '');
}
