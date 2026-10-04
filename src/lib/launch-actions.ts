// Launch commands: each runs in its own read-only terminal, whose log outlives its runs.

import { api } from './ipc';
import { parseTestId } from './recipe';
import { app } from './state.svelte';
import { disposeLog, launchLog } from './terminals';
import type { LaunchState, Project, RunCommand } from './types';

/** The command's log; the running process follows its size. */
export function log(commandId: string) {
  return launchLog(commandId, (cols, rows) => {
    const pty = app.launches[commandId]?.ptyId;
    if (pty) api.termResize(pty, cols, rows).catch(() => {});
  });
}

const time = () => new Date().toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** How long a start waits for a run of the same command being stopped to end. */
const STOP_WAIT_MS = 10_000;

/** Starts a command not running yet. Returns why it could not start, if it could not. */
export async function startLaunch(project: Project, cmd: RunCommand): Promise<string | null> {
  let previous = app.launches[cmd.id];
  // Being stopped ("Tout arrêter" just before): its end first, then a run of its own, not the dying one.
  if (previous?.status === 'running' && previous.stopping) {
    const dying = previous;
    const until = Date.now() + STOP_WAIT_MS;
    while (app.launches[cmd.id] === dying && dying.status === 'running' && Date.now() < until) await sleep(50);
    previous = app.launches[cmd.id];
    if (previous === dying && dying.status === 'running') return `« ${cmd.name} » ne s'arrête pas`;
    // Forgotten meanwhile (removed, its project closed, its ticket done): nothing to start any more.
    if (!previous) return `« ${cmd.name} » a été retiré`;
  }
  if (previous?.status === 'running') return null;
  const x = log(cmd.id);
  app.launches[cmd.id] = { status: 'running', ptyId: null, name: cmd.name, stopping: false, code: null, startedAt: Date.now() };
  // A run killed in a full-screen program, or with its cursor hidden, must not leave the log so.
  const again = previous ? `\x1b[?1049l\x1b[!p\r\n\x1b[2m— relancé à ${time()} —\x1b[0m\r\n\r\n` : '';
  // Once written, the header's end is where the command starts. The process gets the log's size
  // as it is: displayed, it is already fitted.
  await new Promise<void>((done) => x.term.write(`${again}\x1b[2m$ ${cmd.command}\x1b[0m\r\n`, done));
  // A step of an agent's recipe starts through its agent, in its worktree; any other command, through its project.
  const test = parseTestId(cmd.id);
  try {
    const size = { cols: x.term.cols, rows: x.term.rows, cursorRow: x.term.buffer.active.cursorY + 1 };
    const onData = (b: ArrayBuffer) => x.term.write(new Uint8Array(b));
    const info = test
      ? await api.testRunStart({ agentId: test.agentId, kind: test.kind, index: test.index, ...size }, onData)
      : await api.runStart({ projectId: project.id, commandId: cmd.id, ...size }, onData);
    app.launchStarted(cmd.id, info.id);
    return null;
  } catch (e) {
    const why = String(e);
    x.term.write(`\x1b[31m${why}\x1b[0m\r\n`);
    const l = app.launches[cmd.id];
    if (!l) return why;
    if (test) {
      // Refused (its ticket is being validated, its agent was archived…): no run took place, so none shows.
      // The run it was to follow, if any, stays as it was.
      if (previous) app.launches[cmd.id] = previous;
      else {
        delete app.launches[cmd.id];
        disposeLog(cmd.id);
      }
      if (!l.stopping) app.toast(`« ${cmd.name} » n'a pas pu démarrer : ${why}`, 'error');
      return why;
    }
    Object.assign(l, { status: l.stopping ? 'stopped' : 'crashed', code: null, stopping: false });
    if (l.status === 'crashed') app.toast(`« ${cmd.name} » n'a pas pu démarrer : ${why}`, 'error');
    return why;
  }
}

export function stopLaunch(commandId: string) {
  const l = app.launches[commandId];
  if (l?.status !== 'running') return;
  l.stopping = true;
  // Still starting: killed as soon as it is up.
  if (l.ptyId) api.termKill(l.ptyId).catch(() => {});
}

export async function restartLaunch(project: Project, cmd: RunCommand) {
  const run = app.launches[cmd.id];
  stopLaunch(cmd.id);
  const until = Date.now() + STOP_WAIT_MS;
  while (app.launches[cmd.id] === run && run?.status === 'running' && Date.now() < until) await sleep(50);
  // Started again (by another restart) or removed in the meantime: nothing left to restart. Still not
  // stopped: it stays as it is.
  if (app.launches[cmd.id] !== run || run?.status === 'running') return;
  await startLaunch(project, cmd);
}

export async function startAll(project: Project) {
  await Promise.all(project.runCommands.map((c) => startLaunch(project, c)));
}

export function stopAll(project: Project) {
  for (const c of project.runCommands) stopLaunch(c.id);
}

/** The ids of an agent's test launches. */
function testLaunchesOf(agentId: string): string[] {
  return Object.keys(app.launches).filter((id) => parseTestId(id)?.agentId === agentId);
}

/** The test launches of an agent stop (validation, archive, deletion): not crashes. */
export function stopAgentTests(agentId: string) {
  for (const id of testLaunchesOf(agentId)) stopLaunch(id);
}

/** An agent that is gone takes its test launches with it: stopped silently, their logs dropped. */
export function forgetAgentTests(agentId: string) {
  forgetLaunches(testLaunchesOf(agentId));
}

/** The test launches of the agents of a project. */
export function testLaunchIds(projectId: string): string[] {
  return Object.keys(app.launches).filter((id) => {
    const t = parseTestId(id);
    return !!t && app.agents[t.agentId]?.projectId === projectId;
  });
}

/** Commands about to be killed along with their project: their exits are not crashes. Returns the undo. */
export function expectStops(commandIds: string[]) {
  const marked = commandIds.map((id) => app.launches[id]).filter((l): l is LaunchState => l?.status === 'running' && !l.stopping);
  for (const l of marked) l.stopping = true;
  return () => {
    for (const l of marked) if (l.status === 'running') l.stopping = false;
  };
}

/** Commands going away (removed, or their project closed): stopped silently, their logs dropped and no longer on screen. */
export function forgetLaunches(commandIds: string[]) {
  for (const id of commandIds) {
    const pty = app.launches[id]?.ptyId;
    if (pty) api.termKill(pty).catch(() => {});
    delete app.launches[id];
    disposeLog(id);
    for (const [projectId, shown] of Object.entries(app.selectedLaunch)) if (shown === id) app.selectedLaunch[projectId] = null;
  }
}

// The backend stopped the test launches of an agent it removes: they are not crashes, and nothing is left
// to look for when their project closes. (Registered here: the state does not know the logs.)
app.onAgentRemoved(forgetAgentTests);
// Test launches are kept by the index of their step: under a new recipe, step n is another command,
// which must not be taken for the process still running under that id.
app.onRecipeChanged(forgetAgentTests);
// A ticket "Terminé" is tested no more: its agent's test launches leave the section (the backend stopped
// them at its validation).
app.onTicketDone(forgetAgentTests);

export function launchStatus(l: LaunchState | undefined): { label: string; color: string } {
  if (!l) return { label: 'prêt', color: 'var(--dim)' };
  switch (l.status) {
    case 'running':
      return l.stopping ? { label: 'arrêt…', color: 'var(--wait)' } : { label: 'en cours', color: 'var(--ok)' };
    case 'stopped':
      return { label: 'arrêté', color: 'var(--muted)' };
    case 'done':
      return { label: 'terminé', color: 'var(--muted)' };
    case 'crashed':
      return { label: l.code === null ? 'planté' : `planté (code ${l.code})`, color: 'var(--del)' };
  }
}
