<script lang="ts">
  // « À toi » (MO-1, direction B): the agents that stopped first, the longest waiting first, then the requests of the
  // box, of the spaces the chip shows; empty, it says what will come there. MO-5 makes each line unfold and answer.
  import { t } from '$shared/lib/i18n';
  import { ALL, boxAsks, countsOnlySpaces, phone, shownSpaces, stoppedAgents } from '../lib/phone-spaces.svelte';
  import Empty from './Empty.svelte';
  import Row from './Row.svelte';

  const shown = $derived(shownSpaces(phone.spaces, phone.filter));
  const stopped = $derived(stoppedAgents(shown));
  const asks = $derived(boxAsks(shown));
  const hidden = $derived(countsOnlySpaces(shown));
  const one = $derived(phone.filter === ALL ? null : shown[0]);
</script>

{#each hidden as s (s.id)}
  <p class="note">{t('mobile.inbox.countsOnly', { name: s.name, waiting: s.glance?.waiting ?? 0 })}</p>
{/each}
{#if stopped.length}
  <h2 class="grp waits">{t('mobile.inbox.stopped')} <span class="c">{stopped.length}</span></h2>
  <ul>
    {#each stopped as x (`${x.space.id}:${x.item.id}`)}
      <Row
        icon={x.item.request?.kind === 'question' ? 'question' : 'shield'}
        title={x.item.name}
        kind={x.item.request?.kind === 'question'
          ? t('spaces.kind.question')
          : x.item.request
            ? t('spaces.kind.permission')
            : t('spaces.kind.waits')}
        space={x.space}
        project={x.project}
        waits
      />
    {/each}
  </ul>
{/if}
{#if asks.length}
  <h2 class="grp">{t('mobile.inbox.asks')} <span class="c">{asks.length}</span></h2>
  <ul>
    {#each asks as x (`${x.space.id}:${x.item.id}`)}
      <Row icon="inbox" title={x.item.title} kind={t('spaces.kind.ask')} space={x.space} project={x.project} />
    {/each}
  </ul>
{/if}
{#if !stopped.length && !asks.length && !hidden.length}
  <Empty
    icon="inbox"
    title={one ? t('mobile.inbox.emptyIn', { name: one.name }) : t('mobile.inbox.empty')}
    text={t('mobile.inbox.emptyText')}
  />
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .grp {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    padding: 16px var(--gut) 6px;
    font-size: var(--small);
    font-weight: 650;
    color: var(--muted);
  }
  .grp.waits {
    color: var(--wait);
  }
  .c {
    font-variant-numeric: tabular-nums;
    opacity: 0.8;
  }
  .note {
    margin: 12px var(--gut) 0;
    padding: 10px 12px;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: var(--small);
    color: var(--soft);
  }
</style>
