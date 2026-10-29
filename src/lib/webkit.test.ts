import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { isWebKit, markWebKit } from './webkit';

const GTK = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)';
const MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)';
const WEBVIEW2 =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0';

describe('the page under WebKit', () => {
  it('tells WebKitGTK and WKWebView from WebView2', () => {
    expect(isWebKit(GTK)).toBe(true);
    expect(isWebKit(MAC)).toBe(true);
    expect(isWebKit(WEBVIEW2)).toBe(false);
  });

  it('marks the page before the first render', () => {
    const doc = document.implementation.createHTMLDocument('');
    markWebKit(doc, GTK);
    expect(doc.documentElement.classList.contains('webkit')).toBe(true);
    const other = document.implementation.createHTMLDocument('');
    markWebKit(other, WEBVIEW2);
    expect(other.documentElement.classList.contains('webkit')).toBe(false);
  });

  // WebKitGTK 2.50 aborts its web process in `text-wrap: pretty` on some paragraphs (ESC-290): the rule that turns it
  // off under WebKit must stay, whatever a component asks for.
  it('keeps text-wrap: pretty away from WebKit', () => {
    const css = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), '../app.css'), 'utf8');
    expect(css).toMatch(/:root\.webkit \*,\s*:root\.webkit \*::before,\s*:root\.webkit \*::after \{\s*text-wrap-style: auto !important;/);
  });
});
