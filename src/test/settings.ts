import userEvent from '@testing-library/user-event';
import { settingsForm } from '../lib/settings.svelte';

/**
 * What the settings window does after a gesture (ESC-380): the field typed in is left (its `change` saves it, as a
 * click or a choice already did), then the saves started are let finish. No « Enregistrer » to click.
 */
export async function afterGesture() {
  await userEvent.tab();
  // The change seen by the window, and its save started.
  await new Promise((r) => setTimeout(r));
  await new Promise((r) => setTimeout(r));
  await settingsForm.settled();
}
