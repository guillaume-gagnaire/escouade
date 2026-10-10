// Global keyboard shortcuts, routed in one place.

import { findInFiles } from './editor/search.svelte';
import { IS_MAC, primaryKey } from './platform';
import { app } from './state.svelte';

/**
 * The digit of Ctrl+1…9 (Cmd+1…9 on macOS) from the physical key: on AZERTY keyboards the digit row types & é " '…
 * without Shift, so `key` is not a digit there.
 */
function digit(e: KeyboardEvent): number | null {
  const d = /^Digit([1-9])$/.exec(e.code)?.[1] ?? (/^[1-9]$/.test(e.key) ? e.key : null);
  return d ? Number(d) : null;
}

/**
 * Ctrl+Enter (Cmd+Enter on macOS) answers the request Claude waits on: "yes", or "always" with
 * Shift. Not Ctrl+Alt+Enter (AltGr), not an input method confirming its composition, and not a key
 * held down: the press that answers a request must not answer the next one on its repeats.
 */
export function enterAnswer(e: KeyboardEvent, mac = IS_MAC): 'plain' | 'shift' | null {
  if (e.key !== 'Enter' || e.altKey || e.isComposing || e.repeat || !primaryKey(e, mac)) return null;
  return e.shiftKey ? 'shift' : 'plain';
}

/**
 * Alt+1…9 picks the nth option of a question, read from the physical key as for Ctrl+digit. Never
 * with Ctrl (Ctrl+Alt is AltGr on French keyboards), and not from the numpad, where Alt+digits type
 * a character by its code on Windows.
 */
export function optionAnswer(e: KeyboardEvent): number | null {
  if (!e.altKey || e.ctrlKey || e.metaKey || e.shiftKey || e.repeat) return null;
  const d = e.code ? /^Digit([1-9])$/.exec(e.code)?.[1] : /^[1-9]$/.exec(e.key)?.[0];
  return d ? Number(d) : null;
}

/** The `aria-keyshortcuts` of {@link enterAnswer}'s keys. */
export function ariaEnter(shift = false, mac = IS_MAC): string {
  return `${mac ? 'Meta' : 'Control'}+${shift ? 'Shift+' : ''}Enter`;
}

/**
 * Where a waiting request is answered from the keyboard: the conversation (`.conv`, with its
 * message field), or the page itself when nothing has the focus. A terminal and the editor keep
 * their keys, and so does a dialog.
 */
export function answersHere(target: EventTarget | null): boolean {
  if (app.modal || !(target instanceof Element)) return false;
  if (target.closest('.xterm, .cm-editor')) return false;
  return target === document.body || !!target.closest('.conv');
}

/** Ctrl+Tab cycles agents on every system: Cmd+Tab switches applications on macOS. */
const agentCycle = (e: KeyboardEvent) => e.key === 'Tab' && e.ctrlKey && !e.metaKey;

/** Ctrl+Shift+A (Cmd+Shift+A on macOS) opens « Vue d’ensemble ». */
const overviewKey = (e: KeyboardEvent) => e.shiftKey && e.key.toLowerCase() === 'a';

/**
 * Shortcuts the terminal hands over to the app. Shell keys (Ctrl+C, Ctrl+R, Ctrl+N…) stay
 * with the shell; Ctrl+J is only a line feed there, which Enter already sends, and the terminal
 * sends nothing at all for Ctrl+Shift+A. On macOS the app's shortcuts use Cmd, which the shell
 * never gets: they all go to the app, except copy, paste and select all (Cmd+A), handled by the
 * terminal. Ctrl+K stays with the shell (it cuts the end of the line), and so does Ctrl+P
 * (previous command) on Windows: « Ouvrir un fichier » is Cmd+P there.
 */
export function isAppShortcut(e: KeyboardEvent, mac = IS_MAC): boolean {
  if (e.altKey) return false;
  if (agentCycle(e)) return true;
  if (!primaryKey(e, mac)) return false;
  const k = e.key.toLowerCase();
  if (overviewKey(e)) return true;
  if (mac) return digit(e) !== null || ['n', 't', 'j', ',', 'b', 'l', 'k'].includes(k) || isP(e);
  return digit(e) !== null || k === ',' || k === 'j';
}

/**
 * The letter P: by its character, else by its place on the keyboard when the layout types another alphabet (Cyrillic,
 * Greek…), where the WebView still prints on that key. Not by its place alone: on Dvorak that key types an « l ».
 */
function isP(e: KeyboardEvent): boolean {
  const k = e.key.toLowerCase();
  return k === 'p' || (!/^[a-z]$/.test(k) && e.code === 'KeyP');
}

/** What prints the page in the WebView: Ctrl+P, and Ctrl+Shift+P (through the system dialog). Cmd on macOS; never AltGr. */
const printKey = (e: KeyboardEvent, mac: boolean) => !e.altKey && primaryKey(e, mac) && isP(e);

/** Ctrl+P (Cmd+P on macOS): « Ouvrir un fichier ». */
const quickOpenKey = (e: KeyboardEvent, mac: boolean) => printKey(e, mac) && !e.shiftKey;

/**
 * The page is never printed: runs first, in the capture phase, before any field that keeps its keys to itself
 * (`stopPropagation`) hides the key from the window's handler. It only prevents the default; `handleShortcut` opens the
 * palette in the bubble phase. Returns true when the key was held back.
 */
export function holdPrint(e: KeyboardEvent, mac = IS_MAC): boolean {
  if (!printKey(e, mac)) return false;
  e.preventDefault();
  return true;
}

/**
 * « Ouvrir un fichier » over the project on screen. Its source is the editor's when it is open, else the worktree of
 * the agent selected, else the project's own checkout.
 */
function openQuickOpen() {
  const project = app.project;
  if (!project || app.modal) return;
  const source = app.editorOn ? app.editor[project.id].source : app.agent?.worktree ? app.agent.id : 'project';
  app.modal = { kind: 'quickOpen', projectId: project.id, source };
}

/** Runs the shortcut matching `e`. Returns true when the event was handled. */
export function handleShortcut(e: KeyboardEvent, mac = IS_MAC): boolean {
  // Ctrl+Alt is AltGr on French keyboards (e.g. AltGr+2 = ~): never a shortcut.
  if (e.altKey) return false;
  // Taken whatever the window shows (a dialog open, no project): a Ctrl+P left to the WebView prints the page.
  if (quickOpenKey(e, mac)) {
    openQuickOpen();
    return true;
  }
  if (app.modal) return false;
  if (agentCycle(e) && app.project) {
    const list = app.projectAgents;
    if (!list.length) return false;
    const i = list.findIndex((a) => a.id === app.agent?.id);
    app.selectAgent(list[(i + (e.shiftKey ? -1 : 1) + list.length) % list.length].id);
    return true;
  }
  if (!primaryKey(e, mac)) return false;
  const k = e.key.toLowerCase();
  if (k === 's' && !e.shiftKey && app.editorOn) {
    import('./editor/actions').then((m) => m.saveActive());
    return true;
  }
  // The editor's search through the files of its source; Ctrl+F stays the search in the file.
  if (k === 'f' && e.shiftKey && app.editorOn && findInFiles()) return true;
  const n = digit(e);
  if (n !== null && !e.shiftKey) {
    const p = app.projects[n - 1];
    if (!p) return false;
    app.selectProject(p.id);
    return true;
  }
  if (k === 'n' && !e.shiftKey && app.project) {
    app.newAgent();
    return true;
  }
  if (k === 'j') {
    app.nextWaiting();
    return true;
  }
  if (k === ',') {
    app.modal = { kind: 'settings' };
    return true;
  }
  if (k === 'k' && !e.shiftKey) {
    app.modal = { kind: 'convSearch' };
    return true;
  }
  if (k === 't' && !e.shiftKey && app.project) {
    const projectId = app.project.id;
    import('./term-actions').then((m) => m.newTerminal(projectId));
    return true;
  }
  if (k === 'b' && e.shiftKey && !app.split) {
    app.filesOpen = !app.filesOpen;
    return true;
  }
  if (k === 'l' && e.shiftKey) {
    app.toggleLayout();
    return true;
  }
  if (overviewKey(e)) {
    app.toggleOverview();
    return true;
  }
  return false;
}
