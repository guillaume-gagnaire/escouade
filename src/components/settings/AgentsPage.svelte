<script lang="ts">
  import { anyCan, writerOf, writerOptions } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { settingsForm, type NumberSetting } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import McpActivity from './McpActivity.svelte';
  import McpSection from './McpSection.svelte';
  import NumberField from './NumberField.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // The page « Agents » (PROPOSITION.md §2): « Nouveaux agents » and the delay before a message is sent during a turn
  // (from « Tous les harness »), « Textes rédigés » (from « Application »), the remote control of every agent (from its
  // harness's group), « Piloter Escouade » (the 2.0 tab « Serveur MCP »). « Avancé »: the stop of the idle processes, what
  // the MCP server was asked.
  const s = $derived(settingsForm.settings);
  /** Why a number typed is not saved (ESC-380), the one saved staying. */
  const numberError = (key: NumberSetting) =>
    settingsForm.badNumbers.has(key) ? t('settings.problem.number', { value: app.settings[key] }) : '';
  const midTurnDelayOptions = $derived(
    [0, 5, 10, 20, 30].map((n) => ({
      value: n,
      label: n ? t('settings.claude.midTurnDelaySecs', { s: n }) : t('settings.claude.midTurnDelayOff'),
    })),
  );
  /** « Modèle des textes rédigés »: the harnesses found. */
  const writers = $derived(writerOptions());
  /** The harness the texts go to, when it takes the model chosen here (OpenCode): its models, as its account listed them. */
  const writer = $derived(writerOf(s.writerEngine) ?? writers[0]?.value ?? null);
  const writerModels = $derived(
    writer && app.engines.find((e) => e.id === writer)?.writerTakesModel ? (app.engineModels[writer] ?? []) : null,
  );
  /** Its default model (« Par défaut »), named. */
  const writerDefault = $derived(writerModels?.find((m) => m.default));
  /** What the menu shows: the model chosen while it is one of them, else the default. */
  const writerModel = $derived(writerModels?.some((m) => m.id === s.writerModel) ? (s.writerModel ?? '') : '');
  /** The harnesses that have a remote control (Claude Code): the others have none, which the row says. */
  const remote = $derived(app.engines.filter((e) => e.caps.remoteControl));
</script>

<!-- Settings of what only some harnesses do: shown while one of them is there. -->
<Group title={t('settings.groups.newAgents')}>
  {#if anyCan('terminalView')}
    <Row label={t('settings.claude.terminalView')} desc={t('settings.claude.terminalViewDesc')}>
      <Chips
        label={t('settings.claude.terminalView')}
        options={[
          { value: 'escouade', label: t('settings.claude.terminalViewEscouade') },
          { value: 'harness', label: t('settings.claude.terminalViewHarness') },
        ]}
        bind:value={() => (s.terminalView ? 'harness' : 'escouade'), (v) => (s.terminalView = v === 'harness')}
      />
    </Row>
  {/if}
  {#if anyCan('todoTools')}
    <Row label={t('settings.claude.todoTools')} desc={t('settings.claude.todoToolsDesc')}>
      <Switch label={t('settings.claude.todoTools')} bind:on={s.todoTools} />
    </Row>
  {/if}
  <Row label={t('settings.claude.midTurnDelay')} desc={t('settings.claude.midTurnDelayDesc')}>
    <Chips label={t('settings.claude.midTurnDelay')} options={midTurnDelayOptions} bind:value={s.midTurnDelaySecs} />
  </Row>
</Group>

<!-- Only the harnesses found are offered; none found, nothing to choose. -->
{#if writers.length}
  <Group title={t('settings.app.writing')}>
    <Row label={t('settings.app.writer')} desc={t('settings.app.writerHelp')}>
      <Chips
        label={t('settings.app.writer')}
        options={writers}
        bind:value={() => writerOf(s.writerEngine) ?? writers[0].value, (v) => (s.writerEngine = v)}
      />
    </Row>
    {#if writerModels}
      <Row label={t('settings.app.writerModel')} desc={t('settings.app.writerModelHelp')}>
        <select
          class="field select"
          aria-label={t('settings.app.writerModel')}
          disabled={!writerModels.length}
          value={writerModel}
          onchange={(e) => (s.writerModel = e.currentTarget.value)}
        >
          <option value=""
            >{writerDefault
              ? t('settings.app.writerModelDefault', { model: writerDefault.label })
              : t('settings.app.writerModelNone')}</option
          >
          {#each writerModels as m (m.id)}
            <option value={m.id}>{m.provider ? `${m.label} · ${m.provider}` : m.label}</option>
          {/each}
        </select>
      </Row>
    {/if}
  </Group>
{/if}

{#if remote.length}
  <!-- Said where it is set: the other harnesses have none (lot 0 of the 2.0 design). -->
  <Group title={t('settings.groups.remote')}>
    <Row label={t('settings.claude.remoteAll')} desc={t('settings.claude.remoteAllDesc')}>
      {#snippet extra()}
        <span class="only"
          ><Icon name="info" size={14} />{t('settings.claude.remoteOnly', { harness: remote.map((e) => e.name).join(', ') })}</span
        >
      {/snippet}
      <Switch label={t('settings.claude.remoteAll')} bind:on={s.remoteControlAll} />
    </Row>
  </Group>
{/if}

<McpSection />

<Group advanced="agents">
  <Row label={t('settings.claude.idleStop')} error={numberError('idleStopMinutes')} errorId="idleStopMinutes-error">
    <NumberField key="idleStopMinutes" label={t('settings.claude.idleStopLabel')} hint={t('settings.claude.idleStopHint')} />
  </Row>
  <McpActivity />
</Group>

<style>
  .select {
    width: 260px;
    max-width: 100%;
  }
</style>
