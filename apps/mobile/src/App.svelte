<script lang="ts">
  // The phone's shell, direction B « le fil » (MO-1, docs/MOBILE.md § 6 and design/mobile-2-3/): the header (the tab's
  // title and the « + »), the chips of the spaces, the band of a space that is cut, the tab's content, and the three
  // tabs at the bottom; the sheet of the « + » above. With no space paired, « Escouade sur ce téléphone ».
  import Icon from '$shared/components/Icon.svelte';
  import IconSprite from '$shared/components/IconSprite.svelte';
  import { t } from '$shared/lib/i18n';
  import AgentsTab from './components/AgentsTab.svelte';
  import CreateSheet from './components/CreateSheet.svelte';
  import InboxTab from './components/InboxTab.svelte';
  import KanbanTab from './components/KanbanTab.svelte';
  import SpaceBands from './components/SpaceBands.svelte';
  import SpaceChips from './components/SpaceChips.svelte';
  import TabBar from './components/TabBar.svelte';
  import Welcome from './components/Welcome.svelte';
  import { nav, openSheet } from './lib/nav.svelte';
  import { phone } from './lib/phone-spaces.svelte';

  let body = $state<HTMLElement>();

  // A tab opens at its top.
  $effect(() => {
    void nav.tab;
    body?.scrollTo({ top: 0 });
  });
</script>

<IconSprite />
{#if !phone.spaces.length}
  <Welcome />
{:else}
  <div class="shell" inert={nav.sheet !== null}>
    <header class="head">
      <div class="r">
        <h1>{t(`mobile.tabs.${nav.tab}`)}</h1>
        <button type="button" class="plus" aria-label={t('mobile.create.label')} aria-haspopup="dialog" onclick={() => openSheet('create')}>
          <Icon name="plus" size={22} />
        </button>
      </div>
      {#if nav.tab !== 'kanban'}<SpaceChips />{/if}
    </header>
    <div class="body" id="panel" role="tabpanel" aria-labelledby="tab-{nav.tab}" bind:this={body}>
      <SpaceBands />
      {#if nav.tab === 'inbox'}
        <InboxTab />
      {:else if nav.tab === 'agents'}
        <AgentsTab />
      {:else}
        <KanbanTab />
      {/if}
    </div>
    <TabBar />
  </div>
{/if}
{#if nav.sheet === 'create'}<CreateSheet />{/if}

<style>
  .shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--panel);
  }
  .head {
    flex: none;
    display: grid;
    gap: 10px;
    padding: calc(6px + var(--safe-top)) calc(var(--gut) + var(--safe-right)) 8px calc(var(--gut) + var(--safe-left));
  }
  .r {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: var(--target);
  }
  h1 {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  .plus {
    flex: none;
    width: var(--target);
    height: var(--target);
    margin-right: -6px;
    border: 0;
    border-radius: var(--r-m);
    display: grid;
    place-items: center;
    background: transparent;
    color: var(--soft);
    cursor: pointer;
  }
  .plus:active {
    background: var(--hover);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0 var(--safe-right) 12px var(--safe-left);
  }
</style>
