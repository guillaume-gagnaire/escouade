<script lang="ts">
  // « Agents » (MO-1, direction B): the agents of the spaces the chip shows, those that wait first, then those at work
  // with what they do. MO-5 opens a conversation from here.
  import { t } from '$shared/lib/i18n';
  import { ALL, allAgents, phone, shownSpaces } from '../lib/phone-spaces.svelte';
  import Empty from './Empty.svelte';
  import Row from './Row.svelte';

  const shown = $derived(shownSpaces(phone.spaces, phone.filter));
  const agents = $derived(allAgents(shown));
  const one = $derived(phone.filter === ALL ? null : shown[0]);
</script>

{#if agents.length}
  <ul>
    {#each agents as x (`${x.space.id}:${x.item.id}`)}
      <Row
        icon={x.item.status === 'waiting' ? 'shield' : 'agents'}
        title={x.item.name}
        kind={x.item.status === 'waiting' ? t('spaces.kind.waits') : x.item.activity || t('spaces.kind.working')}
        space={x.space}
        project={x.project}
        waits={x.item.status === 'waiting'}
      />
    {/each}
  </ul>
{:else}
  <Empty
    icon="agents"
    title={one ? t('mobile.agents.emptyIn', { name: one.name }) : t('mobile.agents.empty')}
    text={t('mobile.agents.emptyText')}
  />
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 4px 0 0;
  }
</style>
