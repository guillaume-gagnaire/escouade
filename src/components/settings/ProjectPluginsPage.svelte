<script lang="ts">
  import { onMount } from 'svelte';
  import { locale, t } from '../../lib/i18n';
  import { tooBig, visibleLines, type FieldSetting } from '../../lib/plugin-fields';
  import {
    chosenProfile,
    defaultProfile,
    fromRepo,
    kindsOf,
    nameOf,
    profilesApi,
    profilesOf,
    projectFields,
    projectPlugins,
    projectValue,
    proposedName,
    proposedProfile,
    summaryOf,
    teamRefusals,
  } from '../../lib/plugin-profiles';
  import { lockLine } from '../../lib/locks.svelte';
  import { textOf, type PluginView } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import LinedText from '../plugins/LinedText.svelte';
  import PluginTile from '../plugins/PluginTile.svelte';

  // Réglages › Projets › Plugins, a project's (ESC-530, issue #39, design/plugins-contract-2-3 « b-projet »): what the plugins
  // on in this project ask for it, their settings of each project (`"scope": "project"`). The page is in the bar only while
  // a plugin installed and on here declares one (the rule of 2026-11-11: no offer « Non installé » among the settings).
  // A group by plugin, its tile, its name, its id and « Page du plugin ». A setting says where its value comes from, as the
  // other settings of a project: « Défini par .escouade.yml », or this machine's ahead of the file, with « Écrire dans
  // .escouade.yml » (that key alone, comments kept) and « Reprendre la valeur du fichier ». A profile is chosen here, kept
  // on this machine, never in the file: a file that names one only proposes it, in a band, and the user chooses.
  // The project's team (TS-13, ESC-532): a simple setting it imposes is greyed, with the padlock and « Imposé par l'équipe
  // Acme », and nothing goes ahead of it (neither this machine's value nor « Écrire dans .escouade.yml »); one it
  // recommends says so, under the file and this machine. A key of its file the plugin refuses is said, with why.
  let { project }: { project: Project } = $props();

  const lang = $derived(locale.ui);
  const shown = $derived(projectPlugins(plugins.list, project));
  let refused = $state<Record<string, string>>({});
  let error = $state<string | null>(null);

  const keyOf = (p: PluginView, f: FieldSetting) => `${p.id}:${f.key}`;
  const titleOf = (f: FieldSetting) => textOf(f.title, lang) || f.key;

  async function set(p: PluginView, f: FieldSetting, value: unknown, clear = false) {
    error = null;
    delete refused[keyOf(p, f)];
    try {
      plugins.put(await profilesApi.setProject(p.id, project.id, f.key, value, clear));
    } catch (e) {
      error = String(e);
    }
  }

  function setText(p: PluginView, f: FieldSetting, v: string) {
    const big = tooBig(v);
    if (big) {
      refused[keyOf(p, f)] = big;
      return;
    }
    void set(p, f, v);
  }

  async function write(p: PluginView, f: FieldSetting) {
    error = null;
    try {
      const [view, proj] = await profilesApi.writeProject(p.id, project.id, f.key);
      plugins.put(view);
      app.replaceProject(proj);
      app.toast(t('settings.shared.written'));
    } catch (e) {
      error = String(e);
    }
  }

  /** The value of the select of a `profile` setting: `@default` while the project chose none, `@none` for « Aucun profil ». */
  function profileChoice(p: PluginView, f: FieldSetting): string {
    const c = chosenProfile(p, project.id, f.key);
    return c === undefined ? '@default' : c === null ? '@none' : c;
  }

  /** The profile a `profile` setting resolves to here: the one chosen, else the default of the plugin's page. */
  function resolved(p: PluginView, f: FieldSetting): string | null {
    const c = chosenProfile(p, project.id, f.key);
    if (c !== undefined) return c;
    return f.of ? (defaultProfile(p, f.of)?.id ?? null) : null;
  }

  function pickProfile(p: PluginView, f: FieldSetting, v: string) {
    if (v === '@default') void set(p, f, null, true);
    else void set(p, f, v === '@none' ? null : v);
  }

  /** The bands of what the file proposes: a profile setting whose file names a profile not chosen here. */
  const proposals = $derived(
    shown.flatMap((p) =>
      projectFields(p.manifest)
        .filter((f) => f.type === 'profile')
        .flatMap((f) => {
          const name = proposedName(project, p.id, f);
          const kind = kindsOf(p.manifest).find((k) => k.id === f.of);
          if (!name || !kind) return [];
          const match = proposedProfile(p, kind, name);
          if (match && resolved(p, f) === match.id) return [];
          return [{ p, f, name, match, kind }];
        }),
    ),
  );

  onMount(() => {
    if (!plugins.loaded) void plugins.load();
  });
</script>

{#each proposals as b (`${b.p.id}:${b.f.key}`)}
  <div class="band" role="status">
    <Icon name="file" size={14} />
    <p>
      <b>{t('plugins.projectPage.proposed', { name: b.name, plugin: b.p.name })}</b>
      {t('plugins.projectPage.proposedWhy')}
      {#if !b.match}{t('plugins.projectPage.proposedMissing')}{/if}
    </p>
    {#if b.match}
      <button class="btn sm" type="button" onclick={() => set(b.p, b.f, b.match?.id ?? null)}
        >{t('plugins.projectPage.take', { name: nameOf(b.kind, b.match) })}</button
      >
    {/if}
  </div>
{/each}

{#each shown as p (p.id)}
  {@const kinds = kindsOf(p.manifest)}
  <section class="pl" aria-label={p.name}>
    <div class="ph">
      <PluginTile id={p.id} name={p.name} installed size={22} />
      <h3>{p.name}</h3>
      <span class="id">{p.id}</span>
      <button class="btn ghost sm open" type="button" onclick={() => plugins.openFromSettings(p.id)}
        >{t('plugins.projectPage.pluginPage')}</button
      >
    </div>
    {#each teamRefusals(p, project) as r (`${r.part}:${r.key}`)}
      <p class="refused" role="note">
        <Icon name="warn" size={12} />{t('plugins.projectPage.teamRefused', {
          team: project.shared?.teamPlugins?.team ?? '',
          key: r.key,
          why: t(`plugins.projectPage.teamWhy.${r.why}`),
        })}
      </p>
    {/each}
    {#each projectFields(p.manifest) as f (f.key)}
      {@const title = titleOf(f)}
      {#if f.type === 'profile'}
        {@const kind = kinds.find((k) => k.id === f.of)}
        {@const list = kind ? profilesOf(p, kind.id) : []}
        {@const def = f.of ? defaultProfile(p, f.of) : null}
        {@const defKind = def?.id && kind ? list.find((x) => x.id === def.id) : null}
        {@const now = resolved(p, f)}
        <div class="row" role="group" aria-label={title}>
          <div class="lab">
            <b>{title}</b>
            {#if f.description}<span>{textOf(f.description, lang)}</span>{/if}
            <span class="mark" data-shared="machine"><Icon name="lock" size={12} />{t('plugins.projectPage.machine')}</span>
            {#if !now}<span class="none" role="status">{t('plugins.projectPage.noneSays', { plugin: p.name })}</span>{/if}
          </div>
          <div class="ctl">
            {#if list.length}
              <select
                class="field"
                aria-label={title}
                value={!defKind && profileChoice(p, f) === '@none' ? '@default' : profileChoice(p, f)}
                onchange={(e) => pickProfile(p, f, e.currentTarget.value)}
              >
                {#if defKind && kind}<option value="@default"
                    >{t('plugins.projectPage.profileDefault', { name: nameOf(kind, defKind) })}</option
                  >{/if}
                <option value={defKind ? '@none' : '@default'}>{t('plugins.projectPage.profileNone')}</option>
                {#each list as x (x.id)}
                  {@const sum = kind ? summaryOf(kind, x) : ''}
                  <option value={x.id}>{kind ? nameOf(kind, x) : x.id}{sum ? ` · ${sum.split(' · ')[0]}` : ''}</option>
                {/each}
              </select>
            {:else}
              <span class="muted">{t('plugins.projectPage.noProfiles')}</span>
            {/if}
          </div>
        </div>
      {:else}
        {@const v = projectValue(p, project, f)}
        {@const repo = fromRepo(f)}
        {@const locked = v.source === 'enforced'}
        <div class="row" class:top={f.multiline} class:locked role="group" aria-label={title}>
          <div class="lab">
            <b>{title}</b>
            {#if f.description}<span>{textOf(f.description, lang)}</span>{/if}
            {#if locked}
              <span class="mark" data-shared="team" data-testid="plugin-team-lock"
                ><Icon name="lock" size={12} />{lockLine({ kind: 'team', team: v.team ?? '' })}</span
              >
            {:else if !repo}
              <span class="mark" data-shared="machine"><Icon name="lock" size={12} />{t('plugins.projectPage.machine')}</span>
            {:else if v.source === 'file'}
              <span class="mark" data-shared="file"><Icon name="file" size={12} />{t('settings.shared.fromFile')}</span>
            {:else if v.source === 'recommended'}
              <span class="mark" data-shared="recommended" data-testid="plugin-team-recommended"
                ><Icon name="team" size={12} />{t('plugins.projectPage.teamRecommended', { team: v.team ?? '' })}</span
              >
            {:else if v.source === 'local'}
              <span class="shared">
                {#if v.file !== undefined}<span class="mark" data-shared="local"
                    ><Icon name="user" size={12} />{t('settings.shared.local')}</span
                  >{/if}
                <span class="acts">
                  <button class="link" type="button" onclick={() => write(p, f)}>{t('settings.shared.writeKey')}</button>
                  {#if v.file !== undefined}<button class="link" type="button" onclick={() => set(p, f, null, true)}
                      >{t('settings.shared.useFile')}</button
                    >{:else if v.recommended !== undefined}<button class="link" type="button" onclick={() => set(p, f, null, true)}
                      >{t('plugins.projectPage.teamUse', { team: v.team ?? '' })}</button
                    >{/if}
                </span>
              </span>
            {/if}
          </div>
          {#if f.type === 'boolean'}
            <button
              class="switch"
              class:on={!!v.value}
              role="switch"
              aria-checked={!!v.value}
              aria-label={title}
              disabled={locked}
              onclick={() => set(p, f, !v.value)}
            ></button>
          {:else if f.type === 'select'}
            <select
              class="field"
              aria-label={title}
              disabled={locked}
              value={String(v.value ?? '')}
              onchange={(e) => set(p, f, e.currentTarget.value)}
            >
              {#each f.options ?? [] as o (typeof o === 'string' ? o : o.value)}
                {@const val = typeof o === 'string' ? o : o.value}
                <option value={val}>{typeof o === 'string' ? o : textOf(o.title, lang) || o.value}</option>
              {/each}
            </select>
          {:else if f.multiline && f.type === 'string'}
            <div class="wide">
              <LinedText
                label={title}
                rows={visibleLines(f.lines)}
                value={String(v.value ?? '')}
                disabled={locked}
                onchange={(x) => setText(p, f, x)}
              />
            </div>
          {:else}
            <input
              class="field"
              aria-label={title}
              type={f.type === 'number' ? 'number' : 'text'}
              disabled={locked}
              value={String(v.value ?? '')}
              onchange={(e) => {
                const x = e.currentTarget.value;
                if (x === '') void set(p, f, null, true);
                else void set(p, f, f.type === 'number' ? Number(x) : x);
              }}
            />
          {/if}
          {#if refused[keyOf(p, f)]}<p class="err wide" role="alert">{refused[keyOf(p, f)]}</p>{/if}
        </div>
      {/if}
    {/each}
  </section>
{/each}
{#if error}<p class="err" role="alert">{error}</p>{/if}

<style>
  .band {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
    margin-bottom: 16px;
    padding: 10px 12px;
    border-radius: 9px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12px;
    line-height: 1.5;
  }
  .band :global(.icon) {
    color: var(--warn-ink);
  }
  .band p {
    flex: 1 1 320px;
    margin: 0;
  }
  .band b {
    color: var(--text);
    font-weight: 600;
  }
  .pl {
    display: grid;
    margin-bottom: 22px;
  }
  .ph {
    display: flex;
    align-items: center;
    gap: 9px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--line);
  }
  .ph h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }
  .id {
    overflow: hidden;
    color: var(--dim);
    font-family: var(--mono);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .open {
    margin-left: auto;
  }
  .btn.sm {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 20px;
    padding: 11px 0;
    border-bottom: 1px solid var(--line);
  }
  .row.top {
    align-items: flex-start;
  }
  .lab {
    display: grid;
    flex: 1 1 220px;
    gap: 3px;
    justify-items: start;
  }
  .lab b {
    font-size: 12.5px;
    font-weight: 560;
  }
  .lab > span:not(.mark):not(.shared) {
    color: var(--muted);
    font-size: 11.5px;
  }
  .lab .none {
    color: var(--warn-ink) !important;
  }
  .ctl {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .row .field {
    width: 260px;
    max-width: 100%;
  }
  .wide {
    flex: 1 0 100%;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .err {
    margin: 0;
    color: var(--del);
    font-size: 12px;
  }
  .err.wide {
    flex: 1 0 100%;
  }
  /* The marks of the other settings of a project (settings/SharedMark), and one for what stays on this machine. */
  .mark {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    width: fit-content;
    max-width: 100%;
    min-height: 19px;
    margin-top: 2px;
    padding: 1px 6px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--soft);
    font-size: 11px;
    font-weight: 550;
    line-height: 1.4;
  }
  .mark :global(.icon) {
    color: var(--muted);
  }
  .mark[data-shared='file'] {
    background: color-mix(in oklch, var(--brand) 12%, transparent);
    color: var(--teal-ink);
  }
  .mark[data-shared='file'] :global(.icon) {
    color: currentColor;
  }
  .row.locked .field,
  .row.locked .switch,
  .row.locked .wide {
    opacity: 0.55;
  }
  .refused {
    display: flex;
    gap: 6px;
    align-items: baseline;
    margin: 8px 0 0;
    color: var(--warn-ink);
    font-size: 11.5px;
    line-height: 1.45;
  }
  .shared {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 5px;
    margin-top: 2px;
  }
  .acts {
    display: inline-flex;
    gap: 10px;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--p-ink);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
</style>
