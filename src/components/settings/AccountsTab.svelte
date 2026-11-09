<script lang="ts">
  import { openUrl } from '../../lib/opener';
  import { onMount } from 'svelte';
  import {
    accountDefaults,
    accountName,
    accountsTabEngines,
    addAccount,
    agentsOn,
    backToAccounts,
    engineOfAccount,
    engineOrder,
    isPrincipal,
    metersShown,
    PRINCIPAL,
    privateSharing,
    projectionMode,
    projectsWithoutAccount,
    meterKey,
  } from '../../lib/accounts';
  import {
    byKey as goesByKey,
    capsOf,
    dayCostStandsIn,
    defaultModelOf,
    effortsOf,
    engineName,
    modelsOf,
    modesOf,
    found as foundEngines,
    showsCost,
    usagePage,
    usageSite,
  } from '../../lib/engines';
  import { EFFORTS, MODES } from '../../lib/models';
  import { fList, fPct, fUsd, tildify } from '../../lib/format';
  import { t, type Key } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { OS } from '../../lib/platform';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type {
    Account,
    AccountStatus,
    ClaudeLocated,
    ClaudeUpdateStatus,
    ClosedProject,
    EngineDecl,
    EngineId,
    EngineInfo,
    QuotaProjection,
  } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import ModelFavorites from './ModelFavorites.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';
  import Icon from '../Icon.svelte';
  import QuotaMeter from '../QuotaMeter.svelte';
  import { tip } from '../../lib/tip';

  // « Comptes et harness » (R4 of the 2.2 settings, design/settings-2-2/index.html#a-comptes): a group per harness, its
  // state and version beside its name, its accounts under it, « Ajouter un compte… » on the right; a harness whose plugin
  // is not installed installs from the app's seed on a click (ESC-373). Then « Quand un quota est épuisé », everything
  // that decides where an agent goes at a limit, the autopilot's pause (once in Kanban) and the automatic resume (once in
  // Harness) with it; then « Avancé ».

  /** The plugin of the seed that brings each harness of the first hand (`plugins::sort::of_engine`). */
  const HARNESS_PLUGINS: Record<string, EngineId> = {
    'dev.escouade.claude-code': 'claude',
    'dev.escouade.codex': 'codex',
    'dev.escouade.copilot-cli': 'copilot',
    'dev.escouade.cursor-cli': 'cursor',
    'dev.escouade.opencode': 'opencode',
    'dev.escouade.mistral': 'vibe',
    'dev.escouade.antigravity-cli': 'agy',
    'dev.escouade.gemini-cli': 'gemini',
    'dev.escouade.goose': 'goose',
  };

  /**
   * What the link to a harness's usage page says: its cost told, its quota not (Mistral Vibe); its quota in its own
   * interface alone (Antigravity CLI's `/usage`, ESC-271); else its dashboard (Cursor CLI).
   */
  const USAGE_HINTS: Partial<Record<EngineId, Key>> = {
    agy: 'accounts.tab.usageHintInterface',
    gemini: 'accounts.tab.usageHintGemini',
  };
  function usageHint(engine: EngineId): Key {
    return USAGE_HINTS[engine] ?? (showsCost({ engine }) ? 'accounts.tab.usageHintCost' : 'accounts.tab.usageHint');
  }

  // The accounts, a group per harness in « Ordre des harness ». Each change goes to the backend itself (`account_*`,
  // `engine_order_set`), not through the fields of the window saved as they change: an account is made, signed in and
  // removed on the disk.

  const accounts = $derived(app.settings.accounts ?? []);
  /** « Quand un quota est épuisé » is in the window's fields, saved as they change with the other settings (ESC-380). */
  const draft = $derived(settingsForm.settings);
  /** The harnesses found: going on on the next one asks for two. */
  const found = $derived(foundEngines());
  /** An account's meters, as its quota was last read. */
  const metersOf = (a: Account) => metersShown(app.usage.accounts.find((u) => u.id === a.id) ?? null, engineOfAccount(a));

  /** « Modèle de passation » of a harness; empty, its best. */
  function setHandoverModel(engine: EngineId, model: string) {
    const models = { ...(draft.handoverModels ?? {}) };
    if (model) models[engine] = model;
    else delete models[engine];
    draft.handoverModels = models;
  }
  /**
   * Every harness declared (its plugin installed) and every one with accounts, in « Ordre des harness »: its declaration,
   * what is found of it once read, its accounts. One installed without an account says « Aucun compte » (ESC-128).
   */
  const groups = $derived(
    engineOrder().map((engine) => ({
      engine,
      decl: app.engines.find((d) => d.id === engine) ?? null,
      info: infos?.find((i) => i.id === engine) ?? null,
      list: accounts.filter((a) => engineOfAccount(a) === engine),
    })),
  );
  /** The harnesses accounts are added to: found on this machine, or with accounts already (`accountsTabEngines`). */
  const addable = $derived(new Set(accountsTabEngines()));
  /** The harnesses the seed holds whose plugin is not installed: each installs here on a click, with no network. */
  const seeded = $derived(
    plugins.offers
      .map((o) => ({ id: o.id, engine: HARNESS_PLUGINS[o.id], refused: o.refused }))
      .filter(
        (o): o is { id: string; engine: EngineId; refused: string | null } => !!o.engine && !groups.some((g) => g.engine === o.engine),
      ),
  );
  /** The harness whose plugin is installing. */
  let installing = $state<EngineId | null>(null);

  // The plugins installed and what the seed holds, read again with the page: one may have been installed since.
  onMount(() => void plugins.load());

  /** « Installer » of a harness not installed: its plugin from the seed (`Core::need_plugin`), then the harnesses read again. */
  async function installHarness(engine: EngineId) {
    installing = engine;
    await app.run(api.pluginNeedEngine(engine));
    await app.refreshEngines();
    await plugins.load();
    installing = null;
  }

  // Where the `claude` of the path is, asked a moment after the path or the distribution changes (a look into WSL
  // may take seconds): an answer to an earlier path is dropped.
  let located = $state<ClaudeLocated | null>(null);
  $effect(() => {
    const [path, distro] = [draft.claudePath, draft.wslDistro];
    let stale = false;
    const timer = setTimeout(async () => {
      const found = await api.locateClaude(path, distro).catch(() => null);
      if (!stale) located = found ?? null;
    }, 250);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  });

  // Every harness with what is found of it (`engines`): its program, its version, its state. Read again once the path is
  // saved and located, and when a harness is installed; the default harness's lines only show while the field is the
  // saved path (the backend reads the saved settings). Until read, the harnesses as declared.
  let infos = $state<EngineInfo[] | null>(null);
  const saved = $derived(draft.claudePath === app.settings.claudePath && draft.wslDistro === app.settings.wslDistro);
  $effect(() => {
    if (!saved) return;
    void located;
    void app.engines;
    let stale = false;
    api
      .engines()
      .then((all) => {
        if (!stale && all) infos = all;
      })
      .catch(() => {});
    return () => {
      stale = true;
    };
  });

  // Where Escouade's update of Claude Code stands (ESC-133): read with the page, again once the switch is saved, and
  // every 2 s until its first run ends.
  let selfUpdate = $state<ClaudeUpdateStatus | null>(null);
  $effect(() => {
    void app.settings.claudeSelfUpdate;
    let stale = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const read = async () => {
      const st = await api.claudeUpdateStatus().catch(() => null);
      if (stale) return;
      selfUpdate = st;
      if (st?.state === 'running' || st?.state === 'waiting') timer = setTimeout(read, 2000);
    };
    void read();
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  });

  /** What the last update of Claude Code by Escouade did, or why it is not run; empty when there is nothing to say. */
  function selfUpdateText(st: ClaudeUpdateStatus | null): string {
    if (!draft.claudeSelfUpdate) return t('settings.claude.selfUpdateStates.off');
    // Turned on, not saved yet: said once saved.
    if (!st || st.state === 'off') return '';
    switch (st.state) {
      case 'upToDate':
        return st.version
          ? t('settings.claude.selfUpdateStates.upToDate', { version: st.version })
          : t('settings.claude.selfUpdateStates.upToDateNoVersion');
      case 'updated':
        return t('settings.claude.selfUpdateStates.updated', { version: st.version ?? '?', from: st.from ?? '?' });
      case 'failed':
        return t('settings.claude.selfUpdateStates.failed', { line: st.line ?? '' });
      case 'userOff':
        return t('settings.claude.selfUpdateStates.userOff', { line: st.line ?? '' });
      case 'missing':
        return st.line ?? '';
      default:
        return t(`settings.claude.selfUpdateStates.${st.state}`);
    }
  }

  /** The one the path of the settings is for: the default harness, until each has its own. */
  const pathOwner = $derived(app.engines[0]?.id);
  /** « Avancé », unfolded by itself when the default harness is not found: its path is set there. */
  let advancedOpen = $state(false);
  let unfolded = false;
  $effect(() => {
    if (unfolded || !located || located.path) return;
    unfolded = true;
    advancedOpen = true;
  });

  /** The command that installs a harness on this system. */
  const installOf = (d: EngineDecl) => (OS === 'windows' ? d.install.windows : d.install.unix);

  /**
   * Under its name: what keeps it from running, or that it did not tell its version; nothing of the tested ones (ESC-159).
   * Without an account its group says it already.
   */
  function versionText(e: EngineInfo, accounts: number): string {
    const version = e.note ?? (e.version ? '' : t('settings.claude.versionUnknown'));
    const said = e.state === 'notConnected' && accounts ? [version, t('settings.claude.versionNotConnected')] : [version];
    return said.filter(Boolean).join(' ');
  }

  /** The colour of a harness's state: ready, without an account, refused. */
  function toneOf(state: EngineInfo['state']): '' | 'warn' | 'bad' {
    if (state === 'notConnected') return 'warn';
    return state === 'tooOld' || state === 'unsupported' || state === 'absent' ? 'bad' : '';
  }

  /** The line under the path: where its `claude` was found, or how to name one. */
  function locatedText(l: ClaudeLocated | null): string {
    if (!l) return '';
    if (l.path === null) return l.canWsl ? t('settings.claude.notLocated') : t('settings.claude.notLocatedHere');
    return l.wsl ? t('settings.claude.locatedWsl', { distro: l.wsl, path: l.path }) : t('settings.claude.located', { path: l.path });
  }
  /** How many accounts of each harness are active: its last one stays on. */
  const activeOf = (engine: EngineId) => accounts.filter((a) => a.active && engineOfAccount(a) === engine).length;
  /** « Ordre des harness », once there are two harnesses to put in order. */
  const order = $derived(engineOrder());
  /** Principal may be reserved like any account: the projects none serves then, told under the list. */
  const orphans = $derived(projectsWithoutAccount());
  const orphansText = $derived(
    orphans.length
      ? t('accounts.tab.noAccount', {
          count: orphans.length,
          names: fList(orphans.map((p) => t('accounts.tab.quoted', { name: p.name }))),
        })
      : '',
  );
  /** The projects closed with their history kept, read when the tab opens: an account stays reserved to them until it is erased. */
  let closed = $state<ClosedProject[]>([]);
  onMount(async () => {
    closed = await api.closedProjects().catch(() => []);
  });
  /** The closed projects the account is reserved to (a file the app cannot read has no name to show). */
  const closedReserved = (a: Account) => closed.filter((c) => !c.unreadable && (a.projects ?? []).includes(c.id));
  /** Each account's sign-in, as last read. */
  let statuses = $state<Record<string, AccountStatus>>({});
  /** Why an account was not removed, by account. */
  let refused = $state<Record<string, string>>({});
  /** The accounts whose sign-in was asked for (not reactive: asked once each). */
  const asked = new Set<string>();

  // Each account's sign-in, read when the tab opens, and for an account added since.
  $effect(() => {
    for (const a of accounts) {
      if (asked.has(a.id)) continue;
      asked.add(a.id);
      void load(a.id);
    }
  });

  async function load(id: string) {
    try {
      statuses[id] = await api.accountStatus(id);
    } catch {
      // Removed meanwhile: nothing to show.
    }
  }

  /** The account as plain data, to send. */
  const plain = (a: Account): Account => ({ ...$state.snapshot(a) });

  /** What its quota's last reading says of its sign-in when it is there but out of date. */
  function expired(id: string): string | null {
    const u = app.usage.accounts.find((x) => x.id === id);
    return u?.connected && u.reason ? u.reason : null;
  }

  function stateOf(a: Account, s: AccountStatus | undefined): string {
    if (!a.active) return t('accounts.tab.state.inactive');
    if (!s) return t('accounts.tab.state.checking');
    if (s.connected) return s.email ? t('accounts.tab.state.connected', { email: s.email }) : t('accounts.tab.state.connectedNoEmail');
    return a.id === PRINCIPAL ? t('accounts.tab.state.principalNotConnected') : t('accounts.tab.state.notConnected');
  }

  /** « Changer de compte à » (ESC-148): a percentage, or « À la limite » (100). */
  const switchChoices = $derived([
    ...[80, 90, 95].map((n) => ({ value: n, label: fPct(n) })),
    { value: 100, label: t('accounts.quotas.switchAtLimit') },
  ]);

  /** Saved at once, as every setting of this tab. */
  function setSwitchAt(at: number) {
    app.settings.switchAt = at;
    app.run(api.saveSettings($state.snapshot(app.settings)));
  }

  /** « Projection des quotas »: in the status bar past 80 % (the default), always, or nowhere. */
  /** « Pause du pilote auto au-delà du quota »: beside « Changer de compte à », whose choices it shares but « À la limite ». */
  const pauseChoices = $derived([80, 90, 95, 100].map((n) => ({ value: n, label: fPct(n) })));

  const projectionChoices = $derived([
    { value: 'alert' as const, label: t('accounts.quotas.projectionAlert') },
    { value: 'always' as const, label: t('accounts.quotas.projectionAlways') },
    { value: 'off' as const, label: t('accounts.quotas.projectionOff') },
  ]);

  /** Shown at once, saved with the rest of the app's settings as they are, like the accounts. */
  function setProjection(mode: QuotaProjection) {
    app.settings.quotaProjection = mode;
    app.run(api.saveSettings($state.snapshot(app.settings)));
  }

  /** The harness moved by `by` places in « Ordre des harness ». */
  async function moveEngine(i: number, by: number) {
    const next = [...order];
    [next[i], next[i + by]] = [next[i + by], next[i]];
    const saved = await app.run(api.engineOrderSet(next));
    if (saved) app.settings.engineOrder = saved;
  }

  /** Saved as the backend has it: the accounts it then gives are the window's. */
  async function save(next: Account): Promise<boolean> {
    const list = await app.run(api.accountUpdate(next));
    if (list) app.settings.accounts = list;
    return !!list;
  }

  /** The project ticked or unticked in « Réservé aux projets ». */
  function reserve(a: Account, project: string, on: boolean) {
    const now = a.projects ?? [];
    const projects = on ? [...now, project] : now.filter((p) => p !== project);
    void save({ ...plain(a), projects });
  }

  /** A field left: what it says saved when it changed, else (or refused) as it was. */
  async function commit(a: Account, input: HTMLInputElement, field: 'name' | 'claudePath') {
    const shown = field === 'name' ? accountName(a) : a.claudePath;
    if (input.value === shown) return;
    if (!(await save({ ...plain(a), [field]: input.value }))) input.value = shown;
  }

  /** The account `a` swapped with `other`, the one before or after it in its harness's group. */
  async function move(a: Account, other: Account) {
    const ids = accounts.map((x) => x.id);
    const [i, j] = [ids.indexOf(a.id), ids.indexOf(other.id)];
    [ids[i], ids[j]] = [ids[j], ids[i]];
    const list = await app.run(api.accountReorder(ids));
    if (list) app.settings.accounts = list;
  }

  /** Asks before removing it, unless agents still run on it; back to this tab either way. The machine's stays signed in. */
  function remove(a: Account) {
    const n = agentsOn(a.id);
    if (n) {
      refused[a.id] = t('accounts.tab.stillUsed', { count: n });
      return;
    }
    delete refused[a.id];
    const { id } = a;
    const path = tildify(a.configDir || statuses[a.id]?.dir || '');
    const machine = isPrincipal(a);
    const says = machine ? 'removeMachineConfirm' : 'removeConfirm';
    app.modal = {
      kind: 'confirm',
      title: t(`accounts.tab.${says}.title`, { name: accountName(a) }),
      body: t(`accounts.tab.${says}.body`, { path, harness: engineName(engineOfAccount(a)) }),
      confirm: t(`accounts.tab.${says}.confirm`),
      danger: true,
      onConfirm: async () => {
        const list = await app.run(api.accountRemove(id));
        if (list) app.settings.accounts = list;
        backToAccounts();
      },
      onCancel: backToAccounts,
    };
  }

  /** A harness without a quota (OpenCode, Mistral Vibe): a budget of the day stands in for it; not for one whose usage is on its own page only (Cursor CLI, which tells no cost). */
  const noQuota = (engine: EngineId) => dayCostStandsIn(engine);

  /** « Budget par jour ($) » left: empty (or 0) for none; refused, as it was. */
  async function commitBudget(a: Account, input: HTMLInputElement) {
    const typed = input.value.trim().replace(',', '.');
    const budget = typed === '' ? null : Number(typed);
    const shown = a.dailyBudget ? String(a.dailyBudget) : '';
    if ((budget || null) === (a.dailyBudget || null)) {
      input.value = shown;
      return;
    }
    if (budget !== null && (!Number.isFinite(budget) || budget < 0)) {
      refused[a.id] = t('accounts.tab.budgetInvalid');
      input.value = shown;
      return;
    }
    delete refused[a.id];
    if (!(await save({ ...plain(a), dailyBudget: budget || null }))) input.value = shown;
  }

  /** What its new agents go with, as its harness has it (ESC-128): its models, the efforts of its model, its modes. */
  function defaultsOf(a: Account) {
    const engine = engineOfAccount(a);
    const who = { engine, account: a.id };
    const d = accountDefaults(a);
    const models = modelsOf(who);
    // Claude Code's Sonnet; another harness's own default, as it lists it for the account.
    const harnessModel = accountDefaults(undefined, engine).model || defaultModelOf(who);
    const named = models.find((m) => m.id === harnessModel)?.label ?? harnessModel;
    return {
      models,
      // A model it saved that its harness does not list (any more): kept in the menu, as it is.
      kept: a.defaultModel && !models.some((m) => m.id === a.defaultModel) ? a.defaultModel : '',
      harnessLabel: named ? t('accounts.tab.harnessDefaultNamed', { name: named }) : t('accounts.tab.harnessDefault'),
      efforts: effortsOf({ ...who, model: d.model }, EFFORTS),
      modes: modesOf(who, MODES),
    };
  }

  /** One of the defaults of its new agents chosen; empty: its harness's. */
  function setDefault(a: Account, field: 'defaultModel' | 'defaultEffort' | 'defaultMode', value: string) {
    void save({ ...plain(a), [field]: value });
  }

  /** An account of Cursor CLI other than its machine's: signed in by its API key (ESC-43). */
  // Gemini CLI's (ESC-275): as it was added, its key or its Google sign-in.
  const byKey = (a: Account) => (goesByKey(engineOfAccount(a)) || !!a.apiKey) && !isPrincipal(a);

  const blurOnEnter = (e: KeyboardEvent) => {
    if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
  };
</script>

<!-- Always there, empty while every project has an account: a screen reader hears it fill. (Above the groups: between two, it would part their hairline.) -->
<p class="orphans" class:on={orphans.length > 0} role="status">{orphansText}</p>
<span id="accounts-last-active" hidden>{t('accounts.tab.lastActive')}</span>
{#each groups as g (g.engine)}
  {@const harness = engineName(g.engine)}
  {@const list = g.list}
  {@const decl = g.decl}
  {@const info = g.info}
  {@const own = g.engine === pathOwner}
  {@const shown = own ? (info && located?.path && saved ? info : null) : info}
  {@const absent = own ? !!located && !located.path : info?.state === 'absent'}
  {@const versionSaid = shown && shown.state !== 'absent' ? versionText(shown, list.length) : ''}
  {@const bare = !list.length && addable.has(g.engine) && !absent}
  <!-- Installed without an account (ESC-128): « Aucun compte », whatever its program says of its own sign-in. -->
  {@const said = shown && bare && (shown.state === 'ready' || shown.state === 'notConnected') ? 'notConnected' : shown?.state}
  {@const tone = said ? toneOf(said) : absent ? 'bad' : bare ? 'warn' : ''}
  <Group title={harness} note={own && located?.nat ? t('settings.claude.nat') : undefined} data-engine={g.engine} aria-label={harness}>
    {#snippet state()}
      {#if shown && said && said !== 'absent'}
        <span class="hstate {tone}"
          ><i aria-hidden="true"></i>{t(`settings.claude.states.${said}`)} ·
          <span
            class="mono version {tone}"
            data-state={shown.state}
            aria-describedby={versionSaid ? `engine-version-${g.engine}` : undefined}>{shown.version ?? '—'}</span
          ></span
        >
      {:else if absent}
        <span class="hstate bad"
          ><i aria-hidden="true"></i><span data-engine-state="absent">{t('settings.claude.states.absent')}</span></span
        >
      {:else if bare}
        <span class="hstate warn"
          ><i aria-hidden="true"></i><span data-engine-state="noAccount">{t('settings.claude.states.notConnected')}</span></span
        >
      {/if}
    {/snippet}
    {#snippet action()}
      {#if absent && decl}
        <!-- Not found on this machine: the command that installs it, where an account would be added. -->
        <code class="mono cmd" aria-label={t('settings.claude.install')}>{installOf(decl)}</code>
      {:else if addable.has(g.engine)}
        <button class="btn" aria-label={t('accounts.tab.addTo', { harness })} onclick={() => addAccount(g.engine)}
          ><Icon name="plus" size={14} />{t('accounts.tab.addTo', { harness })}</button
        >
      {/if}
    {/snippet}
    {#if versionSaid}<p class="hnote {tone}" id="engine-version-{g.engine}">{versionSaid}</p>{/if}
    {#if absent && decl}
      <!-- Claude Code's says whether its installation was cut half-way, with its repair (ESC-133). -->
      <p class="hnote">{(info?.state === 'absent' && info.note) || t('settings.claude.installDesc')}</p>
    {/if}
    {#if !list.length}
      <!-- A harness installed without an account (ESC-128): nothing starts on it until one is added. -->
      {#if addable.has(g.engine)}<p class="hnote">{t('accounts.tab.noneHint', { harness })}</p>{/if}
    {:else}
      <ul class="list" aria-label={t('accounts.tab.group', { harness })}>
        {#each list as a, i (a.id)}
          {@const name = accountName(a)}
          {@const status = statuses[a.id]}
          {@const late = expired(a.id)}
          {@const last = a.active && activeOf(g.engine) === 1}
          {@const reserved = a.projects ?? []}
          {@const closedOnes = closedReserved(a)}
          {@const defaults = defaultsOf(a)}
          {@const sharing = privateSharing(a)}
          {@const meters = a.active && !noQuota(g.engine) ? metersOf(a) : []}
          <li class="acct" aria-label={name}>
            <div class="l1">
              <input
                class="nm"
                size={Math.max(4, name.length + 1)}
                aria-label={t('accounts.tab.name')}
                spellcheck="false"
                value={name}
                onkeydown={blurOnEnter}
                onblur={(e) => commit(a, e.currentTarget, 'name')}
              />
              <span class="stt" class:on={a.active && status?.connected} class:off={!a.active}>
                <i aria-hidden="true"></i><span>{stateOf(a, status)}</span>
              </span>
              <span class="quota">
                {#if a.active && usagePage(g.engine)}
                  <!-- No quota told to Escouade (Cursor CLI, nor its cost, ESC-43; Mistral Vibe, ESC-267): its own page says its usage. -->
                  <button
                    class="btn ghost small usage"
                    use:tip={t(usageHint(g.engine), {
                      harness,
                      site: usageSite(g.engine),
                    })}
                    onclick={() => app.run(openUrl(usagePage(g.engine)!))}
                    ><Icon name="ext" size={12} />{t('accounts.tab.usage', { site: usageSite(g.engine) })}</button
                  >
                {/if}
                {#each meters as m (meterKey(m))}
                  <QuotaMeter minutes={m.minutes} model={m.model} usage={m.usage} now={app.now} />
                {/each}
                {#if a.active && noQuota(g.engine)}
                  <!-- No quota to gauge (OpenCode): what it spent today, its budget on its own row below. -->
                  <span class="spent"
                    >{t('accounts.tab.today')} <b>{fUsd(app.usage.accounts.find((u) => u.id === a.id)?.todayCost ?? 0)}</b></span
                  >
                {/if}
              </span>
              <button
                class="switch"
                class:on={a.active}
                role="switch"
                aria-checked={a.active}
                aria-label={t('accounts.tab.active')}
                aria-describedby={last ? 'accounts-last-active' : undefined}
                use:tip={last ? t('accounts.tab.lastActive') : t('accounts.tab.active')}
                disabled={last}
                onclick={() => save({ ...plain(a), active: !a.active })}
              ></button>
              <span class="ord">
                <button class="icon-btn" aria-label={t('accounts.tab.up', { name })} disabled={i === 0} onclick={() => move(a, list[i - 1])}
                  ><Icon name="up" size={14} /></button
                >
                <button
                  class="icon-btn down"
                  aria-label={t('accounts.tab.down', { name })}
                  disabled={i === list.length - 1}
                  onclick={() => move(a, list[i + 1])}><Icon name="up" size={14} /></button
                >
              </span>
            </div>
            <div class="l2">
              <span class="k">{t('accounts.tab.folder')}</span>
              <span class="dir mono">{tildify(a.configDir || status?.dir || '')}</span>
              {#if isPrincipal(a)}<span class="smark machine"><Icon name="user" size={12} />{t('accounts.tab.machine')}</span>{/if}
            </div>
            <div class="l2">
              <label class="k" for="account-exe-{a.id}">{t('accounts.tab.executable')}</label>
              <input
                id="account-exe-{a.id}"
                class="field mono exe"
                spellcheck="false"
                placeholder={t('accounts.tab.executableDefault')}
                value={a.claudePath}
                onkeydown={blurOnEnter}
                onblur={(e) => commit(a, e.currentTarget, 'claudePath')}
              />
              <span class="end">
                {#if status && !(engineOfAccount(a) === 'gemini' && isPrincipal(a))}
                  <!-- Gemini's machine's account is a key of the app's: nothing to sign in here (its ~/.gemini is never touched). -->
                  <!-- Not signed in, or out of date: to sign in. Signed in: to sign in again (another claude.ai account, say). -->
                  <button class="btn" onclick={() => (app.modal = { kind: 'account', accountId: a.id })}>
                    {byKey(a)
                      ? t('accounts.tab.changeKey')
                      : status.connected && !late
                        ? t('accounts.tab.signInAgain')
                        : t('accounts.tab.signIn')}
                  </button>
                {/if}
                <button class="btn danger" onclick={() => remove(a)}>{t('accounts.tab.remove')}</button>
              </span>
            </div>
            <!-- What its new agents go with, as its harness has it (ESC-128), always in sight (choice 18). -->
            <div class="l2" role="group" aria-label={t('accounts.tab.defaults')}>
              <span class="k">{t('accounts.tab.defaults')}</span>
              <span class="picks">
                <select
                  class="field"
                  aria-label={t('accounts.tab.defaultModel')}
                  value={a.defaultModel ?? ''}
                  onchange={(e) => setDefault(a, 'defaultModel', e.currentTarget.value)}
                >
                  <option value="">{defaults.harnessLabel}</option>
                  {#each defaults.models as m (m.id)}<option value={m.id}>{m.label}</option>{/each}
                  {#if defaults.kept}<option value={defaults.kept}>{defaults.kept}</option>{/if}
                </select>
                {#if defaults.efforts.length}
                  <select
                    class="field"
                    aria-label={t('accounts.tab.defaultEffort')}
                    value={a.defaultEffort ?? ''}
                    onchange={(e) => setDefault(a, 'defaultEffort', e.currentTarget.value)}
                  >
                    <option value="">{t('accounts.tab.harnessDefault')}</option>
                    {#each defaults.efforts as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
                  </select>
                {/if}
                {#if defaults.modes.length}
                  <select
                    class="field"
                    aria-label={t('accounts.tab.defaultMode')}
                    value={a.defaultMode ?? ''}
                    onchange={(e) => setDefault(a, 'defaultMode', e.currentTarget.value)}
                  >
                    <option value="">{t('accounts.tab.harnessDefault')}</option>
                    {#each defaults.modes as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
                  </select>
                {/if}
              </span>
            </div>
            {#if noQuota(g.engine)}
              <div class="l2">
                <label class="k" for="account-budget-{a.id}">{t('accounts.tab.budget')}</label>
                <input
                  id="account-budget-{a.id}"
                  class="field budget"
                  type="text"
                  inputmode="decimal"
                  spellcheck="false"
                  placeholder={t('accounts.tab.budgetNone')}
                  aria-describedby="account-budget-hint-{a.id}"
                  value={a.dailyBudget ? String(a.dailyBudget) : ''}
                  onkeydown={blurOnEnter}
                  onblur={(e) => commitBudget(a, e.currentTarget)}
                />
                <span class="hint" id="account-budget-hint-{a.id}">{t('accounts.tab.budgetHint')}</span>
              </div>
            {/if}
            {#if app.projects.length || closedOnes.length}
              <div class="l2" role="group" aria-label={t('accounts.tab.reserved')}>
                <span class="k" use:tip={t('accounts.tab.reservedHint')}>{t('accounts.tab.reserved')}</span>
                <span class="chips">
                  {#each app.projects as p (p.id)}
                    <!-- Nothing reserved is every project: all the squares are lit, the boxes stay unticked. -->
                    <label class="chip-t">
                      <input
                        type="checkbox"
                        checked={reserved.includes(p.id)}
                        onchange={(e) => reserve(a, p.id, e.currentTarget.checked)}
                      />
                      <i class:lit={!reserved.length || reserved.includes(p.id)} style:--dot={p.color} aria-hidden="true"></i>{p.name}
                    </label>
                  {/each}
                  <!-- Closed, but their history is kept and so is the reservation: it goes only when they are unticked or erased. -->
                  {#each closedOnes as c (c.id)}
                    <label class="chip-t">
                      <input
                        type="checkbox"
                        checked={reserved.includes(c.id)}
                        onchange={(e) => reserve(a, c.id, e.currentTarget.checked)}
                      />
                      <i class:lit={reserved.includes(c.id)} aria-hidden="true"></i>{t('accounts.tab.closedProject', { name: c.name })}
                    </label>
                  {/each}
                  {#if !reserved.length}<span class="hint">{t('accounts.tab.reservedAll')}</span>{/if}
                </span>
              </div>
            {/if}
            <!-- A private project still goes to it, shared (ESC-141): reserved to that project alone, it is no longer. -->
            {#if sharing.length}
              <div class="l2 sharing-row">
                <p class="sharing">
                  {t('accounts.private.sharing', {
                    count: sharing.length,
                    names: fList(sharing.map((p) => t('accounts.tab.quoted', { name: p.name }))),
                  })}
                </p>
                {#if sharing.length === 1}
                  <button
                    class="btn"
                    use:tip={t('accounts.private.reserveTitle')}
                    onclick={() => save({ ...plain(a), projects: [sharing[0].id] })}
                  >
                    {t('accounts.private.reserveTo', { name: sharing[0].name })}
                  </button>
                {/if}
              </div>
            {/if}
            {#if late}<p class="late">{late}</p>{/if}
            {#if refused[a.id]}<p class="refused" role="alert">{refused[a.id]}</p>{/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if decl && shown && shown.state !== 'absent' && !decl.models.length}
      <!-- A harness that lists the models of its providers (OpenCode): searched, favorites marked. -->
      <Row label={t('settings.claude.models.label')} desc={t('settings.claude.models.desc', { engine: decl.name })} wide>
        <ModelFavorites engine={decl.id} />
      </Row>
    {/if}
    {#if decl?.caps.cost === 'estimated' && !absent}
      {@const cannot = [
        !decl.caps.pdf && t('settings.claude.costNoPdf'),
        !decl.caps.backgroundTasks && t('settings.claude.costNoBackground'),
      ]
        .filter(Boolean)
        .join(' · ')}
      <Row label={t('settings.claude.cost')} desc={t('settings.claude.costEstimated', { engine: decl.name })}>
        {#if cannot}<span class="hint nowrap">{cannot[0].toUpperCase() + cannot.slice(1)}</span>{/if}
      </Row>
    {/if}
  </Group>
{/each}
<!-- The harnesses of the seed not installed (ESC-373): installed here, from the seed, with no network. -->
{#each seeded as o (o.id)}
  {@const harness = engineName(o.engine)}
  <Group title={harness} data-engine={o.engine} aria-label={harness}>
    {#snippet state()}
      <span class="hstate off"
        ><i aria-hidden="true"></i><span data-engine-state="notInstalled">{t('settings.claude.states.notInstalled')}</span></span
      >
    {/snippet}
    {#snippet action()}
      <button
        class="btn"
        disabled={installing !== null}
        aria-label={t('settings.claude.installSeed', { harness })}
        onclick={() => installHarness(o.engine)}
        ><Icon name="download" size={14} />{installing === o.engine
          ? t('settings.claude.installing')
          : t('settings.claude.installSeed', { harness })}</button
      >
    {/snippet}
    <p class="hnote" class:bad={!!o.refused}>{o.refused ?? t('settings.claude.notInstalledHint')}</p>
  </Group>
{/each}
<!-- Where an agent goes at a limit, in one place (cause 1 of design/settings-2-2/PROPOSITION.md): the two thresholds side by
     side, « Changer de compte à » first (choice 20 of the 2.0 design). -->
<Group title={t('accounts.quotas.limitTitle')} anchor="limit">
  <Row label={t('accounts.quotas.switchAt')} desc={t('accounts.quotas.switchAtDesc')}>
    <Chips label={t('accounts.quotas.switchAt')} options={switchChoices} bind:value={() => app.settings.switchAt ?? 100, setSwitchAt} />
  </Row>
  <!-- The autopilot's pause (once in Kanban): an app setting, for all the projects. -->
  <Row label={t('accounts.quotas.pauseAt')} hint={t('accounts.quotas.pauseAtHint')} desc={t('accounts.quotas.pauseAtDesc')}>
    <Chips label={t('accounts.quotas.pauseAt')} options={pauseChoices} bind:value={draft.quotaPause} />
  </Row>
  <!-- With a single account there is nothing to go on on. -->
  {#if accounts.length > 1}
    <Row label={t('accounts.switchOnLimit.label')} desc={t('accounts.switchOnLimit.desc')}>
      <Switch label={t('accounts.switchOnLimit.label')} bind:on={draft.switchOnLimit} />
    </Row>
  {/if}
  <!-- Once in Harness › Tous les harness. -->
  <Row label={t('settings.claude.autoResume')} desc={t('settings.claude.autoResumeDesc')}>
    <Switch label={t('settings.claude.autoResume')} bind:on={draft.autoResume} />
  </Row>
  <!-- Going on on another harness (ESC-88): only with two harnesses found. -->
  {#if found.length > 1}
    <Row label={t('handover.settings.onLimit')} desc={t('handover.settings.onLimitDesc')}>
      <Switch label={t('handover.settings.onLimit')} bind:on={draft.handoverOnLimit} />
    </Row>
  {/if}
</Group>
<!-- The rare ones, folded: they name what they hide. -->
<Group advanced="accounts" bind:open={advancedOpen} aria-label={t('settings.fold.advanced')}>
  {#if order.length > 1}
    <Row label={t('accounts.engineOrder.label')} desc={t('accounts.engineOrder.desc')} wide>
      <ol class="order" aria-label={t('accounts.engineOrder.label')}>
        {#each order as e, i (e)}
          {@const name = engineName(e)}
          <li>
            <span class="rank" aria-hidden="true">{i + 1}</span>
            <span class="engine">{name}</span>
            <button
              class="icon-btn"
              aria-label={t('accounts.engineOrder.up', { name })}
              disabled={i === 0}
              onclick={() => moveEngine(i, -1)}><Icon name="up" size={14} /></button
            >
            <button
              class="icon-btn down"
              aria-label={t('accounts.engineOrder.down', { name })}
              disabled={i === order.length - 1}
              onclick={() => moveEngine(i, 1)}><Icon name="up" size={14} /></button
            >
          </li>
        {/each}
      </ol>
    </Row>
  {/if}
  {#if found.length > 1}
    {#each found.filter((e) => e.models.length) as e (e.id)}
      <Row label={t('handover.settings.model', { harness: e.name })} desc={t('handover.settings.modelDesc')}>
        <select
          class="field"
          aria-label={t('handover.settings.model', { harness: e.name })}
          value={draft.handoverModels?.[e.id] ?? ''}
          onchange={(ev) => setHandoverModel(e.id, ev.currentTarget.value)}
        >
          <option value="">{t('handover.settings.best', { model: e.models[0].label })}</option>
          {#each e.models as m (m.id)}<option value={m.id}>{m.label}</option>{/each}
        </select>
      </Row>
    {/each}
  {/if}
  {#if pathOwner}
    <Row label={t('settings.claude.path')} hint={engineName(pathOwner)}>
      {#snippet extra()}
        {@const text = locatedText(located)}
        {#if text}
          <span class={located?.path ? 'found' : 'more'} id="claude-located"
            >{#if located?.path}<Icon name="check" size={14} />{/if}<span>{text}</span></span
          >
        {/if}
      {/snippet}
      <input
        class="field mono input"
        aria-label={t('settings.claude.path')}
        aria-describedby={located ? 'claude-located' : undefined}
        spellcheck="false"
        placeholder={app.claudePathFound ? t('settings.claude.pathFound') : t('settings.claude.pathMissing')}
        value={draft.claudePath}
        oninput={(e) => (draft.claudePath = e.currentTarget.value)}
      />
    </Row>
  {/if}
  {#if app.engines.some((e) => e.id === 'claude')}
    {@const said = selfUpdateText(selfUpdate)}
    {@const failed = draft.claudeSelfUpdate && (selfUpdate?.state === 'failed' || selfUpdate?.state === 'missing')}
    <Row label={t('settings.claude.selfUpdate')} desc={t('settings.claude.selfUpdateDesc')}>
      {#snippet extra()}
        {#if said}
          <span
            class={failed ? 'bad self-update' : 'found self-update'}
            class:plain={!draft.claudeSelfUpdate ||
              selfUpdate?.state === 'waiting' ||
              selfUpdate?.state === 'running' ||
              selfUpdate?.state === 'userOff'}
            data-update-state={draft.claudeSelfUpdate ? selfUpdate?.state : 'off'}
            role="status"
            >{#if !failed && draft.claudeSelfUpdate && (selfUpdate?.state === 'updated' || selfUpdate?.state === 'upToDate')}<Icon
                name="check"
                size={14}
              />{/if}<span class:bad={failed}>{said}</span></span
          >
        {/if}
      {/snippet}
      <Switch label={t('settings.claude.selfUpdate')} bind:on={draft.claudeSelfUpdate} />
    </Row>
  {/if}
  <Row label={t('accounts.quotas.projection')} desc={t('accounts.quotas.projectionDesc')}>
    <Chips label={t('accounts.quotas.projection')} options={projectionChoices} bind:value={projectionMode, setProjection} />
  </Row>
</Group>

<style>
  /* An account's card (`.acct` of the 2.0 design): its name, its sign-in, its quota and its switch on the first line; its
     folder, executable, new agents, budget and projects under it, each after its key. */
  .list {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
  }
  .acct {
    display: grid;
    gap: 10px;
    padding: 16px 0;
  }
  .acct + .acct {
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .l1,
  .l2 {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  /* Its name and its state before its bars: with a meter of one model more (« Fable 7 j », ESC-312), the bars go
     under them rather than cut the name. */
  .l1 {
    flex-wrap: wrap;
    row-gap: 6px;
  }
  .nm {
    flex: 0 1 auto;
    min-width: 110px;
    max-width: 220px;
    height: 26px;
    margin: 0 -6px;
    padding: 0 6px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    font-size: 13.5px;
    font-weight: 600;
    outline: none;
    text-overflow: ellipsis;
  }
  .nm:hover {
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .nm:focus {
    background: var(--well);
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--p) 60%, transparent),
      0 0 0 3px color-mix(in oklch, var(--p) 14%, transparent);
  }
  /* A harness's state beside its name (`.pill` of design/settings-2-2/index.html#a-comptes): green ready, amber without
     an account, red refused or not found, grey not installed. */
  .hstate {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 20px;
    padding: 0 8px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--run) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--run) 30%, transparent);
    color: var(--add);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .hstate i {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--run);
  }
  .hstate.warn {
    background: color-mix(in oklch, var(--wait) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--wait) 30%, transparent);
    color: var(--warn-ink);
  }
  .hstate.warn i {
    background: var(--wait);
  }
  .hstate.bad {
    background: color-mix(in oklch, var(--del) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 30%, transparent);
    color: var(--del);
  }
  .hstate.bad i {
    background: var(--del);
  }
  .hstate.off {
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
  }
  .hstate.off i {
    background: var(--dim);
  }
  .version {
    font-size: 11px;
  }
  /* Under its name: what keeps it from running, or what it waits for. */
  .hnote {
    max-width: 72ch;
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
    text-wrap: pretty;
  }
  .hnote.warn {
    color: var(--warn-ink);
  }
  .hnote.bad {
    color: var(--del);
  }
  .cmd {
    padding: 4px 8px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12px;
    user-select: all;
    overflow-wrap: anywhere;
  }
  .input {
    width: 320px;
    max-width: 100%;
    flex: 0 1 auto;
  }
  .nowrap {
    white-space: nowrap;
  }
  .self-update.plain {
    color: var(--muted);
  }
  .stt {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .stt span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .stt i {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--wait);
  }
  .stt.on i {
    background: var(--run);
  }
  .stt.off i {
    background: var(--dim);
  }
  .quota {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 10px;
    margin-left: auto;
  }
  .spent {
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .spent b {
    color: var(--soft);
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  .ord {
    display: inline-flex;
    flex: none;
  }
  .icon-btn.down :global(.icon) {
    transform: rotate(180deg);
  }
  .k {
    flex: none;
    width: 90px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.4;
  }
  .dir {
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .smark {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 7px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 550;
  }
  .exe {
    width: 300px;
    min-width: 0;
    flex: 0 1 auto;
    font-size: 12px;
  }
  .end {
    display: inline-flex;
    flex: none;
    gap: 8px;
    margin-left: auto;
  }
  .picks {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    min-width: 0;
  }
  .picks select {
    max-width: 260px;
  }
  .budget {
    width: 64px;
    flex: none;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    min-width: 0;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }
  /* « Réservé aux projets »: a chip per project, pressed when reserved; the box stays for the keyboard and the readers. */
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .chip-t {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 9px 0 7px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
  }
  .chip-t input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }
  .chip-t i {
    width: 7px;
    height: 7px;
    border-radius: 2px;
    /* The light of the toggle: grey when off, the project's colour (neutral for a closed one) when ticked. */
    background: rgba(var(--fg-rgb), 0.28);
    transition:
      background 0.12s,
      box-shadow 0.12s;
  }
  .chip-t i.lit {
    background: var(--dot, var(--text));
    box-shadow: 0 0 5px color-mix(in srgb, var(--dot, var(--text)) 55%, transparent);
  }
  @media (prefers-reduced-motion: reduce) {
    .chip-t i {
      transition: none;
    }
  }
  .chip-t:has(input:checked) {
    background: rgba(var(--fg-rgb), 0.07);
    color: var(--text);
  }
  .chip-t:has(input:focus-visible) {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .sharing-row {
    flex-wrap: wrap;
  }
  .sharing {
    flex: 1;
    margin: 0;
    color: var(--warn-ink);
    font-size: 12px;
  }
  .late,
  .refused {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
  }
  .late,
  .orphans {
    color: var(--warn-ink);
  }
  .orphans {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
  }
  .orphans.on {
    margin-top: 16px;
  }
  .refused {
    color: var(--del);
  }
  /* « Ordre des harness »: each harness in its place, moved by its arrows. */
  .order {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .order li {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 2px 0 9px;
    border-radius: 7px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 12px;
    font-weight: 550;
  }
  .rank {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .order .icon-btn {
    width: 22px;
    height: 22px;
  }
  /* A narrow window: the card's lines wrap, the quota under the name. */
  @container settings (max-width: 620px) {
    .l1,
    .l2 {
      flex-wrap: wrap;
    }
    .end,
    .quota {
      margin-left: 0;
    }
  }
</style>
