<script lang="ts">
  import { t } from '../../lib/i18n';
  import { CONTROL_STEP, type ControlId, type StepId } from '../../lib/wizard';
  import type { WizardForm } from '../../lib/wizard-form.svelte';
  import Icon from '../Icon.svelte';

  // What would keep the way of working from working, each with its correction: those of one step (`step`), or all of
  // them (the summary, which also says when there is none).
  let { form, step }: { form: WizardForm; step?: StepId } = $props();

  const shown = $derived(form.problems.filter((id) => !step || CONTROL_STEP[id] === step));
  /** The checks the assistant corrects by itself. */
  const FIXES: Partial<Record<ControlId, string>> = {
    unread: 'wizard.checks.unread.fix',
    noCommit: 'wizard.checks.noCommit.fix',
    noTarget: 'wizard.checks.noTarget.fix',
    tracked: 'wizard.checks.tracked.fix',
  };
  /** What is wrong: git's error itself when it failed to read the folder (ESC-687). */
  function title(id: ControlId) {
    if (id === 'unread') return t('wizard.checks.unread.title', { error: form.readError ?? '' });
    return t(`wizard.checks.${id}.title`, { branch: form.config.board.target });
  }
  let busy = $state(false);

  async function fix(id: ControlId) {
    busy = true;
    try {
      await form.fix(id);
    } finally {
      busy = false;
    }
  }
</script>

{#if shown.length}
  <ul class="checks">
    {#each shown as id (id)}
      <li class="ck bad" data-check={id}>
        <Icon name="shield" size={14} />
        <div class="tx">
          <b>{title(id)}</b>
          <p>{t(`wizard.checks.${id}.desc`)}</p>
        </div>
        {#if FIXES[id]}
          <button class="btn" disabled={busy} onclick={() => fix(id)}>{t(FIXES[id] as 'wizard.checks.noCommit.fix')}</button>
        {/if}
      </li>
    {/each}
  </ul>
{:else if !step && !form.setup && form.checking}
  <!-- Not read yet: neither ready nor without commit. -->
  <p class="ck wait"><Icon name="clock" size={14} /><b>{t('wizard.checks.reading')}</b></p>
{:else if !step}
  <p class="ck ok"><Icon name="check-c" size={14} /><b>{t('wizard.checks.ok')}</b></p>
{/if}

<style>
  /* What would keep the way of working from working, in red with its correction; or that nothing would (the preview's
     `.ck`). */
  .checks {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .ck {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    align-items: start;
    gap: 10px;
    margin: 0;
    padding: 11px 13px;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .ck > :global(.icon) {
    margin-top: 2px;
  }
  .ck.bad {
    background: oklch(from var(--del) l c h / 0.06);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .ck.bad > :global(.icon) {
    color: var(--del);
  }
  .ck.ok > :global(.icon) {
    color: var(--add);
  }
  .ck.wait > :global(.icon) {
    color: var(--muted);
  }
  .tx {
    min-width: 0;
  }
  b {
    font-weight: 600;
  }
  .tx p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
    text-wrap: pretty;
  }
</style>
