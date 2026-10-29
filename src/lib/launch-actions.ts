// Launch commands: each runs in its own read-only terminal, whose log outlives its runs.

import { fTime } from './format';
import { t } from './i18n';
import { api } from './ipc';
import { inWorktree, parseTestId, parseWorktreeRunId } from './recipe';
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
    if (previous === dying && dying.status === 'running') return t('runs.launch.wontStop', { name: cmd.name });
    // Forgotten meanwhile (removed, its project closed, its ticket done): nothing to start any more.
    if (!previous) return t('runs.launch.removed', { name: cmd.name });
  }
  if (previous?.status === 'running') return null;
  const x = log(cmd.id);
  app.launches[cmd.id] = { status: 'running', ptyId: null, name: cmd.name, stopping: false, code: null, startedAt: Date.now() };
  // A run killed in a full-screen program, or with its cursor hidden, must not leave the log so.
  const again = previous ? `\x1b[?1049l\x1b[!p\r\n\x1b[2m${t('runs.launch.restartedAt', { time: fTime(Date.now()) })}\x1b[0m\r\n\r\n` : '';
  // Once written, the header's end is where the command starts. The process gets the log's size
  // as it is: displayed, it is already fitted.
  await new Promise<void>((done) => x.term.write(`${again}\x1b[2m$ ${cmd.command}\x1b[0m\r\n`, done));
  // A step of an agent's recipe starts through its agent, in its worktree; a launch command, through its project
  // (in the worktree of an agent for a `wt:` id).
  const test = parseTestId(cmd.id);
  const wt = parseWorktreeRunId(cmd.id);
  try {
    const size = { cols: x.term.cols, rows: x.term.rows, cursorRow: x.term.buffer.active.cursorY + 1 };
    const onData = (b: ArrayBuffer) => x.term.write(new Uint8Array(b));
    const info = test
      ? await api.testRunStart({ agentId: test.agentId, kind: test.kind, index: test.index, ...size }, onData)
      : wt
        ? await api.runStart({ projectId: project.id, commandId: wt.commandId, agentId: wt.agentId, ...size }, onData)
        : await api.runStart({ projectId: project.id, commandId: cmd.id, ...size }, onData);
    app.launchStarted(cmd.id, info.id);
    return null;
  } catch (e) {
    const why = String(e);
    x.term.write(`\x1b[31m${why}\x1b[0m\r\n`);
    const l = app.launches[cmd.id];
    if (!l) return why;
    if (test || wt) {
      // Refused (its ticket is being validated, its agent was archived, its folder is not in its worktree…): no
      // run took place, so none shows.
      // The run it was to follow, if any, stays as it was.
      if (previous) app.launches[cmd.id] = previous;
      else {
        delete app.launches[cmd.id];
        disposeLog(cmd.id);
      }
      if (!l.stopping) app.toast(t('runs.launch.couldNotStart', { name: cmd.name, why }), 'error');
      return why;
    }
    Object.assign(l, { status: l.stopping ? 'stopped' : 'crashed', code: null, stopping: false });
    if (l.status === 'crashed') app.toast(t('runs.launch.couldNotStart', { name: cmd.name, why }), 'error');
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

/** « Tout lancer », in the project. The commands already running in a worktree are told first, all at once. */
export async function startAll(project: Project) {
  const elsewhere = project.runCommands.flatMap((c) =>
    runsOfCommand(c.id).filter((id) => id !== c.id && app.launches[id]?.status === 'running'),
  );
  const all = () => Promise.all(project.runCommands.map((c) => startLaunch(project, c)));
  if (!elsewhere.length) {
    await all();
    return;
  }
  const list = elsewhere.map((id) => t('runs.conflict.item', { name: app.launches[id].name })).join(', ');
  app.modal = {
    kind: 'confirm',
    title: t('runs.conflict.titleSome'),
    icon: 'play',
    body: t('runs.conflict.bodySome', { list }),
    confirm: t('runs.conflict.stopOthers'),
    onConfirm: async () => {
      for (const id of elsewhere) stopLaunch(id);
      const until = Date.now() + STOP_WAIT_MS;
      while (elsewhere.some((id) => app.launches[id]?.status === 'running') && Date.now() < until) await sleep(50);
      await all();
    },
    alt: {
      label: t('runs.conflict.anyway'),
      onClick: async () => {
        await all();
      },
    },
  };
}

export function stopAll(project: Project) {
  for (const c of project.runCommands) stopLaunch(c.id);
}

/** The ids of an agent's test launches. */
function testLaunchesOf(agentId: string): string[] {
  return Object.keys(app.launches).filter((id) => parseTestId(id)?.agentId === agentId);
}

/** The agent a launch runs for: a step of its test launch, or a launch command in its worktree. */
function agentOf(id: string): string | undefined {
  return parseTestId(id)?.agentId ?? parseWorktreeRunId(id)?.agentId;
}

/** The ids of what runs for an agent: its test launches and the launch commands in its worktree. */
function agentLaunchesOf(agentId: string): string[] {
  return Object.keys(app.launches).filter((id) => agentOf(id) === agentId);
}

/** What runs for an agent stops (validation, archive, deletion), its worktree launches included: not crashes. */
export function stopAgentTests(agentId: string) {
  for (const id of agentLaunchesOf(agentId)) stopLaunch(id);
}

/** An agent that is gone takes what ran for it with it: stopped silently, their logs dropped. */
export function forgetAgentTests(agentId: string) {
  forgetLaunches(agentLaunchesOf(agentId));
}

/** What runs for the agents of a project: their test launches and the launch commands in their worktrees. */
export function testLaunchIds(projectId: string): string[] {
  return Object.keys(app.launches).filter((id) => {
    const a = agentOf(id);
    return !!a && app.agents[a]?.projectId === projectId;
  });
}

/** The ids of the runs of the launch command `commandId`: in the project, and in any worktree. */
export function runsOfCommand(commandId: string): string[] {
  return Object.keys(app.launches).filter((id) => id === commandId || parseWorktreeRunId(id)?.commandId === commandId);
}

/** Where a run of a launch command is: in the project, or in the worktree of an agent. */
function whereOf(id: string): string {
  const w = parseWorktreeRunId(id);
  const name = w ? app.agents[w.agentId]?.name : undefined;
  return name ? t('runs.where.inWorktree', { name }) : t('runs.where.inProject');
}

/**
 * Starts `cmd` in the project (`agentId` null) or in the worktree of `agentId`. Running elsewhere already,
 * the two would fight over its port: the user says first whether the other stops.
 */
export async function runIn(project: Project, cmd: RunCommand, agentId: string | null) {
  const agent = agentId ? app.agents[agentId] : undefined;
  if (agentId && !agent) return;
  const target = agent ? inWorktree(cmd, agent) : cmd;
  const others = runsOfCommand(cmd.id).filter((id) => id !== target.id && app.launches[id]?.status === 'running');
  if (!others.length) {
    await startLaunch(project, target);
    return;
  }
  app.modal = {
    kind: 'confirm',
    title: t('runs.conflict.title', { name: cmd.name }),
    icon: 'play',
    body: t('runs.conflict.body', { where: others.map(whereOf).join(', ') }),
    confirm: t('runs.conflict.stopOther'),
    onConfirm: async () => {
      for (const id of others) stopLaunch(id);
      const until = Date.now() + STOP_WAIT_MS;
      while (others.some((id) => app.launches[id]?.status === 'running') && Date.now() < until) await sleep(50);
      await startLaunch(project, target);
    },
    alt: {
      label: t('runs.conflict.anyway'),
      onClick: async () => {
        await startLaunch(project, target);
      },
    },
  };
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
// which must not be taken for the process still running under that id. Its worktree launches stay.
app.onRecipeChanged((agentId) => forgetLaunches(testLaunchesOf(agentId)));
// A ticket "Terminé" is tested no more: its agent's test launches leave the section (the backend stopped
// them at its validation).
app.onTicketDone(forgetAgentTests);

/** How a run that crashed is said: with its exit code when it has one (on a command's status and on a test's line). */
export function crashedLabel(code: number | null | undefined): string {
  return code == null ? t('runs.status.crashed') : t('runs.status.crashedCode', { code });
}

export function launchStatus(l: LaunchState | undefined): { label: string; color: string } {
  if (!l) return { label: t('runs.status.ready'), color: 'var(--dim)' };
  switch (l.status) {
    case 'running':
      return l.stopping
        ? { label: t('runs.status.stopping'), color: 'var(--wait)' }
        : { label: t('runs.status.running'), color: 'var(--run)' };
    case 'stopped':
      return { label: t('runs.status.stopped'), color: 'var(--muted)' };
    case 'done':
      return { label: t('runs.status.done'), color: 'var(--muted)' };
    case 'crashed':
      return { label: crashedLabel(l.code), color: 'var(--del)' };
  }
}

/** ▶ on a command that is not running (its row, its log): where it ran, through `runIn` for a launch command. */
export function runHere(project: Project, cmd: RunCommand) {
  const w = parseWorktreeRunId(cmd.id);
  const own = project.runCommands.find((c) => c.id === (w?.commandId ?? cmd.id));
  return own ? runIn(project, own, w?.agentId ?? null) : startLaunch(project, cmd);
}
