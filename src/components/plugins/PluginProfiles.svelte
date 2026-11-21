<script lang="ts">
  import { onMount } from 'svelte';
  import { locale, t } from '../../lib/i18n';
  import {
    kindsOf,
    nameOf,
    profilesApi,
    profilesOf,
    summaryOf,
    titleOf,
    usedBy,
    type Profile,
    type ProfileKind,
    type ProfileSecrets,
  } from '../../lib/plugin-profiles';
  import { textOf, type PluginView } from '../../lib/plugins';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import ProfileModal from './ProfileModal.svelte';

  // A plugin's profiles on its page (ESC-530, issue #38, design/plugins-contract-2-3 « b-plugin »): a group by kind, its
  // title and its count, « Ajouter un profil… »; a row by profile, its name, the values of its other fields, the projects
  // that chose it, how many of its secrets are kept, « Modifier ». The window of a profile makes, changes and deletes it.
  let { plugin }: { plugin: PluginView } = $props();

  const lang = $derived(locale.ui);
  const kinds = $derived(kindsOf(plugin.manifest));
  let secrets = $state<ProfileSecrets[]>([]);
  let editing = $state<{ kind: ProfileKind; profile: Profile | null } | null>(null);

  async function readSecrets() {
    try {
      secrets = await profilesApi.secrets(plugin.id);
    } catch {
      secrets = [];
    }
  }

  const keptOf = (p: Profile) => secrets.find((s) => s.profile === p.id)?.secrets.filter((s) => s.kept).length ?? 0;
  const projectsOf = (kind: string, p: Profile) =>
    usedBy(plugin, kind, p.id)
      .map((id) => app.projects.find((x) => x.id === id))
      .filter((x) => !!x);

  onMount(() => void readSecrets());
</script>

{#each kinds as kind (kind.id)}
  {@const list = profilesOf(plugin, kind.id)}
  {@const title = titleOf(kind.title, lang, kind.id)}
  <section class="grp profiles" aria-label={title}>
    <div class="gh">
      <h4>{title}</h4>
      <span class="cnt">{list.length}</span>
      <button class="btn sm add" type="button" onclick={() => (editing = { kind, profile: null })}
        ><Icon name="plus" size={13} />{t('plugins.profiles.add')}</button
      >
    </div>
    <p class="gsub">{kind.description ? textOf(kind.description, lang) : t('plugins.profiles.intro')}</p>
    {#each list as p (p.id)}
      {@const kept = keptOf(p)}
      {@const used = projectsOf(kind.id, p)}
      <div class="prof" role="group" aria-label={nameOf(kind, p)}>
        <span class="av" aria-hidden="true"><Icon name="globe" size={14} /></span>
        <div class="who">
          <b>{nameOf(kind, p)}</b>
          {#if summaryOf(kind, p)}<span class="sub">{summaryOf(kind, p)}</span>{/if}
        </div>
        <span class="used">
          {#each used as project (project.id)}
            <span class="chip" style:--c={project.color}><i></i>{project.name}</span>
          {:else}
            <span class="muted">{t('plugins.profiles.noProject')}</span>
          {/each}
        </span>
        {#if kind.fields.some((f) => f.secret)}
          <span class="keptpill" class:ok={kept > 0}
            >{kept > 0 ? t('plugins.profiles.kept', { count: kept }) : t('plugins.profiles.keptNone')}</span
          >
        {/if}
        <button class="btn ghost sm" type="button" onclick={() => (editing = { kind, profile: p })}>{t('plugins.profiles.edit')}</button>
      </div>
    {:else}
      <p class="hint">{t('plugins.profiles.none')}</p>
    {/each}
  </section>
{/each}

{#if editing}
  <ProfileModal
    {plugin}
    kind={editing.kind}
    profile={editing.profile}
    secrets={editing.profile ? (secrets.find((s) => s.profile === editing?.profile?.id)?.secrets ?? []) : []}
    onclose={() => {
      editing = null;
      void readSecrets();
    }}
  />
{/if}

<style>
  section {
    display: grid;
    margin-bottom: 18px;
  }
  .gh {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0 6px;
  }
  .gh h4 {
    margin: 0;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
  }
  .cnt {
    color: var(--dim);
    font-size: 12px;
  }
  .add {
    margin-left: auto;
  }
  .btn.sm {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
  .gsub {
    max-width: 62ch;
    margin: -2px 0 6px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .prof {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 10px 0;
    border-top: 1px solid var(--line);
  }
  .av {
    display: grid;
    flex: none;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
  }
  .who {
    display: grid;
    flex: 1 1 160px;
    gap: 2px;
    min-width: 0;
  }
  .who b {
    font-size: 12.5px;
    font-weight: 560;
  }
  .sub {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .used {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 7px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 11px;
    font-weight: 550;
  }
  .chip i {
    width: 6px;
    height: 6px;
    border-radius: 2px;
    background: var(--c);
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .keptpill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 8px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .keptpill.ok {
    background: color-mix(in oklch, var(--run) 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--run) 30%, transparent);
    color: var(--run);
  }
  .keptpill.ok::before {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .hint {
    margin: 0;
    padding: 10px 0;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 12px;
  }
</style>
