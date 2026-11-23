import { afterEach, describe, expect, it } from 'vitest';
import { NONCE_STYLE_ID, shareStyleNonce, styleNonce } from './style-nonce';

/** A page as Tauri serves it: the `<style>` of index.html with the nonce of this load. */
function page(nonce: string | null): Document {
  const doc = document.implementation.createHTMLDocument('Escouade');
  const style = doc.createElement('style');
  style.id = NONCE_STYLE_ID;
  if (nonce !== null) style.setAttribute('nonce', nonce);
  doc.head.appendChild(style);
  return doc;
}

describe('the nonce of the window’s styles (audit SEC-05, ESC-593)', () => {
  afterEach(() => document.getElementById(NONCE_STYLE_ID)?.remove());

  it('is read from the <style> of index.html, none outside Tauri’s build', () => {
    expect(styleNonce(page('8142'))).toBe('8142');
    expect(styleNonce(page(null))).toBe('');
    expect(styleNonce(document.implementation.createHTMLDocument(''))).toBe('');
  });

  it('is given to each <style> the page’s code creates, an SVG’s too, and to nothing else', () => {
    const doc = page('8142');
    shareStyleNonce(doc);
    expect(doc.createElement('style').nonce).toBe('8142');
    expect(doc.createElement('STYLE').nonce).toBe('8142');
    expect((doc.createElementNS('http://www.w3.org/2000/svg', 'style') as SVGStyleElement).nonce).toBe('8142');
    expect((doc.createElementNS('http://www.w3.org/1999/xhtml', 'style') as HTMLStyleElement).nonce).toBe('8142');
    expect(doc.createElement('div').nonce).toBe('');
    expect(doc.createElement('script').nonce).toBe('');
    expect(doc.createElement('div')).toBeInstanceOf(doc.defaultView?.HTMLDivElement ?? HTMLElement);
  });

  it('is never given to a <style> that markup parsed from a text brings', () => {
    const doc = page('8142');
    shareStyleNonce(doc);
    const box = doc.createElement('div');
    box.innerHTML = '<style>.x{color:red}</style><svg><style>.y{fill:red}</style></svg>';
    const styles = [...box.querySelectorAll('style')];
    expect(styles).toHaveLength(2);
    for (const s of styles) expect((s as HTMLStyleElement).nonce).toBe('');
    const parsed = new DOMParser().parseFromString('<style>.z{}</style>', 'text/html');
    expect(doc.importNode(parsed.querySelector('style')!, true).nonce).toBe('');
  });

  it('changes nothing without a nonce (the dev server, the tests)', () => {
    const doc = page(null);
    const create = doc.createElement;
    shareStyleNonce(doc);
    expect(doc.createElement).toBe(create);
  });
});
