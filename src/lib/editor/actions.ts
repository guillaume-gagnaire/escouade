// Saving from the editor's button, Ctrl+S and the close prompts.

import { t } from '../i18n';
import { app } from '../state.svelte';
import { buffers } from './buffers.svelte';

/** The file shown in the editor of the project on screen. */
export function activeKey(): string | null {
  const p = app.project;
  const st = p ? app.editor[p.id] : undefined;
  if (!p || !st?.on) return null;
  const path = st.places[st.source]?.active;
  return path ? buffers.key(p.id, st.source, path) : null;
}

/** Saves a file; false when it changed on disk (its banner says so) or could not be written. */
export async function saveKey(key: string): Promise<boolean> {
  try {
    return await buffers.save(key);
  } catch (e) {
    app.toast(t('editor.toast.saveFailed', { error: String(e) }), 'error');
    return false;
  }
}

export function saveActive(): Promise<boolean> {
  const k = activeKey();
  return k ? saveKey(k) : Promise.resolve(false);
}
