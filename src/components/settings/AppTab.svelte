<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { anyCan, offered } from '../../lib/engines';
  import { IS_MAC, OS } from '../../lib/platform';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { privacyUrl } from '../../lib/telemetry';
  import { CREDITS_URL } from '../../lib/conversation';
  import { DISCORD_URL, siteUrl } from '../../lib/links';
  import type { ClaudeLocated, ClaudeUpdateStatus, EngineDecl, EngineInfo } from '../../lib/types';
  import { checkForUpdate } from '../../lib/updater';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import McpSection from './McpSection.svelte';
  import ModelFavorites from './ModelFavorites.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // The tabs of the app's own settings.
  let { tab }: { tab: 'claude' | 'notifications' | 'terminals' | 'network' | 'about' } = $props();

  const s = $derived(settingsForm.settings);
  const bigTextOptions = $derived([
    { value: 'path' as const, label: t('settings.claude.attachBigTextPath') },
    { value: 'refuse' as const, label: t('settings.claude.attachBigTextRefuse') },
  ]);
  let checking = $state(false);
  /** The harnesses found, for « À propos ». */
  const found = $derived(offered());

  // Where the `claude` of the path is, asked a moment after the path or the distribution changes (a look into WSL
  // may take seconds): an answer to an earlier path is dropped.
  let located = $state<ClaudeLocated | null>(null);
  $effect(() => {
    if (tab !== 'claude') return;
    const [path, distro] = [s.claudePath, s.wslDistro];
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
  // saved and located; the default harness's lines only show while the field is the saved path (the backend reads the
  // saved settings). Until read, the harnesses as declared.
  let infos = $state<EngineInfo[] | null>(null);
  const saved = $derived(s.claudePath === app.settings.claudePath && s.wslDistro === app.settings.wslDistro);
  $effect(() => {
    if (tab !== 'claude' || !saved) return;
    void located;
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
  // Where Escouade's update of Claude Code stands (ESC-133): read with the tab, again once the switch is saved, and
  // every 2 s until its first run ends.
  let selfUpdate = $state<ClaudeUpdateStatus | null>(null);
  $effect(() => {
    if (tab !== 'claude') return;
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
    if (!s.claudeSelfUpdate) return t('settings.claude.selfUpdateStates.off');
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

  /** Each harness: its declaration, and what is found of it once read. */
  const harnesses = $derived(app.engines.map((d) => ({ decl: d, info: infos?.find((i) => i.id === d.id) ?? null })));
  /** The one the path of the settings is for: the default harness, until each has its own. */
  const pathOwner = $derived(app.engines[0]?.id);

  /** The command that installs a harness on this system. */
  const installOf = (d: EngineDecl) => (OS === 'windows' ? d.install.windows : d.install.unix);

  /** Under the version: what keeps it from running, or that it did not tell it; nothing of the tested ones (ESC-159). */
  function versionText(e: EngineInfo): string {
    const version = e.note ?? (e.version ? '' : t('settings.claude.versionUnknown'));
    const said = e.state === 'notConnected' ? [version, t('settings.claude.versionNotConnected')] : [version];
    return said.filter(Boolean).join(' ');
  }

  /** The line under the path: where its `claude` was found, or how to name one. */
  function locatedText(l: ClaudeLocated | null): string {
    if (!l) return '';
    if (l.path === null) return l.canWsl ? t('settings.claude.notLocated') : t('settings.claude.notLocatedHere');
    return l.wsl ? t('settings.claude.locatedWsl', { distro: l.wsl, path: l.path }) : t('settings.claude.located', { path: l.path });
  }

  async function check() {
    checking = true;
    const found = await checkForUpdate(true);
    checking = false;
    // A check that failed has told why: it is not "nothing new".
    if (found === false) app.toast(t('settings.about.noUpdate'), 'info');
  }
</script>

{#snippet text(label: string, hint: string, value: string, set: (v: string) => void, placeholder = '')}
  <Row {label} {hint}>
    <input
      class="field mono input"
      aria-label={label}
      spellcheck="false"
      {placeholder}
      {value}
      oninput={(e) => set(e.currentTarget.value)}
    />
  </Row>
{/snippet}

{#if tab === 'claude'}
  {#each harnesses as { decl, info } (decl.id)}
    {@const own = decl.id === pathOwner}
    {@const shown = own ? (info && located?.path && saved ? info : null) : info}
    <section class="harness" data-engine={decl.id} aria-label={decl.name}>
      <Group title={decl.name} note={own && located?.nat ? t('settings.claude.nat') : undefined}>
        {#if own}
          {@render pathRow()}
        {:else}
          <Row
            label={t('settings.claude.executable')}
            desc={info?.located.path ? t('settings.claude.located', { path: info.located.path }) : t('settings.claude.notLocatedHere')}
          />
        {/if}
        {#if shown && shown.state !== 'absent'}
          {@const versionSaid = versionText(shown)}
          <Row label={t('settings.claude.version')} desc={versionSaid} descId="engine-version-{decl.id}">
            <span
              class="mono version"
              class:warn={shown.state === 'notConnected'}
              class:bad={shown.state === 'tooOld' || shown.state === 'unsupported'}
              data-state={shown.state}
              aria-describedby={versionSaid ? `engine-version-${decl.id}` : undefined}>{shown.version ?? '—'}</span
            >
          </Row>
          <!-- Accounts are signed in from Réglages › Comptes (ESC-128): the version's line says when none of its is. -->
          {#if !decl.models.length}
            <!-- A harness that lists the models of its providers (OpenCode): searched, favorites marked. -->
            <Row label={t('settings.claude.models.label')} desc={t('settings.claude.models.desc', { engine: decl.name })} wide>
              <ModelFavorites engine={decl.id} />
            </Row>
          {/if}
        {:else if own ? located && !located.path : info?.state === 'absent'}
          <!-- Claude Code's says whether its installation was cut half-way, with its repair (ESC-133). -->
          <Row label={t('settings.claude.install')} desc={(info?.state === 'absent' && info.note) || t('settings.claude.installDesc')}>
            <span class="state bad" data-engine-state="absent">{t('settings.claude.states.absent')}</span>
            <code class="mono cmd">{installOf(decl)}</code>
          </Row>
        {/if}
        {#if decl.id === 'claude'}
          {@const said = selfUpdateText(selfUpdate)}
          <Row label={t('settings.claude.selfUpdate')} desc={t('settings.claude.selfUpdateDesc')}>
            <Switch label={t('settings.claude.selfUpdate')} bind:on={s.claudeSelfUpdate} />
          </Row>
          {#if said}
            <p
              class="self-update"
              class:bad={s.claudeSelfUpdate && (selfUpdate?.state === 'failed' || selfUpdate?.state === 'missing')}
              data-update-state={s.claudeSelfUpdate ? selfUpdate?.state : 'off'}
              role="status"
            >
              {said}
            </p>
          {/if}
        {/if}
      </Group>
    </section>
  {/each}
  {#snippet pathRow()}
    <Row label={t('settings.claude.path')} hint={t('settings.claude.pathHint')} desc={locatedText(located)} descId="claude-located">
      <input
        class="field mono input"
        aria-label={t('settings.claude.path')}
        aria-describedby={located ? 'claude-located' : undefined}
        spellcheck="false"
        placeholder={app.claudePathFound ? t('settings.claude.pathFound') : t('settings.claude.pathMissing')}
        value={s.claudePath}
        oninput={(e) => (s.claudePath = e.currentTarget.value)}
      />
    </Row>
  {/snippet}
  <Group title={t('settings.claude.processes')}>
    <Row label={t('settings.claude.autoResume')} desc={t('settings.claude.autoResumeDesc')}>
      <Switch label={t('settings.claude.autoResume')} bind:on={s.autoResume} />
    </Row>
    <!-- With a single account there is nothing to go on on. -->
    {#if (app.settings.accounts?.length ?? 0) > 1}
      <Row label={t('accounts.switchOnLimit.label')} desc={t('accounts.switchOnLimit.desc')}>
        <Switch label={t('accounts.switchOnLimit.label')} bind:on={s.switchOnLimit} />
      </Row>
    {/if}
    <!-- Going on on another harness (ESC-88): only with two harnesses found. -->
    {#if found.length > 1}
      <Row label={t('handover.settings.onLimit')} desc={t('handover.settings.onLimitDesc')}>
        <Switch label={t('handover.settings.onLimit')} bind:on={s.handoverOnLimit} />
      </Row>
      {#each found.filter((e) => e.models.length) as e (e.id)}
        <Row label={t('handover.settings.model', { harness: e.name })} desc={t('handover.settings.modelDesc')}>
          <Chips
            label={t('handover.settings.model', { harness: e.name })}
            options={[
              { value: '', label: t('handover.settings.best', { model: e.models[0].label }) },
              ...e.models.map((m) => ({ value: m.id, label: m.label })),
            ]}
            bind:value={
              () => s.handoverModels?.[e.id] ?? '',
              (v) => {
                const models = { ...(s.handoverModels ?? {}) };
                if (v) models[e.id] = v;
                else delete models[e.id];
                s.handoverModels = models;
              }
            }
          />
        </Row>
      {/each}
    {/if}
    <!-- Settings of what only some harnesses do: shown while one of them is there. -->
    {#if anyCan('terminalView')}
      <Row label={t('settings.claude.terminalView')} desc={t('settings.claude.terminalViewDesc')}>
        <Chips
          label={t('settings.claude.terminalView')}
          options={[
            { value: 'escouade', label: t('settings.claude.terminalViewEscouade') },
            { value: 'harness', label: t('settings.claude.terminalViewHarness') },
          ]}
          bind:value={() => (s.terminalView ? 'harness' : 'escouade'), (v) => (s.terminalView = v === 'harness')}
        />
      </Row>
    {/if}
    {#if anyCan('todoTools')}
      <Row label={t('settings.claude.todoTools')} desc={t('settings.claude.todoToolsDesc')}>
        <Switch label={t('settings.claude.todoTools')} bind:on={s.todoTools} />
      </Row>
    {/if}
    <Row label={t('settings.claude.idleStop')} hint={t('settings.claude.idleStopHint')}>
      <input
        class="field mono input short"
        type="number"
        min="0"
        aria-label={t('settings.claude.idleStopLabel')}
        bind:value={s.idleStopMinutes}
      />
    </Row>
  </Group>
  <Group title={t('settings.claude.attachments')} note={t('settings.claude.attachmentsNote')}>
    <Row label={t('settings.claude.attachMax')} hint={t('settings.claude.attachMaxHint')}>
      <input
        class="field mono input short"
        type="number"
        min="0"
        aria-label={t('settings.claude.attachMaxLabel')}
        bind:value={s.attachMaxMb}
      />
    </Row>
    <Row label={t('settings.claude.attachWarn')} hint={t('settings.claude.attachWarnHint')}>
      <input
        class="field mono input short"
        type="number"
        min="0"
        aria-label={t('settings.claude.attachWarnLabel')}
        bind:value={s.attachWarnMb}
      />
    </Row>
    <Row label={t('settings.claude.attachOrigin')} desc={t('settings.claude.attachOriginDesc')}>
      <Switch label={t('settings.claude.attachOrigin')} bind:on={s.attachOrigin} />
    </Row>
    <Row label={t('settings.claude.attachBigText')} desc={t('settings.claude.attachBigTextDesc')}>
      <Chips label={t('settings.claude.attachBigText')} options={bigTextOptions} bind:value={s.attachBigText} />
    </Row>
    <Row label={t('settings.claude.attachKeep')} hint={t('settings.claude.attachKeepHint')}>
      <input
        class="field mono input short"
        type="number"
        min="0"
        aria-label={t('settings.claude.attachKeepLabel')}
        bind:value={s.attachKeepDays}
      />
    </Row>
  </Group>
  <!-- Beside « Les agents peuvent piloter Escouade », before its activity log, which can be long. -->
  {#if anyCan('remoteControl')}
    <Group title={t('settings.claude.remoteTitle')}>
      <Row label={t('settings.claude.remoteAll')} desc={t('settings.claude.remoteAllDesc')}>
        <Switch label={t('settings.claude.remoteAll')} bind:on={s.remoteControlAll} />
      </Row>
    </Group>
  {/if}
  <McpSection />
{:else if tab === 'notifications'}
  <Group
    title={t('settings.notifications.notifyFor')}
    note={IS_MAC ? t('settings.notifications.noteMac') : t('settings.notifications.noteOther')}
  >
    <Row label={t('settings.notifications.questions')}>
      <Switch label={t('settings.notifications.questions')} bind:on={s.notifyFor.questions} />
    </Row>
    <Row label={t('settings.notifications.done')}>
      <Switch label={t('settings.notifications.done')} bind:on={s.notifyFor.done} />
    </Row>
    <Row label={t('settings.notifications.errors')}>
      <Switch label={t('settings.notifications.errors')} bind:on={s.notifyFor.errors} />
    </Row>
    <Row label={t('settings.notifications.tickets')}>
      <Switch label={t('settings.notifications.tickets')} bind:on={s.notifyFor.tickets} />
    </Row>
  </Group>
  <Group title={t('settings.notifications.channels')}>
    <Row label={t('settings.notifications.system', { os: IS_MAC ? 'macOS' : 'Windows' })}>
      <Switch label={t('settings.notifications.systemLabel')} bind:on={s.osNotifications} />
    </Row>
  </Group>
  <Group title={t('settings.notifications.sound')}>
    <Row label={t('settings.notifications.soundOn')} desc={t('settings.notifications.soundOnDesc')}>
      <Switch label={t('settings.notifications.soundOn')} bind:on={s.sound} />
    </Row>
    <Row label={t('settings.notifications.soundTest')}>
      <button class="btn" onclick={() => api.playChime()}>{t('settings.notifications.soundTestButton')}</button>
    </Row>
  </Group>
{:else if tab === 'terminals'}
  <Group
    title={t('settings.terminals.paths')}
    note={t('settings.terminals.detected', { list: app.shells.map((x) => x.label).join(', ') || t('settings.terminals.none') })}
  >
    {#if IS_MAC}
      {@render text('bash', t('settings.terminals.auto'), s.bashPath, (v) => (s.bashPath = v))}
    {:else}
      {@render text('PowerShell 7', t('settings.terminals.auto'), s.pwshPath, (v) => (s.pwshPath = v))}
      {@render text('Git Bash', t('settings.terminals.auto'), s.bashPath, (v) => (s.bashPath = v))}
      {@render text(t('settings.terminals.wsl'), t('settings.terminals.wslHint'), s.wslDistro, (v) => (s.wslDistro = v), 'Ubuntu')}
    {/if}
  </Group>
{:else if tab === 'network'}
  <Group title={t('settings.network.proxy')} note={t('settings.network.proxyNote')}>
    {@render text(
      t('settings.network.proxyUrl'),
      t('settings.network.proxyUrlHint'),
      s.proxyUrl,
      (v) => (s.proxyUrl = v),
      t('settings.network.proxyUrlNone'),
    )}
    {@render text(t('settings.network.exclusions'), t('settings.network.exclusionsHint'), s.noProxy, (v) => (s.noProxy = v))}
    <Row label={t('settings.network.proxyTerminals')}>
      <Switch label={t('settings.network.proxyTerminals')} bind:on={s.proxyTerminals} />
    </Row>
  </Group>
  <Group title={t('settings.network.certificates')} note={t('settings.network.certificatesNote')}>
    <Row label={t('settings.network.insecureTls')} desc={t('settings.network.insecureTlsDesc')}>
      <Switch label={t('settings.network.insecureTls')} bind:on={s.insecureTls} />
    </Row>
  </Group>
{:else}
  <Group title="Escouade">
    <Row
      label="Escouade {app.version}"
      desc={found.length ? t('settings.about.enginesFound', { list: found.map((e) => e.name).join(', ') }) : t('settings.about.noEngine')}
    >
      <button class="btn" disabled={checking} onclick={check}>{checking ? t('settings.about.checking') : t('settings.about.check')}</button>
    </Row>
    <Row label={t('settings.about.autoUpdate')} desc={t('settings.about.autoUpdateDesc')}>
      <Switch label={t('settings.about.autoUpdate')} bind:on={s.autoUpdate} />
    </Row>
    <Row label={t('settings.about.telemetry')} desc={t('settings.about.telemetryDesc')}>
      <button class="btn ghost" onclick={() => openUrl(privacyUrl()).catch((e) => app.toast(String(e), 'error'))}
        >{t('settings.about.telemetryMore')}</button
      >
      <Switch label={t('settings.about.telemetry')} bind:on={() => s.telemetry === true, (on) => (s.telemetry = on)} />
    </Row>
    <Row label={t('settings.about.localData')} desc="~/.escouade/" />
    <Row label={t('settings.about.credits')} desc={t('settings.about.creditsDesc')}>
      <button class="btn ghost" onclick={() => openUrl(CREDITS_URL).catch((e) => app.toast(String(e), 'error'))}
        >{t('settings.about.creditsOpen')}</button
      >
    </Row>
    <Row label={t('settings.about.site')} desc={t('settings.about.siteDesc')}>
      <button
        class="btn ghost"
        aria-label={t('settings.about.siteOpenLabel')}
        onclick={() => openUrl(siteUrl()).catch((e) => app.toast(String(e), 'error'))}>{t('settings.about.siteOpen')}</button
      >
    </Row>
    <Row label={t('settings.about.discord')} desc={t('settings.about.discordDesc')}>
      <button
        class="btn ghost"
        aria-label={t('settings.about.discordJoinLabel')}
        onclick={() => openUrl(DISCORD_URL).catch((e) => app.toast(String(e), 'error'))}>{t('settings.about.discordJoin')}</button
      >
    </Row>
  </Group>
{/if}

<style>
  .input {
    width: 340px;
    max-width: 50%;
    flex: none;
    height: 32px;
    padding: 0 10px;
    background: var(--panel);
    font-size: 12px;
  }
  .input.short {
    width: 110px;
  }
  .version {
    font-size: 12px;
    color: var(--muted);
  }
  .version.warn {
    color: var(--wait);
  }
  .version.bad {
    color: var(--del);
  }
  .state {
    font-size: 12px;
    color: var(--wait);
  }
  .state.bad {
    color: var(--del);
  }
  .self-update {
    margin: 0;
    padding: 0 16px 12px;
    font-size: 12px;
    color: var(--muted);
  }
  .self-update.bad {
    color: var(--del);
  }
  .cmd {
    padding: 4px 8px;
    border-radius: var(--r-sm);
    background: var(--panel);
    font-size: 11.5px;
    user-select: all;
  }
</style>
