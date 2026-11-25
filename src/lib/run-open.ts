// « Ouvrir à son lancement » (ESC-560): a launch command with an address opens it once it answers, by the same bounded
// wait as "▶ Tester" (`waitUntilUp`), never after a fixed delay. Run in an agent's worktree, the address follows the
// agent's ports and opens in the agent's page (one per agent, loaded again rather than doubled); run in the project,
// in a tab of the project's « Pages » window (one per command). Or in the system's browser, as the command or the
// project says. A command that ends before answering opens nothing, and its log says so.

import { openLaunchPage, openProjectPage } from './devpage.svelte';
import { t } from './i18n';
import { openUrl } from './opener';
import { parseWorktreeRunId } from './recipe';
import { expandRunUrl, portVars, runUrlProblem } from './run-url';
import { app } from './state.svelte';
import type { LaunchState, Project, RunCommand } from './types';
import { waitUntilUp } from './wait-ready';

/** What came of it: opened (where), nothing to open, or why nothing opened. */
export type RunOpenOutcome = 'page' | 'browser' | 'none' | 'invalid' | 'ended' | 'stopped' | 'late' | 'over';

const dim = (text: string) => `\r\n\x1b[2m${text}\x1b[0m\r\n`;
const red = (text: string) => `\r\n\x1b[31m${text}\x1b[0m\r\n`;

/** The address `cmd` opens where it runs (its worktree's ports), or why it cannot be; null when it has none. */
export function addressOf(cmd: RunCommand): { url: string } | { error: string } | null {
  const typed = cmd.url?.trim();
  if (!typed) return null;
  const problem = runUrlProblem(typed);
  if (problem) return { error: problem };
  const w = parseWorktreeRunId(cmd.id);
  const x = expandRunUrl(typed, portVars(w ? app.agents[w.agentId]?.portBase : null));
  return x.error !== undefined ? { error: x.error } : { url: x.url };
}

/**
 * The run of `cmd` just started: its address, once it answers, opens where it says. `write` puts a line in its log.
 * Waits as long as this run lasts: a run started again in its place opens it itself.
 */
export async function openWhenUp(project: Project, cmd: RunCommand, write: (text: string) => void): Promise<RunOpenOutcome> {
  const address = addressOf(cmd);
  if (!address) return 'none';
  if ('error' in address) {
    write(red(t('runs.open.cannot', { why: address.error })));
    return 'invalid';
  }
  const url = address.url;
  const run = app.launches[cmd.id];
  if (run?.status !== 'running') return 'over';
  write(dim(t('runs.open.waiting', { url })));
  const wait = await waitUntilUp(url, {
    since: run.startedAt ?? Date.now(),
    over: () => app.launches[cmd.id] !== run,
    running: () => run.status === 'running' && !run.stopping,
  });
  if (wait === 'over') return 'over';
  if (wait === 'ended') {
    // Stopped on purpose: nothing to say. Ended by itself (crashed, or done) before answering: said.
    const status = (run as LaunchState).status as LaunchState['status'];
    if (run.stopping || status === 'stopped') return 'stopped';
    write(red(t('runs.open.ended', { name: cmd.name, url })));
    return 'ended';
  }
  if (wait === 'late') {
    write(red(t('runs.open.late', { url, minutes: 3 })));
    return 'late';
  }
  try {
    const where = await openAt(project, cmd, url);
    write(dim(t(`runs.open.opened.${where}`, { url })));
    return where;
  } catch (e) {
    write(red(t('runs.open.cannot', { why: String(e) })));
    return 'invalid';
  }
}

/** Opens `url` for `cmd`: its browser's row (its agent's, or its own in the project, ESC-657), or the system's browser. */
async function openAt(project: Project, cmd: RunCommand, url: string): Promise<'page' | 'browser'> {
  if (cmd.openIn === 'browser' || project.pagesOpen === 'system') {
    await openUrl(url);
    return 'browser';
  }
  const w = parseWorktreeRunId(cmd.id);
  const agent = w ? app.agents[w.agentId] : undefined;
  if (w && !agent) throw new Error(t('runs.launch.removed', { name: cmd.name }));
  if (agent) {
    const own = project.runCommands.find((c) => c.id === w?.commandId)?.name ?? cmd.name;
    const where = await openLaunchPage(agent, url, `${agent.name} · ${own}`);
    return where === 'system' ? 'browser' : 'page';
  }
  return (await openProjectPage(project, cmd.id, cmd.name, url)) === 'system' ? 'browser' : 'page';
}
