// The HTML of a document shown in a preview of the VS Code view (ESC-473): a Markdown file rendered, a Word file
// converted. A file of a repository or of an agent is not trusted, so its HTML is sanitized before it is in the page:
//
// - no script runs: DOMPurify's HTML profile (no SVG nor MathML in the page: an image of them is an `<img>`), no event
//   attribute, no `javascript:` address;
// - nothing styles the app: no `<style>`, no `style` attribute, no `<link>`, `<base>`, `<meta>`, form or frame;
// - nothing is loaded by itself: an `<img>` keeps no `src`. Its address is said in an attribute that the pane reads
//   (`data-remote-src` for one of the web, loaded by the core on a click only; `data-local-src` for a file of the
//   source, read by the core; `data-img-off` for any other), a `data:` image alone kept as it is (it names nothing);
// - the ids are prefixed (`user-content-`), so that a document's names never shadow the page's.
//
// Its links are left with their `href`, which the pane's click reads (`linkOf`): Escouade's link opening takes those
// of the web (`guardLinks`), the pane those of the source.

import createDOMPurify from 'dompurify';

/** A `data:` image an `<img>` may show: no address in it, no script runs in an image. */
const DATA_IMAGE = /^data:image\/(png|jpe?g|gif|webp|avif|bmp|x-icon|svg\+xml);base64,[a-z0-9+/=\s]*$/i;

const OWN = new Set(['data-remote-src', 'data-local-src', 'data-img-off', 'data-read', 'data-show-img']);

let purify: ReturnType<typeof createDOMPurify> | null = null;

function instance() {
  if (purify) return purify;
  // An instance of its own: the hooks of the conversation's Markdown (`markdown.ts`) are not these.
  const p = createDOMPurify(window);
  // The pane's own marks are never the document's: an image's address is told by its `src` alone.
  p.addHook('uponSanitizeAttribute', (_node, data) => {
    if (OWN.has(data.attrName)) data.keepAttr = false;
  });
  p.addHook('afterSanitizeAttributes', (node) => {
    const el = node as Element;
    if (el.tagName === 'IMG') {
      const src = (el.getAttribute('src') ?? '').trim();
      el.removeAttribute('src');
      el.removeAttribute('srcset');
      if (DATA_IMAGE.test(src)) el.setAttribute('src', src);
      else if (/^https:\/\//i.test(src)) el.setAttribute('data-remote-src', src);
      else if (src && !/^[a-z][a-z0-9+.-]*:/i.test(src) && !src.startsWith('/') && !src.startsWith('\\'))
        el.setAttribute('data-local-src', src);
      else el.setAttribute('data-img-off', src);
    } else if (el.tagName === 'A') {
      el.removeAttribute('target');
      el.setAttribute('rel', 'noopener noreferrer');
    } else if (el.tagName === 'INPUT') {
      // A task of a list (`- [x]`) alone, and never to be ticked: the preview reads.
      if ((el.getAttribute('type') ?? '').toLowerCase() !== 'checkbox') el.remove();
      else el.setAttribute('disabled', '');
    }
  });
  purify = p;
  return p;
}

/** `html` sanitized for a preview, as a string (see above). */
export function cleanHtml(html: string): string {
  return instance().sanitize(html, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: [
      'style',
      'link',
      'base',
      'meta',
      'form',
      'button',
      'textarea',
      'select',
      'option',
      'iframe',
      'frame',
      'frameset',
      'object',
      'embed',
      'audio',
      'video',
      'source',
      'track',
      'picture',
      'template',
      'dialog',
    ],
    FORBID_ATTR: ['style', 'srcset', 'sizes', 'background', 'poster', 'ping', 'formaction', 'action', 'autofocus', 'tabindex'],
    ALLOW_DATA_ATTR: true,
    SANITIZE_NAMED_PROPS: true,
  });
}
