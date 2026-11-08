<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '../../lib/i18n';
  import { rowsOf, type SettingsPage } from '../../lib/settings-pages';
  import { settingsForm } from '../../lib/settings.svelte';
  import Icon from '../Icon.svelte';

  // A group of settings under its title, a hairline above the next (`.sg` of the 2.0 design): its note under the title,
  // then its rows. Beside the title, `state` (a harness's, « Prêt · 2.1.301 ») and `action` (« Ajouter un compte… », on
  // the right); other attributes go to the section. `anchor`: what the modal scrolls to when opened on it. `plain` is
  // kept for the callers that laid out their own box: there is none any more.
  //
  // `advanced` (direction A, design/settings-2-2/index.html#a-comptes): « Avancé » at the foot of a page, folded into a
  // quiet row that names what it hides, read from the register (the rows of the page given that are in « Avancé »), or
  // given (`hides`). Unfolded in place; `open` is bindable (the search, R8, unfolds it).
  let {
    title,
    note,
    plain: _plain = false,
    anchor,
    state,
    action,
    advanced,
    hides,
    open = $bindable(false),
    children,
    ...rest
  }: {
    title?: string;
    note?: string;
    plain?: boolean;
    anchor?: string;
    state?: Snippet;
    action?: Snippet;
    /** Folded, naming the rows of this page that are in « Avancé ». */
    advanced?: SettingsPage;
    /** What it names folded, in place of the register's. */
    hides?: string[];
    open?: boolean;
    children: Snippet;
    [attr: string]: unknown;
  } = $props();

  // A row of this « Avancé » found by the search: unfolded, for the window to light it.
  $effect(() => {
    const f = settingsForm.found;
    if (advanced && f?.row.advanced && f.row.page === advanced) open = true;
  });

  const names = $derived(
    hides ??
      (advanced
        ? rowsOf(advanced)
            .filter((r) => r.advanced)
            .map((r) => r.label)
        : []),
  );
</script>

{#if advanced || hides}
  <section class="sg folded" data-anchor={anchor} {...rest}>
    <button class="fold" aria-expanded={open} onclick={() => (open = !open)}
      ><Icon name={open ? 'down' : 'right'} size={14} /><span class="ft">{title ?? t('settings.fold.advanced')}</span
      >{#if !open && names.length}<span class="names">{names.join(' · ')}</span>{/if}</button
    >
    {#if open}
      {#if note}<p class="note">{note}</p>{/if}
      {@render children()}
    {/if}
  </section>
{:else}
  <section class="sg" data-anchor={anchor} {...rest}>
    {#if state || action}
      <div class="hg">
        <h2>{title}</h2>
        {#if state}{@render state()}{/if}
        {#if action}<span class="act">{@render action()}</span>{/if}
      </div>
    {:else if title}
      <h2>{title}</h2>
    {/if}
    {#if note}<p class="note">{note}</p>{/if}
    {@render children()}
  </section>
{/if}

<style>
  .sg {
    padding: 20px 0 4px;
  }
  :global(.sg) + .sg {
    border-top: 1px solid var(--line);
  }
  h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 620;
  }
  .hg {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .hg h2 {
    flex: none;
  }
  .act {
    display: inline-flex;
    gap: 8px;
    margin-left: auto;
  }
  .note {
    max-width: 64ch;
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
    text-wrap: pretty;
  }
  /* « Avancé », folded: a quiet row that opens the rare settings in place. */
  .sg.folded {
    padding: 0;
    border-top: none;
  }
  .fold {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    margin-top: 8px;
    padding: 12px 0;
    border: none;
    border-top: 1px solid var(--line);
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 550;
    text-align: left;
    cursor: pointer;
  }
  .fold:hover,
  .fold[aria-expanded='true'] {
    color: var(--text);
  }
  .fold :global(.icon) {
    flex: none;
    color: var(--muted);
  }
  .fold .ft {
    flex: none;
  }
  .names {
    min-width: 0;
    margin-left: 4px;
    overflow: hidden;
    color: var(--dim);
    font-weight: 400;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
