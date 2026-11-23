// The styles of the window under its content security policy (src-tauri/tauri.conf.json, audit SEC-05, ESC-593): a
// `<style>` element applies only with the nonce Tauri gives the page at each load, on the `<style>` of index.html
// (`style-src 'self' 'nonce-…'`, no `'unsafe-inline'`); the `style` attributes Svelte writes stay allowed
// (`style-src-attr 'unsafe-inline'`). The interface's own code and its libraries create `<style>` elements as they go
// (xterm's, CodeMirror's, a plugin's theme): they are given the nonce as they are created. Markup parsed from a text
// (an agent's message that would pass the sanitizer) creates its elements without this way: they keep no nonce, and
// are not applied.

/** The id of the `<style>` of index.html, which carries the page's nonce. */
export const NONCE_STYLE_ID = 'escouade-style-nonce';

/** The nonce of the page's styles: '' outside Tauri's build (the dev server, the tests), where none is given. */
export function styleNonce(doc: Document = document): string {
  const el = doc.getElementById(NONCE_STYLE_ID) as HTMLStyleElement | null;
  // The attribute is emptied once the page is read (nonce hiding): the property keeps it.
  return el?.nonce || el?.getAttribute('nonce') || '';
}

/** Gives the page's nonce to every `<style>` its code creates (`createElement`, `createElementNS`, an SVG's too). */
export function shareStyleNonce(doc: Document = document): void {
  const nonce = styleNonce(doc);
  if (!nonce) return;
  const mark = <E extends Element>(el: E): E => {
    if (el.localName === 'style') (el as unknown as HTMLOrSVGElement).nonce = nonce;
    return el;
  };
  const create = doc.createElement;
  const createNS = doc.createElementNS;
  doc.createElement = function (this: Document, ...args: Parameters<Document['createElement']>) {
    return mark(create.apply(this, args));
  } as Document['createElement'];
  doc.createElementNS = function (this: Document, ...args: [string | null, string, ElementCreationOptions?]) {
    return mark(createNS.apply(this, args as Parameters<Document['createElementNS']>));
  } as Document['createElementNS'];
}
