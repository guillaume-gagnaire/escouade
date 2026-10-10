// "▶ Tester": prepares the worktree (once per recipe), starts its processes, waits until each
// answers over HTTP, then opens the browser on the address that shows the feature. When isola runs
// the worktree's services: `isola up`, then the addresses it lists. A recipe is the agent's own text, run in
// the user's shell outside Claude Code's permission mode: the modal shows it first, and nothing of it runs until
// the user lets it (`approveAndTest`; the backend refuses a recipe that was not approved all the same).

import { openUrl } from '@tauri-apps/plugin-opener';
import { api } from './ipc';
import { startLaunch, stopAgentTests, stopLaunch } from './launch-actions';
import { isolaCommand, openAddress, parseTestId, recipeApproved, recipeCommands } from './recipe';
import { app } from './state.svelte';
import type { Agent, IsolaService, LaunchState, Project, RunCommand } from './types';

/** How often a server is asked whether it is up, and for how long at most. */
export const POLL_MS = 500;
export const READY_LIMIT_MS = 3 * 60_000;

export interface FlowLine {
  id: string;
  label: string;
  /**
   * `stopped`: stopped on purpose ("Tout arrêter", its own ■), not a failure. `skipped`: no longer waited
   * for, the test having failed elsewhere.
   */
  state: 'running' | 'waiting' | 'ready' | 'failed' | 'stopped' | 'skipped';
  detail: string;
  /** Its log among the launches (a step that could not start has none). */
  launchId: string;
}

export interface TestFlow {
  lines: FlowLine[];
  phase: 'running' | 'ready' | 'failed';
  error: string | null;
  /** The address opened in the browser. */
  opened: string | null;
}

class TestFlows {
  /** By agent. */
  all = $state<Record<string, TestFlow>>({});
  /** The preparation that succeeded, by agent (its recipe's steps as JSON). */
  prepared = $state<Record<string, string>>({});
}

export const flows = new TestFlows();

// A test under way on a recipe the agent replaced, or of an agent that is gone, is over: its steps are
// no longer the recipe's, and its launches are forgotten with it (launch-actions).
app.onRecipeChanged((id) => delete flows.all[id]);
app.onAgentRemoved((id) => {
  delete flows.all[id];
  delete flows.prepared[id];
});
// So is the test of an agent whose ticket goes "Terminé", and its modal, whose lines had their logs.
app.onTicketDone((id) => {
  delete flows.all[id];
  if (app.modal?.kind === 'testLaunch' && app.modal.agentId === id) app.modal = null;
});

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const seconds = (ms: number) => (ms / 1000).toLocaleString('fr-FR', { maximumFractionDigits: 1 });
const isPrep = (id: string) => parseTestId(id)?.kind === 'prep';
/** A step that ends once done (a preparation, isola up), unlike a process. */
const ends = (id: string) => isPrep(id) || parseTestId(id)?.kind === 'isola';

function host(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/** All its processes run. */
export function allRunning(agent: Agent): boolean {
  const ids = recipeCommands(agent, '').processes.map((c) => c.id);
  return ids.length > 0 && ids.every((id) => app.launches[id]?.status === 'running');
}

/** One of its test launches runs (isola's services once its test is up: they run apart from the app). */
export function anyRunning(agentId: string): boolean {
  if (app.agents[agentId]?.isola && flows.all[agentId]?.phase === 'ready') return true;
  return Object.entries(app.launches).some(([id, l]) => l.status === 'running' && parseTestId(id)?.agentId === agentId);
}

/** Its ticket is being validated: the backend refuses its test launches until it is done. */
function validating(agentId: string): boolean {
  return Object.values(app.tickets).some((t) => t.agentId === agentId && !!t.step);
}

/**
 * "▶ Tester" makes sense for it: a recipe (or isola to run its services), a worktree to run it in, not archived,
 * no validation under way.
 */
export function canTest(agent: Agent): boolean {
  return (!!agent.recipe || agent.isola) && !!agent.worktree && !agent.archived && !validating(agent.id);
}

/**
 * "Préparer le lancement" makes sense for it: a worktree whose services isola does not run, not archived, and no
 * ticket under way, whose protocol already asks for the recipe (an answer that is only a recipe would count as a
 * missing report).
 */
export function canPrepare(agent: Agent): boolean {
  return !!agent.worktree && !agent.isola && !agent.archived && !app.ticketDoing(agent.id);
}

/** "Préparer le lancement". */
export function prepareLaunch(agent: Agent) {
  return app.run(api.agentPrepareLaunch(agent.id));
}

/** The end of a launch: the run as it ended, or null when it was stopped, or gone, or `live` no longer holds. */
async function ended(id: string, live: () => boolean): Promise<LaunchState | null> {
  // The run started for this step: another one under the same id (a new test) is not its end.
  const run = app.launches[id];
  while (live() && app.launches[id] === run && run?.status === 'running') await sleep(100);
  const l = app.launches[id];
  return live() && l && l === run && l.status !== 'stopped' ? l : null;
}

/** "Tout arrêter": its test launches stop (isola's services with `isola down`), and its lines say so. */
export function stopTests(agentId: string) {
  stopAgentTests(agentId);
  if (app.agents[agentId]?.isola) app.run(api.isolaDown(agentId));
  const flow = flows.all[agentId];
  if (!flow) return;
  for (const l of flow.lines) {
    // What failed keeps its reason, a preparation step (or isola up) that finished stays so.
    if (l.state === 'failed' || l.state === 'stopped' || (l.state === 'ready' && ends(l.id))) continue;
    l.state = 'stopped';
    l.detail = isPrep(l.id) ? 'arrêtée' : 'arrêté';
  }
  // A failure keeps its reason too.
  if (flow.phase !== 'failed') {
    flow.phase = 'failed';
    flow.error = 'Arrêté';
  }
  flow.opened = null;
}

/**
 * "▶ Tester" on `agent`: the modal shows each step as it goes. A recipe the user never read (or that changed since)
 * is only shown, with « Lancer » (`approveAndTest`) and « Annuler ». isola's test has no recipe to read: `isola up`
 * is fixed and the services are the project's, so nothing the agent wrote is run (only the address it gave is opened).
 */
export async function testAgent(agent: Agent, project: Project) {
  app.modal = { kind: 'testLaunch', agentId: agent.id };
  if (!agent.isola && agent.recipe && !recipeApproved(agent)) return;
  await launchTest(agent, project);
}

/**
 * « Lancer » on a recipe shown: the backend is told which recipe was read (the one the modal showed, so that a
 * recipe the agent sent meanwhile is refused rather than approved unseen), then the test goes on.
 */
export async function approveAndTest(agent: Agent, project: Project) {
  const recipe = agent.recipe;
  if (!recipe) return;
  try {
    await api.testRecipeApprove(agent.id, recipe);
  } catch (e) {
    app.toast(String(e), 'error');
    return;
  }
  await launchTest(agent, project);
}

/** The test itself, its recipe approved (or isola's, which has none to approve). */
async function launchTest(agent: Agent, project: Project) {
  const recipe = agent.recipe;
  if (!recipe && !agent.isola) return;
  const current = flows.all[agent.id];
  // Already up: the modal on its state, and the browser again. (isola's services run apart from the app, and may
  // have been stopped since: its test checks again, isola up taking no time when all of them run.)
  if (current?.phase === 'ready' && !agent.isola && allRunning(agent)) {
    if (current.opened) app.run(openUrl(current.opened));
    return;
  }
  if (current?.phase === 'running') return;
  flows.all[agent.id] = { lines: [], phase: 'running', error: null, opened: null };
  // Read back: written through the state's proxy, every change shows in the modal.
  const flow = flows.all[agent.id];
  /** Still the agent's test: not replaced by a new recipe, the agent's removal or another test. */
  const live = () => flows.all[agent.id] === flow;
  /** Nothing more to start or wait for. */
  const over = () => !live() || flow.phase !== 'running';
  /**
   * Starts a step for this test. A run it starts once the test is over ("Tout arrêter" again while it waited for the
   * dying run of the step) is stopped at once: nothing is left running behind a stopped test.
   */
  const start = async (cmd: RunCommand) => {
    const before = app.launches[cmd.id];
    const why = await startLaunch(project, cmd);
    const run = app.launches[cmd.id];
    if (why === null && live() && over() && run !== before && run?.status === 'running') stopLaunch(cmd.id);
    return why;
  };
  const cmds = recipeCommands(agent, app.shells[0]?.id ?? '');
  const addLine = (cmd: RunCommand, label: string, detail: string): FlowLine => {
    flow.lines.push({ id: cmd.id, label, state: 'running', detail, launchId: cmd.id });
    return flow.lines[flow.lines.length - 1];
  };
  /** The test is over, for `error` (under way, or once ready: a process that ended). */
  const fail = (error: string) => {
    // "Tout arrêter", or an earlier failure, already said why.
    if (flow.phase === 'failed') return;
    flow.phase = 'failed';
    flow.error = error;
    flow.opened = null;
    // What was still under way is no longer waited for: no line claims to be.
    for (const l of flow.lines) {
      if (l.state !== 'running' && l.state !== 'waiting') continue;
      l.state = 'skipped';
      l.detail = 'non attendu';
    }
  };
  /** A step the backend refused to start: why, on its line, which has no log. */
  const refused = (line: FlowLine, cmd: RunCommand, why: string) => {
    line.state = 'failed';
    line.detail = why;
    fail(`« ${cmd.name} » n'a pas pu démarrer`);
  };
  /** A process that ended under the test: its line says how, and the test is over. */
  const exited = (line: FlowLine, cmd: RunCommand, run: LaunchState | undefined) => {
    if (!run || run.status === 'stopped') {
      line.state = 'stopped';
      line.detail = 'arrêté';
      return fail('Arrêté');
    }
    line.state = 'failed';
    line.detail = run.status === 'done' ? 'terminé' : run.code == null ? 'planté' : `planté (code ${run.code})`;
    fail(`${cmd.name} s'est arrêté`);
  };

  // isola runs the worktree's services: `isola up`, then the addresses it lists, until each answers.
  if (agent.isola) {
    const cmd = isolaCommand(agent, app.shells[0]?.id ?? '');
    const line = addLine(cmd, 'isola up', 'en cours…');
    const why = await start(cmd);
    if (!live()) return;
    if (why !== null) return over() ? undefined : refused(line, cmd, why);
    const end = await ended(cmd.id, live);
    if (!live()) return;
    if (!end) {
      line.state = 'stopped';
      line.detail = 'arrêté';
      return fail('Arrêté');
    }
    if (end.status !== 'done') {
      line.state = 'failed';
      line.detail = `code ${end.code ?? '?'}`;
      return fail(`isola up en échec (code ${end.code ?? '?'})`);
    }
    line.state = 'ready';
    line.detail = 'terminé';
    let services: IsolaService[];
    try {
      services = await api.isolaServices(agent.id);
    } catch (e) {
      if (!over()) fail(String(e));
      return;
    }
    if (over()) return;
    const started = Date.now();
    // Their logs are isola's: a line without one.
    const lines = services.map((s) => {
      flow.lines.push({ id: `${cmd.id}:${s.name}`, label: s.name, state: 'running', detail: '', launchId: '' });
      return flow.lines[flow.lines.length - 1];
    });
    const ready = await Promise.all(
      services.map(async (s, i) => {
        const line = lines[i];
        if (s.status !== 'running') {
          line.state = 'failed';
          line.detail = s.status || 'arrêté';
          fail(`${s.name} ne tourne pas`);
          return false;
        }
        // Its own port: the proxy answers (with an error page) before the service does.
        const probe = s.probe || s.url;
        if (!probe) {
          line.state = 'ready';
          line.detail = 'démarré';
          return true;
        }
        line.state = 'waiting';
        line.detail = `en attente de ${host(probe)}…`;
        for (;;) {
          if (over()) return false;
          const up = await api.httpReady(probe).catch(() => false);
          if (over()) return false;
          if (up) {
            line.state = 'ready';
            line.detail = `prêt · ${seconds(Date.now() - started)} s`;
            return true;
          }
          if (Date.now() - started >= READY_LIMIT_MS) {
            line.state = 'failed';
            line.detail = `Pas de réponse de ${probe} après 3 min`;
            fail(line.detail);
            return false;
          }
          await sleep(POLL_MS);
        }
      }),
    );
    if (over() || !ready.every(Boolean)) return;
    // The address the agent gave for the feature, else the first service's.
    const address = recipe?.open.trim() || services.map((s) => s.url).find(Boolean) || null;
    flow.phase = 'ready';
    flow.opened = address;
    if (address) app.run(openUrl(address));
    return;
  }
  if (!recipe) return;

  // 1. The preparation, once per recipe, one step after the other.
  const prepared = JSON.stringify(recipe.prepare);
  if (recipe.prepare.length && flows.prepared[agent.id] !== prepared) {
    for (const cmd of cmds.prepare) {
      if (over()) return;
      const line = addLine(cmd, `Préparation : ${cmd.command}`, 'en cours…');
      const why = await start(cmd);
      if (!live()) return;
      // Stopped while it was starting: its line already says so.
      if (why !== null) return over() ? undefined : refused(line, cmd, why);
      const end = await ended(cmd.id, live);
      if (!live()) return;
      if (!end) {
        line.state = 'stopped';
        line.detail = 'arrêtée';
        return fail('Arrêté');
      }
      if (end.status !== 'done') {
        line.state = 'failed';
        line.detail = `code ${end.code ?? '?'}`;
        return fail(`Préparation en échec (code ${end.code ?? '?'})`);
      }
      line.state = 'ready';
      line.detail = 'terminée';
    }
    flows.prepared[agent.id] = prepared;
  }
  if (over()) return;

  // 2. Every process, each in its own terminal (one already running stays).
  const started = Date.now();
  const lines = cmds.processes.map((cmd) => addLine(cmd, cmd.name, 'démarrage…'));
  const refusals = await Promise.all(cmds.processes.map(start));
  // Stopped while they were starting: their lines already say so.
  if (over()) return;
  // Those the backend refused say why: they did not start, and the test is over.
  for (const [i, why] of refusals.entries()) if (why !== null) refused(lines[i], cmds.processes[i], why);

  // 3. Each process with an address, until it answers.
  const ready = await Promise.all(
    cmds.processes.map(async (cmd, i) => {
      // Refused, or the test is over otherwise: its line already says so.
      if (over()) return false;
      const line = lines[i];
      const url = recipe.processes[i].url.trim();
      if (!url) {
        line.state = 'ready';
        line.detail = 'démarré';
        return true;
      }
      line.state = 'waiting';
      line.detail = `en attente de ${host(url)}…`;
      for (;;) {
        if (over()) return false;
        const run = app.launches[cmd.id];
        if (run?.status !== 'running') {
          exited(line, cmd, run);
          return false;
        }
        const up = await api.httpReady(url).catch(() => false);
        if (over()) return false;
        if (up) {
          line.state = 'ready';
          line.detail = `prêt · ${seconds(Date.now() - started)} s`;
          return true;
        }
        if (Date.now() - started >= READY_LIMIT_MS) {
          line.state = 'failed';
          line.detail = `Pas de réponse de ${url} après 3 min`;
          fail(line.detail);
          return false;
        }
        await sleep(POLL_MS);
      }
    }),
  );
  if (over() || !ready.every(Boolean)) return;

  // 4. The browser, on the address that shows the feature.
  const address = openAddress(agent);
  flow.phase = 'ready';
  flow.opened = address;
  if (address) app.run(openUrl(address));

  // 5. While it is up, a process that ends says so: the test is over.
  const watch = async () => {
    while (live() && flow.phase === 'ready') {
      const i = cmds.processes.findIndex((cmd) => app.launches[cmd.id]?.status !== 'running');
      if (i >= 0) return exited(lines[i], cmds.processes[i], app.launches[cmds.processes[i].id]);
      await sleep(POLL_MS);
    }
  };
  if (cmds.processes.length) void watch();
}
