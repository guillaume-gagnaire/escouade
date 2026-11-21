import '@testing-library/jest-dom/vitest';
import { clearMocks } from '@tauri-apps/api/mocks';
import { cleanup, configure } from '@testing-library/svelte';
import { afterEach, beforeEach } from 'vitest';
import { setLang } from '../lib/i18n';

// How long `findBy…` and `waitFor` look for the state they wait for, as the 15 s of a test (vitest.config.ts): a guard,
// not a wait, they answer as soon as it is there. Testing Library's 1 s ran out on a loaded machine before the editor of
// the split view drew its file (App.test.ts, ESC-522).
configure({ asyncUtilTimeout: 5_000 });

// The tests are written in French: each starts in French, whatever language the one before it switched to.
beforeEach(() => setLang('fr'));

// jsdom lacks these browser APIs used by the UI.
class RO {
  observe() {}
  unobserve() {}
  disconnect() {}
}
(globalThis as any).ResizeObserver ??= RO;
if (typeof Element !== 'undefined') {
  Element.prototype.scrollIntoView ??= () => {};
  // At once, whatever the behavior asked: there is no layout to glide over.
  Element.prototype.scrollTo ??= function (this: Element, to?: ScrollToOptions | number, y?: number) {
    const o = typeof to === 'object' ? to : { left: to, top: y };
    if (o.left !== undefined) this.scrollLeft = o.left;
    if (o.top !== undefined) this.scrollTop = o.top;
  } as Element['scrollTo'];
}
// CodeMirror measures its text with ranges: without layout in jsdom there is nothing to measure.
if (typeof Range !== 'undefined') {
  Range.prototype.getClientRects ??= () => [] as unknown as DOMRectList;
  Range.prototype.getBoundingClientRect ??= () => ({
    x: 0,
    y: 0,
    width: 0,
    height: 0,
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
    toJSON: () => ({}),
  });
}

afterEach(() => {
  if (typeof window === 'undefined') return;
  // Unmount first: components unregister their Tauri listeners while the mocks still exist.
  cleanup();
  clearMocks();
});
