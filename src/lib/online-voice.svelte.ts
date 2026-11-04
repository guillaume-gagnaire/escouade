import { untrack } from 'svelte';
import { api } from './ipc';
import type { MistralKey, OnlineModel, OnlineVoice, ServiceCatalog } from './types';

// The services on line of Réglages › Voix (ESC-110, ESC-132), shared by the dictation and the conversation: which keys
// are in the keychain (one by service, for both), and each service's models and voices, as its API listed them (or
// its documentation's).
class OnlineVoiceState {
  keys = $state<string[]>([]);
  /** The Mistral account (ESC-327), shared by the voice and Mistral Vibe: its key by its end, or the Vibe account the voice goes by. */
  mistral = $state<MistralKey>({ own: null, borrowed: null });
  catalogs = $state<Record<string, ServiceCatalog>>({});
  /** Services whose list is being read. */
  loading = $state<Record<string, boolean>>({});

  async loadKeys() {
    this.keys = (await api.voiceOnlineKeys().catch(() => null)) ?? [];
    this.mistral = (await api.voiceMistralKey().catch(() => null)) ?? { own: null, borrowed: null };
  }

  /** The models and voices of `service`; `refresh` reads its API again. */
  async load(service: string, refresh = false) {
    // Called from effects: what it reads does not make them run again.
    if (untrack(() => this.loading[service])) return;
    this.loading[service] = true;
    try {
      const c = await api.voiceOnlineCatalog(service, refresh);
      if (c) this.catalogs[service] = c;
    } catch {
      // The window keeps what it had.
    } finally {
      this.loading[service] = false;
    }
  }
}

export const onlineVoice = new OnlineVoiceState();

/** The models of `c` for `purpose`. */
export function modelsOf(c: ServiceCatalog | undefined, purpose: 'transcribe' | 'speak'): OnlineModel[] {
  return c ? c[purpose] : [];
}

/** The model the settings name, if `c` offers it; its default else. Says whether the one chosen is gone. */
export function modelIn(c: ServiceCatalog | undefined, purpose: 'transcribe' | 'speak', chosen: string | undefined) {
  const def = c ? (purpose === 'transcribe' ? c.defaultTranscribe : c.defaultSpeak) : '';
  const want = (chosen ?? '').trim();
  if (!c || !want || want === def) return { id: def, gone: null as string | null };
  return modelsOf(c, purpose).some((m) => m.id === want) ? { id: want, gone: null } : { id: def, gone: want };
}

/** Where the voice of `model` in `lang` is kept in the settings (as `online_speech::voice_key`). */
export function voiceKey(c: ServiceCatalog, model: string, lang: string) {
  return c.voicesByModel ? `${c.service}|${model}|${lang}` : `${c.service}|${lang}`;
}

/** The voices of `c` that read `lang` with `model`, those matching `query` (name, id, note), the user's own first. */
export function voicesFor(c: ServiceCatalog, model: string, lang: string, query = ''): OnlineVoice[] {
  const q = query.trim().toLowerCase();
  return (c.voices ?? [])
    .filter((v) => !v.langs.length || v.langs.includes(lang))
    .filter((v) => !v.model || v.model === model)
    .filter((v) => !q || `${v.name} ${v.id} ${v.note}`.toLowerCase().includes(q))
    .sort((a, b) => Number(b.own) - Number(a.own));
}
