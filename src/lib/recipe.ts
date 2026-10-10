// An agent's test launch as launch commands: the steps of its recipe's preparation and its
// processes, or `isola up` when isola runs its worktree's services, each with an id
// `test:<agent>:<prep|run|isola>:<n>` among the launches.

import { t } from './i18n';
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
      name: t('runs.recipe.prepStep', { n: i + 1 }),
      command: s.command,
      shell,
      cwd: s.dir,
    })),
    processes: r.processes.map((p, i) => ({
      id: testId(agent.id, 'run', i),
      name: p.name || t('runs.recipe.process', { n: i + 1 }),
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

/**
 * Characters that change how text looks, or draw nothing, without being seen: the Unicode categories of controls, of
 * format characters (zero-width marks, direction marks and overrides, the Arabic letter mark…) and of line and paragraph
 * separators, and every character that is ignored when drawn (variation selectors, hangul fillers, tag characters…).
 * Decided by property, not by a list that has to name each one: what a list forgets is what gets used.
 */
const HIDDEN = /[\p{Cc}\p{Cf}\p{Zl}\p{Zp}\p{Default_Ignorable_Code_Point}]/u;

/** What the property tables of an older WebView may not know yet, and what draws a blank without being ignorable. */
const ALSO_HIDDEN: [number, number][] = [
  [0x180b, 0x180f], // Mongolian free variation selectors (the last one is Unicode 14)
  [0x2800, 0x2800], // braille blank
  [0xfe00, 0xfe0f], // variation selectors
  [0xfff0, 0xfff8], // unassigned, ignorable
  [0x1bca0, 0x1bca3], // shorthand format controls
  [0x1d173, 0x1d17a], // musical format controls
  [0xe0000, 0xe0fff], // tag characters and the variation selectors supplement
];

/** Empty lines in a row from which they are counted instead of shown, and spaces in a row likewise. */
const BLANK_LINES = 2;
const SPACES = 24;

/** The character is not drawn, or is drawn as something else than itself: not the tab and the line break. */
function hidden(ch: string): boolean {
  if (ch === '\t' || ch === '\n') return false;
  const code = ch.codePointAt(0) ?? 0;
  return HIDDEN.test(ch) || ALSO_HIDDEN.some(([from, to]) => code >= from && code <= to);
}

/** The character shows nothing: a space, or one that is not drawn. */
const blank = (ch: string) => hidden(ch) || /\s/.test(ch);

/** The code of a hidden character, as the user is shown it. */
const spelled = (ch: string) => `⟨U+${(ch.codePointAt(0) ?? 0).toString(16).toUpperCase().padStart(4, '0')}⟩`;

/** A stretch of blank characters as it is shown: as it is, with each hidden one spelled out, or counted when it is long. */
function showBlanks(run: string[]): string {
  if (run.length < SPACES) return run.map((ch) => (hidden(ch) ? spelled(ch) : ch)).join('');
  return t(run.some(hidden) ? 'runs.recipe.spacesInvisible' : 'runs.recipe.spaces', { n: run.length });
}

/** A line as it is shown: its hidden characters spelled out, its long stretches of blanks (hidden ones too) counted. */
function showLine(line: string): string {
  const out: string[] = [];
  let run: string[] = [];
  for (const ch of line) {
    if (blank(ch)) {
      run.push(ch);
      continue;
    }
    out.push(showBlanks(run), ch);
    run = [];
  }
  out.push(showBlanks(run));
  return out.join('');
}

/**
 * Text of a recipe as the user is shown it, so that what is read is what runs: the characters that would hide part of
 * it (a carriage return that overwrites the line, an escape sequence, a zero-width space, a variation selector, a
 * right-to-left override) are spelled out, and so are the runs of empty lines or of spaces, invisible characters
 * among them, that would push the rest of a command out of sight.
 */
export function revealHidden(text: string): string {
  // A Windows line ending is a line break; the last line break ends the last line, it does not start an empty one.
  const unix = text.split('\r\n').join('\n');
  const ends = unix.endsWith('\n');
  const lines = (ends ? unix.slice(0, -1) : unix).split('\n');
  const out: string[] = [];
  let empty: string[] = [];
  const flush = () => {
    if (empty.length >= BLANK_LINES) {
      const invisible = empty.some((line) => Array.from(line).some(hidden));
      out.push(t(invisible ? 'runs.recipe.emptyLinesInvisible' : 'runs.recipe.emptyLines', { n: empty.length }));
    } else out.push(...empty.map(showLine));
    empty = [];
  };
  for (const line of lines) {
    // A line that shows nothing is an empty one, whatever it holds.
    if (Array.from(line).every(blank)) {
      empty.push(line);
      continue;
    }
    flush();
    out.push(showLine(line));
  }
  flush();
  return out.join('\n') + (ends ? '\n' : '');
}
