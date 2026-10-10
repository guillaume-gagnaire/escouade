// What a card of a request waiting for an answer needs from the keyboard.

import { answersHere, enterAnswer, optionAnswer } from './shortcuts';

/**
 * How long the keys that answer wait once a card is the one they answer (as « Vue d’ensemble » holds its buttons): a
 * Ctrl+Enter meant to send the message typed, pressed as the request comes, must not answer it unread; nor must the
 * second press of a Ctrl+Enter answer the request that just took the place of the one answered.
 */
export const HOLD_MS = 500;

/**
 * Hands every key press to `handler` while `on()`, before anything else sees it (capture phase on the
 * window): the message field must not also take Ctrl+Enter for "send". Listening only while a card is
 * the one the keyboard answers keeps the cards of a long conversation out of every key press.
 * For `HOLD_MS` after `on()` turns true, the keys that answer (Ctrl+Enter, Ctrl+Shift+Enter, Alt+digit)
 * do nothing at all: the message field does not get them either, which would send its text as the
 * answer.
 */
export function captureKeys(on: () => boolean, handler: (e: KeyboardEvent) => void) {
  $effect(() => {
    if (!on()) return;
    // A monotonic clock: a change of the system's time neither ends the wait early nor makes it last.
    const since = performance.now();
    const listen = (e: KeyboardEvent) => {
      if (performance.now() - since < HOLD_MS && answersHere(e.target) && (enterAnswer(e) || optionAnswer(e) !== null)) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }
      handler(e);
    };
    window.addEventListener('keydown', listen, true);
    return () => window.removeEventListener('keydown', listen, true);
  });
}
