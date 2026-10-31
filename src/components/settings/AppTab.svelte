<script lang="ts">
  import { openPath, openUrl } from '../../lib/opener';
  import { tildify } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { anyCan, offered } from '../../lib/engines';
  import { IS_MAC, OS } from '../../lib/platform';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { privacyUrl } from '../../lib/telemetry';
  import { CREDITS_URL } from '../../lib/conversation';
  import { DISCORD_URL, siteUrl, supportUrl } from '../../lib/links';
  import type { ClaudeLocated, ClaudeUpdateStatus, EngineDecl, EngineInfo } from '../../lib/types';
  import { checkForUpdate, postponeRestart, restartIn } from '../../lib/updater';
  import Markdown from '../conv/Markdown.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import ModelFavorites from './ModelFavorites.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';
  import Icon from '../Icon.svelte';

  // The tabs of the app's own settings.
  let { tab }: { tab: 'claude' | 'notifications' | 'terminals' | 'network' | 'about' } = $props();

  const s = $derived(settingsForm.settings);
  const bigTextOptions = $derived([
    { value: 'path' as const, label: t('settings.claude.attachBigTextPath') },
    { value: 'refuse' as const, label: t('settings.claude.attachBigTextRefuse') },
  ]);
  const midTurnDelayOptions = $derived(
    [0, 5, 10, 20, 30].map((s) => ({
      value: s,
      label: s ? t('settings.claude.midTurnDelaySecs', { s }) : t('settings.claude.midTurnDelayOff'),
    })),
  );
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
  // The folder of Escouade's data, read with « À propos » (another one for a development build or a sandbox).
  let dataDir = $state<string | null>(null);
  $effect(() => {
    if (tab !== 'about' || dataDir) return;
    api
      .dataDir()
      .then((d) => (dataDir = d))
      .catch(() => {});
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

  // The update found (ESC-250), by the button or the automatic check: downloading, then ready. A download that failed keeps
  // its own row while the same version is tried again (choice 13), never both; once ready, or a newer one found, the update's.
  const update = $derived(app.update && !app.update.ready && app.updateTrouble?.version === app.update.version ? null : app.update);
  const restartSeconds = $derived(restartIn());
  let notesOpen = $state(false);
  /** What a screen reader hears of the update as it comes: its region is there from the start, so it is told. */
  const updateLive = $derived(
    !update
      ? ''
      : update.ready
        ? t('settings.about.updateReadyLive', { version: update.version })
        : t('settings.about.updateFoundLive', { version: update.version }),
  );

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
  <!-- A group per harness (choice 2 of the 2.0 design): its state and version beside its name, its executable, what
       holds for it alone; then « Tous les harness ». -->
  {#each harnesses as { decl, info } (decl.id)}
    {@const own = decl.id === pathOwner}
    {@const shown = own ? (info && located?.path && saved ? info : null) : info}
    {@const absent = own ? !!located && !located.path : info?.state === 'absent'}
    {@const versionSaid = shown && shown.state !== 'absent' ? versionText(shown) : ''}
    {@const tone = shown ? toneOf(shown.state) : absent ? 'bad' : ''}
    <Group title={decl.name} note={own && located?.nat ? t('settings.claude.nat') : undefined} data-engine={decl.id} aria-label={decl.name}>
      {#snippet state()}
        {#if shown && shown.state !== 'absent'}
          <span class="hstate {tone}"
            ><i aria-hidden="true"></i>{t(`settings.claude.states.${shown.state}`)} ·
            <span
              class="mono version {tone}"
              data-state={shown.state}
              aria-describedby={versionSaid ? `engine-version-${decl.id}` : undefined}>{shown.version ?? '—'}</span
            ></span
          >
        {:else if absent}
          <span class="hstate bad"
            ><i aria-hidden="true"></i><span data-engine-state="absent">{t('settings.claude.states.absent')}</span></span
          >
        {/if}
      {/snippet}
      {#if own}
        <Row label={t('settings.claude.path')}>
          {#snippet extra()}
            {@render foundLine(locatedText(located), !!located?.path, 'claude-located')}
            {@render versionLine(versionSaid, decl.id, tone)}
          {/snippet}
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
      {:else}
        <Row label={t('settings.claude.path')}>
          {#snippet extra()}
            {@render foundLine(
              // Not found: « Installer » gives its command; its path is not set here, unlike Claude Code's.
              info?.located.path ? t('settings.claude.located', { path: info.located.path }) : t('settings.claude.notLocatedOther'),
              !!info?.located.path,
            )}
            {@render versionLine(versionSaid, decl.id, tone)}
          {/snippet}
        </Row>
      {/if}
      {#if shown && shown.state !== 'absent' && !decl.models.length}
        <!-- A harness that lists the models of its providers (OpenCode): searched, favorites marked. -->
        <Row label={t('settings.claude.models.label')} desc={t('settings.claude.models.desc', { engine: decl.name })} wide>
          <ModelFavorites engine={decl.id} />
        </Row>
      {/if}
      {#if absent}
        <!-- Claude Code's says whether its installation was cut half-way, with its repair (ESC-133). -->
        <Row label={t('settings.claude.install')} desc={(info?.state === 'absent' && info.note) || t('settings.claude.installDesc')}>
          <code class="mono cmd">{installOf(decl)}</code>
        </Row>
      {/if}
      {#if decl.id === 'claude'}
        {@const said = selfUpdateText(selfUpdate)}
        {@const failed = s.claudeSelfUpdate && (selfUpdate?.state === 'failed' || selfUpdate?.state === 'missing')}
        <Row label={t('settings.claude.selfUpdate')} desc={t('settings.claude.selfUpdateDesc')}>
          {#snippet extra()}
            {#if said}
              <span
                class={failed ? 'bad self-update' : 'found self-update'}
                class:plain={!s.claudeSelfUpdate ||
                  selfUpdate?.state === 'waiting' ||
                  selfUpdate?.state === 'running' ||
                  selfUpdate?.state === 'userOff'}
                data-update-state={s.claudeSelfUpdate ? selfUpdate?.state : 'off'}
                role="status"
                >{#if !failed && s.claudeSelfUpdate && (selfUpdate?.state === 'updated' || selfUpdate?.state === 'upToDate')}<Icon
                    name="check"
                    size={14}
                  />{/if}<span class:bad={failed}>{said}</span></span
              >
            {/if}
          {/snippet}
          <Switch label={t('settings.claude.selfUpdate')} bind:on={s.claudeSelfUpdate} />
        </Row>
      {/if}
      {#if decl.caps.cost === 'estimated'}
        {@const cannot = [
          !decl.caps.pdf && t('settings.claude.costNoPdf'),
          !decl.caps.backgroundTasks && t('settings.claude.costNoBackground'),
        ]
          .filter(Boolean)
          .join(' · ')}
        <Row label={t('settings.claude.cost')} desc={t('settings.claude.costEstimated', { engine: decl.name })}>
          {#if cannot}<span class="hint">{cannot[0].toUpperCase() + cannot.slice(1)}</span>{/if}
        </Row>
      {/if}
      {#if decl.caps.remoteControl}
        <!-- Said where it is set: the other harnesses have none (lot 0 of the 2.0 design). -->
        <Row label={t('settings.claude.remoteAll')} desc={t('settings.claude.remoteAllDesc')}>
          {#snippet extra()}
            <span class="only"><Icon name="info" size={14} />{t('settings.claude.remoteOnly', { harness: decl.name })}</span>
          {/snippet}
          <Switch label={t('settings.claude.remoteAll')} bind:on={s.remoteControlAll} />
        </Row>
      {/if}
    </Group>
  {/each}
  {#snippet foundLine(text: string, found: boolean, id?: string)}
    {#if text}
      <span class={found ? 'found' : 'more'} {id}
        >{#if found}<Icon name="check" size={14} />{/if}<span>{text}</span></span
      >
    {/if}
  {/snippet}
  {#snippet versionLine(text: string, engine: string, tone: string)}
    {#if text}<span class={tone || 'more'} id="engine-version-{engine}">{text}</span>{/if}
  {/snippet}
  <Group title={t('settings.claude.allHarnesses')} note={t('settings.claude.allHarnessesNote')}>
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
    <Row label={t('settings.claude.autoResume')} desc={t('settings.claude.autoResumeDesc')}>
      <Switch label={t('settings.claude.autoResume')} bind:on={s.autoResume} />
    </Row>
    <Row label={t('settings.claude.idleStop')}>
      <input class="field input num" type="number" min="0" aria-label={t('settings.claude.idleStopLabel')} bind:value={s.idleStopMinutes} />
      <span class="hint">{t('settings.claude.idleStopHint')}</span>
    </Row>
    <Row label={t('settings.claude.midTurnDelay')} desc={t('settings.claude.midTurnDelayDesc')}>
      <Chips label={t('settings.claude.midTurnDelay')} options={midTurnDelayOptions} bind:value={s.midTurnDelaySecs} />
    </Row>
  </Group>
  <Group title={t('settings.claude.attachments')} note={t('settings.claude.attachmentsNote')}>
    <Row label={t('settings.claude.attachMax')}>
      <input class="field input num" type="number" min="0" aria-label={t('settings.claude.attachMaxLabel')} bind:value={s.attachMaxMb} />
      <span class="hint">{t('settings.claude.attachMaxHint')}</span>
    </Row>
    <Row label={t('settings.claude.attachWarn')}>
      <input class="field input num" type="number" min="0" aria-label={t('settings.claude.attachWarnLabel')} bind:value={s.attachWarnMb} />
      <span class="hint">{t('settings.claude.attachWarnHint')}</span>
    </Row>
    <Row label={t('settings.claude.attachOrigin')} desc={t('settings.claude.attachOriginDesc')}>
      <Switch label={t('settings.claude.attachOrigin')} bind:on={s.attachOrigin} />
    </Row>
    <Row label={t('settings.claude.attachBigText')} desc={t('settings.claude.attachBigTextDesc')}>
      <Chips label={t('settings.claude.attachBigText')} options={bigTextOptions} bind:value={s.attachBigText} />
    </Row>
    <Row label={t('settings.claude.attachKeep')}>
      <input
        class="field input num"
        type="number"
        min="0"
        aria-label={t('settings.claude.attachKeepLabel')}
        bind:value={s.attachKeepDays}
      />
      <span class="hint">{t('settings.claude.attachKeepHint')}</span>
    </Row>
  </Group>
{:else if tab === 'notifications'}
  <Group
    title={t('settings.notifications.notifyFor')}
    note={IS_MAC ? t('settings.notifications.noteMac') : t('settings.notifications.noteOther')}
  >
    <!-- The types, ticked (`.checks` of the 2.0 design, `.w-tick` of the windows). -->
    <div class="checks">
      {#each ['questions', 'done', 'errors', 'tickets'] as const as k (k)}
        <label class="w-tick check-row">
          <input type="checkbox" bind:checked={s.notifyFor[k]} />
          <span class="tick-box" aria-hidden="true"><Icon name="check" size={12} /></span>
          <span>{t(`settings.notifications.${k}`)}</span>
        </label>
      {/each}
    </div>
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
      <button class="btn" onclick={() => api.playChime()}
        ><Icon name="play" size={14} />{t('settings.notifications.soundTestButton')}</button
      >
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
  <!-- Who it is (`.about-id` of the 2.0 design): the logo, the version, the harnesses found. -->
  <div class="about-id">
    <img src="/logo.svg" alt="" draggable="false" />
    <div class="who">
      <b>Escouade {app.version}</b>
      <span
        >{found.length
          ? t('settings.about.enginesFound', { list: found.map((e) => e.name).join(', ') })
          : t('settings.about.noEngine')}</span
      >
    </div>
    <button class="btn" disabled={checking} onclick={check}>{checking ? t('settings.about.checking') : t('settings.about.check')}</button>
    <span class="live" role="status" aria-live="polite" data-testid="update-live">{updateLive}</span>
  </div>
  <Group aria-label="Escouade">
    <Row label={t('settings.about.autoUpdate')} desc={t('settings.about.autoUpdateDesc')}>
      <Switch label={t('settings.about.autoUpdate')} bind:on={s.autoUpdate} />
    </Row>
    <!-- The update found, as the status bar says it (ESC-250): downloading, then ready with its restart and its notes. -->
    {#if update?.ready}
      {@const notes = update.notes.trim()}
      <Row label={t('settings.about.updateReady', { version: update.version })} desc={t('settings.about.updateReadyDesc')}>
        {#snippet extra()}
          <div class="upd" data-update="ready">
            {#if restartSeconds !== null}
              <b class="restart-in">{t('shell.status.restartIn', { seconds: restartSeconds })}</b>
            {/if}
            {#if notes}
              <button class="link" aria-expanded={notesOpen} aria-controls="update-notes" onclick={() => (notesOpen = !notesOpen)}
                >{notesOpen ? t('settings.about.hideNotes') : t('settings.about.seeNotes')}</button
              >
              {#if notesOpen}<div class="notes" id="update-notes"><Markdown text={notes} /></div>{/if}
            {/if}
          </div>
        {/snippet}
        {#if restartSeconds !== null}
          <button class="btn" onclick={postponeRestart}>{t('common.later')}</button>
        {/if}
        <!-- The update's window, as the status bar's: it says what a restart cuts, and « Plus tard » comes back here. -->
        <button class="btn primary" onclick={() => (app.modal = { kind: 'update', back: 'about' })}>{t('shell.update.restartNow')}</button>
      </Row>
    {:else if update}
      <Row label={t('settings.about.updateFound', { version: update.version })}>
        {#snippet extra()}<span class="more upd" data-update="downloading">{t('settings.about.updateDownloading')}</span>{/snippet}
      </Row>
    {/if}
    <!-- An update that does not download, said here while it lasts (choice 13 of the 2.0 design). -->
    {#if app.updateTrouble && !update}
      {@const trouble = app.updateTrouble}
      <Row label={t('settings.about.updateTrouble', { version: trouble.version })}>
        {#snippet extra()}
          <span class="warn">{t('settings.about.updateTroubleDesc', { error: trouble.message })}</span>
        {/snippet}
        {#if trouble.page}
          <button class="btn" onclick={() => openUrl(trouble.page!).catch((e) => app.toast(String(e), 'error'))}
            ><Icon name="ext" size={14} />{t('shell.update.openInBrowser')}</button
          >
        {/if}
      </Row>
    {/if}
    <Row label={t('settings.about.telemetry')} desc={t('settings.about.telemetryDesc')}>
      {#snippet extra()}
        <button class="link" onclick={() => openUrl(privacyUrl()).catch((e) => app.toast(String(e), 'error'))}
          >{t('settings.about.telemetryMore')}</button
        >
      {/snippet}
      <Switch label={t('settings.about.telemetry')} bind:on={() => s.telemetry === true, (on) => (s.telemetry = on)} />
    </Row>
    <Row label={t('settings.about.credits')} desc={t('settings.about.creditsDesc')}>
      <button class="btn" onclick={() => openUrl(CREDITS_URL).catch((e) => app.toast(String(e), 'error'))}
        >{t('settings.about.creditsOpen')}</button
      >
    </Row>
    <Row label={t('settings.about.site')} desc={t('settings.about.siteDesc')}>
      <button
        class="btn"
        aria-label={t('settings.about.siteOpenLabel')}
        onclick={() => openUrl(siteUrl()).catch((e) => app.toast(String(e), 'error'))}
        ><Icon name="ext" size={14} />{t('settings.about.siteOpen')}</button
      >
    </Row>
    <Row label={t('settings.about.discord')} desc={t('settings.about.discordDesc')}>
      <button
        class="btn"
        aria-label={t('settings.about.discordJoinLabel')}
        onclick={() => openUrl(DISCORD_URL).catch((e) => app.toast(String(e), 'error'))}
        ><Icon name="ext" size={14} />{t('settings.about.discordJoin')}</button
      >
    </Row>
    <!-- Support, said once here and never pushed: no window, no reminder (ESC-185). -->
    <Row label={t('settings.about.support')} desc={t('settings.about.supportDesc')}>
      <button class="btn" onclick={() => openUrl(supportUrl()).catch((e) => app.toast(String(e), 'error'))}
        ><Icon name="ext" size={14} />{t('settings.about.supportOpen')}</button
      >
    </Row>
    <Row label={t('settings.about.localData')}>
      {#snippet extra()}<span class="more mono data">{dataDir ? tildify(dataDir) : '~/.escouade'}</span>{/snippet}
      {#if dataDir}
        <button class="btn" onclick={() => openPath(dataDir!).catch((e) => app.toast(String(e), 'error'))}
          ><Icon name="folder" size={14} />{t('nav.titleBar.menu.openFolder')}</button
        >
      {/if}
    </Row>
  </Group>
{/if}

<style>
  .input {
    width: 320px;
    max-width: 100%;
    flex: 0 1 auto;
  }
  .input.num {
    width: 64px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  /* A harness's state beside its name: green ready, amber without an account, red refused or not found. */
  .hstate {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .hstate i {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--run);
  }
  .hstate.warn {
    color: var(--warn-ink);
  }
  .hstate.warn i {
    background: var(--wait);
  }
  .hstate.bad {
    color: var(--del);
  }
  .hstate.bad i {
    background: var(--del);
  }
  .version {
    font-size: 11.5px;
  }
  .self-update.plain {
    color: var(--muted);
  }
  .checks {
    display: grid;
    gap: 2px;
    margin: 8px 0 14px;
  }
  .check-row {
    height: 32px;
    margin: 0 -8px;
    padding: 0 8px;
    border-radius: 7px;
    color: var(--text);
  }
  .check-row > input {
    left: 8px;
  }
  .check-row:hover {
    background: var(--hover);
  }
  .about-id {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 22px 0 18px;
  }
  .about-id img {
    width: 44px;
    height: 44px;
  }
  .who {
    display: grid;
    min-width: 0;
  }
  .who b {
    font-size: 16px;
    font-weight: 640;
  }
  .who span {
    color: var(--muted);
    font-size: 12.5px;
  }
  .about-id .btn {
    flex: none;
    margin-left: auto;
  }
  /* Heard, not seen: the update found, told to a screen reader. */
  .live {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .upd {
    display: grid;
    justify-items: start;
    gap: 4px;
    margin-top: 2px;
  }
  .restart-in {
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
  }
  /* The notes, as the update's window has them: soft, their items close together. */
  .notes {
    max-width: 520px;
    color: var(--soft);
  }
  .notes :global(.md) {
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .notes :global(.md li) {
    margin: 0 0 3px;
  }
  .notes :global(.md ul) {
    margin: 0;
    padding-left: 18px;
  }
  .link {
    justify-self: start;
    padding: 0;
    border: none;
    background: none;
    color: var(--p-ink);
    font-size: 12px;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .data {
    color: var(--soft);
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
</style>
