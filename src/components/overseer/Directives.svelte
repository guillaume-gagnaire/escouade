<script lang="ts">
  import { t } from '../../lib/i18n';
  import { actionsLabel, overseer, scopeLabel } from '../../lib/overseer.svelte';

  // The user's instructions (« consignes », ESC-462): each with what it allows, where, and the words of the user's message
  // it was born of; « Retirer » withdraws it. Shown in Réglages › Overseer and in its discussion.
  const directives = $derived(overseer.view.directives);
</script>

{#if directives.length}
  <ul class="directives" data-directives>
    {#each directives as d (d.id)}
      <li data-directive={d.id}>
        <div class="what">
          <strong>{d.summary}</strong>
          <span class="scope">{actionsLabel(d.actions)}{' · '}{scopeLabel(d)}</span>
          <span class="from">{t('overseer.directive.from', { quote: d.source.quote })}</span>
        </div>
        <button
          class="btn ghost small"
          aria-label={t('overseer.directive.removeTitle', { summary: d.summary })}
          onclick={() => overseer.revoke(d.id)}>{t('overseer.directive.remove')}</button
        >
      </li>
    {/each}
  </ul>
{:else}
  <p class="none" data-directives-none>{t('overseer.settings.none')}</p>
{/if}

<style>
  .directives {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--bg-2, transparent);
  }
  .what {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .scope,
  .from {
    color: var(--muted);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .none {
    color: var(--muted);
    margin: 0;
  }
</style>
