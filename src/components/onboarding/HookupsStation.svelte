<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { onlineService } from '../../lib/dictation';
  import { dictation, type DictationTarget } from '../../lib/dictation.svelte';
  import { fBytes, fList } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { BY_FORGE, SERVICES } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { lockLine } from '../../lib/locks.svelte';
  import { locks } from '../../lib/locks.svelte';
  import { act, cardStatus, refresh, type Primary } from '../../lib/onboarding.svelte';
  import { onlineVoice } from '../../lib/online-voice.svelte';
  import { textOf } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { app } from '../../lib/state.svelte';
  import { storeApi, type CatalogPlugin } from '../../lib/store';
  import type { Hookup, HookupOffer, Service, VoiceInfo } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import MicButton from '../MicButton.svelte';
  import ForgesSection from '../settings/ForgesSection.svelte';
  import OnlineKey from '../settings/OnlineKey.svelte';
  import SourceAccount from '../settings/SourceAccount.svelte';
  import Choices from './Choices.svelte';
  import StationHead from './StationHead.svelte';

  // « Ce que tu veux brancher » (ESC-512, docs/ONBOARDING.md § 10, direction C): a card per hookup, none ticked, each
  // with what it is for and what it asks, and its own « Plus tard ». What a card offers is the core's real list (the
  // registry of the sources, the voice services, the code hosts, the seed, the installed plugins, the policies), never
  // written down here. Ticking shows it; choosing a thing in it is the gesture that installs its plugin, from the seed
  // with no network (and nothing else: the sort of the seed), then opens its connection right here, by the components of
  // Réglages › Services connectés and › Voix: one way to plug in, its secrets kept where they always are. A card left
  // unticked, or passed, waits in the discovery and in its page of Réglages. A card a policy locks says who locks it.
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  const CARDS: Hookup[] = ['sources', 'voice', 'forge'];

  let offers = $state<HookupOffer[]>([]);
  async function loadOffers() {
    offers = (await api.onboardingOffers().catch(() => null)) ?? [];
  }
  /** The store's other plugins of a kind, from its catalogue as last read: nothing is asked of the network here. */
  let catalog = $state<CatalogPlugin[]>([]);
  $effect(() =>
    untrack(() => {
      void loadOffers();
      if (!plugins.loaded) void plugins.load();
      storeApi
        .view()
        .then((v) => (catalog = v?.catalog?.plugins ?? []))
        .catch(() => {});
    }),
  );

  /** Online, as the system says it: what needs the network says so, and waits. */
  let online = $state(typeof navigator === 'undefined' ? true : navigator.onLine);
  $effect(() => {
    const on = () => (online = true);
    const off = () => (online = false);
    window.addEventListener('online', on);
    window.addEventListener('offline', off);
    return () => {
      window.removeEventListener('online', on);
      window.removeEventListener('offline', off);
    };
  });

  const of = (h: Hookup) => offers.filter((o) => o.kind === h);
  /** A card locked: by a policy (the dictation off), or every thing it offers denied. */
  function lockOf(h: Hookup): string | null {
    if (cardStatus(h) === 'policy') {
      const by = locks.current.locks['voice.dictation']?.by;
      return by ? lockLine(by) : t('settings.locks.byDevice');
    }
    const list = of(h);
    if (h !== 'voice' && list.length && list.every((o) => o.denied)) return list[0].denied;
    return null;
  }

  /** Ticked by the user: none at first (« rien de coché d'avance »). A card plugged in already says so. */
  let ticked = $state<Record<Hookup, boolean>>({ sources: false, voice: false, forge: false });
  /** What was passed by a card's own « Plus tard »: unticked, and told so. */
  let passed = $state<Record<Hookup, boolean>>({ sources: false, voice: false, forge: false });
  const plugged = (h: Hookup) => cardStatus(h) === 'done';

  function toggle(h: Hookup) {
    if (lockOf(h)) return;
    ticked[h] = !ticked[h];
    if (ticked[h]) passed[h] = false;
  }
  async function later(h: Hookup) {
    ticked[h] = false;
    passed[h] = true;
    await act({ kind: 'hookup', hookup: h, outcome: 'skipped' });
  }

  /** What each card has chosen, and the plugin being installed for it. */
  let source = $state<string | null>(null);
  let forge = $state<string | null>(null);
  let installing = $state<string | null>(null);
  let refused = $state<Partial<Record<Hookup, string>>>({});

  /**
   * The gesture on a thing of a card: its plugin from the seed (no network), the installed plugins read again so that
   * the components of Réglages show it. Nothing when it is installed, or the store's (said so).
   */
  async function need(h: Hookup, id: string): Promise<boolean> {
    const o = of(h).find((x) => x.id === id);
    if (!o || o.denied) return false;
    delete refused[h];
    if (o.installed || !o.seed) return o.installed;
    installing = id;
    try {
      await api.onboardingNeed(h, id);
      await Promise.all([plugins.load(), loadOffers()]);
      return true;
    } catch (e) {
      refused[h] = e && typeof e === 'object' && 'message' in e ? String((e as { message: unknown }).message) : String(e);
      return false;
    } finally {
      installing = null;
    }
  }

  // The sources: choosing one installs its plugin, then its form opens (a source of a code host: the host's).
  let forges = $state<ReturnType<typeof ForgesSection>>();
  let hostForm = $state(false);
  // A choice is a gesture: what it starts runs once per choice, never again because the lists were read again.
  $effect(() => {
    const id = source;
    if (!id) return;
    untrack(() =>
      need('sources', id).then(async (ok) => {
        const host = BY_FORGE[id as Service];
        if (ok && host) {
          hostForm = true;
          await tick();
          void forges?.connectKind(host);
        }
      }),
    );
  });
  $effect(() => {
    const kind = forge;
    if (!kind) return;
    untrack(() =>
      need('forge', kind).then(async (ok) => {
        if (!ok) return;
        await tick();
        void forges?.connectKind(kind);
      }),
    );
  });

  // A slim row each, the one chosen whole: the list stays in view at 1024 × 728.
  const sourceRows = $derived(of('sources').map((o) => ({ ...o, disabled: !!o.denied, compact: true })));
  const forgeRows = $derived(of('forge').map((o) => ({ ...o, disabled: !!o.denied, compact: true })));
  const chosenSource = $derived(of('sources').find((o) => o.id === source));
  const chosenForge = $derived(of('forge').find((o) => o.id === forge));
  /** What a source imports and asks: said for the sources this version knows, in general for another one. */
  const KNOWN = ['jira', 'trello', 'github', 'linear', 'clickup', 'gitlab', 'azure'] as const;
  const asks = (id: string) =>
    (KNOWN as readonly string[]).includes(id)
      ? t(`onboarding.hookups.sources.asks.${id as (typeof KNOWN)[number]}`)
      : t('onboarding.hookups.sources.asks.other');
  /** The store's other plugins of the category, from its catalogue read before, neither installed nor in the seed. */
  function alsoStore(category: string): string[] {
    const own = new Set(offers.map((o) => o.plugin));
    return catalog
      .filter((p) => p.categories.includes(category) && !own.has(p.id) && !plugins.byId(p.id))
      .map((p) => textOf(p.name, app.settings.language) || p.id);
  }

  // The voice: Whisper on the machine (its model downloaded on the gesture, followed, cancelled), or a service on line
  // (its plugin, then its key in the keychain, as Réglages › Voix keeps it); then a sentence dictated, sent to nobody.
  let info = $state<VoiceInfo | null>(null);
  async function loadInfo() {
    info = await api.voiceInfo().catch(() => ({ devices: [], models: [] }));
  }
  $effect(() => {
    if (!ticked.voice) return;
    untrack(() => {
      if (!info) void loadInfo();
      void onlineVoice.loadKeys();
    });
  });
  let voiceMode = $state<'local' | 'online' | null>(null);
  let voiceService = $state<string | null>(null);
  const model = $derived(info?.models.find((m) => m.id === (app.settings.voice?.model || 'base')) ?? info?.models[0]);
  const progress = $derived(dictation.progress);
  const downloading = $derived(!!progress && progress.model === model?.id);

  /** The settings of the voice, written as Réglages › Voix writes them (the core installs nothing more for them). */
  async function saveVoice(patch: Partial<NonNullable<typeof app.settings.voice>>) {
    const settings = { ...$state.snapshot(app.settings), voice: { ...$state.snapshot(app.settings.voice), ...patch } };
    if (await app.run(api.saveSettings(settings).then(() => true))) app.settings = settings;
  }
  $effect(() => {
    const mode = voiceMode;
    untrack(() => {
      if (mode === 'local' && app.settings.voice?.transcription === 'online') void saveVoice({ transcription: 'local' });
    });
  });
  $effect(() => {
    const id = voiceService;
    if (!id || voiceMode !== 'online') return;
    untrack(() =>
      need('voice', id).then(async (ok) => {
        const o = of('voice').find((x) => x.id === id);
        if (ok || o?.installed) await saveVoice({ transcription: 'online', onlineService: id });
      }),
    );
  });
  async function download() {
    if (!model || !online) return;
    dictation.offer = { model: model.id, size: model.size };
    dictation.phase = 'offer';
    await dictation.download();
    await loadInfo();
  }
  const voiceRows = $derived([
    { id: 'local', disabled: false },
    { id: 'online', disabled: of('voice').every((o) => !!o.denied) },
  ]);
  const serviceRows = $derived(of('voice').map((o) => ({ ...o, disabled: !!o.denied, compact: true })));
  const service = $derived(voiceService ? onlineService(voiceService) : null);
  const keyed = $derived(!!voiceService && onlineVoice.keys.includes(voiceService));
  const voiceReady = $derived(
    voiceMode === 'local' ? !!model?.downloaded : voiceMode === 'online' ? keyed && app.settings.voice?.transcription === 'online' : false,
  );

  let said = $state('');
  let tryEl = $state<HTMLTextAreaElement>();
  const trial: DictationTarget = {
    kind: 'ticket',
    id: 'onboarding-voice',
    el: () => tryEl,
    insert: (text) => (said = said ? `${said} ${text}` : text),
  };

  /** « Continuer »: each card plugged in said done; each other one passed, to find again in the discovery. */
  async function next(): Promise<boolean> {
    // What was plugged in here (an account of a source, of a code host) is the core's to tell.
    await refresh();
    for (const h of CARDS) {
      if (cardStatus(h) === 'policy') continue;
      const ready = plugged(h) || (h === 'voice' && ticked.voice && voiceReady);
      await act({ kind: 'hookup', hookup: h, outcome: ready ? 'done' : 'skipped' });
    }
    return true;
  }
  $effect(() => {
    primary = { label: t('onboarding.continue'), run: next };
  });

  /** Échap in a field leaves the field, not the station. */
  function onkey(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    if (e.key === 'Escape' && el.closest('input, textarea, select')) {
      e.stopPropagation();
      el.blur();
    }
  }
  /** On a card: Espace ticks it, ↑ ↓ go from card to card; Entrée is the journey's (« Continuer »). */
  function cardKey(e: KeyboardEvent, h: Hookup) {
    if (e.key === ' ') {
      e.preventDefault();
      toggle(h);
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      e.stopPropagation();
      const all = [...document.querySelectorAll<HTMLElement>('.hk [data-card]')];
      const at = all.indexOf(e.currentTarget as HTMLElement);
      all[(at + (e.key === 'ArrowDown' ? 1 : -1) + all.length) % all.length]?.focus();
    }
  }

  const listOf = (h: Hookup) => fList(of(h).map((o) => o.name));
</script>

<!-- svelte-ignore a11y_no_static_element_interactions: its keys (↑ ↓ between cards, Échap in a field) are the cards'. -->
<div class="q hk" data-testid="onboarding-hookups" onkeydown={onkey}>
  <StationHead step="hookups" question={t('onboarding.hookups.question')}>
    {#snippet why()}{t('onboarding.hookups.why')}{/snippet}
  </StationHead>
  <div class="cards" role="group" aria-label={t('onboarding.hookups.cards')}>
    {#each CARDS as h (h)}
      {@const lock = lockOf(h)}
      {@const done = plugged(h)}
      {@const on = !lock && (ticked[h] || done)}
      {@const status = cardStatus(h)}
      <section class="card" class:on class:lock={!!lock} data-hookup={h}>
        <div class="head">
          <div
            class="tick"
            role="checkbox"
            tabindex="0"
            aria-checked={on}
            aria-disabled={lock ? true : undefined}
            aria-describedby="hk-{h}-d"
            data-card={h}
            onclick={() => toggle(h)}
            onkeydown={(e) => cardKey(e, h)}
          >
            <span class="box" aria-hidden="true"
              >{#if on}<Icon name="check" size={12} />{/if}</span
            >
            <span class="tt">{t(`onboarding.hookups.${h}.title`)}</span>
          </div>
          <span class="st">
            {#if lock}
              <span class="locked"><Icon name="lock" size={12} />{t('onboarding.hookups.locked', { by: lock })}</span>
            {:else if done}
              <span class="ok"><Icon name="check" size={12} />{t('onboarding.hookups.done')}</span>
            {:else if passed[h] || (status === 'skipped' && !ticked[h])}
              <span class="was">{t('onboarding.hookups.passed')}</span>
            {/if}
            {#if ticked[h] && !done}
              <button
                class="btn ghost small"
                aria-label={t('onboarding.hookups.laterLabel', { name: t(`onboarding.hookups.${h}.title`) })}
                onclick={() => later(h)}>{t('onboarding.hookups.later')}</button
              >
            {/if}
          </span>
        </div>
        <p class="d" id="hk-{h}-d">
          {h === 'voice' ? t('onboarding.hookups.voice.body') : t(`onboarding.hookups.${h}.body`, { list: listOf(h) })}
        </p>
        {#if ticked[h] && !lock}
          <div class="in">
            {#if h === 'sources'}
              <Choices items={sourceRows} bind:value={source} label={t('onboarding.hookups.sources.pick')}>
                {#snippet head(o)}
                  <span class="t"
                    ><b>{o.name}</b>{#if o.denied}<span class="no"><Icon name="lock" size={12} />{o.denied}</span>{/if}</span
                  >
                  <span class="dd">{asks(o.id)}</span>
                {/snippet}
              </Choices>
              {@const others = alsoStore('kanban')}
              {#if others.length}<p class="note">{t('onboarding.hookups.alsoStore', { list: fList(others) })}</p>{/if}
              {#if chosenSource}
                {@render plugin(chosenSource, 'sources')}
                {#if chosenSource.installed}
                  {#if !online}<p class="note warn" role="status">{t('onboarding.hookups.offline')}</p>{/if}
                  <div class="embed">
                    <SourceAccount
                      id={chosenSource.id as Service}
                      open
                      onforge={async (kind) => {
                        hostForm = true;
                        await tick();
                        void forges?.connectKind(kind);
                      }}
                    />
                    {#if hostForm && BY_FORGE[chosenSource.id as Service]}<ForgesSection bind:this={forges} />{/if}
                  </div>
                  <p class="note">{t('onboarding.hookups.sources.link')}</p>
                {/if}
              {/if}
            {:else if h === 'voice'}
              <Choices items={voiceRows} bind:value={voiceMode} label={t('onboarding.hookups.voice.pick')}>
                {#snippet head(r)}
                  {#if r.id === 'local'}
                    <span class="t"><b>{t('onboarding.hookups.voice.local')}</b></span>
                    <span class="dd"
                      >{model ? t('onboarding.hookups.voice.localHint', { model: model.id, size: fBytes(model.size) }) : ''}</span
                    >
                  {:else}
                    <span class="t"
                      ><b>{t('onboarding.hookups.voice.online')}</b>{#if r.disabled && of('voice')[0]?.denied}<span class="no"
                          ><Icon name="lock" size={12} />{of('voice')[0].denied}</span
                        >{/if}</span
                    >
                    <span class="dd">{t('onboarding.hookups.voice.onlineHint')}</span>
                  {/if}
                {/snippet}
              </Choices>
              {#if voiceMode === 'local' && model}
                <div class="dl" data-voice-model={model.id}>
                  {#if model.downloaded}
                    <span class="ok"><Icon name="check" size={13} />{t('onboarding.hookups.voice.ready', { model: model.id })}</span>
                  {:else if downloading && progress}
                    {@const pct = Math.round((progress.done / Math.max(1, progress.total)) * 100)}
                    <span
                      class="gauge"
                      role="progressbar"
                      aria-label={t('onboarding.hookups.voice.local')}
                      aria-valuemin="0"
                      aria-valuemax="100"
                      aria-valuenow={pct}><span class="fill" style:width="{pct}%"></span></span
                    >
                    <span class="dd"
                      >{t('onboarding.hookups.voice.downloading', { done: fBytes(progress.done), total: fBytes(progress.total) })}</span
                    >
                    <button class="btn" onclick={() => dictation.cancel()}>{t('onboarding.hookups.voice.cancel')}</button>
                  {:else}
                    <button class="btn" disabled={!online || dictation.phase !== 'idle'} onclick={download}
                      ><Icon name="download" size={13} />{t('onboarding.hookups.voice.download', { size: fBytes(model.size) })}</button
                    >
                    {#if !online}<p class="note warn" role="status">{t('onboarding.hookups.offline')}</p>{/if}
                  {/if}
                </div>
              {:else if voiceMode === 'online'}
                <Choices items={serviceRows} bind:value={voiceService} label={t('onboarding.hookups.voice.service')}>
                  {#snippet head(o)}
                    <span class="t"><b>{o.name}</b></span>
                  {/snippet}
                </Choices>
                {#if voiceService}
                  {@const o = of('voice').find((x) => x.id === voiceService)}
                  {#if o}{@render plugin(o, 'voice')}{/if}
                  {#if service && o?.installed}
                    <p class="note">{t('onboarding.hookups.voice.sends', { service: service.label, host: service.host })}</p>
                    {#if !online}<p class="note warn" role="status">{t('onboarding.hookups.offline')}</p>{/if}
                    <div class="embed"><OnlineKey {service} /></div>
                  {/if}
                {/if}
              {/if}
              {#if voiceReady}
                <div class="try">
                  <b>{t('onboarding.hookups.voice.tryTitle')}</b>
                  <p class="dd">{t('onboarding.hookups.voice.tryHint')}</p>
                  <div class="tryrow">
                    <textarea class="field" rows="2" aria-label={t('onboarding.hookups.voice.tryField')} bind:this={tryEl} bind:value={said}
                    ></textarea>
                    <MicButton target={trial} below />
                  </div>
                </div>
                <p class="note">{t('onboarding.hookups.voice.conversation')}</p>
              {/if}
            {:else}
              <Choices items={forgeRows} bind:value={forge} label={t('onboarding.hookups.forge.pick')}>
                {#snippet head(o)}
                  <span class="t"
                    ><b>{o.name}</b>{#if o.denied}<span class="no"><Icon name="lock" size={12} />{o.denied}</span>{/if}</span
                  >
                {/snippet}
              </Choices>
              {@const others = alsoStore('forges')}
              {#if others.length}<p class="note">{t('onboarding.hookups.alsoStore', { list: fList(others) })}</p>{/if}
              {#if chosenForge}
                {@render plugin(chosenForge, 'forge')}
                {#if chosenForge.installed}
                  {#if !online}<p class="note warn" role="status">{t('onboarding.hookups.offline')}</p>{/if}
                  <div class="embed"><ForgesSection bind:this={forges} /></div>
                {/if}
              {/if}
            {/if}
            {#if refused[h]}<p class="note bad" role="alert">{refused[h]}</p>{/if}
          </div>
        {/if}
      </section>
    {/each}
  </div>
</div>

{#snippet plugin(o: HookupOffer, h: Hookup)}
  {#if installing === o.id}
    <p class="note" role="status">{t('onboarding.hookups.plugin', { plugin: o.name })}</p>
  {:else if !o.installed && !o.seed}
    <p class="note">{t('onboarding.hookups.pluginStore')}</p>
  {:else if o.installed && !refused[h]}
    <p class="note ok"><Icon name="check" size={12} />{t('onboarding.hookups.pluginInstalled', { plugin: o.name })}</p>
  {/if}
{/snippet}

<style>
  .q {
    display: grid;
    gap: 22px;
    width: min(680px, 100%);
  }
  .cards {
    display: grid;
    gap: 10px;
  }
  .card {
    display: grid;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.025);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .card.on {
    background: color-mix(in oklch, var(--brand) 5%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--brand);
  }
  .card.lock {
    opacity: 0.7;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .tick {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 12px;
    min-width: 0;
    border-radius: 6px;
    cursor: pointer;
    outline: none;
  }
  .card.lock .tick {
    cursor: default;
  }
  .tick:focus-visible {
    box-shadow: 0 0 0 2px var(--p-ink);
  }
  .box {
    display: grid;
    flex: none;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    color: #08090a;
    box-shadow: inset 0 0 0 1.5px var(--line-3);
  }
  .card.on .box {
    background: var(--brand);
    box-shadow: none;
  }
  .tt {
    color: var(--text);
    font-weight: 620;
  }
  .st {
    display: inline-flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }
  .ok,
  .locked,
  .was,
  .no {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .ok {
    color: var(--teal-ink);
  }
  .locked,
  .was,
  .no {
    color: var(--muted);
  }
  .small {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
  .d {
    margin: 0 0 0 28px;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.55;
  }
  .in {
    display: grid;
    gap: 12px;
    margin: 8px 0 2px 28px;
  }
  .t {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .t b {
    color: var(--text);
    font-weight: 600;
  }
  .no {
    font-size: 12px;
  }
  .dd {
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .note.ok {
    color: var(--teal-ink);
  }
  .note.warn {
    color: var(--warn-ink);
  }
  .note.bad {
    color: var(--del);
  }
  .embed {
    padding: 0 12px;
    border-radius: 8px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .dl {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .gauge {
    position: relative;
    width: 180px;
    height: 6px;
    overflow: hidden;
    border-radius: 3px;
    background: var(--line-2);
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    background: var(--brand);
  }
  .try {
    display: grid;
    gap: 6px;
  }
  .try b {
    font-weight: 600;
  }
  .try .dd {
    margin: 0;
  }
  .tryrow {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .tryrow textarea {
    flex: 1;
    min-width: 0;
    resize: vertical;
  }
  @media (max-width: 1100px) {
    .card {
      padding: 10px 12px;
    }
    .d,
    .in {
      margin-left: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
</style>
