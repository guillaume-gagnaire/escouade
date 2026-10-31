// Sending a draft, for the composer and the outbox.

import type { DraftAttachment } from './attachments';
import { conversationOf } from './conversations.svelte';
import { api } from './ipc';
import { errorText, missingAccountOf } from './refusal';
import { app } from './state.svelte';

export function sendDraft(agentId: string, body: string, files: DraftAttachment[]): Promise<void> {
  return api.sendMessage(
    agentId,
    body,
    files.filter((f) => f.kind !== 'file').map(({ name, mediaType, data }) => ({ name, mediaType, data })),
    files.filter((f) => f.file).map((f) => $state.snapshot(f.file!)),
  );
}

/** Says why a message was refused: in its conversation for want of an account (choice 19), else in a toast. */
export function reportRefusal(owner: string, e: unknown) {
  const missing = missingAccountOf(e);
  if (missing) conversationOf(owner).noAccount = { engine: missing, text: errorText(e) };
  else app.toast(errorText(e), 'error');
}
