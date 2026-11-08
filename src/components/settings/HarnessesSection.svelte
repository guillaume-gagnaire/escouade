<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { OS } from '../../lib/platform';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { ClaudeLocated, ClaudeUpdateStatus, EngineDecl, EngineInfo } from '../../lib/types';
  import Group from './Group.svelte';
  import ModelFavorites from './ModelFavorites.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';
  import Icon from '../Icon.svelte';

  // The harnesses of « Comptes et harness » (from the 2.0 tab « Harness »): a group per harness, its state and version
  // beside its name, its executable, what holds for it alone; then what holds for all of them that R4 moves to « Quand un
  // quota est épuisé ». Their remote control went to « Agents », their attachments to « Avancé ».
  const s = $derived(settingsForm.settings);

  // Where the `claude` of the path is, asked a moment after the path or the distribution changes (a look into WSL
  // may take seconds): an answer to an earlier path is dropped.
  let located = $state<ClaudeLocated | null>(null);
  $effect(() => {
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
    if (!saved) return;
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
</script>

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
  <Row label={t('settings.claude.autoResume')} desc={t('settings.claude.autoResumeDesc')}>
    <Switch label={t('settings.claude.autoResume')} bind:on={s.autoResume} />
  </Row>
</Group>

<style>
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
