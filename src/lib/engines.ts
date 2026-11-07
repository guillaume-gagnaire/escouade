// What a harness can do, as it declares it (`EngineCaps`, docs/ENGINES.md): the window shows a function from these,
// never from the name of the agent's harness. A function its harness lacks is hidden, not greyed out.

import { t } from './i18n';
import { intlLocale } from './i18n/locale.svelte';
import { api } from './ipc';
import { app } from './state.svelte';
import { modelFamily, modelVersion, supportsAuto, toldAuto } from './models';
import type { EngineCaps, EngineDecl, EngineId, KnownEngineId, ToldModel } from './types';

/** The flags of `EngineCaps` that say yes or no. */
type Flag = { [K in keyof EngineCaps]: EngineCaps[K] extends boolean ? K : never }[keyof EngineCaps];

/**
 * What `engine` declares, null for one the backend does not know (none of its functions shown). No harness yet (null):
 * the default one, which the backend lists first.
 */
export function engineOf(engine: EngineId | null | undefined): EngineDecl | null {
  return (engine ? app.engines.find((e) => e.id === engine) : app.engines[0]) ?? null;
}

/** The capabilities of `engine` (null: the default harness's). */
export function capsOf(engine: EngineId | null | undefined): EngineCaps | null {
  return engineOf(engine)?.caps ?? null;
}

/** Whether the harness of `who` (an agent, or anything that runs on one) has the capability `flag`. */
export function can(who: { engine: EngineId | null | undefined }, flag: Flag): boolean {
  return capsOf(who.engine)?.[flag] === true;
}

/** Whether the harness of `who` copies a conversation (« Dupliquer la conversation »). */
export function canFork(who: { engine: EngineId | null | undefined }): boolean {
  const fork = capsOf(who.engine)?.fork;
  return !!fork && fork !== 'none';
}

/** Whether the harness of `who` tells what a turn costs, exactly or estimated (none: no cost shown). */
export function showsCost(who: { engine: EngineId | null | undefined }): boolean {
  const cost = capsOf(who.engine)?.cost;
  return !!cost && cost !== 'none';
}

/**
 * Where the usage of an account of `engine` is seen, for a harness that tells Escouade none (Cursor CLI: its dashboard, as its
 * `/usage` says it; ESC-43). Undefined for the others, whose quota or cost Escouade reads.
 */
export function usagePage(engine: EngineId | null | undefined): string | undefined {
  return capsOf(engine)?.usagePage ?? undefined;
}

/** The site of `usagePage(engine)`, as its link names it (« cursor.com », « admin.mistral.ai »). */
export function usageSite(engine: EngineId | null | undefined): string {
  const page = usagePage(engine);
  return page ? new URL(page).hostname : '';
}

/**
 * Whether the day's cost of an account of `engine` stands in for a quota (and its « Budget par jour ($) »): a harness
 * without a quota that tells what its turns cost (OpenCode; Mistral Vibe, its usage page beside, ESC-267). Not Cursor CLI,
 * whose usage is on its own page only.
 */
export function dayCostStandsIn(engine: EngineId | null | undefined): boolean {
  return capsOf(engine)?.quota === 'none' && (!usagePage(engine) || showsCost({ engine }));
}

/** Whether an account of `engine` other than its machine's goes by an API key rather than a sign-in (Cursor CLI, ESC-43; Mistral Vibe, ESC-267). */
export function byKey(engine: EngineId | null | undefined): boolean {
  return !!capsOf(engine)?.accountsByKey;
}

/** The modes the harness of `who` takes, among `all` (in their order). */
export function modesOf<T extends { value: string }>(who: { engine: EngineId | null | undefined }, all: T[]): T[] {
  const ids = new Set(capsOf(who.engine)?.modes.map((m) => m.id) ?? []);
  return all.filter((m) => ids.has(m.value));
}

/** Whether any harness Escouade knows has the capability `flag` (for a setting of them all). */
export function anyCan(flag: Flag): boolean {
  return app.engines.some((e) => e.caps[flag]);
}

/** The harnesses offered (menus « Harness », brand): those declared whose program is found, in the backend's order. */
export function offered(): EngineDecl[] {
  return app.engines.filter((e) => app.enginesFound.includes(e.id));
}

/** Whether there is a harness to choose: the menus « Harness » show from two harnesses found on. */
export function choosesEngine(): boolean {
  return offered().length > 1;
}

/** The harnesses offered, as the options of a menu « Harness »; `current` stays listed even if it is not found any more. */
export function engineOptions(current?: EngineId | null): { value: EngineId; label: string }[] {
  const list = offered();
  const kept = current && !list.some((e) => e.id === current) ? app.engines.filter((e) => e.id === current) : [];
  return [...list, ...kept].map((e) => ({ value: e.id, label: e.name }));
}

/** The usual names of the harnesses, before the backend declared them (the window's first frames). */
const NAMES: Record<KnownEngineId, string> = {
  claude: 'Claude Code',
  codex: 'Codex',
  copilot: 'Copilot CLI',
  cursor: 'Cursor CLI',
  opencode: 'OpenCode',
  vibe: 'Mistral Vibe',
  agy: 'Antigravity CLI',
  gemini: 'Gemini CLI',
};

/**
 * The name of a harness as the interface says it: as the backend declares it, else its usual name; one neither knows is
 * « harness absent » (INT-3: a later version's, a plugin's gone), its agents kept and none of its functions shown.
 */
export function engineName(engine: EngineId | null | undefined): string {
  const declared = engineOf(engine)?.name;
  if (declared || !engine) return declared ?? '';
  return NAMES[engine as KnownEngineId] ?? t('settings.claude.states.missing', { id: engine });
}

/**
 * The name of the agents of a harness as the texts speak of them (« {engine} attend ta réponse »): `Claude` for Claude Code,
 * as it declares it (null: the default harness's); its name when it does not say one.
 */
export function speakerOf(engine: EngineId | null | undefined): string {
  const decl = engineOf(engine);
  return decl?.speaker || decl?.name || engine || '';
}

/** The harnesses the agents run on (archived ones aside). */
export function enginesInUse(): Set<EngineId> {
  return new Set(
    Object.values(app.agents)
      .filter((a) => !a.archived)
      .map((a) => a.engine ?? 'claude'),
  );
}

/** Whether the brand of an agent's harness shows (its card, the sidebar, the overview, its header): once agents run on two. */
export function showsMark(): boolean {
  return enginesInUse().size > 1;
}

/**
 * The harnesses whose models are read by account, without an agent (`Core::read_account_models`): Claude Code (ESC-124),
 * Codex, Copilot CLI, Cursor CLI and OpenCode (ESC-186), Mistral Vibe (ESC-266), Antigravity CLI (`agy models`, ESC-270), Gemini CLI (the `models` of its `session/new`, ESC-274).
 */
const BY_ACCOUNT: EngineId[] = ['claude', 'codex', 'copilot', 'cursor', 'opencode', 'vibe', 'agy', 'gemini'];

/**
 * The models the harness of `who` listed for its account, none until it has. Claude Code and Codex: those its account
 * listed (ESC-124, ESC-186; Codex's harness-wide list while its account has none); without an account (the Kanban, a
 * ticket, the settings), those of its active accounts, each once, in their order.
 */
function toldOf(who: { engine: EngineId | null | undefined; account?: string }): ToldModel[] {
  const id = who.engine ?? engineOf(null)?.id;
  if (id === 'claude') return accountTold('claude', who.account);
  if (id && BY_ACCOUNT.includes(id)) {
    const own = accountTold(id, who.account);
    if (own.length) return own;
  }
  return (id && app.engineModels[id]) || [];
}

/** Whether `model` is in the menu of `who` because its account's config names it, its harness not listing it (ESC-186). */
export function fromConfig(who: { engine: EngineId | null | undefined; account?: string }, model: string): boolean {
  return !!toldOf(who).find((m) => m.id === model)?.fromConfig;
}

/**
 * Whether the menu of the models of `who` is its harness's stand-in (Codex's GPT-6-Luna), its account's not read yet
 * (ESC-186): the menu says the list is being read.
 */
export function readingModels(who: { engine: EngineId | null | undefined; account?: string }): boolean {
  const id = who.engine ?? engineOf(null)?.id;
  return !!id && id !== 'claude' && BY_ACCOUNT.includes(id) && toldOf(who).length === 0;
}

/**
 * The menu of the models of `who` opened: its account's read again when they are a day old (`Core::refresh_account_models`,
 * ESC-186). Not Claude Code's, read at each start of its agents.
 */
export function refreshModels(who: { engine: EngineId | null | undefined; account?: string }): void {
  const id = who.engine ?? engineOf(null)?.id;
  if (!id || id === 'claude' || !BY_ACCOUNT.includes(id) || !who.account) return;
  void api.accountModelsRefresh(who.account).catch(() => {});
}

/** The models of Claude Code the `claude` of its active accounts told (ESC-124), each once: the Kanban's, a ticket's, the settings' menu. */
export function claudeToldModels(): { id: string; label: string }[] {
  return accountTold('claude', undefined).map((m) => ({ id: m.id, label: m.label }));
}

/** The models `account` listed (ESC-124, ESC-186), or every active account of `engine` when none is said. */
function accountTold(engine: EngineId, account: string | undefined): ToldModel[] {
  if (account) return app.accountModels[account]?.catalog ?? [];
  const out: ToldModel[] = [];
  for (const a of app.settings.accounts ?? []) {
    if ((a.engine ?? 'claude') !== engine || !a.active) continue;
    for (const m of app.accountModels[a.id]?.catalog ?? []) if (!out.some((o) => o.id === m.id)) out.push(m);
  }
  return out;
}

/**
 * What `model` costs beside the others, as its harness bills it (Copilot): its multiplier (`×3`), else its tier of
 * price; nothing for a harness that tells neither. With what it means, for its title.
 */
export function modelCost(
  who: { engine: EngineId | null | undefined; account?: string },
  model: string,
): { label: string; title: string } | undefined {
  const m = toldOf(who).find((x) => x.id === model);
  if (!m) return undefined;
  const n = m.multiplier !== undefined ? Number(m.multiplier) : NaN;
  if (Number.isFinite(n)) {
    const shown = n.toLocaleString(intlLocale(), { maximumFractionDigits: 2 });
    return { label: t('composer.model.multiplier', { n: shown }), title: t('composer.model.multiplierTitle', { n: shown }) };
  }
  if (m.price) {
    const label = t(`composer.model.price.${m.price === 'very_high' ? 'veryHigh' : m.price}`);
    return { label, title: label };
  }
  return undefined;
}

/** The name a harness that lists its models gives `model` (`gpt-6-luna` → `GPT-6-Luna`, `mythos` → `Mythos`), if one does. */
export function toldLabel(model: string): string | undefined {
  const lists = [...Object.values(app.engineModels), ...Object.values(app.accountModels).map((a) => a.catalog)];
  for (const list of lists) {
    const found = list?.find((m) => m.id === model);
    if (found) return found.label;
  }
  // By the full id Claude Code ran it under (the statistics': `claude-mythos-1`), the name its `/model` gives it.
  for (const told of Object.values(app.accountModels)) {
    const found = told.models.find((m) => m.value !== 'default' && m.resolvedModel === model && m.displayName);
    if (found) return found.displayName;
  }
  return undefined;
}

/**
 * The models of the composer's menu for the harness of `who`: those it listed for its account when it lists them (Claude
 * Code: those of the account of `who`), else those it declares (none for a harness the backend does not know).
 */
export function modelsOf(who: { engine: EngineId | null | undefined; account?: string }): { id: string; label: string }[] {
  const told = toldOf(who);
  if (told.length) return told.map((m) => ({ id: m.id, label: m.label }));
  return engineOf(who.engine)?.models ?? [];
}

/**
 * The model the harness of `who` runs when it is given none (`Core::model_for`): the one it lists as its default for the
 * account, else the first it lists, else the first it declares.
 */
export function defaultModelOf(who: { engine: EngineId | null | undefined; account?: string }): string {
  const told = toldOf(who);
  return (told.find((m) => m.default) ?? told[0])?.id ?? engineOf(who.engine)?.models[0]?.id ?? '';
}

/** The efforts of the harness of `who`, among `all` (in their order): none when it takes none. */
export function effortsOf<T extends { value: string }>(
  who: { engine: EngineId | null | undefined; model?: string; account?: string },
  all: T[],
): T[] {
  const decl = engineOf(who.engine);
  if (!decl?.caps.effort) return [];
  // Those of its model, as its harness listed it.
  const told = toldOf(who).find((m) => m.id === who.model);
  const efforts = told ? told.efforts : decl.efforts;
  return all.filter((e) => efforts.includes(e.value));
}

/**
 * Whether `model` takes an effort or the mode `auto` on the harness of `who`, as its declaration says: by its id, else by
 * the declared model its full id holds (`claude-haiku-4-5` is Haiku). A model it does not declare takes them. The mode
 * `auto` on Claude Code: as the `claude` of the account of `who` told it, else by its rule (`supportsAuto`), as the tickets
 * and the handover have it (`Core::takes_auto`).
 */
export function modelTakes(
  who: { engine: EngineId | null | undefined; account?: string },
  model: string,
  what: 'effort' | 'auto',
): boolean {
  if (what === 'auto' && (who.engine ?? engineOf(null)?.id) === 'claude') {
    const told = who.account ? app.accountModels[who.account]?.models : undefined;
    return toldAuto(told ?? [], model) ?? supportsAuto(model);
  }
  const m = model.toLowerCase();
  const told = toldOf(who).find((x) => x.id.toLowerCase() === m);
  if (told) return what === 'effort' ? told.efforts.length > 0 : true;
  const models = engineOf(who.engine)?.models ?? [];
  const decl = models.find((x) => x.id === m) ?? models.find((x) => m.includes(x.id));
  return decl ? decl[what] : true;
}

/**
 * The harness the texts written by a model go to (« Modèle des textes rédigés »), as the backend picks it
 * (`oneshot::writer`): the one `chosen` while it is found, else Claude Code when it is (its Haiku), else the first found;
 * null when none is.
 */
export function writerOf(chosen: EngineId | null | undefined): EngineId | null {
  const found = offered().map((e) => e.id);
  if (chosen && found.includes(chosen)) return chosen;
  return found.includes('claude') ? 'claude' : (found[0] ?? null);
}

/** The choices of « Modèle des textes rédigés »: the model each harness found writes them with (`Claude Haiku`). */
export function writerOptions(): { value: EngineId; label: string }[] {
  return offered().map((e) => ({ value: e.id, label: e.writer || e.name }));
}

/** The models of `engine` marked as favorites, in the order they were marked. */
export function favoritesOf(engine: EngineId | null | undefined): string[] {
  const id = engine ?? engineOf(null)?.id;
  return (id && app.settings.favoriteModels?.[id]) || [];
}

/** `model` of `engine` marked as a favorite, or no longer: kept at once (not part of the settings' draft). */
export async function setFavorite(engine: EngineId, model: string, favorite: boolean): Promise<void> {
  const all = await app.run(api.favoriteModelSet(engine, model, favorite));
  if (all) app.settings = { ...app.settings, favoriteModels: all };
}

/** Whether the menu of the models of `who` takes a search and favorites: its harness lists many, or from several providers. */
export function searchesModels(who: { engine: EngineId | null | undefined; account?: string }): boolean {
  const told = toldOf(who);
  return told.length > 8 || told.some((m) => !!m.provider);
}

/** A model of the composer's menu (`modelMenu`): the group it is shown under, and whether it is a favorite. */
export interface MenuModel {
  id: string;
  label: string;
  provider?: string;
  favorite: boolean;
  /** The heading of its group: « Favoris », its family (its provider on OpenCode), else « Autres ». */
  group: string;
}

/** The full id Claude Code runs for `model` (`opus` → `claude-opus-5-5`), as the `claude` of an account told it. */
function resolvedOf(model: string, account: string | undefined): string | undefined {
  const all = account ? [app.accountModels[account]] : Object.values(app.accountModels);
  for (const told of all) {
    const found = told?.models.find((m) => m.value === model);
    if (found?.resolvedModel) return found.resolvedModel;
  }
  return undefined;
}

/**
 * The models of the composer's menu for `who` (what `modelsOf` gives, each with its provider), in groups (ESC-127): the
 * favorites first, in their order, then one group by family, each in a stable order: Claude Code's by family (Fable, Opus,
 * Sonnet, Haiku, Mythos, then any other its `claude` lists), the newest first in each; OpenCode's by provider, in the
 * order it lists them; the others' by the vendor of their id (GPT, Claude, Gemini…). What is of no family comes last, in
 * « Autres ».
 */
export function modelMenu(who: { engine: EngineId | null | undefined; account?: string }): MenuModel[] {
  const favorites = favoritesOf(who.engine);
  const told = toldOf(who);
  const all: { id: string; label: string; provider?: string }[] = told.length
    ? told.map((m) => ({ id: m.id, label: m.label, provider: m.provider }))
    : modelsOf(who);
  const claude = (who.engine ?? engineOf(null)?.id) === 'claude';
  const byProvider = all.some((m) => !!m.provider);
  const favored = favorites.flatMap((id) => all.filter((m) => m.id === id));
  const groups: { name: string; rank: number; models: { m: (typeof all)[number]; version?: number }[] }[] = [];
  for (const m of all) {
    if (favorites.includes(m.id)) continue;
    const resolved = claude ? resolvedOf(m.id, who.account) : undefined;
    const family = byProvider
      ? m.provider
        ? { name: m.provider, rank: 0 }
        : undefined
      : modelFamily([m.id, resolved ?? '', m.label], claude);
    const name = family?.name ?? t('nav.dropdown.others');
    const rank = family ? family.rank : Infinity;
    let group = groups.find((g) => g.name === name && g.rank === rank);
    if (!group) groups.push((group = { name, rank, models: [] }));
    const version = claude ? (modelVersion(resolved ?? m.id) ?? modelVersion(m.label)) : undefined;
    group.models.push({ m, version });
  }
  // The newest first among Claude Code's (an alias without its full id is the newest of its family): the order listed else.
  const newest = (v?: number) => v ?? Infinity;
  const byAge = (a: { version?: number }, b: { version?: number }) =>
    newest(a.version) === newest(b.version) ? 0 : newest(b.version) > newest(a.version) ? 1 : -1;
  if (claude) for (const g of groups) g.models.sort(byAge);
  groups.sort((a, b) => a.rank - b.rank);
  const menu = (m: (typeof all)[number], group: string, favorite: boolean): MenuModel => ({ ...m, favorite, group });
  return [
    ...favored.map((m) => menu(m, t('nav.dropdown.favorites'), true)),
    ...groups.flatMap((g) => g.models.map(({ m }) => menu(m, g.name, false))),
  ];
}
