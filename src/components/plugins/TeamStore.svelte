<script lang="ts">
  // « Équipe » in the view Plugins (TS-14): the plugins each team of this device published, signed by its key. Their
  // origin says « Acme · privé »; « Installer » reads the version from the team's server, checks it again, and opens the
  // window of its permissions: installing stays the member's gesture. A plugin of the same id from elsewhere is never
  // replaced; a version the team withdrew does not install.
  import { onMount, untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { app } from '../../lib/state.svelte';
  import { errorText } from '../../lib/teams.svelte';
  import { privateLabel, rowAction, teamPluginsApi, type TeamPluginRow, type TeamPluginsView } from '../../lib/team-plugins';
  import PluginTile from './PluginTile.svelte';

  let stores = $state<TeamPluginsView[] | null>(null);
  let failure = $state('');

  onMount(async () => {
    try {
      stores = (await teamPluginsApi.all()) ?? [];
    } catch (e) {
      failure = errorText(e);
      stores = [];
    }
  });

  // Read again once a plugin is installed: its line says so.
  $effect(() => {
    void plugins.list.length;
    untrack(() => {
      if (stores)
        void teamPluginsApi.all().then(
          (s) => (stores = s ?? []),
          () => {},
        );
    });
  });

  function install(team: string, r: TeamPluginRow) {
    void plugins.fromTeam(team, r.id, r.version);
  }
</script>

<div class="tstore">
  {#if stores === null}
    <p class="hint">{t('teamPlugins.loading')}</p>
  {:else if !stores.length}
    <p class="hint">{failure || t('teamPlugins.store.none')}</p>
  {:else}
    {#each stores as s (s.team)}
      <section aria-label={privateLabel(s.name, t('teamPlugins.store.private'))} data-testid="team-store">
        <div class="shead">
          <h3>{privateLabel(s.name, t('teamPlugins.store.private'))}</h3>
          {#if s.key}<span class="ns mono">{s.key.namespace}</span>{/if}
          <span class="grow"></span>
          <button class="lnk" onclick={() => (app.modal = { kind: 'team', step: 'plugins', teamId: s.team })}
            >{t('teamPlugins.store.manage')}</button
          >
        </div>
        {#if s.conflict}<p class="warn" role="alert">{t('teamPlugins.conflict', { id: s.conflict })}</p>{/if}
        {#if !s.key}
          <p class="hint">{t('teamPlugins.store.noKey', { team: s.name })}</p>
        {:else if !s.plugins.length}
          <p class="hint">{t('teamPlugins.store.empty', { team: s.name })}</p>
        {/if}
        {#each s.plugins as r (r.id)}
          {@const action = rowAction(r)}
          <div class="line" data-testid="team-store-plugin" data-id={r.id}>
            <PluginTile id={r.id} name={r.name || r.id} />
            <div class="what">
              <b>{r.name || r.id}</b>
              <span class="mono sub">{r.id} · {t('plugins.version', { version: r.version })}</span>
              {#if r.description}<span class="sub">{r.description}</span>{/if}
              <span class="chip">{privateLabel(s.name, t('teamPlugins.store.private'))}</span>
              {#if action === 'withdrawn'}<span class="warn">{t('teamPlugins.store.withdrawn', { reason: r.withdrawn ?? '' })}</span>{/if}
              {#if action === 'elsewhere'}<span class="warn">{t('teamPlugins.store.elsewhere')}</span>{/if}
            </div>
            {#if action === 'install'}
              <button class="btn primary" disabled={!!s.conflict} onclick={() => install(s.team, r)}
                >{t('teamPlugins.store.install')}</button
              >
            {:else if action === 'update'}
              <button class="btn warn" disabled={!!s.conflict} onclick={() => install(s.team, r)}
                >{t('teamPlugins.store.update', { version: r.version })}</button
              >
            {:else if action === 'installed'}
              <span class="ok">{t('teamPlugins.store.installed', { version: r.installed ?? '' })}</span>
            {:else}
              <span></span>
            {/if}
          </div>
        {/each}
      </section>
    {/each}
  {/if}
</div>

<style>
  .tstore {
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .shead {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 620;
    color: var(--text);
  }
  .ns {
    font-size: 11.5px;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  .mono {
    font-family: var(--mono);
  }
  .line {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 12px;
    align-items: center;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .what {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .what b {
    color: var(--text);
    font-weight: 600;
  }
  .sub {
    font-size: 12px;
    color: var(--muted);
  }
  .chip {
    align-self: flex-start;
    margin-top: 2px;
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 11px;
    color: var(--soft);
    background: var(--hover);
  }
  .hint {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .warn {
    font-size: 12px;
    color: var(--warn-ink);
  }
  .ok {
    font-size: 12px;
    color: var(--run);
  }
  .lnk {
    border: 0;
    background: none;
    padding: 0;
    font: inherit;
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
  }
</style>
