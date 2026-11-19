// Which files the window hands the page are files of the disk, so that one can be attached where it lies instead of copied. The
// WebView gives a `File` no path, and Tauri's own drag and drop, which would, is off (it would break the HTML drag and drop of
// the app).
// The page must never be the one to say where a file is: a path it gave (read from `text/uri-list`, typed in a message) is a path
// it could forge, and the backend would read it, and hand it to Claude, wherever it pointed. So the path is only ever read by the
// host, which keeps it, and the page holds an opaque id for the file: `attach_origin` takes that id, never a path.
// Only WebView2 (Windows) has a bridge that lets the host read the path of a `File`. Elsewhere no file has an origin: it is
// copied, as is a file the host does not know (a screenshot pasted) or one whose host fails to answer. Whatever fails gives null,
// never an error.

import { onServer } from './connection.svelte';
import { api } from './ipc';
import { OS, type Os } from './platform';

interface WebView2 {
  postMessageWithAdditionalObjects(message: string, objects: unknown[]): void;
}

/**
 * For each of `files`, in their order, the id the host knows it by (the one to give `attach_origin`), or null when it has no
 * origin to attach it by: it is no file of the disk, or the system has no host to read it. The path itself is not kept here.
 */
export async function fileOrigins(files: File[], os: Os = OS): Promise<(string | null)[]> {
  // On a server (ESC-538), a file of this computer is never where the agents are: always a copy, sent.
  if (os === 'windows' && !onServer()) return fromWebView2(files);
  return files.map(() => null);
}

/**
 * Windows: WebView2 lets a page send its host a message with `File` objects, whose paths the host reads (`ICoreWebView2File`).
 * Each file goes with an id of its own; the backend keeps the path of the file it received under that id, and says whether it
 * found one. That id is what stands for the file from then on.
 */
async function fromWebView2(files: File[]): Promise<(string | null)[]> {
  const bridge = (window as unknown as { chrome?: { webview?: WebView2 } }).chrome?.webview;
  if (typeof bridge?.postMessageWithAdditionalObjects !== 'function') return files.map(() => null);
  return Promise.all(
    files.map(async (f) => {
      const id = crypto.randomUUID();
      try {
        bridge.postMessageWithAdditionalObjects(JSON.stringify({ escouadeFileOrigin: id }), [f]);
        // The path says only that the host read one; it stays with the host.
        return (await api.fileOrigin(id)) ? id : null;
      } catch {
        return null;
      }
    }),
  );
}
