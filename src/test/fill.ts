import { fireEvent } from '@testing-library/svelte';

/**
 * Puts `text` in a field at once, by one `input` (what `bind:value` reads), in place of what it held: for a test of what
 * a form keeps, not of typing. `userEvent.type` sends some ten events and a timer per character: a test that fills a form
 * of a hundred characters that way spends its 5 s typing on a loaded machine (ESC-522).
 */
export async function fill(field: HTMLElement, text: string) {
  // The focus too, as typing gives it: the keys pressed next go there (`userEvent.keyboard`).
  field.focus();
  await fireEvent.input(field, { target: { value: text } });
}
