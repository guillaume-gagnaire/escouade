import { createEvent, fireEvent, waitFor } from '@testing-library/svelte';
import type { ILinkHandler } from '@xterm/xterm';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend, resetApp } from '../test/ipc';

// What xterm.js was given to paste, in the order it came, and the key handlers it was given.
const pasted = vi.hoisted(() => [] as string[]);
const keyHandlers = vi.hoisted(() => [] as ((e: KeyboardEvent) => boolean)[]);
// Whether the program in the terminal asked for bracketed pastes (Claude Code does as it starts).
const modes = vi.hoisted(() => ({ bracketedPasteMode: true }));
// The xterm.js instances made, with what they were given to type back (`onData`) and to draw (`write`), and who listens to
// their size (`onResize`: `resize` is xterm.js changing it, which tells them).
interface FakeTerm {
  cols: number;
  rows: number;
  /** What `open` made in the host, where the fit addon looks for the size of the place. */
  element?: HTMLElement;
  dataHandlers: ((d: string) => void)[];
  resizeHandlers: ((size: { cols: number; rows: number }) => void)[];
  writes: { data: Uint8Array; done?: () => void }[];
  resize(cols: number, rows: number): void;
}
const terms = vi.hoisted(() => [] as FakeTerm[]);
// The size of the host (its inline style) each time a terminal was fitted.
const fits = vi.hoisted(() => [] as { width: string; height: string }[]);
// The WebGL addon: `fail` is why loading it throws (none: it loads), `losses` its context loss listeners.
const gl = vi.hoisted(() => ({ fail: null as string | null, losses: [] as (() => void)[], disposed: 0 }));
// What each terminal was built with, and what its web links addon does with a link it found.
const termOptions = vi.hoisted(() => [] as { linkHandler?: ILinkHandler }[]);
const webLinks = vi.hoisted(() => [] as ((e: MouseEvent, uri: string) => void)[]);

// xterm.js draws on a canvas: stood in for, only the spawn, the keys, the links and what is pasted are looked at.
vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    cols = 100;
    rows = 30;
    options = {};
    modes = modes;
    unicode = { activeVersion: '' };
    element?: HTMLElement;
    dataHandlers: ((d: string) => void)[] = [];
    resizeHandlers: ((size: { cols: number; rows: number }) => void)[] = [];
    writes: { data: Uint8Array; done?: () => void }[] = [];
    constructor(options: { linkHandler?: ILinkHandler }) {
      terms.push(this);
      termOptions.push(options);
    }
    loadAddon(addon: { activate?: (term: unknown) => void }) {
      addon.activate?.(this);
    }
    open(host: HTMLElement) {
      // As xterm.js: its element in the host, which is what the fit addon measures.
      this.element = document.createElement('div');
      host.appendChild(this.element);
    }
    resize(cols: number, rows: number) {
      // As xterm.js: a size it already has is no change.
      if (cols === this.cols && rows === this.rows) return;
      this.cols = cols;
      this.rows = rows;
      for (const handler of this.resizeHandlers) handler({ cols, rows });
    }
    write(data: Uint8Array, done?: () => void) {
      this.writes.push({ data, done });
    }
    attachCustomKeyEventHandler(handler: (e: KeyboardEvent) => boolean) {
      keyHandlers.push(handler);
    }
    hasSelection() {
      return false;
    }
    onData(handler: (d: string) => void) {
      this.dataHandlers.push(handler);
    }
    onResize(handler: (size: { cols: number; rows: number }) => void) {
      this.resizeHandlers.push(handler);
    }
    paste(text: string) {
      pasted.push(text);
    }
    dispose() {}
  },
}));
// The fit addon sizes the terminal to its host as the real one does, with a character cell of 8 x 20 px: a host in pixels gives
// `width / 8` columns and `height / 20` rows, one that fills its box (100 %) leaves the size as it is.
vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    term?: FakeTerm;
    activate(term: FakeTerm) {
      this.term = term;
    }
    fit() {
      const term = this.term!;
      const { width, height } = term.element!.parentElement!.style;
      fits.push({ width, height });
      if (width.endsWith('px') && height.endsWith('px')) {
        term.resize(Math.floor(parseFloat(width) / 8), Math.floor(parseFloat(height) / 20));
      }
    }
  },
}));
vi.mock('@xterm/addon-search', () => ({ SearchAddon: class {} }));
vi.mock('@xterm/addon-unicode11', () => ({ Unicode11Addon: class {} }));
vi.mock('@xterm/addon-web-links', () => ({
  WebLinksAddon: class {
    constructor(handler: (e: MouseEvent, uri: string) => void) {
      webLinks.push(handler);
    }
  },
}));
vi.mock('@xterm/addon-webgl', () => ({
  WebglAddon: class {
    activate() {
      if (gl.fail) throw new Error(gl.fail);
    }
    onContextLoss(listener: () => void) {
      gl.losses.push(listener);
    }
    dispose() {
      gl.disposed++;
    }
  },
}));

import { app } from './state.svelte';
import {
  disposeLog,
  disposeTerminal,
  getXTerm,
  launchLog,
  openAccountLogin,
  openAgentTerminal,
  openTerminal,
  pastedContent,
  roomOf,
  setLogLive,
  setTerminalFontSize,
  ANSI_TOKENS,
} from './terminals';

beforeEach(() => {
  terms.length = 0;
  fits.length = 0;
  gl.fail = null;
  gl.losses.length = 0;
  gl.disposed = 0;
  // The theme's colors are read back from a 1px canvas, which jsdom does not draw.
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    fillRect() {},
    getImageData: () => ({ data: [0, 0, 0] }),
  } as unknown as CanvasRenderingContext2D);
});

const PNG = [137, 80, 78, 71, 13, 10, 26, 10];
const capture = () => new File([new Uint8Array(PNG)], 'image.png', { type: 'image/png' });

/** A paste's clipboard as the WebView gives it: its text (none: no text/plain) and its files. */
function clipboard(o: { text?: string; files?: File[]; types?: string[] }) {
  const types = o.types ?? [...(o.text === undefined ? [] : ['text/plain']), ...(o.files?.length ? ['Files'] : [])];
  return {
    types,
    files: o.files ?? [],
    getData: (type: string) => (type === 'text/plain' ? (o.text ?? '') : ''),
  };
}

/** What is dragged over a terminal: the same as a clipboard, with the effect the drop is given. */
function dragged(o: { text?: string; files?: File[] }) {
  return { ...clipboard(o), dropEffect: 'none' };
}

describe('what a paste in Claude Code’s interface carries', () => {
  it('leaves text to the terminal', () => {
    expect(pastedContent(clipboard({ text: 'npm test' }))).toEqual({ kind: 'text', text: 'npm test' });
  });

  it('leaves text to the terminal even with an image beside it (cells of a spreadsheet)', () => {
    expect(pastedContent(clipboard({ text: 'a\tb', files: [capture()] }))).toEqual({ kind: 'text', text: 'a\tb' });
  });

  it('takes a screenshot, an image without text', () => {
    const image = capture();
    expect(pastedContent(clipboard({ files: [image] }))).toEqual({ kind: 'files', files: [image] });
  });

  it('takes every file copied, images and others, in their order', () => {
    const notes = new File(['x'], 'notes.txt', { type: 'text/plain' });
    const photo = new File([new Uint8Array(PNG)], 'photo.JPG', { type: '' });
    const image = capture();
    expect(pastedContent(clipboard({ files: [photo, notes, image] }))).toEqual({ kind: 'files', files: [photo, notes, image] });
  });

  it('takes a file that is no image as it takes an image', () => {
    const doc = new File(['%PDF'], 'devis.pdf', { type: 'application/pdf' });
    expect(pastedContent(clipboard({ files: [doc] }))).toEqual({ kind: 'files', files: [doc] });
  });

  it('takes an empty text, or a blank one beside a file, for none', () => {
    const image = capture();
    expect(pastedContent(clipboard({ text: '', files: [image] }))).toEqual({ kind: 'files', files: [image] });
    // Some applications give a line break with the image they copy.
    expect(pastedContent(clipboard({ text: '\r\n', files: [image] }))).toEqual({ kind: 'files', files: [image] });
    // Alone, a blank is pasted as it is.
    expect(pastedContent(clipboard({ text: ' ' }))).toEqual({ kind: 'text', text: ' ' });
  });

  it('looks at the system’s clipboard when the WebView gives neither text nor file (WebKitGTK)', () => {
    expect(pastedContent(clipboard({}))).toEqual({ kind: 'clipboard' });
    // A browser's « Copier l'image »: the page only sees its HTML.
    expect(pastedContent(clipboard({ types: ['text/html'] }))).toEqual({ kind: 'clipboard' });
    expect(pastedContent(null)).toEqual({ kind: 'clipboard' });
  });
});

describe('starting Claude Code’s interface', () => {
  it('tells the backend whether the user asked for it (« Relancer »): not unless said', async () => {
    resetApp();
    let n = 0;
    const backend = fakeBackend({ agent_terminal: () => ({ id: `tui-start-${++n}`, projectId: '', name: 'agent', shell: 'claude' }) });
    await openAgentTerminal('a1');
    await openAgentTerminal('a1', null, true);
    await openAgentTerminal('a1', null, false);
    expect(backend.called('agent_terminal').map((c) => ({ id: c.args.id, manual: c.args.manual }))).toEqual([
      { id: 'a1', manual: false },
      { id: 'a1', manual: true },
      { id: 'a1', manual: false },
    ]);
  });
});

describe('pasting in Claude Code’s interface', () => {
  const info = { id: 'tui-1', projectId: '', name: 'agent', shell: 'claude' };
  const PATH = 'C:\\Users\\Jean Dupont\\.escouade\\tui\\pastes\\4f2a.png';

  beforeEach(() => {
    resetApp();
    pasted.length = 0;
    modes.bracketedPasteMode = true;
  });

  /**
   * The text field of a new terminal, where `paste` pastes and says whether the WebView's paste
   * was stopped; `xterm` counts the pastes xterm.js got (its own `paste` listener stands for it).
   */
  async function terminalField(open: () => Promise<{ id: string }> = () => openAgentTerminal('a1')) {
    const { id } = await open();
    const field = document.createElement('textarea');
    getXTerm(id)!.host.appendChild(field);
    let xterm = 0;
    field.addEventListener('paste', () => xterm++);
    return {
      paste(data: ReturnType<typeof clipboard>) {
        const e = createEvent.paste(field, { clipboardData: data });
        fireEvent(field, e);
        return e.defaultPrevented;
      },
      /** What a drag over the terminal gets: whether the drop is accepted, and with what effect. */
      dragOver(data: ReturnType<typeof dragged>) {
        const e = createEvent.dragOver(field, { dataTransfer: data });
        fireEvent(field, e);
        return { accepted: e.defaultPrevented, effect: data.dropEffect };
      },
      /** Whether the drop was stopped (the WebView would open the file, or xterm.js take the text). */
      drop(data: ReturnType<typeof dragged>) {
        const e = createEvent.drop(field, { dataTransfer: data });
        fireEvent(field, e);
        return e.defaultPrevented;
      },
      dragEnter: (data: ReturnType<typeof dragged>) => void fireEvent(field, createEvent.dragEnter(field, { dataTransfer: data })),
      dragLeave: (data: ReturnType<typeof dragged>) => void fireEvent(field, createEvent.dragLeave(field, { dataTransfer: data })),
      /** The veil shown over the terminal while a file is held above it. */
      veil: () => getXTerm(id)!.host.querySelector<HTMLElement>('.drop-veil')?.style.display,
      xterm: () => xterm,
    };
  }

  async function paste(data: ReturnType<typeof clipboard>, open?: () => Promise<{ id: string }>) {
    const field = await terminalField(open);
    const prevented = field.paste(data);
    return { prevented, xterm: () => field.xterm() > 0 };
  }

  it('writes a pasted image to a file of the agent and types its path, which Claude Code takes for the image', async () => {
    const backend = fakeBackend({ agent_terminal: () => info, paste_image: () => PATH });
    const { prevented, xterm } = await paste(clipboard({ files: [capture()] }));
    expect(prevented).toBe(true);
    expect(xterm()).toBe(false);
    await waitFor(() => expect(pasted).toEqual([PATH]));
    expect(backend.called('paste_image')[0].args).toEqual({ id: 'a1', mediaType: 'image/png', data: btoa(String.fromCharCode(...PNG)) });
    expect(backend.called('clipboard_image')).toHaveLength(0);
  });

  it('waits for Claude Code to take pastes bracketed before typing an image’s path: typed raw, the path is no image', async () => {
    modes.bracketedPasteMode = false;
    fakeBackend({ agent_terminal: () => info, paste_image: () => PATH });
    await paste(clipboard({ files: [capture()] }));
    await new Promise((r) => setTimeout(r, 200));
    expect(pasted).toEqual([]);
    modes.bracketedPasteMode = true;
    await waitFor(() => expect(pasted).toEqual([PATH]));
  });

  it('types an image’s path all the same when the program never asks for bracketed pastes', async () => {
    modes.bracketedPasteMode = false;
    fakeBackend({ agent_terminal: () => info, paste_image: () => PATH });
    await paste(clipboard({ files: [capture()] }));
    vi.useFakeTimers();
    try {
      await vi.advanceTimersByTimeAsync(4_000);
      expect(pasted).toEqual([]);
      await vi.advanceTimersByTimeAsync(1_500);
      expect(pasted).toEqual([PATH]);
    } finally {
      vi.useRealTimers();
    }
  });

  it('pastes each image copied on its own, in their order: Claude Code takes one path a paste, spaces and all', async () => {
    const backend = fakeBackend({
      agent_terminal: () => info,
      paste_image: ({ mediaType }: { mediaType: string }) => (mediaType === 'image/png' ? PATH : 'C:\\x\\b.jpg'),
    });
    await paste(clipboard({ files: [capture(), new File([new Uint8Array(PNG)], 'b.jpg', { type: 'image/jpeg' })] }));
    await waitFor(() => expect(pasted).toEqual([PATH, 'C:\\x\\b.jpg']));
    expect(backend.called('paste_image').map((c) => c.args.mediaType)).toEqual(['image/png', 'image/jpeg']);
  });

  it('keeps the order of the pastes: text pasted while an image is being kept comes after it', async () => {
    let keep: (path: string) => void = () => {};
    fakeBackend({ agent_terminal: () => info, paste_image: () => new Promise((r) => (keep = r)) });
    const field = await terminalField();
    expect(field.paste(clipboard({ files: [capture()] }))).toBe(true);
    expect(field.paste(clipboard({ text: ' décris-la' }))).toBe(true);
    expect(field.xterm()).toBe(0);
    await new Promise((r) => setTimeout(r, 20));
    expect(pasted).toEqual([]);
    keep(PATH);
    await waitFor(() => expect(pasted).toEqual([PATH, ' décris-la']));
    // Nothing waiting: text goes to xterm.js again.
    expect(field.paste(clipboard({ text: 'npm test' }))).toBe(false);
    expect(field.xterm()).toBe(1);
  });

  it('gives a JPEG copied without a type its type from its name', async () => {
    const backend = fakeBackend({ agent_terminal: () => info, paste_image: () => PATH });
    await paste(clipboard({ files: [new File([new Uint8Array(PNG)], 'photo.jpeg', { type: '' })] }));
    await waitFor(() => expect(backend.called('paste_image')).toHaveLength(1));
    expect(backend.called('paste_image')[0].args.mediaType).toBe('image/jpeg');
  });

  it('leaves pasted text to xterm.js, as before', async () => {
    const backend = fakeBackend({ agent_terminal: () => info, paste_image: () => PATH });
    const { prevented, xterm } = await paste(clipboard({ text: 'npm test', files: [capture()] }));
    expect(prevented).toBe(false);
    expect(xterm()).toBe(true);
    await new Promise((r) => setTimeout(r, 20));
    expect(backend.called('paste_image')).toHaveLength(0);
    expect(backend.called('clipboard_image')).toHaveLength(0);
    expect(pasted).toEqual([]);
  });

  it('reads the image WebKitGTK leaves out of a paste (Linux) from the system clipboard', async () => {
    const backend = fakeBackend({
      agent_terminal: () => info,
      clipboard_image: () => new Uint8Array(PNG).buffer,
      paste_image: () => '/home/jean/.escouade/tui/pastes/4f2a.png',
    });
    const { prevented, xterm } = await paste(clipboard({}));
    expect(prevented).toBe(true);
    expect(xterm()).toBe(false);
    await waitFor(() => expect(pasted).toEqual(['/home/jean/.escouade/tui/pastes/4f2a.png']));
    expect(backend.called('paste_image')[0].args).toEqual({ id: 'a1', mediaType: 'image/png', data: btoa(String.fromCharCode(...PNG)) });
  });

  it('pastes nothing but an empty paste, as xterm.js did, when the system clipboard holds no image either', async () => {
    const backend = fakeBackend({ agent_terminal: () => info, clipboard_image: () => new ArrayBuffer(0) });
    await paste(clipboard({}));
    await waitFor(() => expect(pasted).toEqual(['']));
    expect(backend.called('clipboard_image')).toHaveLength(1);
    expect(backend.called('paste_image')).toHaveLength(0);
    expect(app.toasts).toEqual([]);
  });

  it('says why when the image cannot be kept', async () => {
    fakeBackend({
      agent_terminal: () => info,
      paste_image: () => Promise.reject('disque plein'),
    });
    await paste(clipboard({ files: [capture()] }));
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'disque plein', kind: 'error' }));
    expect(pasted).toEqual([]);
  });

  it('refuses an image over 18 MB before reading it, and pastes the others', async () => {
    const backend = fakeBackend({ agent_terminal: () => info, paste_image: () => PATH });
    const huge = new File([new Uint8Array(PNG)], 'ecran-5k.png', { type: 'image/png' });
    Object.defineProperty(huge, 'size', { value: 18 * 1024 * 1024 + 1 });
    await paste(clipboard({ files: [huge, capture()] }));
    await waitFor(() => expect(pasted).toEqual([PATH]));
    expect(app.toasts.at(-1)).toMatchObject({ text: 'ecran-5k.png dépasse 18 Mo : Claude Code ne la recevra pas.', kind: 'error' });
    expect(backend.called('paste_image')).toHaveLength(1);
  });

  it('leaves the pastes of a shell’s terminal to xterm.js', async () => {
    const backend = fakeBackend({ term_spawn: () => ({ ...info, id: 'sh-1', shell: 'pwsh' }) });
    const { prevented, xterm } = await paste(clipboard({ files: [capture()] }), () => openTerminal('p1', 'pwsh', 'pwsh-1'));
    expect(prevented).toBe(false);
    expect(xterm()).toBe(true);
    expect(backend.called('paste_image')).toHaveLength(0);
    expect(backend.called('attach_open')).toHaveLength(0);
  });

  const doc = () => new File(['%PDF-1.4'], 'devis.pdf', { type: 'application/pdf' });
  const zip = () => new File(['PK'], 'sources.zip', { type: 'application/zip' });
  const copyPath = (name: string) => `/home/jean/.escouade/attachments/a1/4f2a9c1e/${name}`;
  /** The backend of a copy: the handle is named after the file, and the copy comes back with the path Claude Code reads. */
  const copying = (over: Record<string, (a: any) => unknown> = {}) => ({
    agent_terminal: () => info,
    attach_open: ({ name }: { name: string }) => `h:${name}`,
    attach_finish: ({ handle }: { handle: string }) => ({
      name: handle.slice(2),
      size: 8,
      origin: false,
      path: `C:\\Users\\Jean Dupont\\.escouade\\attachments\\a1\\4f2a9c1e\\${handle.slice(2)}`,
      claudePath: copyPath(handle.slice(2)),
    }),
    ...over,
  });

  describe('a file that is no image', () => {
    it('is copied to the folder of the agent, and the path Claude Code reads it at is typed', async () => {
      const backend = fakeBackend(copying());
      const { prevented, xterm } = await paste(clipboard({ files: [doc()] }));
      expect(prevented).toBe(true);
      expect(xterm()).toBe(false);
      await waitFor(() => expect(pasted).toEqual([copyPath('devis.pdf')]));
      expect(backend.called('attach_open')[0].args).toEqual({ owner: 'a1', name: 'devis.pdf', size: 8 });
      expect(new TextDecoder().decode(backend.called('attach_chunk')[0].args as unknown as Uint8Array)).toBe('%PDF-1.4');
      expect(backend.called('attach_finish')).toHaveLength(1);
      expect(backend.called('paste_image')).toHaveLength(0);
    });

    it('is always copied: the view of Claude Code cannot be given a folder to read afterwards', async () => {
      const backend = fakeBackend(copying());
      app.settings.attachOrigin = true;
      await paste(clipboard({ files: [doc()] }));
      await waitFor(() => expect(pasted).toHaveLength(1));
      expect(backend.called('attach_origin')).toHaveLength(0);
      expect(backend.called('file_origin')).toHaveLength(0);
    });

    it('is typed after the image copied with it, each on its own paste, in their order', async () => {
      const backend = fakeBackend(copying({ paste_image: () => PATH }));
      await paste(clipboard({ files: [doc(), capture(), new File(['PK'], 'sources.zip', { type: 'application/zip' })] }));
      await waitFor(() => expect(pasted).toEqual([copyPath('devis.pdf'), PATH, copyPath('sources.zip')]));
      expect(backend.called('paste_image')).toHaveLength(1);
      expect(backend.called('attach_open').map((c) => c.args.name)).toEqual(['devis.pdf', 'sources.zip']);
    });

    it('has its path typed once Claude Code takes pastes bracketed, as an image’s', async () => {
      modes.bracketedPasteMode = false;
      fakeBackend(copying());
      await paste(clipboard({ files: [doc()] }));
      await new Promise((r) => setTimeout(r, 200));
      expect(pasted).toEqual([]);
      modes.bracketedPasteMode = true;
      await waitFor(() => expect(pasted).toEqual([copyPath('devis.pdf')]));
    });

    it('keeps the order of the pastes: text pasted while it is copied comes after its path', async () => {
      let open: (handle: string) => void = () => {};
      fakeBackend(copying({ attach_open: () => new Promise((r) => (open = r)) }));
      const field = await terminalField();
      expect(field.paste(clipboard({ files: [doc()] }))).toBe(true);
      expect(field.paste(clipboard({ text: ' résume-le' }))).toBe(true);
      await new Promise((r) => setTimeout(r, 20));
      expect(pasted).toEqual([]);
      open('h:devis.pdf');
      await waitFor(() => expect(pasted).toEqual([copyPath('devis.pdf'), ' résume-le']));
    });

    it('says why when it cannot be copied, and types nothing for it', async () => {
      const backend = fakeBackend(
        copying({
          attach_open: () => Promise.reject('devis.pdf dépasse 100 Mo'),
        }),
      );
      await paste(clipboard({ files: [doc()] }));
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'devis.pdf dépasse 100 Mo', kind: 'error' }));
      expect(pasted).toEqual([]);
      expect(backend.called('attach_chunk')).toHaveLength(0);
    });

    it('erases the half-written copy when a chunk fails', async () => {
      const backend = fakeBackend(copying({ attach_chunk: () => Promise.reject('disque plein') }));
      await paste(clipboard({ files: [doc()] }));
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'disque plein', kind: 'error' }));
      expect(backend.called('attach_cancel').map((c) => c.args)).toEqual([{ handle: 'h:devis.pdf' }]);
      expect(pasted).toEqual([]);
    });

    it('follows the settings for a text over 256 Ko: refused when they say so, copied otherwise', async () => {
      const backend = fakeBackend(copying());
      const journal = new File(['x'], 'journal.log', { type: 'text/plain' });
      Object.defineProperty(journal, 'size', { value: 300 * 1024 });
      app.settings.attachBigText = 'refuse';
      await paste(clipboard({ files: [journal] }));
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'journal.log dépasse 256 Ko', kind: 'error' }));
      expect(backend.called('attach_open')).toHaveLength(0);
      expect(pasted).toEqual([]);
      app.settings.attachBigText = 'path';
      await paste(clipboard({ files: [journal] }));
      await waitFor(() => expect(pasted).toEqual([copyPath('journal.log')]));
    });

    it('is not held to the 18 MB of an image: a big file is copied by chunks', async () => {
      const backend = fakeBackend(copying());
      const film = new File(['x'], 'film.mkv', { type: 'video/x-matroska' });
      Object.defineProperty(film, 'size', { value: 20 * 1024 * 1024 });
      await paste(clipboard({ files: [film] }));
      await waitFor(() => expect(pasted).toEqual([copyPath('film.mkv')]));
      expect(backend.called('attach_chunk')).toHaveLength(5);
      expect(app.toasts).toEqual([]);
    });
  });

  describe('a file dropped on it', () => {
    it('accepts a drag that holds a file, as a copy', async () => {
      fakeBackend(copying());
      const field = await terminalField();
      expect(field.dragOver(dragged({ files: [zip()] }))).toEqual({ accepted: true, effect: 'copy' });
    });

    it('leaves a drag of text alone', async () => {
      fakeBackend(copying());
      const field = await terminalField();
      expect(field.dragOver(dragged({ text: 'npm test' }))).toEqual({ accepted: false, effect: 'none' });
    });

    it('copies a .zip to the folder of the agent and types the path Claude Code reads it at', async () => {
      const backend = fakeBackend(copying());
      const field = await terminalField();
      expect(field.drop(dragged({ files: [zip()] }))).toBe(true);
      await waitFor(() => expect(pasted).toEqual([copyPath('sources.zip')]));
      expect(backend.called('attach_open')[0].args).toEqual({ owner: 'a1', name: 'sources.zip', size: 2 });
      expect(new TextDecoder().decode(backend.called('attach_chunk')[0].args as unknown as Uint8Array)).toBe('PK');
      expect(backend.called('attach_finish')).toHaveLength(1);
      expect(backend.called('paste_image')).toHaveLength(0);
    });

    it('is always copied, never attached where it is: this view’s process cannot be given another folder to read', async () => {
      const backend = fakeBackend(copying());
      app.settings.attachOrigin = true;
      const field = await terminalField();
      field.drop(dragged({ files: [zip()] }));
      await waitFor(() => expect(pasted).toHaveLength(1));
      expect(backend.called('attach_origin')).toHaveLength(0);
      expect(backend.called('file_origin')).toHaveLength(0);
    });

    it('writes a dropped image as a pasted one is, and types its path', async () => {
      const backend = fakeBackend(copying({ paste_image: () => PATH }));
      const field = await terminalField();
      expect(field.drop(dragged({ files: [capture()] }))).toBe(true);
      await waitFor(() => expect(pasted).toEqual([PATH]));
      expect(backend.called('paste_image')[0].args).toEqual({ id: 'a1', mediaType: 'image/png', data: btoa(String.fromCharCode(...PNG)) });
      expect(backend.called('attach_open')).toHaveLength(0);
    });

    it('types the path of each file on its own paste, in their order: Claude Code takes one path a paste, spaces and all', async () => {
      const backend = fakeBackend(copying());
      const field = await terminalField();
      field.drop(dragged({ files: [doc(), zip()] }));
      await waitFor(() => expect(pasted).toEqual([copyPath('devis.pdf'), copyPath('sources.zip')]));
      expect(backend.called('attach_open').map((c) => c.args.name)).toEqual(['devis.pdf', 'sources.zip']);
    });

    it('leaves a drop of text alone: xterm.js and the WebView do what they did', async () => {
      const backend = fakeBackend(copying());
      const field = await terminalField();
      expect(field.drop(dragged({ text: 'npm test' }))).toBe(false);
      await new Promise((r) => setTimeout(r, 20));
      expect(pasted).toEqual([]);
      expect(backend.called('attach_open')).toHaveLength(0);
      expect(backend.called('paste_image')).toHaveLength(0);
    });

    it('has its path typed once Claude Code takes pastes bracketed, as a pasted file’s', async () => {
      modes.bracketedPasteMode = false;
      fakeBackend(copying());
      const field = await terminalField();
      field.drop(dragged({ files: [zip()] }));
      await new Promise((r) => setTimeout(r, 200));
      expect(pasted).toEqual([]);
      modes.bracketedPasteMode = true;
      await waitFor(() => expect(pasted).toEqual([copyPath('sources.zip')]));
    });

    it('keeps the order of the pastes: text pasted while it is copied comes after its path', async () => {
      let open: (handle: string) => void = () => {};
      fakeBackend(copying({ attach_open: () => new Promise((r) => (open = r)) }));
      const field = await terminalField();
      expect(field.drop(dragged({ files: [zip()] }))).toBe(true);
      expect(field.paste(clipboard({ text: ' décompresse-le' }))).toBe(true);
      await new Promise((r) => setTimeout(r, 20));
      expect(pasted).toEqual([]);
      open('h:sources.zip');
      await waitFor(() => expect(pasted).toEqual([copyPath('sources.zip'), ' décompresse-le']));
    });

    it('says why when the backend refuses it, and types nothing for it', async () => {
      const backend = fakeBackend(copying({ attach_open: () => Promise.reject('sources.zip dépasse 100 Mo') }));
      const field = await terminalField();
      field.drop(dragged({ files: [zip()] }));
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'sources.zip dépasse 100 Mo', kind: 'error' }));
      expect(pasted).toEqual([]);
      expect(backend.called('attach_chunk')).toHaveLength(0);
    });

    it('shows a veil over the terminal while a file is held above it, however many elements it crosses', async () => {
      fakeBackend(copying());
      const field = await terminalField();
      expect(field.veil()).toBe('none');
      const held = dragged({ files: [zip()] });
      field.dragEnter(held);
      field.dragEnter(held);
      expect(field.veil()).toBe('block');
      field.dragLeave(held);
      expect(field.veil()).toBe('block');
      field.dragLeave(held);
      expect(field.veil()).toBe('none');
    });

    it('takes the veil off on the drop, and shows none for a drag of text', async () => {
      fakeBackend(copying());
      const field = await terminalField();
      field.dragEnter(dragged({ text: 'npm test' }));
      expect(field.veil()).toBe('none');
      field.dragEnter(dragged({ files: [zip()] }));
      expect(field.veil()).toBe('block');
      field.drop(dragged({ files: [zip()] }));
      expect(field.veil()).toBe('none');
    });

    it('is left to the WebView in a shell’s terminal', async () => {
      const backend = fakeBackend({ term_spawn: () => ({ ...info, id: 'sh-1', shell: 'pwsh' }) });
      const field = await terminalField(() => openTerminal('p1', 'pwsh', 'pwsh-1'));
      expect(field.dragOver(dragged({ files: [zip()] })).accepted).toBe(false);
      expect(field.drop(dragged({ files: [zip()] }))).toBe(false);
      expect(backend.called('attach_open')).toHaveLength(0);
      expect(field.veil()).toBeUndefined();
    });
  });
});

describe('Ctrl+V in a terminal', () => {
  /** The key handler of a terminal opened on macOS or elsewhere. */
  async function keys(mac: boolean) {
    vi.resetModules();
    vi.doMock('./platform', async (original) => ({ ...(await original<typeof import('./platform')>()), IS_MAC: mac }));
    const terminals = await import('./terminals');
    fakeBackend({ term_spawn: () => ({ id: `t-${mac}`, projectId: 'p1', name: 'zsh-1', shell: 'zsh' }) });
    await terminals.openTerminal('p1', 'zsh', 'zsh-1');
    return keyHandlers.at(-1)!;
  }
  const press = (handler: (e: KeyboardEvent) => boolean, init: KeyboardEventInit) =>
    handler(new KeyboardEvent('keydown', { key: 'v', ...init }));

  afterEach(() => {
    vi.doUnmock('./platform');
    vi.resetModules();
  });

  it('pastes under Windows and Linux: the WebView’s paste follows', async () => {
    expect(press(await keys(false), { ctrlKey: true })).toBe(false);
  });

  it('goes to the terminal under macOS, where Cmd+V pastes: Claude Code reads the clipboard’s image on it', async () => {
    const handler = await keys(true);
    expect(press(handler, { ctrlKey: true })).toBe(true);
    expect(press(handler, { metaKey: true })).toBe(false);
  });
});

describe.each([
  ['a shell’s terminal', () => openTerminal('p1', 'pwsh', 'pwsh-1')],
  ['Claude Code’s interface', () => openAgentTerminal('a1')],
])('the links of %s', (_name, open) => {
  const info = { id: 'tl-1', projectId: 'p1', name: 'term', shell: 'pwsh' };
  const click = () => new MouseEvent('click');

  /** A terminal opened; the links it was built to open. */
  async function opened() {
    resetApp();
    const backend = fakeBackend({ term_spawn: () => info, agent_terminal: () => info });
    await open();
    const activate = (uri: string) => termOptions.at(-1)!.linkHandler!.activate(click(), uri, {} as never);
    return { backend, activate, urls: () => backend.called('open_url').map((c) => c.args.url) };
  }

  it('open in the system’s browser: the hyperlinks Claude Code writes (OSC 8) and the addresses printed as text', async () => {
    const { activate, urls } = await opened();
    activate('https://example.com/pr/1');
    webLinks.at(-1)!(click(), 'http://localhost:4111/connexion');
    await vi.waitFor(() => expect(urls()).toEqual(['https://example.com/pr/1', 'http://localhost:4111/connexion']));
  });

  // Defence in depth: xterm.js only offers http(s) OSC 8 links (no `allowNonHttpProtocols`), so it never gets here.
  it('would open a mail address too, should xterm.js offer one', async () => {
    const { activate, urls } = await opened();
    activate('mailto:jean@example.com');
    await vi.waitFor(() => expect(urls()).toEqual(['mailto:jean@example.com']));
  });

  it('leave a file or a script alone: nothing is opened', async () => {
    const { activate, urls } = await opened();
    activate('file:///C:/Users/Jean/notes.txt');
    activate('javascript:alert(1)');
    webLinks.at(-1)!(click(), 'file:///C:/x');
    await new Promise((r) => setTimeout(r, 20));
    expect(urls()).toEqual([]);
  });

  it('never ask xterm.js’s confirm() nor open a window of their own, which neither WebView2 nor a native dialog can honor', async () => {
    const confirm = vi.spyOn(window, 'confirm');
    const open = vi.spyOn(window, 'open');
    const { activate, urls } = await opened();
    activate('https://example.com/pr/1');
    await vi.waitFor(() => expect(urls()).toHaveLength(1));
    expect(confirm).not.toHaveBeenCalled();
    expect(open).not.toHaveBeenCalled();
  });

  it('only offer the web’s addresses as hyperlinks: xterm.js leaves the others as text', async () => {
    await opened();
    expect(termOptions.at(-1)!.linkHandler!.allowNonHttpProtocols).toBeFalsy();
  });
});

describe('opening a terminal', () => {
  const info = { id: 't1', projectId: 'p1', name: 'pwsh-1', shell: 'pwsh' };

  it('opens in the project’s folder when no place is given', async () => {
    const backend = fakeBackend({ term_spawn: () => info });
    await openTerminal('p1', 'pwsh', 'pwsh-1');
    expect(backend.called('term_spawn')[0].args).toMatchObject({ projectId: 'p1', shell: 'pwsh', name: 'pwsh-1', cols: 100, rows: 30 });
    expect(backend.called('term_spawn')[0].args).not.toHaveProperty('agentId');
    expect(backend.called('term_spawn')[0].args).not.toHaveProperty('subdir');
  });

  it('asks the backend for the agent’s worktree and the folder of it', async () => {
    const backend = fakeBackend({ term_spawn: () => info });
    await openTerminal('p1', 'pwsh', 'pwsh · lib', { agentId: 'a1', subdir: 'src/lib' });
    expect(backend.called('term_spawn')[0].args).toMatchObject({ projectId: 'p1', agentId: 'a1', subdir: 'src/lib', name: 'pwsh · lib' });
  });

  it('passes on the backend’s refusal of a place', async () => {
    fakeBackend({ term_spawn: () => Promise.reject('chemin hors du dossier : ..') });
    await expect(openTerminal('p1', 'pwsh', 'pwsh · ..', { subdir: '..' })).rejects.toBe('chemin hors du dossier : ..');
  });
});

describe('freeing a terminal', () => {
  const info = { id: 't1', projectId: 'p1', name: 'pwsh-1', shell: 'pwsh' };

  it('lets go of it at once, and resolves once the backend has stopped its process', async () => {
    let stop!: () => void;
    const backend = fakeBackend({
      term_spawn: () => info,
      term_kill: () => new Promise<void>((resolve) => (stop = resolve)),
    });
    await openTerminal('p1', 'pwsh', 'pwsh-1');
    expect(getXTerm('t1')).toBeDefined();
    let done = false;
    const freed = disposeTerminal('t1').then(() => (done = true));
    // The window no longer holds it; the kill is asked, not yet answered.
    expect(getXTerm('t1')).toBeUndefined();
    await waitFor(() => expect(backend.called('term_kill')).toEqual([{ cmd: 'term_kill', args: { id: 't1' } }]));
    await new Promise((r) => setTimeout(r, 20));
    expect(done).toBe(false);
    stop();
    await freed;
    expect(done).toBe(true);
  });

  it('resolves all the same when the backend cannot stop it: it may be gone already', async () => {
    fakeBackend({ term_spawn: () => info, term_kill: () => Promise.reject('terminal introuvable') });
    await openTerminal('p1', 'pwsh', 'pwsh-1');
    await expect(disposeTerminal('t1')).resolves.toBeUndefined();
  });

  it('asks nothing for a terminal this window does not hold', async () => {
    const backend = fakeBackend();
    await expect(disposeTerminal('t404')).resolves.toBeUndefined();
    expect(backend.called('term_kill')).toEqual([]);
  });
});

describe('the room of an element', () => {
  /** An element jsdom gives the size of a laid out one: `client` its width and height, `padding` its padding (CSS). */
  function laidOut(client: { width: number; height: number }, padding?: string) {
    const el = document.createElement('div');
    if (padding) el.style.padding = padding;
    Object.defineProperty(el, 'clientWidth', { value: client.width });
    Object.defineProperty(el, 'clientHeight', { value: client.height });
    return el;
  }

  it('is what the element holds, its padding left out', () => {
    expect(roomOf(laidOut({ width: 1140, height: 944 }, '8px 4px 4px 12px'))).toEqual({ width: 1124, height: 932 });
  });

  it('is the whole of an element without padding', () => {
    expect(roomOf(laidOut({ width: 800, height: 600 }))).toEqual({ width: 800, height: 600 });
  });

  it('is none for an element without width or height: not laid out yet, or hidden', () => {
    expect(roomOf(laidOut({ width: 0, height: 600 }))).toBeNull();
    expect(roomOf(laidOut({ width: 800, height: 0 }))).toBeNull();
    expect(roomOf(laidOut({ width: 0, height: 0 }))).toBeNull();
  });

  it('is none for an element its padding fills', () => {
    expect(roomOf(laidOut({ width: 16, height: 600 }, '8px 4px 4px 12px'))).toBeNull();
    expect(roomOf(laidOut({ width: 800, height: 12 }, '8px 4px 4px 12px'))).toBeNull();
    expect(roomOf(laidOut({ width: 10, height: 10 }, '8px 4px 4px 12px'))).toBeNull();
  });
});

describe('the size of the pseudo-console of Claude Code’s interface', () => {
  const info = { id: 'tui-1', projectId: 'p1', name: 'agent', shell: 'claude' };
  // The size of the place it is shown in; with a cell of 8 x 20 px, 140 columns and 46 rows.
  const ROOM = { width: 1124, height: 932 };

  /** The channels of the terminals spawned, which the backend writes the output to. */
  const outputs: { onmessage: (d: ArrayBuffer) => void }[] = [];
  /** The sizes the backend was told that have not been answered yet: each is answered by the test (`held`). */
  const resizes: { resolve: () => void; reject: (why: string) => void }[] = [];
  let held = false;

  beforeEach(() => {
    outputs.length = 0;
    resizes.length = 0;
    held = false;
    resetApp();
  });

  /** A backend that starts every terminal as `info`, and answers a size at once, or when the test says (`held`). */
  const backend = () =>
    fakeBackend({
      agent_terminal: ({ output }) => (outputs.push(output), info),
      term_spawn: ({ output }) => (outputs.push(output), info),
      account_login: ({ output }) => (outputs.push(output), info),
      term_resize: () => (held ? new Promise<void>((resolve, reject) => resizes.push({ resolve, reject })) : undefined),
    });
  const told = (b: ReturnType<typeof backend>) => b.called('term_resize').map((c) => c.args);
  /** The output the program writes: any chunk of it makes the terminal look at its size again. */
  const output = () => outputs.at(-1)!.onmessage(new ArrayBuffer(4));
  const settle = () => new Promise((r) => setTimeout(r, 20));

  it('is that of the room it will be shown in: the program starts at the size it will have, not at the parking lot’s', async () => {
    const b = backend();
    await openAgentTerminal('a1', ROOM);
    expect(b.called('agent_terminal')[0].args).toMatchObject({ id: 'a1', cols: 140, rows: 46 });
  });

  it('is the default one without a room, or with none known', async () => {
    const b = backend();
    await openAgentTerminal('a1');
    await openAgentTerminal('a2', null);
    expect(b.called('agent_terminal').map((c) => c.args)).toMatchObject([
      { id: 'a1', cols: 100, rows: 30 },
      { id: 'a2', cols: 100, rows: 30 },
    ]);
  });

  it('is told to no one by the start itself: the size the program started at is the size it has', async () => {
    const b = backend();
    await openAgentTerminal('a1', ROOM);
    await settle();
    expect(told(b)).toEqual([]);
  });

  it('is fitted on a host the size of the room, which then fills its box again', async () => {
    backend();
    await openAgentTerminal('a1', ROOM);
    // Fitted once, while the host had the room’s size in pixels…
    expect(fits).toEqual([{ width: '1124px', height: '932px' }]);
    // …and given back to the box it is shown in: whatever size that is from now on, it fills it.
    const { host } = getXTerm('tui-1')!;
    expect(host.style.width).toBe('100%');
    expect(host.style.height).toBe('100%');
    expect(host.style.cssText).not.toContain('px');
  });

  it('is fitted on a host that fills the parking lot when there is no room', async () => {
    backend();
    await openAgentTerminal('a1');
    expect(fits).toEqual([{ width: '100%', height: '100%' }]);
  });

  describe('is told to the backend, once it has started', () => {
    it('when xterm.js changes its size: with the id of the terminal and the new columns and rows', async () => {
      const b = backend();
      await openAgentTerminal('a1', ROOM);
      terms.at(-1)!.resize(120, 40);
      await waitFor(() => expect(told(b)).toEqual([{ id: 'tui-1', cols: 120, rows: 40 }]));
    });

    it('for each change, back to the size it started at included, and for none of the sizes it already told', async () => {
      const b = backend();
      await openAgentTerminal('a1', ROOM);
      const term = terms.at(-1)!;
      term.resize(120, 40);
      await waitFor(() => expect(told(b)).toHaveLength(1));
      await settle();
      term.resize(140, 46);
      await waitFor(() => expect(told(b)).toHaveLength(2));
      await settle();
      output();
      await settle();
      expect(told(b)).toEqual([
        { id: 'tui-1', cols: 120, rows: 40 },
        { id: 'tui-1', cols: 140, rows: 46 },
      ]);
    });

    it('not when the size xterm.js tells is the one the program has (a fit that changes nothing, a size told twice)', async () => {
      const b = backend();
      await openAgentTerminal('a1', ROOM);
      const term = terms.at(-1)!;
      // The host fills its box, as it does once shown: fitted again, nothing changes.
      getXTerm('tui-1')!.fit.fit();
      term.resize(140, 46);
      for (const handler of term.resizeHandlers) handler({ cols: 140, rows: 46 });
      output();
      await settle();
      expect(told(b)).toEqual([]);
    });

    it('one at a time and in order: the sizes that come while one is on its way wait for it, and only the last of them goes', async () => {
      held = true;
      const b = backend();
      await openAgentTerminal('a1', ROOM);
      const term = terms.at(-1)!;
      term.resize(120, 40);
      term.resize(110, 35);
      term.resize(90, 25);
      await settle();
      expect(told(b)).toEqual([{ id: 'tui-1', cols: 120, rows: 40 }]);
      // The first is taken: the next goes, with the size xterm.js has now, not with the one that came second.
      resizes[0].resolve();
      await waitFor(() => expect(told(b)).toHaveLength(2));
      expect(told(b)[1]).toEqual({ id: 'tui-1', cols: 90, rows: 25 });
      // That one is on its way, and nothing waits behind it.
      await settle();
      resizes[1].resolve();
      await settle();
      expect(told(b)).toHaveLength(2);
    });

    it('and the size it came back to while another was on its way is told, though it is the one it started at', async () => {
      held = true;
      const b = backend();
      await openAgentTerminal('a1', ROOM);
      const term = terms.at(-1)!;
      term.resize(120, 40);
      term.resize(140, 46);
      resizes[0].resolve();
      await waitFor(() => expect(told(b)).toHaveLength(2));
      expect(told(b)[1]).toEqual({ id: 'tui-1', cols: 140, rows: 46 });
    });

    it('but not the sizes xterm.js went through, once it is back to the one on its way', async () => {
      held = true;
      const b = backend();
      await openAgentTerminal('a1', ROOM);
      const term = terms.at(-1)!;
      term.resize(120, 40);
      term.resize(110, 35);
      term.resize(120, 40);
      resizes[0].resolve();
      await settle();
      expect(told(b)).toEqual([{ id: 'tui-1', cols: 120, rows: 40 }]);
    });

    describe('when the backend did not take it', () => {
      // The waits are those of the code (500 ms between two tries): the clock is the test's.
      beforeEach(() => vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] }));
      afterEach(() => vi.useRealTimers());

      it('again after a short while, and not before', async () => {
        held = true;
        const b = backend();
        await openAgentTerminal('a1', ROOM);
        terms.at(-1)!.resize(120, 40);
        expect(told(b)).toHaveLength(1);
        resizes[0].reject('terminal introuvable');
        await vi.advanceTimersByTimeAsync(499);
        expect(told(b)).toHaveLength(1);
        await vi.advanceTimersByTimeAsync(1);
        expect(told(b)).toHaveLength(2);
        expect(told(b)[1]).toEqual(told(b)[0]);
        // Taken this time: it is not told again.
        resizes[1].resolve();
        await vi.advanceTimersByTimeAsync(10_000);
        expect(told(b)).toHaveLength(2);
      });

      it('never two at once: the wait starts when the answer comes, not before', async () => {
        held = true;
        const b = backend();
        await openAgentTerminal('a1', ROOM);
        terms.at(-1)!.resize(120, 40);
        // No answer yet: nothing else goes, however long it takes.
        await vi.advanceTimersByTimeAsync(10_000);
        expect(told(b)).toHaveLength(1);
      });

      it('a few times at most, then no more until the size changes again (a terminal that is gone refuses for good)', async () => {
        held = true;
        const b = backend();
        await openAgentTerminal('a1', ROOM);
        terms.at(-1)!.resize(120, 40);
        // The first try, then five more.
        for (let i = 0; i < 6; i++) {
          expect(told(b)).toHaveLength(i + 1);
          resizes[i].reject('refusé');
          await vi.advanceTimersByTimeAsync(500);
        }
        await vi.advanceTimersByTimeAsync(60_000);
        expect(told(b)).toHaveLength(6);
        // A new size is told at once, as any: the backend may take it.
        terms.at(-1)!.resize(100, 31);
        expect(told(b)).toHaveLength(7);
        expect(told(b)[6]).toEqual({ id: 'tui-1', cols: 100, rows: 31 });
      });

      it('and the count starts again once it took one', async () => {
        held = true;
        const b = backend();
        await openAgentTerminal('a1', ROOM);
        const term = terms.at(-1)!;
        term.resize(120, 40);
        // Four refusals, then it takes it…
        for (let i = 0; i < 4; i++) {
          resizes[i].reject('refusé');
          await vi.advanceTimersByTimeAsync(500);
        }
        resizes[4].resolve();
        await vi.advanceTimersByTimeAsync(0);
        expect(told(b)).toHaveLength(5);
        // …so the next size has its tries again: six sends in all, instead of the two that were left.
        term.resize(100, 31);
        for (let i = 5; i < 11; i++) {
          expect(told(b)).toHaveLength(i + 1);
          resizes[i].reject('refusé');
          await vi.advanceTimersByTimeAsync(500);
        }
        await vi.advanceTimersByTimeAsync(60_000);
        expect(told(b)).toHaveLength(11);
      });
    });

    it('again at the next change of size, with the size it has then, when the backend did not take the one before', async () => {
      held = true;
      const b = backend();
      await openAgentTerminal('a1', ROOM);
      const term = terms.at(-1)!;
      term.resize(120, 40);
      resizes[0].reject('refusé');
      await settle();
      term.resize(100, 31);
      await waitFor(() => expect(told(b)).toHaveLength(2));
      expect(told(b)[1]).toEqual({ id: 'tui-1', cols: 100, rows: 31 });
    });

    it('a size xterm.js took while the program was starting, as soon as it has started', async () => {
      let started: () => void = () => {};
      const b = fakeBackend({
        agent_terminal: ({ output }) => {
          outputs.push(output);
          return new Promise((resolve) => (started = () => resolve(info)));
        },
        term_resize: () => undefined,
      });
      const opening = openAgentTerminal('a1', ROOM);
      await waitFor(() => expect(b.called('agent_terminal')).toHaveLength(1));
      expect(b.called('agent_terminal')[0].args).toMatchObject({ cols: 140, rows: 46 });
      // The window changed under it before the backend answered: there is no id to tell it with yet.
      terms.at(-1)!.resize(120, 40);
      await settle();
      expect(told(b)).toEqual([]);
      started();
      await opening;
      await waitFor(() => expect(told(b)).toEqual([{ id: 'tui-1', cols: 120, rows: 40 }]));
    });
  });

  describe('of the other terminals', () => {
    it.each([
      ['a shell’s terminal', () => openTerminal('p1', 'pwsh', 'pwsh-1'), 'term_spawn'],
      ['an account’s sign-in', () => openAccountLogin('principal'), 'account_login'],
    ])('is not changed for %s: it starts at the default size, fitted to the parking lot', async (_name, open, command) => {
      const b = backend();
      await open();
      expect(b.called(command)[0].args).toMatchObject({ cols: 100, rows: 30 });
      expect(fits).toEqual([{ width: '100%', height: '100%' }]);
      await settle();
      expect(told(b)).toEqual([]);
    });

    it.each([
      ['a shell’s terminal', () => openTerminal('p1', 'pwsh', 'pwsh-1')],
      ['an account’s sign-in', () => openAccountLogin('principal')],
    ])('keeps for %s the size it tells the backend: one at a time, and again after a short while when refused', async (_name, open) => {
      vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
      try {
        held = true;
        const b = backend();
        await open();
        const term = terms.at(-1)!;
        term.resize(120, 40);
        term.resize(90, 25);
        expect(told(b)).toEqual([{ id: 'tui-1', cols: 120, rows: 40 }]);
        resizes[0].reject('refusé');
        await vi.advanceTimersByTimeAsync(500);
        expect(told(b)).toHaveLength(2);
        // The size xterm.js has by then, not the one that was refused.
        expect(told(b)[1]).toEqual({ id: 'tui-1', cols: 90, rows: 25 });
      } finally {
        vi.useRealTimers();
      }
    });
  });
});

describe('what a terminal tells the log', () => {
  const info = { id: 't1', projectId: 'p1', name: 'pwsh-1', shell: 'pwsh' };
  const GPU = 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4080 Direct3D11 vs_5_0 ps_5_0, D3D11)';
  const DEBUG_INFO = { UNMASKED_RENDERER_WEBGL: 37446 };

  // The clock the terminal's times are read from, and the animation frames waiting for it to move on.
  const clock = { t: 5_000 };
  const frames: (() => void)[] = [];
  const at = (t: number) => performance.timeOrigin + t;
  /** The next animation frame, drawn at `t`. */
  const draw = (t: number) => {
    clock.t = t;
    frames.shift()!();
  };

  // The throwaway WebGL context the GPU's name is read from, which must be given back at once.
  const lose = vi.fn();
  const webgl = (o: { debugInfo?: boolean } = {}) => ({
    getExtension: (name: string) =>
      name === 'WEBGL_debug_renderer_info'
        ? o.debugInfo === false
          ? null
          : DEBUG_INFO
        : name === 'WEBGL_lose_context'
          ? { loseContext: lose }
          : null,
    getParameter: (what: number) => (what === DEBUG_INFO.UNMASKED_RENDERER_WEBGL ? GPU : 'WebKit WebGL'),
    RENDERER: 7937,
  });
  let probe: (kind: string) => unknown;
  /** The contexts asked of a canvas, but the 2D one of the colors: the throwaway one `probe` gives. */
  const getContext = vi.fn();

  /** A fresh copy of the module: the GPU's name is read once per run of the app. */
  async function load() {
    vi.resetModules();
    return await import('./terminals');
  }

  /** The channels of the terminals spawned, which the backend writes the output to. */
  const outputs: { onmessage: (d: ArrayBuffer) => void }[] = [];
  /** The number of the last write the backend numbered. */
  let written = 0;
  const backend = (spawned: () => unknown = () => info) =>
    fakeBackend({
      term_spawn: ({ output }) => {
        outputs.push(output);
        return spawned();
      },
      agent_terminal: ({ output }) => {
        outputs.push(output);
        return spawned();
      },
      term_write: () => ++written,
      term_report: () => undefined,
    });
  const reports = (b: ReturnType<typeof backend>) => b.called('term_report').map((c) => ({ id: c.args.id, ...c.args.report }));

  beforeEach(() => {
    clock.t = 5_000;
    frames.length = 0;
    outputs.length = 0;
    written = 0;
    lose.mockClear();
    probe = (kind) => (kind === 'webgl2' ? webgl() : null);
    getContext.mockReset().mockImplementation((kind: string) => probe(kind));
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(((kind: string) =>
      kind === '2d' ? { fillRect() {}, getImageData: () => ({ data: [0, 0, 0] }) } : getContext(kind)) as never);
    vi.spyOn(performance, 'now').mockImplementation(() => clock.t);
    vi.stubGlobal('requestAnimationFrame', (frame: () => void) => frames.push(frame));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  describe('a slow echo', () => {
    const encode = new TextEncoder();
    /** The channel of the last terminal receives `text` (four zero bytes: none). */
    const send = (text?: string) => outputs.at(-1)!.onmessage(text === undefined ? new ArrayBuffer(4) : encode.encode(text).slice().buffer);

    /** A terminal opened on a fresh copy of the module, with the host xterm.js listens to pastes on. */
    async function terminal(open: (m: typeof import('./terminals')) => Promise<unknown> = (m) => m.openTerminal('p1', 'pwsh', 'pwsh-1')) {
      const m = await load();
      await open(m);
      const term = terms.at(-1)!;
      return { term, host: m.getXTerm('t1')!.host };
    }
    type Opened = Awaited<ReturnType<typeof terminal>>;

    /** The user presses `key`: the terminal's key handler gets it, and xterm.js then emits `data` (as the key types). */
    const press = ({ term }: Opened, key: string, data = key, init: KeyboardEventInit = {}) => {
      const toTerminal = keyHandlers.at(-1)!(new KeyboardEvent('keydown', { key, ...init }));
      if (toTerminal) term.dataHandlers[0](data);
    };
    /** The user pastes `text`: the WebView's paste reaches the terminal's host, then xterm.js emits what it pastes. */
    const paste = ({ term, host }: Opened, text: string, data = text) => {
      const field = document.createElement('textarea');
      host.appendChild(field);
      fireEvent(field, createEvent.paste(field, { clipboardData: clipboard({ text }) }));
      term.dataHandlers[0](data);
    };
    /** xterm.js parsed the last chunk it was given: it is drawn at the next animation frame, at `t`. */
    const paint = ({ term }: Opened, t: number) => {
      term.writes.at(-1)!.done?.();
      draw(t);
    };
    const slowEchoes = (b: ReturnType<typeof backend>) => reports(b).filter((r) => r.kind === 'slowEcho');

    it('is reported with the terminal’s id, the number of the write, its size in bytes and both durations', async () => {
      const b = backend();
      const t = await terminal();
      // Numbered 1, and no key to measure.
      press(t, 'Enter', '\r');
      clock.t = 10_000;
      press(t, 'é');
      clock.t = 10_400;
      send('é');
      paint(t, 10_420);
      await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
      expect(slowEchoes(b)[0]).toEqual({
        id: 't1',
        kind: 'slowEcho',
        seq: 2,
        bytes: 2,
        sentToReceived: 400,
        receivedToPainted: 20,
        sentToAcked: expect.any(Number),
        outputEnd: 2,
      });
      expect(b.called('term_write').map((c) => c.args)).toEqual([
        { id: 't1', data: '\r' },
        { id: 't1', data: 'é' },
      ]);
    });

    it('is not reported when the echo came in time', async () => {
      const b = backend();
      const t = await terminal();
      clock.t = 10_000;
      press(t, 'a');
      clock.t = 10_020;
      send('a');
      paint(t, 10_035);
      await new Promise((r) => setTimeout(r, 20));
      expect(slowEchoes(b)).toEqual([]);
    });

    it('is drawn on the frame after xterm.js parsed the output, which is when its write calls back', async () => {
      backend();
      const t = await terminal();
      press(t, 'a');
      send('a');
      // Not parsed yet: no frame is waited for.
      expect(frames).toHaveLength(0);
      t.term.writes[0].done!();
      expect(frames).toHaveLength(1);
      expect([...t.term.writes[0].data]).toEqual([...encode.encode('a')]);
    });

    it('asks for no callback of the output nobody is waiting for, nor of the ones that are no echo', async () => {
      const b = backend();
      const t = await terminal();
      // The shell’s own output: no input waits for it.
      send('PS C:\\>');
      press(t, 'x');
      // A spinner redrawn, then the letter.
      send('\x1b[2K⠋ Thinking…');
      send('> x');
      send('> x again');
      expect(t.term.writes.map((w) => typeof w.done)).toEqual(['undefined', 'undefined', 'function', 'undefined']);
      expect(slowEchoes(b)).toEqual([]);
    });

    it('is reported by the terminal of Claude Code’s interface too', async () => {
      const b = backend();
      const t = await terminal((m) => m.openAgentTerminal('a1'));
      clock.t = 1_000;
      press(t, 'a');
      clock.t = 2_000;
      send('a');
      paint(t, 2_050);
      await waitFor(() => expect(slowEchoes(b)[0]).toMatchObject({ id: 't1', kind: 'slowEcho', seq: 1, bytes: 1 }));
    });

    describe('tells where its chunk stands in the output', () => {
      const size = (...texts: string[]) => texts.reduce((n, text) => n + encode.encode(text).length, 0);
      const SPINNER = '\x1b[2K⠋ Thinking…';

      it('by the bytes the terminal had received once it came in: the chunks before and between count, the ones after do not', async () => {
        const b = backend();
        const t = await terminal();
        const prompt = 'PS C:\\> ';
        const echo = '\x1b[2K> é';
        // Before the key, then between the key and its echo (the spinner is no echo): none of them is measured, all count.
        send(prompt);
        clock.t = 10_000;
        press(t, 'é');
        clock.t = 10_100;
        send(SPINNER);
        send();
        clock.t = 10_500;
        send(echo);
        // After the echo, before it is painted.
        send('> é again');
        send(SPINNER);
        // xterm.js parses the chunk of the echo, the only one asked to call back, and the next frame draws it.
        t.term.writes.find((w) => w.done)!.done!();
        draw(10_520);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        // The four bytes of `send()` are a chunk like any other.
        expect(slowEchoes(b)[0]).toMatchObject({ seq: 1, sentToReceived: 500, outputEnd: size(prompt, SPINNER, echo) + 4 });
      });

      it('by the bytes of every message of the channel, whatever the text they hold', async () => {
        const b = backend();
        const t = await terminal();
        // 6 bytes of 2 characters: the count is of bytes, not of characters.
        send('🙂é');
        clock.t = 10_000;
        press(t, 'a');
        clock.t = 10_400;
        send('a');
        paint(t, 10_410);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toMatchObject({ outputEnd: 7 });
      });

      it('of the output of this terminal only', async () => {
        const b = backend();
        const m = await load();
        await m.openTerminal('p1', 'pwsh', 'pwsh-1');
        // A long output on the first one.
        outputs[0].onmessage(new ArrayBuffer(1000));
        await m.openTerminal('p1', 'pwsh', 'pwsh-2');
        const t = { term: terms.at(-1)!, host: m.getXTerm('t1')!.host };
        clock.t = 10_000;
        press(t, 'a');
        clock.t = 10_400;
        outputs[1].onmessage(encode.encode('a').slice().buffer);
        paint(t, 10_410);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toMatchObject({ outputEnd: 1 });
      });

      it('for the terminal of Claude Code’s interface too', async () => {
        const b = backend();
        const t = await terminal((m) => m.openAgentTerminal('a1'));
        send(SPINNER);
        clock.t = 10_000;
        press(t, 'a');
        clock.t = 10_400;
        send('a');
        paint(t, 10_410);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toMatchObject({ outputEnd: size(SPINNER, 'a') });
      });
    });

    describe('tells how long the write took to be answered', () => {
      const answering = () => {
        const writes: ((seq: number) => void)[] = [];
        const b = fakeBackend({
          term_spawn: ({ output }) => {
            outputs.push(output);
            return info;
          },
          term_write: () => new Promise<number>((r) => writes.push(r)),
          term_report: () => undefined,
        });
        return { b, writes };
      };
      /** Lets the answer of a write reach the terminal's code, with the clock where it is. */
      const settle = () => new Promise((r) => setTimeout(r, 20));

      it('from the call of term_write to the resolution of its promise, apart from the way to the echo', async () => {
        const { b, writes } = answering();
        const t = await terminal();
        clock.t = 10_000;
        press(t, 'a');
        clock.t = 10_150;
        writes[0](7);
        await settle();
        clock.t = 10_400;
        send('a');
        paint(t, 10_420);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toEqual({
          id: 't1',
          kind: 'slowEcho',
          seq: 7,
          bytes: 1,
          sentToReceived: 400,
          receivedToPainted: 20,
          sentToAcked: 150,
          outputEnd: 1,
        });
      });

      it('even when the backend answers after the echo was drawn: the report waits for it', async () => {
        const { b, writes } = answering();
        const t = await terminal();
        clock.t = 10_000;
        press(t, 'a');
        clock.t = 10_400;
        send('a');
        paint(t, 10_420);
        await settle();
        expect(slowEchoes(b)).toEqual([]);
        clock.t = 10_900;
        writes[0](3);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toMatchObject({ seq: 3, sentToReceived: 400, receivedToPainted: 20, sentToAcked: 900 });
      });
    });

    describe('is measured only for what the user typed or pasted', () => {
      const SPINNER = '\x1b[2K⠋ Thinking…';

      it('and not for the reports xterm.js writes by itself: focus, cursor, device attributes, mouse, OSC', async () => {
        const b = backend();
        const t = await terminal();
        for (const auto of ['\x1b[I', '\x1b[O', '\x1b[12;40R', '\x1b[?1;2c', '\x1b[<0;10;5M', '\x1b]11;rgb:1b1b/1515/1212\x1b\\']) {
          clock.t += 100;
          t.term.dataHandlers[0](auto);
          clock.t += 2_000;
          send(SPINNER);
          // Nothing waits for it.
          expect(t.term.writes.at(-1)!.done).toBeUndefined();
        }
        expect(frames).toHaveLength(0);
        await new Promise((r) => setTimeout(r, 20));
        expect(slowEchoes(b)).toEqual([]);
        // They went out all the same.
        expect(b.called('term_write')).toHaveLength(6);
      });

      it('and not for a character xterm.js wrote long after the last key (more than 50 ms)', async () => {
        const b = backend();
        const t = await terminal();
        clock.t = 10_000;
        keyHandlers.at(-1)!(new KeyboardEvent('keydown', { key: 'Shift' }));
        clock.t = 10_100;
        t.term.dataHandlers[0]('a');
        clock.t = 12_000;
        send('a');
        expect(t.term.writes.at(-1)!.done).toBeUndefined();
        expect(slowEchoes(b)).toEqual([]);
      });

      it('and for a character xterm.js emits right after the key (within 50 ms)', async () => {
        const b = backend();
        const t = await terminal();
        clock.t = 10_000;
        keyHandlers.at(-1)!(new KeyboardEvent('keydown', { key: 'a' }));
        clock.t = 10_040;
        t.term.dataHandlers[0]('a');
        clock.t = 10_600;
        send('a');
        paint(t, 10_610);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
      });

      it('and not for a key the terminal hands over to the app (it does not go to the shell)', async () => {
        const b = backend();
        const t = await terminal();
        // Ctrl+V pastes: the keydown is not the terminal's, whatever comes within 50 ms.
        expect(keyHandlers.at(-1)!(new KeyboardEvent('keydown', { key: 'v', ctrlKey: true }))).toBe(false);
        t.term.dataHandlers[0]('v');
        clock.t += 2_000;
        send('v');
        expect(t.term.writes.at(-1)!.done).toBeUndefined();
        expect(slowEchoes(b)).toEqual([]);
      });

      it('and not for a key that prints nothing: an arrow, Enter, Tab, Backspace, Ctrl+C', async () => {
        const b = backend();
        const t = await terminal();
        for (const [key, data] of [
          ['ArrowUp', '\x1b[A'],
          ['Enter', '\r'],
          ['Tab', '\t'],
          ['Backspace', '\x7f'],
          ['c', '\x03'],
        ]) {
          clock.t += 100;
          press(t, key, data, key === 'c' ? { ctrlKey: true } : {});
          clock.t += 2_000;
          send(SPINNER);
          expect(t.term.writes.at(-1)!.done).toBeUndefined();
        }
        expect(slowEchoes(b)).toEqual([]);
      });

      it('and not for a letter that is never echoed (a password): the output that follows is unrelated', async () => {
        const b = backend();
        const t = await terminal();
        clock.t = 10_000;
        press(t, 'p');
        clock.t = 12_000;
        send(SPINNER);
        clock.t = 14_000;
        send('\x1b[2K⠙ Thinking…');
        expect(t.term.writes.map((w) => w.done)).toEqual([undefined, undefined]);
        expect(frames).toHaveLength(0);
        await new Promise((r) => setTimeout(r, 20));
        expect(slowEchoes(b)).toEqual([]);
      });

      it('and the letter is looked for in the text, not in the escape sequences around it', async () => {
        const b = backend();
        const t = await terminal();
        clock.t = 10_000;
        press(t, 'm');
        clock.t = 10_300;
        send('\x1b[0m\x1b[1;31m');
        expect(t.term.writes.at(-1)!.done).toBeUndefined();
        clock.t = 10_400;
        send('\x1b[0mm');
        paint(t, 10_410);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toMatchObject({ sentToReceived: 400, receivedToPainted: 10 });
      });

      it('is told for a letter the output redrawn around does not hide: the spinner does not answer, the letter does', async () => {
        const b = backend();
        const t = await terminal();
        clock.t = 10_000;
        press(t, 'x');
        clock.t = 10_100;
        send(SPINNER);
        clock.t = 10_200;
        send('\x1b[2K⠙ Thinking…');
        clock.t = 10_500;
        send('\x1b[2K⠹ Thinking…\r\n> x');
        paint(t, 10_540);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toMatchObject({ seq: 1, bytes: 1, sentToReceived: 500, receivedToPainted: 40 });
      });
    });

    describe('a paste', () => {
      it('is answered by the first output after it, and reported when late', async () => {
        const b = backend();
        const t = await terminal();
        clock.t = 10_000;
        // Bracketed, as Claude Code asks: 12 bytes of markers around the 6 of the text.
        paste(t, 'é🙂', '\x1b[200~é🙂\x1b[201~');
        clock.t = 10_500;
        send('\x1b[2K⠋ Thinking…');
        paint(t, 10_520);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toEqual({
          id: 't1',
          kind: 'slowEcho',
          seq: 1,
          bytes: 18,
          sentToReceived: 500,
          receivedToPainted: 20,
          sentToAcked: expect.any(Number),
          outputEnd: encode.encode('\x1b[2K⠋ Thinking…').length,
        });
      });

      it('is the one of Claude Code’s interface, too', async () => {
        const b = backend();
        const t = await terminal((m) => m.openAgentTerminal('a1'));
        clock.t = 10_000;
        paste(t, 'npm test');
        clock.t = 10_500;
        send('npm test');
        paint(t, 10_510);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
      });
    });

    describe('with the window out of sight', () => {
      const show = (state: DocumentVisibilityState) =>
        Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => state });
      afterEach(() => {
        delete (document as { visibilityState?: unknown }).visibilityState;
      });

      it('measures nothing typed while it is hidden', async () => {
        const b = backend();
        const t = await terminal();
        show('hidden');
        clock.t = 10_000;
        press(t, 'a');
        show('visible');
        clock.t = 12_000;
        send('a');
        expect(t.term.writes.at(-1)!.done).toBeUndefined();
        expect(slowEchoes(b)).toEqual([]);
      });

      it('reports nothing of an echo drawn while it is hidden: the frame comes when it shows again', async () => {
        const b = backend();
        const t = await terminal();
        clock.t = 10_000;
        press(t, 'a');
        clock.t = 10_050;
        send('a');
        show('hidden');
        paint(t, 12_000);
        await new Promise((r) => setTimeout(r, 20));
        expect(slowEchoes(b)).toEqual([]);
      });

      it('goes on measuring once it shows again', async () => {
        const b = backend();
        const t = await terminal();
        show('hidden');
        press(t, 'a');
        show('visible');
        clock.t = 10_000;
        press(t, 'b');
        clock.t = 10_500;
        send('b');
        paint(t, 10_510);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toMatchObject({ seq: 2 });
      });
    });

    describe('with writes and reports that fail', () => {
      it('does not stop the writes: they go to the backend as before, the report failing or not', async () => {
        const b = fakeBackend({
          term_spawn: ({ output }) => {
            outputs.push(output);
            return info;
          },
          term_write: () => 1,
          term_report: () => Promise.reject('refusé'),
        });
        const t = await terminal();
        clock.t = 10_000;
        press(t, 'l');
        clock.t = 11_000;
        send('l');
        paint(t, 11_010);
        press(t, 'x');
        expect(b.called('term_write').map((c) => c.args)).toEqual([
          { id: 't1', data: 'l' },
          { id: 't1', data: 'x' },
        ]);
      });

      it('reports nothing of a write the backend refused, and the next writes still go out', async () => {
        let refuse = true;
        const b = fakeBackend({
          term_spawn: ({ output }) => {
            outputs.push(output);
            return info;
          },
          term_write: () => (refuse ? Promise.reject('terminal introuvable') : 2),
          term_report: () => undefined,
        });
        const t = await terminal();
        clock.t = 10_000;
        press(t, 'a');
        clock.t = 11_000;
        send('a');
        paint(t, 11_010);
        await new Promise((r) => setTimeout(r, 20));
        expect(slowEchoes(b)).toEqual([]);

        refuse = false;
        clock.t = 30_000;
        press(t, 'b');
        press(t, 'c');
        expect(b.called('term_write').map((c) => c.args)).toEqual([
          { id: 't1', data: 'a' },
          { id: 't1', data: 'b' },
          { id: 't1', data: 'c' },
        ]);
        clock.t = 30_500;
        send('bc');
        paint(t, 30_510);
        await waitFor(() => expect(slowEchoes(b)).toHaveLength(1));
        expect(slowEchoes(b)[0]).toMatchObject({ seq: 2 });
      });
    });
  });

  describe('its renderer', () => {
    it('is reported once the terminal has its id: WebGL with the name of the GPU', async () => {
      let opened!: (i: typeof info) => void;
      const b = backend(() => new Promise((r) => (opened = r)));
      const { openTerminal } = await load();
      const pending = openTerminal('p1', 'pwsh', 'pwsh-1');
      await new Promise((r) => setTimeout(r, 20));
      expect(b.called('term_report')).toEqual([]);
      opened(info);
      await pending;
      await waitFor(() => expect(reports(b)).toEqual([{ id: 't1', kind: 'renderer', renderer: 'webgl', detail: GPU }]));
    });

    it('gives back the throwaway WebGL context the GPU’s name was read from, so it does not count among the 16 of the WebView', async () => {
      backend();
      const { openTerminal } = await load();
      await openTerminal('p1', 'pwsh', 'pwsh-1');
      expect(getContext).toHaveBeenCalledWith('webgl2');
      expect(lose).toHaveBeenCalledOnce();
    });

    it('reads the GPU’s name once per run of the app, not at each terminal', async () => {
      const b = backend(() => info);
      const { openTerminal } = await load();
      await openTerminal('p1', 'pwsh', 'pwsh-1');
      await openTerminal('p1', 'pwsh', 'pwsh-2');
      expect(getContext).toHaveBeenCalledTimes(1);
      expect(lose).toHaveBeenCalledOnce();
      await waitFor(() => expect(reports(b)).toHaveLength(2));
      expect(reports(b).map((r) => r.detail)).toEqual([GPU, GPU]);
    });

    it('says a software renderer for what it is', async () => {
      const software = 'ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)';
      probe = () => ({ ...webgl(), getParameter: () => software });
      const b = backend();
      const { openTerminal } = await load();
      await openTerminal('p1', 'pwsh', 'pwsh-1');
      await waitFor(() => expect(reports(b)).toEqual([{ id: 't1', kind: 'renderer', renderer: 'webgl', detail: software }]));
    });

    it('falls back to the WebView’s own name of the renderer when it hides the GPU’s', async () => {
      probe = () => webgl({ debugInfo: false });
      const b = backend();
      const { openTerminal } = await load();
      await openTerminal('p1', 'pwsh', 'pwsh-1');
      await waitFor(() => expect(reports(b)).toEqual([{ id: 't1', kind: 'renderer', renderer: 'webgl', detail: 'WebKit WebGL' }]));
    });

    it('is WebGL without a name when none could be read', async () => {
      probe = () => null;
      const b = backend();
      const { openTerminal } = await load();
      await openTerminal('p1', 'pwsh', 'pwsh-1');
      await waitFor(() => expect(reports(b)).toEqual([{ id: 't1', kind: 'renderer', renderer: 'webgl' }]));
    });

    it('is the DOM renderer, and why, when WebGL does not load', async () => {
      gl.fail = 'WebGL2 not supported null';
      const b = backend();
      const { openTerminal } = await load();
      await openTerminal('p1', 'pwsh', 'pwsh-1');
      await waitFor(() =>
        expect(reports(b)).toEqual([{ id: 't1', kind: 'renderer', renderer: 'dom', detail: 'WebGL2 not supported null' }]),
      );
      // No context was made to read the name of a GPU nobody uses.
      expect(getContext).not.toHaveBeenCalled();
    });

    it('is told again when the WebGL context is lost: the terminal then falls back to the DOM renderer', async () => {
      const b = backend();
      const { openTerminal } = await load();
      await openTerminal('p1', 'pwsh', 'pwsh-1');
      await waitFor(() => expect(reports(b)).toHaveLength(1));
      gl.losses[0]();
      expect(gl.disposed).toBe(1);
      await waitFor(() => expect(reports(b)).toHaveLength(2));
      expect(reports(b).at(-1)).toEqual({ id: 't1', kind: 'renderer', renderer: 'dom', detail: 'WebGL context lost' });
    });

    it('tells a context lost before the terminal had its id once it has', async () => {
      let opened!: (i: typeof info) => void;
      const b = backend(() => new Promise((r) => (opened = r)));
      const { openTerminal } = await load();
      const pending = openTerminal('p1', 'pwsh', 'pwsh-1');
      await new Promise((r) => setTimeout(r, 20));
      gl.losses[0]();
      expect(b.called('term_report')).toEqual([]);
      opened(info);
      await pending;
      await waitFor(() => expect(reports(b)).toEqual([{ id: 't1', kind: 'renderer', renderer: 'dom', detail: 'WebGL context lost' }]));
    });

    it('tells nothing of the terminal that never started', async () => {
      const b = backend(() => Promise.reject('shell introuvable'));
      const { openTerminal } = await load();
      await expect(openTerminal('p1', 'pwsh', 'pwsh-1')).rejects.toBe('shell introuvable');
      gl.losses[0]();
      await new Promise((r) => setTimeout(r, 20));
      expect(b.called('term_report')).toEqual([]);
    });
  });

  describe('a launch command’s log', () => {
    it('reports nothing, neither its renderer, nor a lost context', async () => {
      const b = backend();
      const { launchLog } = await load();
      const x = launchLog('c1', () => {});
      expect(x.id).toBeUndefined();
      gl.losses[0]();
      await new Promise((r) => setTimeout(r, 20));
      expect(b.called('term_report')).toEqual([]);
    });
  });
});

describe('a launch command’s log and the keyboard (ESC-396)', () => {
  let now = 1_000;
  beforeEach(() => {
    resetApp();
    vi.spyOn(performance, 'now').mockImplementation(() => now);
  });
  afterEach(() => {
    vi.restoreAllMocks();
    disposeLog('kb');
  });

  const press = (init: KeyboardEventInit) => keyHandlers.at(-1)!(new KeyboardEvent('keydown', init));

  it('sends what the user types and pastes to its run while it lasts, and nothing once it ended', () => {
    const b = fakeBackend({ term_write: () => 1 });
    let pty: string | null = 'pty-1';
    const x = launchLog(
      'kb',
      () => {},
      () => pty,
    );
    const term = terms.at(-1)!;
    // Made read-only: a key goes to the app, not to the log.
    expect(press({ key: 'a' })).toBe(false);
    setLogLive('kb', true);
    expect(x.term.options).toMatchObject({ disableStdin: false, cursorBlink: true });
    // An answer to a prompt, then Ctrl+C with nothing selected: both go to the program, as in a terminal.
    expect(press({ key: 'y' })).toBe(true);
    term.dataHandlers[0]('y');
    expect(press({ key: 'Enter' })).toBe(true);
    term.dataHandlers[0]('\r');
    expect(press({ key: 'c', ctrlKey: true })).toBe(true);
    term.dataHandlers[0]('\x03');
    // A paste of several lines goes whole.
    x.host.dispatchEvent(new Event('paste'));
    term.dataHandlers[0]('un\rdeux');
    // What xterm.js answers by itself (a cursor report), no key nor paste just before: the backend answers those.
    now += 1_000;
    term.dataHandlers[0]('\x1b[5;1R');
    // An app shortcut still goes to the app.
    expect(press({ key: ',', ctrlKey: true })).toBe(false);
    expect(b.called('term_write').map((c) => c.args)).toEqual([
      { id: 'pty-1', data: 'y' },
      { id: 'pty-1', data: '\r' },
      { id: 'pty-1', data: '\x03' },
      { id: 'pty-1', data: 'un\rdeux' },
    ]);
    // Its run ended: a log again, which takes nothing.
    pty = null;
    setLogLive('kb', false);
    expect(x.term.options).toMatchObject({ disableStdin: true, cursorBlink: false });
    expect(press({ key: 'a' })).toBe(false);
    x.host.dispatchEvent(new Event('paste'));
    term.dataHandlers[0]('encore');
    expect(b.called('term_write')).toHaveLength(4);
    // Started again: it takes the keyboard again, for its new run.
    pty = 'pty-2';
    setLogLive('kb', true);
    now += 1_000;
    expect(press({ key: 'q' })).toBe(true);
    term.dataHandlers[0]('q');
    expect(b.called('term_write').at(-1)!.args).toEqual({ id: 'pty-2', data: 'q' });
  });
});

describe('« Taille du texte des terminaux »', () => {
  beforeEach(() => resetApp());

  it('is the size a new terminal is made with, 12.5 px by default', () => {
    launchLog('size-default', () => {});
    expect(termOptions.at(-1)).toMatchObject({ fontSize: 12.5 });
    app.settings.terminalFontSize = 15;
    launchLog('size-set', () => {});
    expect(termOptions.at(-1)).toMatchObject({ fontSize: 15 });
    disposeLog('size-default');
    disposeLog('size-set');
  });

  it('changes every terminal made, and fits again only those on screen: a parked one is fitted once shown', () => {
    const shown = launchLog('size-shown', () => {});
    const parked = launchLog('size-parked', () => {});
    const box = document.createElement('div');
    document.body.appendChild(box);
    box.appendChild(shown.host);
    fits.length = 0;
    setTerminalFontSize(18);
    expect([shown.term.options.fontSize, parked.term.options.fontSize]).toEqual([18, 18]);
    expect(fits).toHaveLength(1);
    // The same size again changes nothing.
    setTerminalFontSize(18);
    expect(fits).toHaveLength(1);
    disposeLog('size-shown');
    disposeLog('size-parked');
    box.remove();
  });
});

describe('the theme of the terminals', () => {
  afterEach(() => {
    delete document.documentElement.dataset.theme;
  });

  it('draws its 16 colours with tokens of the app that have a value in both themes', async () => {
    const { readFileSync } = await import('node:fs');
    const { tokensOf } = await import('../test/color');
    const css = readFileSync('src/app.css', 'utf8');
    const dark = tokensOf(css, ':root');
    const light = { ...dark, ...tokensOf(css, ":root[data-theme='light']") };
    const tokens = Object.values(ANSI_TOKENS);
    expect(Object.keys(ANSI_TOKENS)).toHaveLength(16);
    expect(tokens.filter((t) => !(t in dark) || !(t in light))).toEqual([]);
  });

  it('follows the theme at once, without opening the terminal again, and keeps colours readable in the light one', async () => {
    const x = launchLog('theme-hot', () => {});
    const before = x.term.options.theme;
    expect(termOptions.at(-1)).toMatchObject({ minimumContrastRatio: 1 });
    document.documentElement.dataset.theme = 'light';
    await new Promise((r) => setTimeout(r, 0));
    expect(x.term.options.theme).not.toBe(before);
    expect(x.term.options.minimumContrastRatio).toBe(4.5);
    document.documentElement.dataset.theme = 'dark';
    await new Promise((r) => setTimeout(r, 0));
    expect(x.term.options.minimumContrastRatio).toBe(1);
    disposeLog('theme-hot');
  });
});
