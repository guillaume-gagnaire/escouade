<script lang="ts">
  import { t } from '../../lib/i18n';
  import { filterNames, labelColor } from '../../lib/prcontrol';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  // A picker of the control of a pull request (ESC-488): who to ask for a review, or which labels to put, among what the
  // host offers (`pr_candidates`). Each box ticked or unticked is one gesture, sent on the click. The names are the
  // host's: shown as text, a colour kept only when it is six hexadecimal digits.
  let {
    label,
    items,
    chosen,
    busy = null,
    failure = null,
    onToggle,
    onClose,
  }: {
    label: string;
    /** Null while the host is read. */
    items: { name: string; color?: string; description?: string }[] | null;
    chosen: string[];
    /** The name whose gesture is on its way. */
    busy?: string | null;
    failure?: string | null;
    onToggle: (name: string, on: boolean) => void;
    onClose: () => void;
  } = $props();

  let query = $state('');
  const shown = $derived(items ? filterNames(items, (x) => x.name, query) : []);

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="picker" role="group" aria-label={label} onkeydown={onKey}>
  <div class="ph">
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="q"
      type="search"
      aria-label={t('prs.page.control.search')}
      placeholder={t('prs.page.control.search')}
      bind:value={query}
      autofocus
    />
    <button class="icon-btn" aria-label={t('prs.page.control.close')} onclick={onClose}><Icon name="close" size={12} /></button>
  </div>
  {#if failure}<p class="fail" role="alert"><Icon name="info" size={12} />{failure}</p>{/if}
  {#if !items}
    <p class="muted"><span class="spinner" aria-hidden="true"></span>{t('prs.page.control.loading')}</p>
  {:else}
    <ul>
      {#each shown as it (it.name)}
        {@const on = chosen.includes(it.name)}
        {@const color = labelColor(it.color)}
        <li>
          <label class:busy={busy === it.name} use:tip={it.description ? { text: it.description, aria: false } : null}>
            <input type="checkbox" checked={on} disabled={busy !== null} onchange={() => onToggle(it.name, !on)} />
            {#if color}<i class="dot" style:background={color}></i>{/if}
            <span class="ell">{it.name}</span>
          </label>
        </li>
      {:else}
        <li class="muted">{t('prs.page.control.noneFound')}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .picker {
    margin-top: 6px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 6px 24px oklch(0 0 0 / 0.18);
    padding: 6px;
    display: grid;
    gap: 4px;
  }
  .ph {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .q {
    flex: 1;
    min-width: 0;
    height: 26px;
    font-size: 12.5px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 200px;
    overflow: auto;
  }
  li.muted {
    padding: 4px 6px;
  }
  label {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 6px;
    border-radius: 5px;
    min-width: 0;
    cursor: pointer;
  }
  label:hover {
    background: var(--hover);
  }
  label.busy {
    opacity: 0.6;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
    box-shadow: inset 0 0 0 1px oklch(0 0 0 / 0.2);
  }
  .ell {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fail {
    margin: 0;
    display: flex;
    gap: 6px;
    color: var(--del);
    font-size: 12px;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
