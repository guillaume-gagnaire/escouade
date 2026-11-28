<script lang="ts">
  // The sheet of the projects (MO-7): each space that said its projects, its plate and name, then its projects; the one
  // shown is checked. Touched, a project is shown and the sheet closes. The system's back, the scrim and « Fermer »
  // close it.
  import { onMount } from 'svelte';
  import Icon from '$shared/components/Icon.svelte';
  import SpacePlate from '$shared/components/servers/SpacePlate.svelte';
  import { t } from '$shared/lib/i18n';
  import { choose, kanban } from '../lib/board.svelte';
  import { closeSheet } from '../lib/nav.svelte';
  import { phone } from '../lib/phone-spaces.svelte';

  let sheet = $state<HTMLElement>();
  const spaces = $derived(phone.spaces.filter((s) => kanban.lists[s.id]?.length));

  onMount(() => {
    const before = document.activeElement as HTMLElement | null;
    sheet?.focus();
    return () => before?.focus?.();
  });

  function pick(space: string, project: string) {
    choose({ space, project });
    closeSheet();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" aria-hidden="true" onclick={closeSheet}></div>
<div
  class="sheet"
  role="dialog"
  aria-modal="true"
  aria-labelledby="projects-title"
  tabindex="-1"
  bind:this={sheet}
  onkeydown={(e) => e.key === 'Escape' && closeSheet()}
>
  <span class="grab" aria-hidden="true"></span>
  <div class="hd">
    <h2 id="projects-title">{t('mobile.kanban.projects')}</h2>
    <button type="button" class="close" onclick={closeSheet}>{t('mobile.kanban.close')}</button>
  </div>
  <div class="list">
    {#each spaces as s (s.id)}
      <h3><SpacePlate name={s.name} here={s.here} size={20} />{s.name}</h3>
      <ul>
        {#each kanban.lists[s.id] as p (p.id)}
          {@const current = kanban.choice?.space === s.id && kanban.choice?.project === p.id}
          <li>
            <button type="button" aria-current={current ? 'true' : undefined} onclick={() => pick(s.id, p.id)}>
              <i class="dot" style:background={p.color} aria-hidden="true"></i>
              <span class="nm">{p.name}</span>
              {#if current}<Icon name="check" size={18} />{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/each}
  </div>
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
    max-height: 80%;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
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
    flex: none;
    width: 38px;
    height: 5px;
    border-radius: 3px;
    background: var(--line-3);
    margin: 8px auto 4px;
  }
  .hd {
    flex: none;
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
  .list {
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 14px 0 6px;
    font-size: var(--small);
    font-weight: 650;
    color: var(--muted);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--gap-target);
  }
  li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: calc(var(--target) + 4px);
    padding: 0 14px;
    border: 0;
    border-radius: var(--r-l);
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--text);
    font-size: var(--body);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  li button[aria-current='true'] {
    box-shadow: inset 0 0 0 1px var(--line-3);
    background: var(--raise-2);
  }
  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
