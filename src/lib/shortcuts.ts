// Global keyboard shortcuts, routed in one place.

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

/**
 * Shortcuts the terminal hands over to the app. Shell keys (Ctrl+C, Ctrl+R, Ctrl+N…) stay
 * with the shell; Ctrl+J is only a line feed there, which Enter already sends. On macOS the
 * app's shortcuts use Cmd, which the shell never gets: they all go to the app, except copy and
 * paste, handled by the terminal. Ctrl+K stays with the shell (it cuts the end of the line).
 */
export function isAppShortcut(e: KeyboardEvent, mac = IS_MAC): boolean {
  if (e.altKey) return false;
  if (agentCycle(e)) return true;
  if (!primaryKey(e, mac)) return false;
  const k = e.key.toLowerCase();
  if (mac) return digit(e) !== null || ['n', 't', 'j', ',', 'b', 'l', 'k'].includes(k);
  return digit(e) !== null || k === ',' || k === 'j';
}

/** Runs the shortcut matching `e`. Returns true when the event was handled. */
export function handleShortcut(e: KeyboardEvent, mac = IS_MAC): boolean {
  // Ctrl+Alt is AltGr on French keyboards (e.g. AltGr+2 = ~): never a shortcut.
  if (e.altKey || app.modal) return false;
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
  return false;
}
