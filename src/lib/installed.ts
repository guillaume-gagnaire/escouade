// What the installed plugins bring (ESC-407): a page of the settings lists a harness, a ticket source, a code host or a
// voice service only while a plugin installed brings it. Installing is done in Réglages › Plugins and on the home screen,
// never by an offer « Non installé » among the settings; what is kept of a plugin uninstalled (its accounts, its choices,
// its keys in the keychain) stays on the disk, hidden, and comes back with it.
import { ONLINE_SERVICES, type OnlineService } from './dictation';
import type { Manifest } from './plugins';
import { plugins } from './plugins-view.svelte';
import { app } from './state.svelte';
import type { EngineDecl, EngineId } from './types';

/** What a plugin may bring that a page of the settings lists. */
export type Brought = 'harnesses' | 'ticketSources' | 'forges' | 'voice' | 'launchEnvironments';

/** The ids a manifest brings of `kind`: its `contributes` entries, or for a voice service its activation `voice:<id>`. */
export function broughtBy(m: Manifest | null, kind: Brought): string[] {
  if (!m) return [];
  if (kind === 'voice') return (m.activation ?? []).filter((a) => a.startsWith('voice:')).map((a) => a.slice('voice:'.length));
  const list = m.contributes?.[kind];
  return Array.isArray(list) ? list.map((i) => String((i as { id?: unknown }).id ?? '')).filter(Boolean) : [];
}

/**
 * Whether an installed plugin brings `id` of `kind` (and is on, with `active`: a voice service switched off leaves its
 * page, a harness, a source or a code host stays with its state said). Until the plugins are read (the app's first
 * frames, a test that does not read them), everything is: nothing shown is taken away.
 */
export function installed(kind: Brought, id: string, active = false): boolean {
  if (!plugins.loaded) return true;
  return plugins.list.some((p) => (!active || p.choices.enabled) && broughtBy(p.manifest, kind).includes(id));
}

/** `ids` brought by an installed plugin, in their order. */
export function installedOf<T extends string>(kind: Brought, ids: readonly T[]): T[] {
  return ids.filter((id) => installed(kind, id));
}

/** The voice services on line an installed plugin on brings, in their order. */
export function voiceServices(): OnlineService[] {
  return ONLINE_SERVICES.filter((s) => installed('voice', s.id, true));
}

/** The plugin of the first hand behind each harness of the core (`plugins::sort::of_engine`). */
const HARNESS_PLUGINS: Record<string, string> = {
  claude: 'dev.escouade.claude-code',
  codex: 'dev.escouade.codex',
  copilot: 'dev.escouade.copilot-cli',
  cursor: 'dev.escouade.cursor-cli',
  opencode: 'dev.escouade.opencode',
  vibe: 'dev.escouade.mistral',
  agy: 'dev.escouade.antigravity-cli',
  gemini: 'dev.escouade.gemini-cli',
  goose: 'dev.escouade.goose',
};

/**
 * Whether the harness `id` is brought by an installed plugin. The core keeps its own harnesses declared whether their
 * plugin is there or not (an agent of one says it at its start): its plugin decides, by its id as the core knows it, or
 * by what its manifest declares. A harness a third party's plugin describes (`plugin:<id>:<harness>`) is declared only
 * while its plugin is on.
 */
export function harnessInstalled(id: EngineId): boolean {
  if (id.startsWith('plugin:') || !plugins.loaded) return true;
  const own = HARNESS_PLUGINS[id];
  return plugins.list.some((p) => p.id === own || broughtBy(p.manifest, 'harnesses').includes(id));
}

/** The harnesses the core declares whose plugin is installed, in its order. */
export function installedEngines(): EngineDecl[] {
  return app.engines.filter((e) => harnessInstalled(e.id));
}
