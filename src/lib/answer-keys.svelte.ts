// What a card of a request waiting for an answer needs from the keyboard and the focus.

import { app } from './state.svelte';

/**
 * Hands every key press to `handler` while `on()`, before anything else sees it (capture phase on the
 * window): the message field must not also take Ctrl+Enter for "send". Listening only while a card is
 * the one the keyboard answers keeps the cards of a long conversation out of every key press.
 */
export function captureKeys(on: () => boolean, handler: (e: KeyboardEvent) => void) {
  $effect(() => {
    if (!on()) return;
    window.addEventListener('keydown', handler, true);
    return () => window.removeEventListener('keydown', handler, true);
  });
}

/**
 * Ctrl+J brought this agent in for its request (`app.focusPending`): when `on`, the request's card
 * takes the focus, and the message field stays out of the way (see Composer). One tick later, so that
 * the field, which reads the request in its own effect, has seen it before it is cleared.
 */
export function takeFocusOnRequest(card: () => HTMLElement | undefined, agentId: () => string, on: () => boolean) {
  $effect(() => {
    const el = card();
    if (!on() || !el || app.focusPending !== agentId()) return;
    queueMicrotask(() => {
      el.focus();
      app.focusPending = null;
    });
  });
}
