<script lang="ts">
  import { t } from '../../lib/i18n';
  import { modelMenu, setFavorite, type MenuModel } from '../../lib/engines';
  import { app } from '../../lib/state.svelte';
  import type { EngineId } from '../../lib/types';
  import Star from '../Star.svelte';

  // The models a harness listed for its providers (OpenCode), searched, each marked as a favorite or not, grouped as in the
  // composer's menu (the favorites first, then by provider). Kept at once, as the accounts are (not part of the settings'
  // draft).
  let { engine }: { engine: EngineId } = $props();

  let query = $state('');
  const models = $derived(app.engineModels[engine] ?? []);
  // The models searched, by group: a group the search leaves empty is not shown.
  const sections = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const out: { name: string; models: MenuModel[] }[] = [];
    for (const m of modelMenu({ engine })) {
      if (q && !`${m.label} ${m.provider ?? ''} ${m.id} ${m.group}`.toLowerCase().includes(q)) continue;
      const last = out.at(-1);
      if (last?.name === m.group) last.models.push(m);
      else out.push({ name: m.group, models: [m] });
    }
    return out;
  });
</script>

<div class="favorites" data-testid="model-favorites-{engine}">
  <input
    class="field search"
    type="search"
    bind:value={query}
    placeholder={t('settings.claude.models.search')}
    aria-label={t('settings.claude.models.search')}
  />
  {#if !models.length}
    <p class="empty">{t('settings.claude.models.none')}</p>
  {:else}
    <ul aria-label={t('settings.claude.models.label')}>
      {#each sections as section (section.name)}
        <li class="gtitle" aria-hidden="true">{section.name}</li>
        {#each section.models as m (m.id)}
          <li>
            <button
              class="star"
              class:on={m.favorite}
              aria-pressed={m.favorite}
              aria-label={t(m.favorite ? 'nav.dropdown.unfavorite' : 'nav.dropdown.favorite') + ' : ' + m.label}
              onclick={() => setFavorite(engine, m.id, !m.favorite)}><Star on={m.favorite} /></button
            >
            <span class="name">{m.label}</span>
            {#if m.provider && m.favorite}<span class="provider">{m.provider}</span>{/if}
            <span class="id mono">{m.id}</span>
          </li>
        {/each}
      {:else}
        <li class="empty">{t('nav.dropdown.none')}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .favorites {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }
  .search {
    max-width: 320px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: 7px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    font-size: 12.5px;
  }
  li + li {
    border-top: 1px solid var(--line);
  }
  /* The star of the composer's menu: an outline without a background, filled in the accent's color for a favorite. */
  .star {
    display: flex;
    align-items: center;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: color-mix(in oklch, var(--dim) 70%, transparent);
    cursor: pointer;
    padding: 2px;
  }
  li:hover .star,
  .star:focus-visible {
    color: var(--muted);
  }
  .star:hover {
    color: var(--text);
  }
  .star.on {
    color: var(--p-ink);
  }
  .gtitle {
    padding-top: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
  }
  .name {
    font-weight: 600;
  }
  .provider,
  .id {
    color: var(--muted);
  }
  .id {
    margin-left: auto;
    font-size: 11px;
  }
  .empty {
    color: var(--muted);
    font-size: 12.5px;
    margin: 0;
  }
</style>
