<script lang="ts">
  import { tick } from 'svelte';
  import { t } from '../../lib/i18n';
  import { IS_MAC } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import type { WorktreeStep } from '../../lib/types';

  // The commands a project runs in its worktrees (their setup, or their teardown, `kind`), in order: shell, command
  // line, folder of the worktree.
  let { steps = $bindable(), kind, onadd }: { steps: WorktreeStep[]; kind: 'setup' | 'teardown'; onadd: () => void } = $props();

  /** Alt as the system writes it in a shortcut. */
  const ALT = IS_MAC ? '⌥' : 'Alt+';

  /**
   * Moves step `i` one place up (-1) or down (1). The focus stays on what had it in the step, which moves with it
   * (its rows are keyed); on the other button when the one pressed has no way left to go.
   */
  async function move(i: number, by: -1 | 1) {
    const to = i + by;
    if (to < 0 || to >= steps.length) return;
    const focused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const [step] = steps.splice(i, 1);
    steps.splice(to, 0, step);
    await tick();
    if (!focused?.isConnected) return;
    const other = focused.closest('.step')?.querySelector<HTMLButtonElement>(by < 0 ? '.down' : '.up');
    if (focused instanceof HTMLButtonElement && focused.disabled && other) other.focus();
    else if (document.activeElement !== focused) focused.focus();
  }

  /** Alt+↑ / Alt+↓ on any field of a step moves it: not with Ctrl (Ctrl+Alt is AltGr), Shift or Cmd. */
  function onKeydown(e: KeyboardEvent, i: number) {
    if (!e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
    // Taken from the field: a list would open, a cursor would go to the start of the line.
    e.preventDefault();
    move(i, e.key === 'ArrowUp' ? -1 : 1);
  }
</script>

<div class="steps">
  {#each steps as s, i (s.id)}
    <!-- The keys come from the step's fields and buttons, which keep their own. -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="step" role="group" aria-label={t(`settings.steps.${kind}.group`, { n: i + 1 })} onkeydown={(e) => onKeydown(e, i)}>
      <span class="n mono" aria-hidden="true">{i + 1}</span>
      <select class="field" aria-label={t('settings.fields.shell')} bind:value={s.shell}>
        {#each app.shells as sh (sh.id)}<option value={sh.id}>{sh.label}</option>{/each}
        {#if !app.shells.some((sh) => sh.id === s.shell)}<option value={s.shell}
            >{t('settings.fields.shellNotFound', { name: s.shell || t('settings.fields.shell') })}</option
          >{/if}
      </select>
      <input
        class="field mono cmd"
        aria-label={t('common.command')}
        placeholder={t('settings.steps.commandPlaceholder')}
        spellcheck="false"
        bind:value={s.command}
      />
      <input
        class="field mono dir"
        aria-label={t('settings.fields.subfolder')}
        title={t('settings.steps.subfolderTitle')}
        placeholder={t('settings.steps.subfolderPlaceholder')}
        spellcheck="false"
        bind:value={s.cwd}
      />
      <button
        class="icon up"
        title={t('settings.steps.up', { keys: `${ALT}↑` })}
        aria-label={t('settings.steps.upN', { n: i + 1 })}
        aria-keyshortcuts="Alt+ArrowUp"
        disabled={i === 0}
        onclick={() => move(i, -1)}>↑</button
      >
      <button
        class="icon down"
        title={t('settings.steps.down', { keys: `${ALT}↓` })}
        aria-label={t('settings.steps.downN', { n: i + 1 })}
        aria-keyshortcuts="Alt+ArrowDown"
        disabled={i === steps.length - 1}
        onclick={() => move(i, 1)}>↓</button
      >
      <button
        class="icon del"
        title={t('common.delete')}
        aria-label={t('settings.steps.deleteN', { n: i + 1 })}
        onclick={() => steps.splice(i, 1)}>×</button
      >
    </div>
  {/each}
  <div>
    <button class="btn" aria-label={t(`settings.steps.${kind}.add`)} onclick={onadd}>{t('settings.fields.addCommand')}</button>
  </div>
</div>

<style>
  .steps {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .step {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .n {
    width: 16px;
    font-size: 11px;
    color: var(--dim);
    text-align: right;
  }
  .field {
    height: 32px;
    background: var(--panel);
    font-size: 12.5px;
  }
  select.field {
    width: 150px;
  }
  .cmd {
    flex: 1;
    min-width: 180px;
  }
  .dir {
    width: 130px;
  }
  .icon {
    width: 32px;
    height: 32px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 15px;
    cursor: pointer;
  }
  .up,
  .down {
    font-size: 13px;
  }
  .up:hover:not(:disabled),
  .down:hover:not(:disabled) {
    border-color: var(--text);
    color: var(--text);
  }
  .icon:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .del:hover {
    border-color: var(--del);
    color: var(--del);
  }
</style>
