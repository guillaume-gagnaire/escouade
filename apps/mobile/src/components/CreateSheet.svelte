<script lang="ts">
  // The sheet of the header's « + » (MO-1, docs/MOBILE.md § 3.12): what the phone will create, a project on a server,
  // an agent and its first message, a dictated ticket. Its gestures come with MO-10 and MO-11: here, they are named,
  // not offered. Below, the spaces of this phone (MO-4): « Retirer » one (the core asked to revoke it, then its keys
  // forgotten), « Appairer un espace ». The system's back, the scrim and « Fermer » close it.
  import { onMount } from 'svelte';
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import type { IconName } from '$shared/lib/icons';
  import { closeSheet } from '../lib/nav.svelte';
  import { keptSpaces } from '../lib/paired';
  import { openPairing, removeSpace } from '../lib/pairing.svelte';
  import { phone } from '../lib/phone-spaces.svelte';

  let sheet = $state<HTMLElement>();
  let confirming = $state<string | null>(null);
  let removing = $state(false);
  const kept = new Set(keptSpaces().map((s) => s.id));

  async function remove(id: string) {
    removing = true;
    try {
      await removeSpace(id, true);
    } finally {
      removing = false;
      confirming = null;
    }
  }

  function pairAnother() {
    closeSheet();
    openPairing();
  }
  const ITEMS: { icon: IconName; title: 'project' | 'agent' | 'ticket' }[] = [
    { icon: 'folder-plus', title: 'project' },
    { icon: 'agents', title: 'agent' },
    { icon: 'mic', title: 'ticket' },
  ];

  onMount(() => {
    const before = document.activeElement as HTMLElement | null;
    sheet?.focus();
    return () => before?.focus?.();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') closeSheet();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" aria-hidden="true" onclick={closeSheet}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-labelledby="create-title" tabindex="-1" bind:this={sheet} onkeydown={onKey}>
  <span class="grab" aria-hidden="true"></span>
  <div class="hd">
    <h2 id="create-title">{t('mobile.create.title')}</h2>
    <button type="button" class="close" onclick={closeSheet}>{t('mobile.create.close')}</button>
  </div>
  <ul>
    {#each ITEMS as item (item.title)}
      <li>
        <span class="k"><Icon name={item.icon} size={18} /></span>
        <span class="tx">
          <span class="tt">{t(`mobile.create.${item.title}`)}</span>
          <span class="ht">{t(`mobile.create.${item.title}Hint`)}</span>
        </span>
      </li>
    {/each}
  </ul>
  <p class="soon">{t('mobile.create.soon')}</p>
  <h3 class="sub">{t('mobile.manage.title')}</h3>
  <ul class="spaces">
    {#each phone.spaces as s (s.id)}
      <li class="space">
        <span class="k"><Icon name={s.here ? 'monitor' : 'remote'} size={18} /></span>
        {#if confirming === s.id}
          <span class="tx"><span class="ht" role="alert">{t('mobile.manage.confirm', { name: s.name })}</span></span>
          <button type="button" class="rm danger" disabled={removing} onclick={() => remove(s.id)}>{t('mobile.manage.confirmYes')}</button>
        {:else}
          <span class="tx"><span class="tt">{s.name}</span></span>
          {#if kept.has(s.id)}
            <button
              type="button"
              class="rm"
              aria-label={t('mobile.manage.removeLabel', { name: s.name })}
              onclick={() => (confirming = s.id)}>{t('mobile.manage.remove')}</button
            >
          {/if}
        {/if}
      </li>
    {/each}
  </ul>
  <button type="button" class="add" onclick={pairAnother}><Icon name="plus" size={18} />{t('mobile.manage.add')}</button>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 10;
    background: var(--scrim);
  }
  .sheet {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 11;
    max-width: 600px;
    margin: 0 auto;
    background: var(--raise);
    border-radius: 18px 18px 0 0;
    box-shadow:
      inset 0 1px 0 var(--line-2),
      0 -20px 50px -12px rgba(0, 0, 0, calc(0.6 * var(--shade)));
    padding: 0 calc(var(--gut) + var(--safe-right)) calc(12px + var(--safe-bottom)) calc(var(--gut) + var(--safe-left));
    outline: none;
  }
  .grab {
    display: block;
    width: 38px;
    height: 5px;
    border-radius: 3px;
    background: var(--line-3);
    margin: 8px auto 4px;
  }
  .hd {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 680;
    letter-spacing: -0.015em;
  }
  .close {
    min-height: var(--target);
    padding: 0 12px;
    margin-right: -12px;
    border: 0;
    border-radius: var(--r-m);
    background: transparent;
    color: var(--p-ink);
    font-size: var(--body);
    font-weight: 600;
    cursor: pointer;
  }
  ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: grid;
    gap: var(--gap-target);
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: calc(var(--target) + 12px);
    padding: 8px 12px;
    border-radius: var(--r-l);
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }
  .k {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    flex: none;
    background: var(--raise-2);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .tx {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .tt {
    color: var(--soft);
    font-weight: 620;
  }
  .ht {
    font-size: var(--small);
  }
  .sub {
    margin: 18px 0 8px;
    font-size: var(--small);
    font-weight: 650;
    color: var(--muted);
    text-transform: none;
  }
  .spaces .space {
    color: var(--soft);
  }
  .space .tx {
    flex: 1;
  }
  .rm {
    flex: none;
    min-height: var(--target);
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-m);
    background: transparent;
    color: var(--p-ink);
    font-size: var(--body);
    font-weight: 600;
    cursor: pointer;
  }
  .rm.danger {
    color: var(--del);
  }
  .add {
    margin-top: 10px;
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: calc(var(--target) + 4px);
    border: 0;
    border-radius: var(--r-l);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: var(--body);
    font-weight: 620;
    cursor: pointer;
  }
  .soon {
    margin: 12px 0 0;
    font-size: var(--small);
    color: var(--muted);
    text-align: center;
  }
</style>
