// The discovery of the features (ESC-506, docs/ONBOARDING.md § 7, direction C « La ligne »): the map « À découvrir »
// (Aide › À découvrir), each discovery leading to the real gesture on the real screen, and three bubbles at the right
// moment (the microphone hovered, the first ticket made, the first pull request opened). The core keeps what became of
// each discovery and ticks it by the real state of the app (`discover.rs`): the window shows it, and tells the core only
// what it alone sees (the tab Docs, a file in the editor, a pull request opened).
//
// Never more than one bubble on screen, never two at the right moment in one session, never while typing nor while an
// agent waits for a permission, never twice the same one. Nothing here installs, connects, downloads nor sends: the
// gesture stays the user's, on the real control, which says what it costs before.

import { api } from './ipc';
import { onboarding } from './onboarding.svelte';
import { app } from './state.svelte';
import type { DiscoverAct, Discovery, DiscoverItem, DiscoverLane, DiscoverView, UiEvent } from './types';

/** The lanes of the map, in their order. */
export const LANES: DiscoverLane[] = ['plan', 'code', 'review', 'ship', 'drive'];

/** A bubble on screen: offered at the right moment, or showing the real control a gesture of the map leads to. */
export interface Tip {
  id: Discovery;
  /** The real control it points at. */
  el: HTMLElement;
  /** `offer`: at the right moment (« Compris », « Écarter »); `spot`: where « Essayer » leads (« Compris »). */
  mode: 'offer' | 'spot';
}

export const discover = $state<{
  view: DiscoverView | null;
  /** The discovery whose detail the map shows. */
  selected: Discovery | null;
  /** The map filtered on what is new in `release` (« Voir » of the band « Nouveau dans la 2.3 »). */
  onlyNew: boolean;
  release: string | null;
  tip: Tip | null;
}>({ view: null, selected: null, onlyNew: false, release: null, tip: null });

/** The microphones this window found: undefined until read, null when it cannot tell (a client of the remote API). */
let microphones: number | null | undefined;
/** A bubble at the right moment was shown in this session: no other one before the next start. */
let offeredThisSession = false;
/** When a key was last pressed: a bubble never comes while the user types. */
let lastKey = 0;
/** How long a hover rests on a control before its bubble comes. */
export const HOVER_REST_MS = 700;
/** How long after a key press the user still counts as typing. */
export const TYPING_MS = 1500;

async function mics(): Promise<number | null> {
  if (microphones === undefined) {
    microphones = await api
      .voiceInfo()
      .then((i) => i.devices.length)
      .catch(() => null);
  }
  return microphones;
}

/** The core's view, read again. */
export async function load(): Promise<DiscoverView | null> {
  const v = await api.discoverView(await mics()).catch(() => null);
  if (v) discover.view = v;
  return v;
}

/** `a` told to the core; its view taken. */
export async function act(a: DiscoverAct): Promise<DiscoverView | null> {
  const v = await api.discoverAct(a, await mics()).catch(() => null);
  if (v) discover.view = v;
  return v;
}

let soon: ReturnType<typeof setTimeout> | undefined;
/** The events after which the real state may have done a discovery: the view read again, once, a moment later. */
const TOLD_BY: UiEvent['type'][] = ['discoverChanged', 'ticket', 'project', 'accounts', 'codeHosts', 'prReview', 'agent', 'teams', 'locks'];

/** An event of the core heard: the view read again when it may have changed. */
export function hear(type: UiEvent['type']) {
  if (!TOLD_BY.includes(type) || !discover.view) return;
  clearTimeout(soon);
  soon = setTimeout(() => void load(), type === 'discoverChanged' ? 0 : 400);
}

/** A key pressed: the user is typing. */
export function keyPressed() {
  lastKey = Date.now();
}

export function typing(now = Date.now()): boolean {
  return now - lastKey < TYPING_MS;
}

/**
 * Not the moment for a bubble: the user types, an agent waits for a permission, or the first launch is on screen (its
 * first agent's real composer has the microphone: no bubble over the journey, ESC-505).
 */
function busy(): boolean {
  return typing() || permissionWaiting() || onboarding.shown;
}

/** An agent waits for a permission: no bubble over it. */
export function permissionWaiting(): boolean {
  return Object.values(app.agents).some((a) => a.requests?.some((r) => r.kind === 'permission'));
}

/** The discovery `id` as the core tells it. */
export function itemOf(id: Discovery): DiscoverItem | undefined {
  return discover.view?.items.find((i) => i.id === id);
}

/** What the window alone sees done (the tab Docs, a file in the editor, a pull request opened). */
export function seen(id: 'docs' | 'editor' | 'prView') {
  const it = itemOf(id);
  if (!it || it.status === 'done') return;
  void act({ kind: 'seen', id });
}

/** Aide › À découvrir: the map; on what is new in `release` when given (« Voir » of the band). */
export function openMap(onlyNew = false, release: string | null = null) {
  discover.onlyNew = onlyNew;
  discover.release = release;
  discover.tip = null;
  app.ui.view = 'discover';
  app.persistUi();
  void load();
}

/** Back from the map to the project. */
export function closeMap() {
  if (app.ui.activeProject) app.selectProject(app.ui.activeProject);
  else {
    app.ui.view = 'project';
    app.persistUi();
  }
}

/**
 * A bubble at the right moment for `id`, on `el`: only when the core still allows it (on this device, to try, never
 * shown, not « Tout masquer »), no other bubble is on screen nor was offered in this session, the user is not typing and
 * no agent waits for a permission. True when it shows. `allowed`: what the core said before the moment came (the first
 * ticket made, which the autopilot starts at once: done by then, it is still the moment to say what it does).
 */
export function offer(id: Discovery, el: HTMLElement | null | undefined, allowed = mayOffer(id)): boolean {
  if (!el || !discover.view || offeredThisSession || discover.tip) return false;
  if (!allowed) return false;
  if (busy()) return false;
  offeredThisSession = true;
  discover.tip = { id, el, mode: 'offer' };
  void act({ kind: 'tipped', id });
  return true;
}

/**
 * `offer` once the moment is quiet: while the user types or an agent waits for a permission, it waits (every half
 * second, six seconds at most), then lets the moment go. `find`: the real control, read when it shows.
 */
export function offerWhenQuiet(id: Discovery, find: () => HTMLElement | null, allowed = mayOffer(id), tries = 12) {
  if (!allowed || offeredThisSession) return;
  if (busy()) {
    if (tries > 0) setTimeout(() => offerWhenQuiet(id, find, allowed, tries - 1), 500);
    return;
  }
  offer(id, find(), allowed);
}

/** Whether the core still allows the bubble of `id` (on this device, to try, never shown, not « Tout masquer »). */
export function mayOffer(id: Discovery): boolean {
  return !!discover.view?.tips.includes(id);
}

/** The bubble closed (« Compris », Échap, a key typed): the discovery stays to try. */
export function closeTip() {
  discover.tip = null;
}

/** « Écarter »: never offered again by itself. */
export async function dismiss(id: Discovery) {
  if (discover.tip?.id === id) discover.tip = null;
  await act({ kind: 'dismiss', id });
}

/** The real control of `id` on screen (`data-discover`), once what « Essayer » opened is drawn. */
async function control(id: Discovery, tries = 20): Promise<HTMLElement | null> {
  for (let i = 0; i < tries; i++) {
    const el = document.querySelector<HTMLElement>(`[data-discover="${id}"]`);
    if (el && el.getClientRects().length) return el;
    await new Promise((r) => setTimeout(r, 75));
  }
  return null;
}

/** The bubble that shows the real control of `id`, when there is one on screen. */
async function spot(id: Discovery) {
  const el = await control(id);
  if (el) discover.tip = { id, el, mode: 'spot' };
}

/** The project the gestures act on: the one on screen, else the first. */
function projectId(): string | null {
  return app.ui.activeProject ?? app.projects[0]?.id ?? null;
}

/** An agent of the project to show: the one selected, else its first. */
function agentOf(pid: string): string | null {
  const sel = app.ui.selectedAgent[pid];
  if (sel && app.agents[sel]) return sel;
  return Object.values(app.agents).find((a) => a.projectId === pid && !a.archived)?.id ?? null;
}

/** The project on screen, with its agent when it has one; the control of `id` shown. */
function inProject(pid: string, id: Discovery, withAgent: boolean) {
  app.selectProject(pid);
  app.closeBoard(pid);
  app.closeDocs(pid);
  const agent = withAgent ? agentOf(pid) : null;
  if (agent) app.selectAgent(agent);
  // No agent yet: the button « Nouvel agent » is where the gesture begins.
  void spot(withAgent && !agent ? 'secondAgent' : id);
}

/**
 * « Essayer »: the real screen of `id`, its real control shown. Nothing is made, installed, connected nor sent here: the
 * user does the gesture, which says what it costs before.
 */
export function go(id: Discovery) {
  discover.tip = null;
  const pid = projectId();
  // What needs a project starts with one.
  const needsProject = !['plugin', 'mcp', 'account', 'team', 'forge', 'remote'].includes(id);
  if (needsProject && !pid) {
    app.modal = { kind: 'newProject' };
    return;
  }
  switch (id) {
    case 'ticket':
    case 'autopilot':
    case 'approve':
      app.openBoard(pid);
      void spot(id);
      return;
    case 'docs':
      app.openDocs(pid);
      return;
    case 'dictation':
    case 'voice':
      inProject(pid!, id, true);
      return;
    case 'terminal':
    case 'secondAgent':
    case 'editor':
      inProject(pid!, id, false);
      return;
    case 'launch':
      app.modal = { kind: 'settings', page: 'launch', projectId: pid! };
      return;
    case 'forge':
      app.modal = { kind: 'settings', page: 'services' };
      return;
    case 'prView':
    case 'prReview':
      app.openPrs(pid);
      return;
    case 'openPr':
      inProject(pid!, id, true);
      app.panelTab = 'hosts';
      app.filesOpen = true;
      return;
    case 'plugin':
      app.openPlugins();
      return;
    case 'mcp':
      app.modal = { kind: 'settings', page: 'agents' };
      return;
    case 'account':
      app.modal = { kind: 'settings', page: 'accounts' };
      return;
    case 'team':
      app.modal = { kind: 'team', step: 'join' };
      return;
    case 'remote':
      return;
  }
}

/** For the tests: a new session. */
export function resetDiscover() {
  discover.view = null;
  discover.selected = null;
  discover.onlyNew = false;
  discover.release = null;
  discover.tip = null;
  microphones = undefined;
  offeredThisSession = false;
  lastKey = 0;
  clearTimeout(soon);
}
