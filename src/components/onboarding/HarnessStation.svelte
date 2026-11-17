<script lang="ts">
  import { engineOrder, hasAccount } from '../../lib/accounts';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { denial, HARNESSES_KEY } from '../../lib/locks.svelte';
  import { onboarding, type Primary } from '../../lib/onboarding.svelte';
  import { OS } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import type { EngineDecl, EngineId, EngineInfo, MachineAccount } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Choices from './Choices.svelte';
  import StationHead from './StationHead.svelte';

  // Station 1, the harness (docs/ONBOARDING.md § 4.1): every harness Escouade knows, with what is found of it here
  // (signed in, installed, too old, not found), the one recommended selected: the first signed in on the machine, else the
  // first found. One not found carries the command that installs it, copied in one click, and « Chercher à nouveau »; the
  // window back in front looks again by itself. A harness a policy denies is greyed, with why, and never recommended.
  // « Continuer » writes what Réglages › Comptes et harness writes: the harness first in « Ordre des harness », its plugin
  // from the seed (ESC-373), and, when it is signed in on the machine, its account (the account's station folded in, O6).
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  type Kind = 'ready' | 'installed' | 'tooOld' | 'unsupported' | 'absent';
  type Row = {
    id: EngineId;
    decl: EngineDecl;
    kind: Kind;
    version: string | null;
    email: string | null;
    denied: string | null;
    disabled: boolean;
  };

  /** What is found of each harness (its version, too old), read again when one is found or installed. */
  let infos = $state<EngineInfo[] | null>(null);
  $effect(() => {
    void app.enginesFound;
    void app.engines;
    let stale = false;
    api
      .engines()
      .then((all) => {
        if (!stale) infos = all;
      })
      .catch(() => {
        if (!stale) infos = [];
      });
    return () => {
      stale = true;
    };
  });

  /** The machine's account of each harness found: signed in, as whom. */
  let machine = $state<Partial<Record<EngineId, MachineAccount | null>>>({});
  $effect(() => {
    for (const e of app.enginesFound)
      if (!(e in machine))
        api
          .accountMachine(e)
          .then((m) => (machine[e] = m))
          .catch(() => (machine[e] = null));
  });

  const KIND_TEXT = {
    installed: 'onboarding.harness.installed',
    tooOld: 'onboarding.harness.tooOld',
    unsupported: 'onboarding.harness.unsupported',
    absent: 'onboarding.harness.absent',
  } as const;
  const RANK: Record<Kind, number> = { ready: 0, installed: 1, tooOld: 2, unsupported: 2, absent: 3 };
  const rows = $derived.by((): Row[] => {
    const order = engineOrder();
    const place = (id: EngineId) => (order.includes(id) ? order.indexOf(id) : order.length + app.engines.findIndex((e) => e.id === id));
    return app.engines
      .map((decl): Row => {
        const id = decl.id;
        const info = infos?.find((i) => i.id === id);
        const m = machine[id];
        const found = app.enginesFound.includes(id);
        const kind: Kind = !found
          ? 'absent'
          : info?.state === 'tooOld' || info?.state === 'unsupported'
            ? info.state
            : hasAccount(id) || m?.connected
              ? 'ready'
              : 'installed';
        const denied = denial(HARNESSES_KEY, id);
        return {
          id,
          decl,
          kind,
          version: info?.version ?? null,
          email: m?.email ?? null,
          denied,
          disabled: !!denied || kind === 'tooOld' || kind === 'unsupported',
        };
      })
      .sort((a, b) => Number(!!a.denied) - Number(!!b.denied) || RANK[a.kind] - RANK[b.kind] || place(a.id) - place(b.id));
  });
  /** The one recommended: the first signed in, else the first found, never one denied. */
  const recommended = $derived(
    rows.find((r) => !r.disabled && r.kind === 'ready')?.id ?? rows.find((r) => !r.disabled && r.kind === 'installed')?.id ?? null,
  );
  const allowed = $derived(rows.filter((r) => !r.denied));
  const none = $derived(!rows.some((r) => r.kind !== 'absent'));

  let chosen = $state<EngineId | null>(null);
  // The one chosen before (back from a later station), else the one an account runs on (done again), else the recommended
  // one, else the first not found (its command shown).
  $effect(() => {
    if (chosen && rows.some((r) => r.id === chosen && !r.disabled)) return;
    const had = onboarding.engine ?? engineOrder().find((e) => hasAccount(e));
    chosen =
      (had && rows.some((r) => r.id === had && !r.disabled) ? had : null) ?? recommended ?? rows.find((r) => !r.disabled)?.id ?? null;
  });
  const row = $derived(rows.find((r) => r.id === chosen));

  const install = (d: EngineDecl) => (OS === 'windows' ? d.install.windows : d.install.unix);
  async function copy(d: EngineDecl) {
    await navigator.clipboard?.writeText(install(d)).catch(() => {});
    app.toast(t('onboarding.harness.copied'), 'ok');
  }

  let searching = $state(false);
  /** « Chercher à nouveau »: the harnesses looked for again (one may have been installed since). */
  async function searchAgain() {
    searching = true;
    try {
      await app.refreshEngines();
      machine = {};
    } finally {
      searching = false;
    }
  }

  /** « Continuer »: what Réglages › Comptes et harness writes, and nothing else. */
  async function next(): Promise<boolean> {
    const e = chosen;
    if (!e || !row || row.kind === 'absent' || row.disabled) return false;
    onboarding.engine = e;
    const known = engineOrder();
    const now = app.settings.engineOrder?.length ? app.settings.engineOrder : known;
    if (now[0] !== e) {
      const order = [e, ...known.filter((x) => x !== e), ...(app.settings.engineOrder ?? []).filter((x) => x !== e && !known.includes(x))];
      const saved = await app.run(api.engineOrderSet(order));
      if (!saved) return false;
      app.settings.engineOrder = saved;
    }
    const m = machine[e];
    if (!hasAccount(e) && m?.connected && !m.added) {
      // Signed in on the machine: its account added as « Ajouter un compte… » adds it (its plugin with it).
      const made = await app.run(api.accountAddMachine(e));
      if (!made) return false;
      if (!app.settings.accounts.some((a) => a.id === made.id)) app.settings.accounts = [...app.settings.accounts, made];
      void app.run(api.refreshUsage());
    } else {
      // Its plugin from the seed, with no network, when it is not installed (the sort of the seed, ESC-373).
      void api.pluginNeedEngine(e).catch(() => {});
    }
    return true;
  }

  $effect(() => {
    primary = {
      label: t('onboarding.continue'),
      disabled: !row || row.kind === 'absent' || row.disabled,
      run: next,
    };
  });

  /** What is said of a harness's state, beside its name. */
  function stateText(r: Row): string {
    if (r.denied) return r.denied;
    if (r.kind === 'ready') return r.email ? t('onboarding.harness.ready', { email: r.email }) : t('onboarding.harness.readyNoEmail');
    return t(KIND_TEXT[r.kind as Exclude<Kind, 'ready'>]);
  }
</script>

<div class="q" data-testid="onboarding-harness">
  <StationHead step="harness" question={t('onboarding.harness.question')}>
    {#snippet why()}{t('onboarding.harness.why')}{/snippet}
  </StationHead>
  {#if none}<p class="note" role="status">{t('onboarding.harness.none')}</p>{/if}
  {#if allowed.length === 1 && rows.length > 1}
    <p class="note"><Icon name="lock" size={12} />{t('onboarding.harness.onlyAllowed', { harness: allowed[0].decl.name })}</p>
  {/if}
  <Choices items={rows} bind:value={chosen} label={t('onboarding.harness.question')}>
    {#snippet head(r: Row)}
      <span class="t">
        <b>{r.decl.name}</b>
        {#if r.version}<span class="ver">{r.version}</span>{/if}
        {#if r.id === recommended}<span class="rec">{t('settings.permissions.recommended')}</span>{/if}
        <span class="state {r.denied ? 'no' : r.kind}">
          {#if r.denied}<Icon name="lock" size={12} />{:else if r.kind === 'ready'}<Icon name="check" size={12} />{/if}{stateText(r)}
        </span>
      </span>
      {#if r.kind === 'installed' && !r.denied && r.id === chosen}<span class="d">{t('onboarding.harness.installedHint')}</span>{/if}
    {/snippet}
    {#snippet extra(r: Row, on: boolean)}
      {#if on && r.kind === 'absent' && !r.denied}
        <div class="x">
          <span class="cmd"
            ><code class="mono">{install(r.decl)}</code><button
              class="btn"
              aria-label={t('onboarding.harness.copyLabel', { harness: r.decl.name })}
              onclick={() => copy(r.decl)}><Icon name="copy" size={12} />{t('onboarding.harness.copy')}</button
            ></span
          >
          {#if r.decl.installFrom}
            <!-- Worded by a plugin of anyone's, not by Escouade (SEC-30). -->
            <small class="from">{t('plugins.harness.commandFrom', { plugin: r.decl.installFrom })}</small>
          {/if}
        </div>
      {/if}
    {/snippet}
  </Choices>
  {#if row && row.kind !== 'absent' && !row.disabled}
    <div class="notes">
      {#if row.kind === 'ready' && !hasAccount(row.id) && machine[row.id]?.connected}<p>{t('onboarding.harness.machineAccount')}</p>{/if}
      {#if app.enginesOff.includes(row.id)}<p>{t('onboarding.harness.plugin', { harness: row.decl.name })}</p>{/if}
      {#if engineOrder()[0] !== row.id && allowed.length > 1}<p>{t('onboarding.harness.first')}</p>{/if}
    </div>
  {/if}
  <div class="search">
    <button class="btn" disabled={searching} onclick={searchAgain}><Icon name="restart" size={14} />{t('shell.welcome.searchAgain')}</button
    >
    <span>{app.settings.wslDistro ? t('shell.welcome.searchedWsl', { distro: app.settings.wslDistro }) : t('shell.welcome.searched')}</span>
  </div>
</div>

<style>
  .q {
    display: grid;
    gap: 22px;
    width: min(640px, 100%);
  }
  .t {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .t b {
    color: var(--text);
    font-weight: 620;
  }
  .ver {
    color: var(--dim);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .rec {
    padding: 1px 8px;
    border-radius: 999px;
    color: var(--teal-ink);
    font-size: 11.5px;
    font-weight: 600;
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--brand) 45%, transparent);
    background: color-mix(in oklch, var(--brand) 8%, transparent);
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .state.ready {
    color: var(--run);
  }
  .state.installed {
    color: var(--wait);
  }
  .state.tooOld,
  .state.unsupported {
    color: var(--del);
  }
  .d,
  .note,
  .notes p {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--soft);
  }
  .notes {
    display: grid;
    gap: 4px;
  }
  .x {
    display: grid;
    gap: 6px;
    padding: 0 14px 12px 42px;
  }
  .cmd {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 4px 0 10px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .cmd code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--code-fg);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: all;
  }
  .cmd .btn {
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
  }
  .from {
    color: var(--warn-ink);
    font-size: 12px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 12px;
  }
  @media (max-width: 1100px) {
    .q {
      gap: 16px;
    }
  }
</style>
