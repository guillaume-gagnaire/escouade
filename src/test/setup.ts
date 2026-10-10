import '@testing-library/jest-dom/vitest';
import { clearMocks } from '@tauri-apps/api/mocks';
import { cleanup } from '@testing-library/svelte';
import { afterEach, beforeEach } from 'vitest';
import { setLang } from '../lib/i18n';

// The tests are written in French: each starts in French, whatever language the one before it switched to.
beforeEach(() => setLang('fr'));

// jsdom lacks these browser APIs used by the UI.
class RO {
  observe() {}
  unobserve() {}
  disconnect() {}
}
(globalThis as any).ResizeObserver ??= RO;
if (typeof Element !== 'undefined') Element.prototype.scrollIntoView ??= () => {};
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
