<script lang="ts">
  import { engineName } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { modelOptions } from '../../lib/models';
  import { app } from '../../lib/state.svelte';
  import { DEFAULT_WORKTREES } from '../../lib/worktrees';
  import { validationText } from '../../lib/wizard';
  import type { WizardForm } from '../../lib/wizard-form.svelte';
  import Switch from '../settings/Switch.svelte';
  import Checks from './Checks.svelte';

  // « Récapitulatif »: every check that fails, with its correction, what will be set, and the first agent (new
  // project only).
  let { form }: { form: WizardForm } = $props();

  const c = $derived(form.config);
  const b = $derived(c.board);
  const rows = $derived([
    [
      t('wizard.summary.rows.worktrees'),
      `${c.worktreePerAgent ? t('wizard.summary.perAgent') : t('wizard.summary.shared')} · ${c.worktreeDir.trim() || DEFAULT_WORKTREES}`,
    ],
    [
      t('wizard.summary.rows.setup'),
      c.worktreeSetup
        .map((s) => s.command.trim())
        .filter(Boolean)
        .join(' ; ') || t('wizard.summary.noneValue'),
    ],
    [
      t('wizard.summary.rows.launch'),
      c.runCommands.length ? t('wizard.summary.commands', { count: c.runCommands.length }) : t('wizard.summary.noneValue'),
    ],
    [t('wizard.summary.rows.validation'), validationText(b)],
    [t('wizard.summary.rows.conflicts'), t(`boardSettings.tab.conflicts.${b.conflict}`)],
    [
      t('wizard.summary.rows.agents'),
      [
        t('wizard.summary.agentsValue', { count: b.maxParallel }),
        b.model,
        b.effort,
        b.mode,
        b.autopilot ? t('wizard.summary.autopilot') : '',
      ]
        .filter(Boolean)
        .join(' · '),
    ],
    [t('wizard.summary.rows.harness'), engineName(c.engine ?? 'claude')],
    [
      t('wizard.summary.rows.private'),
      c.private
        ? [
            c.reserved.length ? t('wizard.summary.privateValue', { count: c.reserved.length }) : t('wizard.summary.privateNoAccount'),
            c.sharedOnLimit ? t('wizard.summary.sharedOnLimit') : '',
          ]
            .filter(Boolean)
            .join(' · ')
        : t('wizard.no'),
    ],
    [t('wizard.summary.rows.commit'), t(`settings.project.commitModes.${c.commitMode}`)],
    [t('wizard.summary.rows.integrations'), form.integrations.links.map((l) => l.name).join(', ') || t('wizard.summary.noneValue')],
  ]);
</script>

<Checks {form} />
<dl class="sum">
  {#each rows as [k, v] (k)}
    <div>
      <dt>{k}</dt>
      <dd>{v}</dd>
    </div>
  {/each}
</dl>
{#if !form.project}
  <!-- « Indexer le code » (ESC-254): offered at the adding, as « Index du code sur tous les projets » says (ESC-293). -->
  <div class="agent index" data-wizard-code-index>
    <div class="tx">
      <span class="tt">{t('codeindex.wizard.title')}</span>
      <span class="td">{t('codeindex.what')}</span>
      <span class="td">{t('codeindex.local')}</span>
      {#if c.private}<span class="td">{t('codeindex.privateNote')}</span>{/if}
    </div>
    <Switch label={t('codeindex.wizard.title')} bind:on={form.codeIndex} />
  </div>
  <div class="agent">
    <div class="tx">
      <span class="tt">{t('wizard.summary.firstAgent')}</span>
      <span class="td">{t('wizard.summary.firstAgentDesc')}</span>
    </div>
    <Switch label={t('wizard.summary.firstAgent')} bind:on={form.firstAgent} />
    {#if form.firstAgent}
      <div class="w-chips models" role="group" aria-label={t('common.model')}>
        {#each modelOptions(app.models) as m (m.value)}
          <button class="w-chip" aria-pressed={form.model === m.value} onclick={() => (form.model = m.value)}>{m.label}</button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  /* What will be set, a row each (the preview's `.sum`). */
  .sum {
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    margin: 0;
    border-top: 1px solid var(--line);
  }
  .sum div {
    display: contents;
  }
  dt,
  dd {
    margin: 0;
    padding: 8px 0;
    border-bottom: 1px solid rgba(var(--fg-rgb), 0.05);
    font-size: 12.5px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    min-width: 0;
    color: var(--soft);
    overflow-wrap: anywhere;
  }
  .agent {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 14px;
  }
  .tx {
    flex: 1;
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .tt {
    font-weight: 550;
  }
  .td {
    color: var(--muted);
    font-size: 12px;
  }
  .models {
    flex-basis: 100%;
  }
</style>
