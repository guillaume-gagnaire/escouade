<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { anyCan, engineName } from '../../lib/engines';
  import { accountName, accountsFor, dedicated, defaultEngine, isPrincipal, sharedBy } from '../../lib/accounts';
  import { api } from '../../lib/ipc';
  import { fList } from '../../lib/format';
  import { activeIn, markOf } from '../../lib/plugins';
  import { wholeApp } from '../../lib/plugins-settings';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Account, Project } from '../../lib/types';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // Projets › <projet> › Confidentialité: what the project keeps to itself. Private or not (what the other projects'
  // agents see of it), its files harness by harness and its accounts when private, the remote control of its agents,
  // and the plugins on in it (the other side of the « Projets » tab of a plugin's page).
  let { project }: { project: Project } = $props();

  onMount(() => void plugins.load());

  const draft = $derived(settingsForm.project!);
  /** The project as the backend last found it: what a plugin's default reads (a private project is off by default). */
  const live = $derived(app.projects.find((p) => p.id === project.id) ?? project);

  /**
   * A private project's accounts (ESC-141), as drafted: those of its default harness reserved to it alone, and those its
   * agents may still go to that it shares with other projects (sessions, memory and history common there).
   */
  const privateEngine = $derived(draft.engine ?? defaultEngine());
  const asDrafted = $derived({ id: project.id, account: draft.account, private: draft.private, sharedOnLimit: draft.sharedOnLimit });
  const ownAccounts = $derived(accountsFor(asDrafted, privateEngine).filter((a) => dedicated(a, project.id)));
  const sharedAccounts = $derived(sharedBy(asDrafted, privateEngine));
  const quotedNames = (list: Account[]) => fList(list.map((a) => t('accounts.tab.quoted', { name: accountName(a) })));

  /** « Réserver <compte> à ce projet »: saved at once, as Réglages › Comptes et harness saves its accounts. */
  async function reserveHere(a: Account) {
    const list = await app.run(api.accountUpdate({ ...$state.snapshot(a), projects: [project.id] }));
    if (list) app.settings.accounts = list;
  }

  /** The setting for all agents, as drafted in this window (not as saved), is on: it covers this project's agents already. */
  const coveredByAll = $derived(!!settingsForm.settings.remoteControlAll);

  /** What keeps a private project's files from the other projects' agents, harness by harness (ESC-140). */
  const FENCE = [
    { id: 'claude', name: 'Claude Code', kind: 'guard' },
    { id: 'codex', name: 'Codex', kind: 'barrier' },
    { id: 'opencode', name: 'OpenCode', kind: 'guard' },
    { id: 'copilot', name: 'Copilot CLI', kind: 'guard' },
    { id: 'cursor', name: 'Cursor CLI', kind: 'guard' },
    { id: 'vibe', name: 'Mistral Vibe', kind: 'guard' },
    { id: 'agy', name: 'Antigravity CLI', kind: 'guard' },
    { id: 'gemini', name: 'Gemini CLI', kind: 'guard' },
  ] as const;

  /** The plugins that reach a project one by one (a theme reaches the whole app: it has no switch here). */
  const perProject = $derived(plugins.list.filter((p) => !wholeApp(p)));
</script>

<!-- « Projet privé »: what the agents of the other projects see of it through the MCP server; for a private one only
     (choice 28 of the 2.0 design), what keeps its files from them harness by harness, then its accounts. -->
<Group anchor="private">
  <Row label={t('settings.project.private')} desc={t('settings.project.privateDesc')} shared="private">
    <Switch label={t('settings.project.private')} bind:on={draft.private} />
  </Row>
  {#if draft.private}
    {@const harness = engineName(privateEngine)}
    <div class="sub-h"><b>{t('settings.project.fenceTitle')}</b><span>{t('settings.project.fenceDesc')}</span></div>
    <ul class="fence" aria-label={t('settings.project.fenceTitle')}>
      {#each FENCE as f (f.id)}
        <li>
          <span class="engine">{f.name}</span>
          <span class="kind smark" class:barrier={f.kind === 'barrier'}
            ><Icon name={f.kind === 'barrier' ? 'lock' : 'shield'} size={12} /><span>{t(`settings.project.fenceKind.${f.kind}`)}</span
            ></span
          >
          <span class="how">{t(`settings.project.fenceHow.${f.id}`)}</span>
        </li>
      {/each}
    </ul>
    <p class="note">{t('settings.project.fenceNote')}</p>
    <!-- Its accounts (ESC-141): reserved to it alone, or still shared, with what to do about it. -->
    <div class="sub-h" data-anchor="private-accounts"><b>{t('accounts.tab.list')}</b></div>
    <div class="private-accounts">
      {#if ownAccounts.length}
        <p class="acct-note" role="status">
          <Icon name="check-c" size={14} /><span class="grow"
            >{t('accounts.private.own', { count: ownAccounts.length, names: quotedNames(ownAccounts), harness })}
            {#if ownAccounts.some(isPrincipal)}{t('accounts.private.machineToo', { harness })}{/if}</span
          >
        </p>
      {/if}
      {#if sharedAccounts.length || !ownAccounts.length}
        <div class="acct-note warn">
          <Icon name="shield" size={14} />
          <span class="grow" role="status"
            >{#if sharedAccounts.length}{t('accounts.private.shared', {
                count: sharedAccounts.length,
                names: quotedNames(sharedAccounts),
                harness,
              })}{/if}</span
          >
          {#each sharedAccounts as a (a.id)}
            <button class="btn" use:tip={t('accounts.private.reserveTitle')} onclick={() => reserveHere(a)}>
              {t('accounts.private.reserve', { name: accountName(a) })}
            </button>
          {/each}
          <button class="btn" onclick={() => (app.modal = { kind: 'account', engine: privateEngine, reserveFor: project.id })}>
            {t('accounts.private.add')}
          </button>
        </div>
      {/if}
    </div>
    <Row label={t('accounts.private.sharedOnLimit')} desc={t('accounts.private.sharedOnLimitDesc')}>
      <Switch label={t('accounts.private.sharedOnLimit')} bind:on={draft.sharedOnLimit} />
    </Row>
  {/if}
</Group>

<!-- Only while a harness has Remote Control. -->
{#if anyCan('remoteControl')}
  <Group title={t('settings.project.remoteTitle')}>
    <Row label={t('settings.project.remote')} desc={t('settings.project.remoteDesc')}>
      {#snippet extra()}
        {#if coveredByAll}<span class="more" id="remote-covered">{t('settings.project.remoteGlobal')}</span>{/if}
      {/snippet}
      <!-- The setting for all agents covers every project: the switch shows it, the project's own value stays as it is. -->
      <Switch
        label={t('settings.project.remote')}
        disabled={coveredByAll}
        describedby={coveredByAll ? 'remote-covered' : undefined}
        bind:on={() => coveredByAll || draft.remoteControl, (v) => (draft.remoteControl = v)}
      />
    </Row>
  </Group>
{/if}

<!-- Its plugins (PROPOSITION.md §2): a switch per installed plugin, set at once as on the plugin's page. -->
<Group title={t('plugins.projects.title')} note={t('plugins.projects.intro')} anchor="plugins" aria-label={t('plugins.projects.title')}>
  {#if plugins.loaded && !perProject.length}
    <p class="note">{t('plugins.projects.noneInstalled')}</p>
  {/if}
  {#each perProject as p (p.id)}
    {@const on = activeIn(p, live)}
    <div class="plugin" class:off={!p.choices.enabled}>
      <span class="mark" aria-hidden="true">{markOf(p.name)}</span>
      <span class="nm"
        ><b>{p.name}</b>{#if !p.choices.enabled}<span class="why">
            · {t('plugins.projects.appOffShort')}</span
          >{:else if live.private && !p.choices.projects[project.id]}<span class="why"> · {t('plugins.projects.private')}</span>{/if}</span
      >
      <button
        class="switch"
        class:on
        role="switch"
        aria-checked={on}
        aria-label={t('plugins.projects.pluginLabel', { name: p.name })}
        disabled={!p.choices.enabled}
        onclick={() => plugins.setProject(p.id, project.id, !on)}
      ></button>
    </div>
  {/each}
</Group>

<style>
  /* A title inside a group, its explanation beside it. */
  .sub-h {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
    margin-top: 18px;
  }
  .sub-h b {
    font-weight: 600;
  }
  .sub-h span {
    color: var(--muted);
    font-size: 12px;
  }
  .note {
    max-width: 64ch;
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  /* « Ses fichiers », harness by harness (choice 28): « Garde-fou » neutral, « Barrière » green. */
  .fence {
    display: grid;
    margin: 10px 0 12px;
    padding: 0;
    list-style: none;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .fence li {
    display: grid;
    grid-template-columns: 110px 92px minmax(0, 1fr);
    align-items: baseline;
    gap: 10px;
    padding: 9px 12px;
    font-size: 12.5px;
  }
  .fence li + li {
    border-top: 1px solid var(--line);
  }
  .engine {
    font-weight: 560;
  }
  .smark {
    display: inline-flex;
    align-items: center;
    justify-self: start;
    gap: 5px;
    width: fit-content;
    height: 19px;
    padding: 0 6px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--soft);
    font-size: 11px;
    font-weight: 550;
    white-space: nowrap;
  }
  .smark > :global(.icon) {
    color: var(--muted);
  }
  .smark.barrier {
    background: oklch(from var(--run) l c h / 0.1);
    color: var(--run);
  }
  .smark.barrier > :global(.icon) {
    color: currentColor;
  }
  .how {
    color: var(--muted);
    line-height: 1.45;
    text-wrap: pretty;
  }
  .private-accounts {
    display: grid;
    gap: 8px;
    margin-top: 10px;
  }
  .acct-note {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 8px;
    margin: 0;
    padding: 9px 11px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.03);
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .acct-note > :global(.icon) {
    flex: none;
    margin-top: 2px;
    color: var(--muted);
  }
  .acct-note.warn {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .acct-note .btn {
    flex: none;
    height: 26px;
  }
  .acct-note.warn > :global(.icon) {
    color: var(--wait);
  }
  .grow {
    flex: 1;
    min-width: 200px;
  }
  .acct-note.warn .grow {
    flex-basis: calc(100% - 30px);
  }
  /* A plugin per line, its mark, its name and why it is off, its switch (as on Réglages › Plugins). */
  .plugin {
    display: grid;
    grid-template-columns: 26px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
    font-size: 13px;
  }
  .plugin:last-child {
    border-bottom: none;
  }
  .mark {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
    font: 650 11px/1 var(--font);
  }
  .plugin.off .mark,
  .plugin.off b {
    opacity: 0.55;
  }
  .nm b {
    font-weight: 560;
  }
  .why {
    color: var(--muted);
    font-size: 12px;
  }
  .switch:disabled {
    opacity: 0.4;
  }
  @container settings (max-width: 600px) {
    .fence li {
      grid-template-columns: 90px minmax(0, 1fr);
    }
    .fence .how {
      grid-column: 1 / -1;
    }
  }
</style>
