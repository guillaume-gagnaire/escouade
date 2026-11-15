<script lang="ts">
  import { accountsOf } from '../../lib/accounts';
  import { shortcutLabel, shortcutOf } from '../../lib/dictation';
  import { dictation } from '../../lib/dictation.svelte';
  import { engineOptions, engineOf } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { defaultShortcut, overseer } from '../../lib/overseer.svelte';
  import { api } from '../../lib/ipc';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { OverseerSettings } from '../../lib/types';
  import Directives from '../overseer/Directives.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // Réglages › Overseer (ESC-462, docs/OVERSEER.md § « Comment il est fait »): whether it runs, its harness, its account,
  // its model (Haiku on Claude Code by default) and its effort; then the user's instructions, each withdrawn by a click.
  // Its global shortcut (ESC-468): registered as it is chosen, refused when another app holds it.
  const DEFAULTS: OverseerSettings = {
    enabled: false,
    engine: 'claude',
    account: '',
    model: 'haiku',
    effort: '',
    shortcut: defaultShortcut(),
  };
  const s = $derived(settingsForm.settings);
  const o = $derived<OverseerSettings>({ ...DEFAULTS, ...(s.overseer ?? {}) });
  const set = (patch: Partial<OverseerSettings>) => (s.overseer = { ...o, ...patch });
  const harnesses = $derived(engineOptions(o.engine));
  const engine = $derived(engineOf(o.engine));
  const accounts = $derived(accountsOf(o.engine));
  const models = $derived(engine?.models ?? []);
  const efforts = $derived(engine?.efforts ?? []);
  /**
   * Turned on, it needs Escouade's tools: « Claude et les agents peuvent piloter Escouade » goes on first, through its own
   * call (as its switch in Agents does: it declares the server in the harnesses), then the Overseer.
   */
  async function enable(on: boolean) {
    if (on && !app.settings.mcpEnabled) {
      const saved = await app.run(api.mcpSetEnabled(true).then(() => true));
      if (!saved) return;
      app.settings.mcpEnabled = true;
      s.mcpEnabled = true;
    }
    set({ enabled: on });
  }

  // « Raccourci global »: the keys pressed after « Changer » are the new one, registered at once; refused, it says why.
  let recording = $state(false);
  let shortcutError = $state<string | null>(null);
  const shortcut = $derived(o.shortcut ?? defaultShortcut());
  const hotkey = $derived(overseer.view.shortcut);

  async function choose(spec: string) {
    shortcutError = null;
    try {
      await api.overseerShortcutSet(spec);
      set({ shortcut: spec });
    } catch (e) {
      shortcutError = String(e);
    }
  }

  function record(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') return stopRecording();
    const s = shortcutOf(e);
    if (!s) return;
    stopRecording();
    void choose(s);
  }

  function startRecording() {
    recording = true;
    dictation.recording = true;
  }

  function stopRecording() {
    recording = false;
    dictation.recording = false;
  }

  $effect(() => () => (dictation.recording = false));
</script>

<svelte:window onkeydowncapture={record} />

<Group title={t('overseer.settings.groupAgent')}>
  <Row label={t('overseer.settings.enabled')} desc={t('overseer.settings.enabledDesc')} row="overseerAgent">
    <Switch label={t('overseer.settings.enabled')} bind:on={() => o.enabled, (v) => void enable(v)} />
  </Row>
  {#if o.enabled && !app.settings.mcpEnabled}
    <p class="warn" role="note">{t('overseer.settings.needsMcp')}</p>
  {/if}
  {#if harnesses.length > 1}
    <Row label={t('overseer.settings.harness')} desc={t('overseer.settings.harnessDesc')}>
      <Chips
        label={t('overseer.settings.harness')}
        options={harnesses}
        bind:value={() => o.engine, (v) => set({ engine: v, account: '', model: v === 'claude' ? 'haiku' : '', effort: '' })}
      />
    </Row>
  {/if}
  <Row label={t('overseer.settings.account')} desc={t('overseer.settings.accountDesc')}>
    <select
      class="field select"
      aria-label={t('overseer.settings.account')}
      value={o.account}
      onchange={(e) => set({ account: e.currentTarget.value })}
    >
      <option value="">{t('overseer.settings.accountAuto')}</option>
      {#each accounts as a (a.id)}
        <option value={a.id}>{a.name}</option>
      {/each}
    </select>
  </Row>
  <Row label={t('overseer.settings.model')} desc={t('overseer.settings.modelDesc')}>
    <select
      class="field select"
      aria-label={t('overseer.settings.model')}
      value={o.model}
      onchange={(e) => set({ model: e.currentTarget.value })}
    >
      {#if !models.some((m) => m.id === o.model)}
        <option value={o.model}>{o.model || '—'}</option>
      {/if}
      {#each models as m (m.id)}
        <option value={m.id}>{m.label}</option>
      {/each}
    </select>
  </Row>
  {#if efforts.length}
    <Row label={t('overseer.settings.effort')}>
      <select
        class="field select"
        aria-label={t('overseer.settings.effort')}
        value={o.effort}
        onchange={(e) => set({ effort: e.currentTarget.value })}
      >
        <option value="">{t('overseer.settings.effortDefault')}</option>
        {#each efforts as e (e)}
          <option value={e}>{e}</option>
        {/each}
      </select>
    </Row>
  {/if}
  <Row label={t('overseer.settings.memory')} desc={t('overseer.settings.memoryDesc')} />
</Group>

<Group title={t('overseer.settings.groupScreen')}>
  <Row label={t('overseer.settings.shortcut')} desc={t('overseer.settings.shortcutDesc')} row="overseerShortcut">
    <div class="keys">
      <kbd data-overseer-shortcut
        >{recording
          ? t('overseer.settings.shortcutRecording')
          : shortcut
            ? shortcutLabel(shortcut)
            : t('overseer.settings.shortcutNone')}</kbd
      >
      <button class="btn ghost small" onclick={() => (recording ? stopRecording() : startRecording())}
        >{t('overseer.settings.shortcutChange')}</button
      >
      {#if shortcut !== defaultShortcut()}
        <button class="btn ghost small" onclick={() => void choose(defaultShortcut())}>{t('overseer.settings.shortcutDefault')}</button>
      {/if}
      {#if shortcut}
        <button class="btn ghost small" aria-label={t('overseer.settings.shortcutRemove')} onclick={() => void choose('')}
          >{t('overseer.screen.remove')}</button
        >
      {/if}
    </div>
  </Row>
  {#if hotkey?.unsupported}
    <p class="warn" role="note" data-shortcut-wayland>{t('overseer.settings.shortcutWayland')}</p>
  {:else if shortcutError || hotkey?.error}
    <p class="warn" role="alert" data-shortcut-error>{shortcutError ?? hotkey?.error}</p>
  {:else if shortcut}
    <p class="note" data-shortcut-state>
      {hotkey?.active === shortcut ? t('overseer.settings.shortcutActive') : t('overseer.settings.shortcutOff')}
    </p>
  {/if}
</Group>

<Group title={t('overseer.settings.groupDirectives')}>
  <Row label={t('overseer.settings.groupDirectives')} desc={t('overseer.settings.directivesDesc')} wide row="overseerDirectives">
    <Directives />
  </Row>
  {#if overseer.error}
    <p class="warn" role="alert">{overseer.error}</p>
  {/if}
</Group>

<style>
  .keys {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .keys kbd {
    padding: 2px 8px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    font: inherit;
    font-size: 12.5px;
  }
  .note {
    color: var(--muted);
    font-size: 12.5px;
    margin: 4px 0 8px;
  }
  .warn {
    color: var(--warn-ink);
    font-size: 12.5px;
    margin: 4px 0 8px;
  }
</style>
