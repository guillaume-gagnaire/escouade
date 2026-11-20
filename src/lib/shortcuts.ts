// Global keyboard shortcuts, routed in one place.

import { handleZoomKey, zoomKey } from './display.svelte';
import { findInFiles } from './editor/search.svelte';
import { DEFAULT_SHORTCUT, matchesShortcut, parseShortcut } from './dictation';
import { dictation } from './dictation.svelte';
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
 * their keys, and so does a dialog, and a field of the conversation whose keys are its own
 * (`data-own-keys`: « Lui écrire » of a subagent, where Ctrl+Entrée sends the message).
 */
export function answersHere(target: EventTarget | null): boolean {
  if (app.modal || !(target instanceof Element)) return false;
  if (target.closest('.xterm, .cm-editor, [data-own-keys]')) return false;
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
  // The split screen's keys (ESC-392) leave a terminal for the app while it is split, Ctrl+\ always (it would be SIGQUIT).
  if (paneKey(e, mac)) return true;
  // The spaces' keys (ESC-539), once there are spaces.
  if (spaceKey(e, mac) !== null && spaceKeys.shown()) return true;
  if (e.altKey) return false;
  if (agentCycle(e)) return true;
  // The zoom of the whole window, as in VS Code's terminal (Ctrl+- would only be readline's undo there).
  if (zoomKey(e, mac) !== null) return true;
  if (!primaryKey(e, mac)) return false;
  const k = e.key.toLowerCase();
  if (overviewKey(e)) return true;
  if (mac) return digit(e) !== null || ['n', 't', 'j', ',', 'b', 'l', 'k'].includes(k) || isP(e);
  return digit(e) !== null || k === ',' || k === 'j';
}

/**
 * The letter P: by its character, else by its place on the keyboard when the layout types another alphabet (Cyrillic,
 * Greek…), where the WebView still prints on that key. Not by its place alone: on Dvorak that key types an « l », and
 * on Colemak or Workman a « ; », which is no letter at all.
 */
function isP(e: KeyboardEvent): boolean {
  const k = e.key.toLowerCase();
  return k === 'p' || (/^\p{L}$/u.test(k) && !/^[a-z]$/.test(k) && e.code === 'KeyP');
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
  // The palette of the files is the editor's (F-1): off, Ctrl+P does nothing (still taken: it would print the page).
  if (!project || app.modal || !app.feature('editor')) return;
  const source = app.editorActive ? app.editor[project.id].source : app.agent?.worktree ? app.agent.id : 'project';
  app.modal = { kind: 'quickOpen', projectId: project.id, source };
}

/** The key was pressed in the VS Code view (its workbench, ESC-464), out of the page's own elements. */
export const inVscode = (target: EventTarget | null): boolean => target instanceof Element && !!target.closest('.vscode-host');

/**
 * The keys Escouade keeps when the VS Code view has the focus (ESC-464), the rule of the spec (Éditeur › Les touches):
 * those that go elsewhere in the app or open one of its windows. Ctrl+P (« Ouvrir un fichier », on the core's index),
 * Ctrl+1…9, Ctrl+N, Ctrl+Shift+N, Ctrl+J, Ctrl+T, Ctrl+, , Ctrl+Shift+A, Ctrl+Shift+B, the zoom, Ctrl+\ and, while the
 * screen is split, Ctrl+Shift+\, Ctrl+Shift+W and Ctrl+Alt+arrows. Every other key is VS Code's: Ctrl+S, Ctrl+F,
 * Ctrl+Shift+F, Ctrl+Shift+P, Ctrl+K and its chords, Ctrl+B, Ctrl+Tab, Ctrl+Shift+L, Ctrl+W… The view takes the keys
 * the app keeps off VS Code's keybindings (`lib/vscode/keys.ts`); its palette still runs those commands by their names.
 */
export function appWinsInVscode(e: KeyboardEvent, mac = IS_MAC): boolean {
  if (paneKey(e, mac)) return true;
  if (e.altKey) return false;
  if (zoomKey(e, mac) !== null) return true;
  if (!primaryKey(e, mac)) return false;
  if (quickOpenKey(e, mac)) return true;
  if (digit(e) !== null && !e.shiftKey) return true;
  const k = e.key.toLowerCase();
  if (e.shiftKey) return k === 'n' || k === 'a' || k === 'b';
  return k === 'n' || k === 'j' || k === 't' || k === ',';
}

/**
 * The spaces' keys (ESC-539, S3), read by the key's place (AZERTY too): `Alt+1`…`Alt+9` (`⌃1`…`⌃9` on macOS, where
 * Alt types characters) for the nth space, `Ctrl+²` (the key under Escape, `Backquote`) for the next one. Never from
 * the numpad (Alt+digits type a character by its code on Windows), never with Shift or AltGr.
 */
export function spaceKey(e: KeyboardEvent, mac = IS_MAC): number | 'next' | null {
  if (e.repeat || e.shiftKey || e.metaKey) return null;
  if (e.code === 'Backquote' && e.ctrlKey && !e.altKey) return 'next';
  const d = /^Digit([1-9])$/.exec(e.code)?.[1];
  if (!d) return null;
  return (mac ? e.ctrlKey && !e.altKey : e.altKey && !e.ctrlKey) ? Number(d) : null;
}

/** Where the spaces' keys go (`spaces.svelte.ts` sets it): whether one was taken. */
export const spaceKeys: { go: ((k: number | 'next', target: EventTarget | null) => boolean) | null; shown: () => boolean } = {
  go: null,
  shown: () => false,
};

/** Runs the shortcut matching `e`. Returns true when the event was handled. */
export function handleShortcut(e: KeyboardEvent, mac = IS_MAC): boolean {
  // The spaces' keys (ESC-539), in the VS Code view too, never over a dialog.
  const space = spaceKey(e, mac);
  if (space !== null && !app.modal && spaceKeys.go?.(space, e.target)) return true;
  // In the VS Code view, its keys are its own: the app only takes those it keeps (`appWinsInVscode`).
  if (inVscode(e.target) && !appWinsInVscode(e, mac)) return false;
  // Ctrl+Alt+arrows go from pane to pane (ESC-392): an arrow types nothing, even with AltGr.
  if (e.altKey && !app.modal && handlePaneKey(e, mac)) return true;
  // Ctrl+Alt is AltGr on French keyboards (e.g. AltGr+2 = ~): never a shortcut.
  if (e.altKey) return false;
  // Taken whatever the window shows (a dialog open, no project): a Ctrl+P left to the WebView prints the page.
  if (quickOpenKey(e, mac)) {
    openQuickOpen();
    return true;
  }
  // The zoom too: a dialog grows with the rest.
  if (handleZoomKey(e, mac)) return true;
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
  if (k === 's' && !e.shiftKey && app.editorActive) {
    import('./editor/actions').then((m) => m.saveActive());
    return true;
  }
  // The editor's search through the files of its source; Ctrl+F stays the search in the file.
  if (k === 'f' && e.shiftKey && app.editorActive && findInFiles()) return true;
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
  // The menu « Harness » beside « Nouvel agent », its harnesses numbered (choice 15 of the 2.0 design).
  if (k === 'n' && e.shiftKey && app.project && !app.boardOn) {
    app.newAgentMenu++;
    return true;
  }
  // Ctrl+Maj+J opens the box « À toi » (ESC-510, choice I4), only with the MCP server on; Ctrl+J goes to the next agent
  // stopped, as before.
  if (k === 'j' && e.shiftKey) {
    if (!app.settings.mcpEnabled) return false;
    app.toggleInbox('shortcut');
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
  // « Nouvelle discussion » of the Docs tab (ESC-261).
  if (k === 'b' && !e.shiftKey && app.docsOn && app.project) {
    app.newDiscussion(app.project.id);
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
  if (handlePaneKey(e, mac)) return true;
  if (overviewKey(e)) {
    app.toggleOverview();
    return true;
  }
  return false;
}

/** The arrows of Ctrl+Alt+arrow: the pane on that side. */
const PANE_SIDES: Record<string, 'left' | 'right' | 'top' | 'bottom'> = {
  ArrowLeft: 'left',
  ArrowRight: 'right',
  ArrowUp: 'top',
  ArrowDown: 'bottom',
};

/** Ctrl+\ (by its place: Ctrl+* on a French AZERTY keyboard), with or without Shift. */
const splitKey = (e: KeyboardEvent, mac: boolean) =>
  primaryKey(e, mac) && !e.altKey && (e.code === 'Backslash' || e.key === '\\' || e.key === '|');

/**
 * The split screen's keys (ESC-392): Ctrl+\ splits or comes back to one pane, Ctrl+Shift+\ turns the active pane's
 * split, and, while split, Ctrl+Shift+W closes the active pane and Ctrl+Alt+arrows go to the pane on that side.
 */
export function paneKey(e: KeyboardEvent, mac = IS_MAC): 'split' | 'flip' | 'close' | 'left' | 'right' | 'top' | 'bottom' | null {
  if (e.repeat && !(e.key in PANE_SIDES)) return null;
  if (splitKey(e, mac)) return e.shiftKey ? (app.panesOn ? 'flip' : null) : 'split';
  if (!app.panesOn) return null;
  if (primaryKey(e, mac) && e.shiftKey && !e.altKey && (e.key.toLowerCase() === 'w' || e.code === 'KeyW')) return 'close';
  if (primaryKey(e, mac) && e.altKey && !e.shiftKey && e.key in PANE_SIDES) return PANE_SIDES[e.key];
  return null;
}

function handlePaneKey(e: KeyboardEvent, mac: boolean): boolean {
  const k = paneKey(e, mac);
  if (!k || !app.project) return false;
  if (k === 'split') app.toggleSplit();
  else if (k === 'flip') app.flipPanes();
  else if (k === 'close') app.closePaneOf(app.panes!.active);
  else app.focusPane(k);
  return true;
}

/**
 * The dictation's keys, taken in the capture phase, before any field (a terminal would type them): its shortcut of the
 * settings, pressed (it starts, or ends one under way) and let go (after a long press, it ends); Escape while it listens
 * or transcribes calls it off. Into the field that has the focus, else the composer on screen. Returns true when the
 * key was the dictation's (the caller stops it there).
 */
export function dictationKey(e: KeyboardEvent, mac = IS_MAC): boolean {
  if (dictation.recording) return false;
  // The feature Voix off (F-1): its shortcut is no one's.
  if (!app.feature('voice')) return false;
  const spec = app.settings.voice?.shortcut || DEFAULT_SHORTCUT;
  if (e.type === 'keyup') {
    if (e.code !== parseShortcut(spec)?.code) return false;
    dictation.release();
    return false;
  }
  if (e.key === 'Escape' && ['starting', 'listening', 'transcribing'].includes(dictation.phase)) {
    dictation.cancel();
    return true;
  }
  if (!matchesShortcut(e, spec, mac)) return false;
  if (!e.repeat) dictation.press(dictation.phase === 'idle' ? dictation.focused() : null);
  return true;
}
