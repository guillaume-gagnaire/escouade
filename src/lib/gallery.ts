import { api } from './ipc';
import { openUrl } from './opener';

/** Where an image the viewer shows comes from: what « Ouvrir » and « Afficher dans le dossier » act on. */
export type ImageSource =
  | { kind: 'sent'; agent: string; item: string; index: number }
  | { kind: 'local'; agent: string; src: string }
  | { kind: 'remote'; src: string };

export interface ViewedImage {
  name: string;
  url: string;
  source: ImageSource | null;
}

/**
 * The images to page through from the one clicked: every image marked `data-view` of the conversation it is in
 * (`data-gallery`, one per pane), in their order; out of a conversation, those of its message or of its card.
 */
// ponytail: only the images drawn (the last 80 items of a conversation, more once « Afficher les précédents »); read
// the conversation's items instead if the older ones are missed.
export function galleryOf(img: HTMLImageElement): { images: ViewedImage[]; index: number } {
  const root = img.closest('[data-gallery]') ?? img.closest('.md, .sent');
  const all = [...(root?.querySelectorAll<HTMLImageElement>('img[data-view]') ?? [])];
  const index = all.indexOf(img);
  return index < 0 ? { images: [viewed(img)], index: 0 } : { images: all.map(viewed), index };
}

/** An image as the viewer shows it: its name, its `data:` URL, its source by the marks its opener left on it. */
function viewed(img: HTMLImageElement): ViewedImage {
  const d = img.dataset;
  const source: ImageSource | null =
    d.item && d.agent
      ? { kind: 'sent', agent: d.agent, item: d.item, index: Number(d.index) }
      : d.localSrc && d.agent
        ? { kind: 'local', agent: d.agent, src: d.localSrc }
        : d.remoteSrc
          ? { kind: 'remote', src: d.remoteSrc }
          : null;
  const path = (d.localSrc ?? d.remoteSrc ?? '').replace(/[?#].*$/, '');
  return { name: d.name || img.alt || path.split(/[\\/]/).pop() || '', url: img.src, source };
}

/** The image's file opened with its application or shown in its folder (`reveal`); one of the web, in the browser. */
export function openSource(s: ImageSource, reveal: boolean): Promise<void> {
  if (s.kind === 'sent') return api.sentFileOpen(s.agent, s.item, s.index, reveal);
  if (s.kind === 'local') return api.agentLocalOpen(s.agent, s.src, reveal);
  return openUrl(s.src);
}

/** The image shown put on the clipboard as PNG, the format every system pastes: drawn on a canvas, a GIF by its first frame. */
// ponytail: WebKitGTK may refuse an image to `navigator.clipboard.write`; write it by GTK (clipboard.rs) if Linux misses it.
export function copyImage(img: HTMLImageElement): Promise<void> {
  const png = new Promise<Blob>((ok, ko) => {
    const canvas = document.createElement('canvas');
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    canvas.getContext('2d')?.drawImage(img, 0, 0);
    canvas.toBlob((b) => (b ? ok(b) : ko(new Error('PNG'))), 'image/png');
  });
  // The promise itself goes on the clipboard: WebKit keeps the click's gesture for it.
  return navigator.clipboard.write([new ClipboardItem({ 'image/png': png })]);
}
