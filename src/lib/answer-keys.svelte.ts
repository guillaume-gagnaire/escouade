// What a card of a request waiting for an answer needs from the keyboard.

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
