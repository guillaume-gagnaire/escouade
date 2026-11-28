<script lang="ts">
  // The phone's shell, direction B « le fil » (MO-1, docs/MOBILE.md § 6 and design/mobile-2-3/): the header (the tab's
  // title and the « + »), the chips of the spaces (on the Kanban, its project and its columns: MO-7), the band of a
  // space that is cut, the tab's content, and the three tabs at the bottom; the sheet of the « + » (or of the Kanban's
  // projects) above. An agent's conversation and the sheets of MO-5 (a render in the viewer, a PDF, a file, a command
  // read in full screen) above the tabs. With no space paired, « Escouade sur ce téléphone » and its pairing (MO-4); a
  // space that revoked this phone, its own screen above all.
  import Icon from '$shared/components/Icon.svelte';
  import IconSprite from '$shared/components/IconSprite.svelte';
  import { t } from '$shared/lib/i18n';
  import AgentsTab from './components/AgentsTab.svelte';
  import CommandSheet from './components/CommandSheet.svelte';
  import CreateSheet from './components/CreateSheet.svelte';
  import FileSheet from './components/FileSheet.svelte';
  import InboxTab from './components/InboxTab.svelte';
  import KanbanHead from './components/KanbanHead.svelte';
  import KanbanTab from './components/KanbanTab.svelte';
  import ProjectSheet from './components/ProjectSheet.svelte';
  import SpaceBands from './components/SpaceBands.svelte';
  import SpaceChips from './components/SpaceChips.svelte';
  import Pairing from './components/Pairing.svelte';
  import PdfSheet from './components/PdfSheet.svelte';
  import Revoked from './components/Revoked.svelte';
  import TabBar from './components/TabBar.svelte';
  import Talk from './components/Talk.svelte';
  import Viewer from './components/Viewer.svelte';
  import { pairing } from './lib/pairing.svelte';
  import { nav, openSheet } from './lib/nav.svelte';
  import { phone, revokedSpaces } from './lib/phone-spaces.svelte';

  let body = $state<HTMLElement>();
  const revoked = $derived(revokedSpaces(phone.spaces)[0] ?? null);

  // A tab opens at its top.
  $effect(() => {
    void nav.tab;
    body?.scrollTo({ top: 0 });
  });
</script>

<IconSprite />
{#if revoked}
  <Revoked space={revoked} />
{:else if !phone.spaces.length || pairing.open}
  <Pairing />
{:else}
  <div class="shell" inert={nav.sheet !== null || nav.talk !== null}>
    <header class="head">
      <div class="r">
        <h1>{t(`mobile.tabs.${nav.tab}`)}</h1>
        <button type="button" class="plus" aria-label={t('mobile.create.label')} aria-haspopup="dialog" onclick={() => openSheet('create')}>
          <Icon name="plus" size={22} />
        </button>
      </div>
      {#if nav.tab !== 'kanban'}<SpaceChips />{:else}<KanbanHead />{/if}
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
{#if nav.talk && !revoked}
  {#key `${nav.talk.space}:${nav.talk.agent}`}<div class="layer" inert={nav.sheet !== null}><Talk talk={nav.talk} /></div>{/key}
{/if}
{#if nav.sheet === 'create'}<CreateSheet />{:else if nav.sheet === 'project'}<ProjectSheet />{:else if nav.sheet === 'viewer'}<Viewer
  />{:else if nav.sheet === 'pdf'}<PdfSheet />{:else if nav.sheet === 'file'}<FileSheet />{:else if nav.sheet === 'command'}<CommandSheet
  />{/if}

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
