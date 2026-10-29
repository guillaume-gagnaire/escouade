// xterm.js instances live outside of components so switching views keeps their scrollback.

import { Terminal, type ITheme } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { Unicode11Addon } from '@xterm/addon-unicode11';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import { MAX_TOTAL, base64, clipboardImage, copyToDisk, imageType, planByPath, sizeLabel } from './attachments';
import { DEFAULT_FONT_SIZE } from './display.svelte';
import { echoWatch, type InputKind } from './echo-watch';
import { t } from './i18n';
import { api } from './ipc';
import { openOutside, terminalLinks } from './links';
import { IS_MAC } from './platform';
import { isAppShortcut } from './shortcuts';
import { engineName } from './engines';
import { app } from './state.svelte';
import type { TermInfo } from './types';

/** What draws a terminal; the DOM renderer says why it is not WebGL, WebGL the GPU's name. */
export interface Renderer {
  renderer: 'webgl' | 'dom';
  detail?: string;
}

export interface XTerm {
  term: Terminal;
  fit: FitAddon;
  search: SearchAddon;
  host: HTMLDivElement;
  /** A log: no input, no cursor. */
  readOnly?: boolean;
  /** The backend's id of an interactive terminal, once it has one. */
  id?: string;
  renderer: Renderer;
  /** When (`performance.now()`) a key went to the terminal, and the WebView's paste reached it: what tells the user's input from xterm.js's own. */
  keyAt?: number;
  pasteAt?: number;
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

/** The size, in pixels, of the place a terminal is about to be shown in. */
export interface Room {
  width: number;
  height: number;
}

/** What `el` has to draw in (its padding left out), or null when it has no size yet (not laid out, or hidden). */
export function roomOf(el: HTMLElement): Room | null {
  const style = getComputedStyle(el);
  const px = (v: string) => parseFloat(v) || 0;
  const width = el.clientWidth - px(style.paddingLeft) - px(style.paddingRight);
  const height = el.clientHeight - px(style.paddingTop) - px(style.paddingBottom);
  return width > 0 && height > 0 ? { width, height } : null;
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

/**
 * The 16 ANSI colours of the terminals, by the tokens of src/app.css that give them their value in each theme (the
 * preview's palette): the bright ones are the same, but for white. Dim text is `--code-dim`.
 */
export const ANSI_TOKENS = {
  black: '--ansi-k',
  red: '--del',
  green: '--add',
  yellow: '--ansi-y',
  blue: '--syn-f',
  magenta: '--syn-k',
  cyan: '--ansi-cy',
  white: '--ansi-w',
  brightBlack: '--code-dim',
  brightRed: '--del',
  brightGreen: '--add',
  brightYellow: '--ansi-y',
  brightBlue: '--syn-f',
  brightMagenta: '--syn-k',
  brightCyan: '--ansi-cy',
  brightWhite: '--ansi-w-2',
} as const;

/**
 * The theme of a terminal, in the theme the window shows (dark or light): the well's background, the code's ink, the
 * cursor of the text's ink (none in a log, which takes no input), the selection of the project's colour.
 */
export function terminalTheme(readOnly = false): ITheme {
  const background = resolveColor('var(--well)');
  const ansi = Object.fromEntries(Object.entries(ANSI_TOKENS).map(([k, v]) => [k, resolveColor(`var(${v})`)]));
  return {
    background,
    foreground: resolveColor('var(--code-fg)'),
    cursor: readOnly ? background : resolveColor('var(--text)'),
    cursorAccent: background,
    selectionBackground: resolveColor('var(--p)') + '42',
    ...ansi,
  };
}

/**
 * The least contrast xterm.js keeps between a cell's ink and its background: in the light theme, what a program draws
 * in colours made for a dark terminal (most TUIs do) stays readable.
 */
export function terminalContrast(): number {
  return document.documentElement.dataset.theme === 'light' ? 4.5 : 1;
}

/**
 * Every terminal takes the theme the window shows: at once when it changes (Réglages › Application › Thème, or the
 * system's under Système), without opening it again; and once `--p` has glided to the next project's colour.
 */
export function retheme() {
  const contrast = terminalContrast();
  for (const x of xterms.values()) {
    x.term.options.theme = terminalTheme(x.readOnly);
    x.term.options.minimumContrastRatio = contrast;
  }
}

/** How long `--p` glides from a project's colour to the next (src/app.css), after which the terminals take it. */
const GLIDE_MS = 750;
let watched = false;
let glide: ReturnType<typeof setTimeout> | undefined;

/** Follows the theme of the window (`data-theme`), a plugin's colours (`data-plugin-theme`) and its tint (`--p`, in the root's style), once. */
function watchTheme() {
  if (watched || typeof MutationObserver === 'undefined') return;
  watched = true;
  new MutationObserver((changes) => {
    if (changes.some((c) => c.attributeName === 'data-theme' || c.attributeName === 'data-plugin-theme')) retheme();
    clearTimeout(glide);
    glide = setTimeout(retheme, GLIDE_MS);
  }).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'data-plugin-theme', 'style'] });
}

const SCROLL_KEYS = new Set(['PageUp', 'PageDown', 'Home', 'End']);

let gpu: { name?: string } | null = null;

/**
 * The GPU's name WebGL gives (a software one such as SwiftShader says so), read once per run of the app from a
 * context of its own, given back at once: Chromium keeps 16 WebGL contexts and drops the oldest of the others.
 */
function gpuName(): string | undefined {
  if (gpu) return gpu.name;
  gpu = {};
  let gl: WebGL2RenderingContext | null = null;
  try {
    gl = document.createElement('canvas').getContext('webgl2');
    if (gl) {
      const info = gl.getExtension('WEBGL_debug_renderer_info');
      // Without it (hidden by the WebView) the generic name still tells WebGL is there.
      gpu.name = String(gl.getParameter(info ? info.UNMASKED_RENDERER_WEBGL : gl.RENDERER));
    }
  } catch {
    // The name is only for the log.
  }
  try {
    gl?.getExtension('WEBGL_lose_context')?.loseContext();
  } catch {
    // Left to the garbage collector.
  }
  return gpu.name;
}

/** Tells the log which renderer an interactive terminal has, once it has an id. */
function reportRenderer(x: XTerm) {
  if (x.id) api.termReport(x.id, { kind: 'renderer', ...x.renderer }).catch(() => {});
}

/**
 * A parked xterm.js instance; a read-only one takes no input (launch command logs). With `room`, the size of the place it
 * will be shown in, it is fitted to that place from the start, rather than to the parking lot's: the program it runs in a
 * pseudo-console is started at that size, never at one a message of resize has to correct.
 */
function createXTerm(readOnly: boolean, room?: Room | null): XTerm {
  watchTheme();
  const term = new Terminal({
    fontFamily: "'JetBrains Mono', ui-monospace, monospace",
    fontSize: app.settings.terminalFontSize || DEFAULT_FONT_SIZE,
    lineHeight: 1.2,
    cursorBlink: !readOnly,
    cursorInactiveStyle: readOnly ? 'none' : 'outline',
    disableStdin: readOnly,
    allowProposedApi: true,
    // The hyperlinks Claude Code writes (OSC 8) open in the system's browser, not through xterm.js's confirm() and window.open().
    linkHandler: terminalLinks,
    scrollback: 10000,
    theme: terminalTheme(readOnly),
    minimumContrastRatio: terminalContrast(),
    // Rows added by a resize stay blank at the bottom, as in the pseudo-console, which repaints
    // its screen at absolute positions: pulling scrollback down would shift the log under it.
    windowsPty: { backend: 'conpty' },
  });
  const host = document.createElement('div');
  const filling = 'width:100%;height:100%;';
  host.style.cssText = room ? `width:${room.width}px;height:${room.height}px;` : filling;
  parkingLot().appendChild(host);
  const fit = new FitAddon();
  const search = new SearchAddon();
  term.loadAddon(fit);
  term.loadAddon(search);
  term.loadAddon(new Unicode11Addon());
  term.unicode.activeVersion = '11';
  term.loadAddon(new WebLinksAddon((_e, uri) => openOutside(uri)));
  term.open(host);
  const x: XTerm = { term, fit, search, host, readOnly, renderer: { renderer: 'dom' } };
  try {
    const webgl = new WebglAddon();
    webgl.onContextLoss(() => {
      webgl.dispose();
      x.renderer = { renderer: 'dom', detail: 'WebGL context lost' };
      reportRenderer(x);
    });
    term.loadAddon(webgl);
    x.renderer = { renderer: 'webgl', detail: gpuName() };
  } catch (e) {
    // Falls back to the DOM renderer.
    x.renderer = { renderer: 'dom', detail: e instanceof Error ? e.message : String(e) };
  }
  fit.fit();
  // Whatever place it is shown in from now on, it fills it.
  host.style.cssText = filling;
  // The user's paste, noted on its way down, before xterm.js's own listeners get it.
  host.addEventListener('paste', () => (x.pasteAt = performance.now()), true);
  const keyToTerminal = (e: KeyboardEvent): boolean => {
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
    // Not on macOS, where Cmd+V pastes: Ctrl+V is Claude Code's key to read the clipboard's image.
    if (!IS_MAC && e.ctrlKey && e.key.toLowerCase() === 'v') return false;
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'c') {
      if (term.hasSelection()) navigator.clipboard.writeText(term.getSelection());
      return false;
    }
    // A log only scrolls (Shift+PageUp…): every other key goes on to the app's shortcuts.
    return !readOnly || (e.shiftKey && SCROLL_KEYS.has(e.key));
  };
  term.attachCustomKeyEventHandler((e) => {
    const toTerminal = keyToTerminal(e);
    if (toTerminal && e.type === 'keydown') x.keyAt = performance.now();
    return toTerminal;
  });
  return x;
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

/**
 * Claude Code's interface for the agent (« Vue Claude Code »): its session resumed in a terminal. Its full-screen interface
 * draws on the rows of its pseudo-console and nowhere else, so it starts at the size of `room`, where it will be shown.
 * `manual`: the user asked for it (« Relancer »), not the view of its own accord.
 */
export async function openAgentTerminal(agentId: string, room?: Room | null, manual = false): Promise<TermInfo> {
  const info = await interactive((cols, rows, onData) => api.agentTerminal(agentId, cols, rows, onData, manual), room);
  pasteImages(xterms.get(info.id)!, agentId);
  return info;
}

/**
 * What a paste in Claude Code’s interface carries: text, which xterm.js types (also beside a file:
 * the cells of a spreadsheet, but not a mere line break some applications add to one); files
 * without text (a screenshot, files copied); or, the WebView giving neither text nor file,
 * maybe an image only the system's clipboard holds (WebKitGTK leaves it out).
 */
export type Pasted = { kind: 'text'; text: string } | { kind: 'files'; files: File[] } | { kind: 'clipboard' };

export function pastedContent(data: { getData(type: string): string; files: Iterable<File> } | null): Pasted {
  const text = data?.getData('text/plain') ?? '';
  const files = [...(data?.files ?? [])];
  if (text.trim() || (text && !files.length)) return { kind: 'text', text };
  return files.length ? { kind: 'files', files } : { kind: 'clipboard' };
}

/**
 * Claude Code reads the clipboard's image on a key of its own (Alt+V under Windows, Ctrl+V
 * elsewhere), never on the terminal's paste: a pasted image is written to a file of the agent's,
 * and its path pasted, which Claude Code takes for the image (« [Image #1] ») as it does a file
 * dropped on a terminal. Any other file is copied to the agent's folder and its path pasted the same
 * way. Text goes on to xterm.js, unless a file pasted before it is still being written: it then
 * waits its turn. Files dropped on the terminal are taken as files pasted are: same queue, same paths.
 */
function pasteImages(x: XTerm, agentId: string) {
  let queue = Promise.resolve();
  let waiting = 0;
  const enqueue = (pasted: Pasted) => {
    waiting++;
    queue = queue
      .then(() => paste(x.term, agentId, pasted))
      .catch((err) => app.toast(err instanceof Error ? err.message : String(err), 'error'))
      .finally(() => waiting--);
  };
  // Caught on its way down, before xterm.js's own listeners.
  x.host.addEventListener(
    'paste',
    (e) => {
      const pasted = pastedContent(e.clipboardData);
      if (pasted.kind === 'text' && !waiting) return;
      e.preventDefault();
      e.stopPropagation();
      enqueue(pasted);
    },
    true,
  );
  dropFiles(x, (files) => enqueue({ kind: 'files', files }));
  // Said on the terminal once its pastes go through here (after its interface started): what the
  // end-to-end tests wait for before pasting.
  x.host.dataset.pastes = 'images';
}

/**
 * Accepts the files dropped on a terminal's host and hands them to `take`. A drag without a file (text) is left alone,
 * and with it the WebView's and xterm.js's own handling.
 */
function dropFiles(x: XTerm, take: (files: File[]) => void) {
  const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes('Files');
  // The terminal's canvas covers what the host paints itself, so the cue is a veil over it, of the theme's accent.
  const veil = document.createElement('div');
  veil.className = 'drop-veil';
  veil.style.cssText =
    'display:none;position:absolute;inset:0;z-index:2;pointer-events:none;border:2px dashed var(--p-ink);background:color-mix(in oklch, var(--p-ink) 10%, transparent);';
  x.host.style.position = 'relative';
  x.host.appendChild(veil);
  // dragenter and dragleave come for every element the drag crosses, not for the host alone: the veil goes with the last.
  let crossed = 0;
  x.host.addEventListener(
    'dragenter',
    (e) => {
      if (!hasFiles(e)) return;
      crossed++;
      veil.style.display = 'block';
    },
    true,
  );
  x.host.addEventListener(
    'dragleave',
    () => {
      if (crossed > 0 && --crossed === 0) veil.style.display = 'none';
    },
    true,
  );
  x.host.addEventListener(
    'dragover',
    (e) => {
      if (!hasFiles(e)) return;
      // Without it the WebView refuses the drop.
      e.preventDefault();
      e.dataTransfer!.dropEffect = 'copy';
    },
    true,
  );
  x.host.addEventListener(
    'drop',
    (e) => {
      crossed = 0;
      veil.style.display = 'none';
      // The drag's data is emptied once this handler returns: its files are taken now.
      const files = [...(e.dataTransfer?.files ?? [])];
      if (!files.length) return;
      // Left to the WebView, a dropped file would replace the app.
      e.preventDefault();
      e.stopPropagation();
      take(files);
    },
    true,
  );
}

async function paste(term: Terminal, agentId: string, pasted: Pasted) {
  if (pasted.kind === 'text') return term.paste(pasted.text);
  const files = pasted.kind === 'files' ? pasted.files : [await clipboardImage()].filter((f) => f !== null);
  // No file after all: the empty paste xterm.js would have made.
  if (!files.length) return term.paste('');
  // One paste a path: Claude Code reads two paths pasted at once only when neither has a space.
  for (const f of files) {
    const path = await pastedPath(f, agentId);
    if (path === null) continue;
    await bracketed(term);
    term.paste(path);
  }
}

/**
 * The path, as Claude Code reads it, of a file pasted: an image is written for its pastes, any other file is copied for the
 * agent, never attached where it is (this view's process is started once, with no way to give it another folder to read
 * afterwards). Null for an image refused, which a toast says.
 */
async function pastedPath(f: File, agentId: string): Promise<string | null> {
  const type = imageType(f);
  if (!type) {
    // As the composer's: a text over what a message carries, refused when the settings say so.
    const plan = planByPath(f, app.settings);
    if ('refusal' in plan) {
      app.toast(plan.refusal, 'error');
      return null;
    }
    return (await copyToDisk(f, agentId)).claudePath;
  }
  // Refused before it is read, as the backend would after.
  if (f.size > MAX_TOTAL) {
    const engine = engineName(app.agents[agentId]?.engine);
    app.toast(t('conv.terminal.imageTooBig', { name: f.name, max: sizeLabel(MAX_TOTAL), engine }), 'error');
    return null;
  }
  return api.pasteImage(agentId, type, await base64(f));
}

/**
 * How long an image's path waits for the interface to take pastes bracketed: Claude Code asks for
 * them as it starts, and a path pasted before is typed, never taken for the image.
 */
const BRACKETED_WAIT = 5000;

/**
 * Text dictated into the terminal `id` (the Claude Code view): pasted as the user's own paste, bracketed once the
 * program asks for it, so that Claude Code takes it into its prompt without running it (a line break in it is not Enter).
 */
export async function typeIntoTerminal(id: string, text: string) {
  const x = xterms.get(id);
  if (!x) return;
  await bracketed(x.term);
  x.pasteAt = performance.now();
  x.term.paste(text);
  x.term.focus();
}

/** Once `term` takes pastes bracketed, or after `BRACKETED_WAIT` (a program that never asks). */
async function bracketed(term: Terminal) {
  const until = Date.now() + BRACKETED_WAIT;
  while (!term.modes.bracketedPasteMode && Date.now() < until) await new Promise((r) => setTimeout(r, 50));
}

const decoder = new TextDecoder();

/**
 * How soon after a key or a paste xterm.js emits what it types (synchronously, from the keydown, keypress, input or
 * paste handling). What comes later is its own: focus, cursor and mouse reports, replies to the program's queries.
 */
const INPUT_WINDOW = 50;

/** What the data xterm.js emits at `at` is, when the user typed or pasted it; null for what it writes by itself. */
function userInput(x: XTerm, at: number): InputKind | null {
  if (x.pasteAt !== undefined && at - x.pasteAt <= INPUT_WINDOW) return 'paste';
  if (x.keyAt !== undefined && at - x.keyAt <= INPUT_WINDOW) return 'key';
  return null;
}

/** How long after the backend refused a size it is told again (ms), and how many times at most. */
const RESIZE_RETRY_AFTER = 500;
const RESIZE_RETRIES = 5;

/**
 * An interactive terminal that `spawn` starts at the size of a new xterm.js instance, which then shows it (that of `room`,
 * when it is known where it will be shown).
 */
async function interactive(
  spawn: (cols: number, rows: number, onData: (d: ArrayBuffer) => void) => Promise<TermInfo>,
  room?: Room | null,
): Promise<TermInfo> {
  const x = createXTerm(false, room);
  const { term } = x;
  // The size the pseudo-console has: the one the program started at, then the last one the backend took. A program that draws
  // on its rows (Claude Code's interface) never shows more than that, whatever xterm.js shows. The sizes are told in order and
  // one at a time (the one that comes while another is on its way goes next), and one the backend did not take is told again
  // soon, a few times, rather than left to a program that draws on fewer rows than the window shows.
  const pty = { cols: term.cols, rows: term.rows };
  let telling = false;
  let refused = 0;
  let again: ReturnType<typeof setTimeout> | undefined;
  const tell = () => {
    if (telling || !x.id) return;
    const { cols, rows } = term;
    if (cols === pty.cols && rows === pty.rows) return;
    telling = true;
    api.termResize(x.id, cols, rows).then(
      () => {
        pty.cols = cols;
        pty.rows = rows;
        refused = 0;
        telling = false;
        tell();
      },
      () => {
        telling = false;
        // A terminal that is gone refuses for good: not for ever.
        if (++refused <= RESIZE_RETRIES) {
          clearTimeout(again);
          again = setTimeout(tell, RESIZE_RETRY_AFTER);
        }
      },
    );
  };
  // Made before the spawn, whose output can come in before its answer: what is told waits for the id.
  const watch = echoWatch({
    now: () => performance.now(),
    visible: () => document.visibilityState === 'visible',
    report: (r) => {
      if (x.id) api.termReport(x.id, r).catch(() => {});
    },
  });
  // Every read of the backend is one message of the same bytes, in order: the bytes received so far name the read of an
  // echo (measured or not, every chunk counts).
  let outputBytes = 0;
  let info: TermInfo;
  try {
    info = await spawn(term.cols, term.rows, (buf) => {
      outputBytes += buf.byteLength;
      // The text is only decoded while a typed key waits for its echo.
      const token = watch.received(() => decoder.decode(buf), outputBytes);
      // xterm.js calls back once the data is parsed: the next animation frame is when it is drawn.
      term.write(new Uint8Array(buf), token === null ? undefined : () => requestAnimationFrame(() => watch.painted(token)));
    });
  } catch (e) {
    // Nothing to attach to: free the xterm instance, its WebGL context and its host.
    free(x);
    throw e;
  }
  x.id = info.id;
  reportRenderer(x);
  term.onData((d) => {
    const kind = userInput(x, performance.now());
    // The write goes out first and waits for nothing: its number is only for the report of a slow echo.
    const seq = api.termWrite(info.id, d);
    seq.catch(() => {});
    if (kind) watch.sent({ data: d, kind, seq });
  });
  term.onResize(tell);
  // Resized while the spawn was on its way (before it could be told).
  tell();
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

/**
 * « Taille du texte des terminaux », for every terminal: those on screen are fitted to their place again at once (their
 * program told its new size), those parked when they are shown (fitted to the parking lot, a program would be told a
 * size it never has).
 */
export function setTerminalFontSize(size: number) {
  for (const x of xterms.values()) {
    if (x.term.options.fontSize === size) continue;
    x.term.options.fontSize = size;
    if (!x.host.isConnected || x.host.parentElement === parking) continue;
    try {
      x.fit.fit();
    } catch {
      /* not measurable yet */
    }
  }
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

/**
 * Lets go of the terminal and stops its process. The promise settles once the backend has done it (or could not: it may
 * be gone already), for what must not start again before: the backend attaches to an interface still running.
 */
export function disposeTerminal(id: string): Promise<void> {
  const x = xterms.get(id);
  if (!x) return Promise.resolve();
  free(x);
  xterms.delete(id);
  return api.termKill(id).catch(() => {});
}
