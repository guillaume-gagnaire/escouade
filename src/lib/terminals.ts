// xterm.js instances live outside of components so switching views keeps their scrollback.

import { Terminal, type ITheme } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { Unicode11Addon } from '@xterm/addon-unicode11';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import { openUrl } from '@tauri-apps/plugin-opener';
import { api } from './ipc';
import { IS_MAC } from './platform';
import { isAppShortcut } from './shortcuts';
import type { TermInfo } from './types';

export interface XTerm {
  term: Terminal;
  fit: FitAddon;
  search: SearchAddon;
  host: HTMLDivElement;
  /** A log: no input, no cursor. */
  readOnly?: boolean;
}

const xterms = new Map<string, XTerm>();
let parking: HTMLDivElement | null = null;

function parkingLot(): HTMLDivElement {
  if (!parking) {
    parking = document.createElement('div');
    parking.style.cssText = 'position:fixed;left:-10000px;top:0;width:900px;height:500px;visibility:hidden;';
    document.body.appendChild(parking);
  }
  return parking;
}

/** Resolves any CSS color (oklch, color-mix, var()) to #rrggbb through a 1px canvas. */
export function resolveColor(css: string): string {
  const probe = document.createElement('div');
  probe.style.color = css;
  document.body.appendChild(probe);
  const computed = getComputedStyle(probe).color;
  probe.remove();
  const c = document.createElement('canvas');
  c.width = c.height = 1;
  const ctx = c.getContext('2d', { willReadFrequently: true })!;
  ctx.fillStyle = computed;
  ctx.fillRect(0, 0, 1, 1);
  const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
  return '#' + [r, g, b].map((x) => x.toString(16).padStart(2, '0')).join('');
}

export function terminalTheme(readOnly = false): ITheme {
  const background = resolveColor('var(--term)');
  return {
    background,
    foreground: '#ede7df',
    cursor: readOnly ? background : resolveColor('var(--accent)'),
    cursorAccent: '#1b1512',
    selectionBackground: resolveColor('color-mix(in oklch, var(--accent) 35%, transparent)') + '99',
    black: '#2a2724',
    red: '#e2735f',
    green: '#7cc48d',
    yellow: '#e9c46a',
    blue: '#6fa8dc',
    magenta: '#c792ea',
    cyan: '#6cc5c1',
    white: '#d8d0c4',
    brightBlack: '#6f685f',
    brightRed: '#f08c78',
    brightGreen: '#9bd8a9',
    brightYellow: '#f4d68a',
    brightBlue: '#8fbfea',
    brightMagenta: '#d9aef2',
    brightCyan: '#8ad8d4',
    brightWhite: '#f5efe7',
  };
}

const SCROLL_KEYS = new Set(['PageUp', 'PageDown', 'Home', 'End']);

/** A parked xterm.js instance; a read-only one takes no input (launch command logs). */
function createXTerm(readOnly: boolean): XTerm {
  const term = new Terminal({
    fontFamily: "'JetBrains Mono', ui-monospace, monospace",
    fontSize: 12.5,
    lineHeight: 1.2,
    cursorBlink: !readOnly,
    cursorInactiveStyle: readOnly ? 'none' : 'outline',
    disableStdin: readOnly,
    allowProposedApi: true,
    scrollback: 10000,
    theme: terminalTheme(readOnly),
    // Rows added by a resize stay blank at the bottom, as in the pseudo-console, which repaints
    // its screen at absolute positions: pulling scrollback down would shift the log under it.
    windowsPty: { backend: 'conpty' },
  });
  const host = document.createElement('div');
  host.style.cssText = 'width:100%;height:100%;';
  parkingLot().appendChild(host);
  const fit = new FitAddon();
  const search = new SearchAddon();
  term.loadAddon(fit);
  term.loadAddon(search);
  term.loadAddon(new Unicode11Addon());
  term.unicode.activeVersion = '11';
  term.loadAddon(new WebLinksAddon((_e, uri) => openUrl(uri).catch(() => {})));
  term.open(host);
  try {
    const webgl = new WebglAddon();
    webgl.onContextLoss(() => webgl.dispose());
    term.loadAddon(webgl);
  } catch {
    // Falls back to the DOM renderer.
  }
  fit.fit();
  term.attachCustomKeyEventHandler((e) => {
    if (e.type !== 'keydown') return true;
    // Navigation shortcuts go to the app (the event keeps bubbling to its window handler).
    if (isAppShortcut(e)) return false;
    // macOS: Cmd+C copies the selection, Cmd+V pastes (through the Edit menu's paste event);
    // Ctrl keys all go to the shell.
    if (IS_MAC && e.metaKey) {
      const k = e.key.toLowerCase();
      if (k === 'c' && term.hasSelection()) navigator.clipboard.writeText(term.getSelection());
      return !(k === 'c' || k === 'v');
    }
    // Windows Terminal conventions: Ctrl+C copies when there is a selection, Ctrl+V pastes.
    if (e.ctrlKey && !e.shiftKey && e.key.toLowerCase() === 'c' && term.hasSelection()) {
      navigator.clipboard.writeText(term.getSelection());
      term.clearSelection();
      return false;
    }
    if (e.ctrlKey && e.key.toLowerCase() === 'v') return false;
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'c') {
      if (term.hasSelection()) navigator.clipboard.writeText(term.getSelection());
      return false;
    }
    // A log only scrolls (Shift+PageUp…): every other key goes on to the app's shortcuts.
    return !readOnly || (e.shiftKey && SCROLL_KEYS.has(e.key));
  });
  return { term, fit, search, host, readOnly };
}

function free(x: XTerm) {
  x.term.dispose();
  x.host.remove();
}

/**
 * Where a terminal opens when not in the project's folder: the worktree of an agent (the project's
 * folder for one without), or the folder `subdir` of it (a path of the editor's tree).
 */
export interface TermPlace {
  agentId?: string | null;
  subdir?: string;
}

export function openTerminal(projectId: string, shell: string, name: string, place?: TermPlace): Promise<TermInfo> {
  return interactive((cols, rows, onData) => api.termSpawn({ projectId, shell, name, ...place, cols, rows }, onData));
}

/** « Se connecter… »: the Claude account's `claude` in a terminal of its own, where the user signs in. */
export function openAccountLogin(accountId: string): Promise<TermInfo> {
  return interactive((cols, rows, onData) => api.accountLogin(accountId, cols, rows, onData));
}

/** An interactive terminal that `spawn` starts at the size of a new xterm.js instance, which then shows it. */
async function interactive(spawn: (cols: number, rows: number, onData: (d: ArrayBuffer) => void) => Promise<TermInfo>): Promise<TermInfo> {
  const x = createXTerm(false);
  const { term } = x;
  let info: TermInfo;
  try {
    info = await spawn(term.cols, term.rows, (buf) => term.write(new Uint8Array(buf)));
  } catch (e) {
    // Nothing to attach to: free the xterm instance, its WebGL context and its host.
    free(x);
    throw e;
  }
  term.onData((d) => api.termWrite(info.id, d).catch(() => {}));
  term.onResize(({ cols, rows }) => api.termResize(info.id, cols, rows).catch(() => {}));
  xterms.set(info.id, x);
  return info;
}

/** Key of a launch command's log among the terminals. */
export const logKey = (commandId: string) => `run:${commandId}`;

/**
 * The read-only log of a launch command, kept across its runs.
 * `onResize` gets the new size so the running process can follow it.
 */
export function launchLog(commandId: string, onResize: (cols: number, rows: number) => void): XTerm {
  const key = logKey(commandId);
  let x = xterms.get(key);
  if (!x) {
    x = createXTerm(true);
    x.term.onResize(({ cols, rows }) => onResize(cols, rows));
    xterms.set(key, x);
  }
  return x;
}

export function disposeLog(commandId: string) {
  const x = xterms.get(logKey(commandId));
  if (!x) return;
  free(x);
  xterms.delete(logKey(commandId));
}

export function getXTerm(id: string): XTerm | undefined {
  return xterms.get(id);
}

/** Moves the terminal's DOM into `container` (or back to the parking lot). */
export function mountTerminal(id: string, container: HTMLElement | null) {
  const x = xterms.get(id);
  if (!x) return;
  (container ?? parkingLot()).appendChild(x.host);
  if (container) {
    x.term.options.theme = terminalTheme(x.readOnly);
    requestAnimationFrame(() => {
      try {
        x.fit.fit();
      } catch {
        /* not measurable yet */
      }
      x.term.focus();
    });
  }
}

export function disposeTerminal(id: string) {
  const x = xterms.get(id);
  if (!x) return;
  free(x);
  xterms.delete(id);
  api.termKill(id).catch(() => {});
}
