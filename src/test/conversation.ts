// Test utility: the element a conversation lives in. The keys that answer a request only act there.

import { afterEach, beforeEach, vi } from 'vitest';

/** A `<main class="conv">`, to render a card into as Conversation.svelte hosts it. */
export function conversationHost(): HTMLElement {
  const main = document.createElement('main');
  main.className = 'conv';
  document.body.appendChild(main);
  return main;
}

/** How long the keys that answer a request wait once its card is the one they answer, and a little more. */
export const HELD = 550;

/** The clock that wait reads (`performance`), the tests' own in the `describe` this is called in. */
export function answerClock() {
  beforeEach(() => vi.useFakeTimers({ toFake: ['performance'] }));
  afterEach(() => vi.useRealTimers());
}

/** Lets pass the moment the keys of a card that just became the one they answer wait. */
export const settle = () => vi.advanceTimersByTime(HELD);
