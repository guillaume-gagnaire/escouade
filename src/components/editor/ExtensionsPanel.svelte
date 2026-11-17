<script lang="ts">
  // The extensions of Open VSX, beside the VS Code view (ESC-481, A5): search, a card, install, update, turn off, remove,
  // choose a theme. Everything goes through the core (`vsx_*`): it searches, downloads, checks the sum, the signature and
  // the archive, holds the allowed list (the settings, each project's `.escouade.yml`, the team, the managed device) and
  // serves the files. What a card shows is Open VSX's, a third party's: its icon is asked of the core when shown, its
  // README rendered as a third party's Markdown (nothing remote without a click). VS Code's question of trust per
  // publisher stays, with whether Open VSX verified the namespace.
  import { onMount } from 'svelte';
  import { locale, t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import {
    brings,
    chosenTheme,
    downloads,
    status,
    statusLabel,
    themeKey,
    type Card,
    type ExtState,
    type Installed,
    type Refusal,
    type Summary,
    type Update,
  } from '../../lib/vscode/extensions';
  import Markdown from '../conv/Markdown.svelte';
  import ExtIcon from './ExtIcon.svelte';
  import Icon from '../Icon.svelte';

  let { projects, onclose, onlaid }: { projects: string[]; onclose: () => void; onlaid: (installed: Installed) => void | Promise<void> } =
    $props();

  let installed = $state<Installed | null>(null);
  let query = $state('');
  let searched = $state('');
  let results = $state<Summary[] | null>(null);
  let searching = $state(false);
  let card = $state<Card | null>(null);
  let opening = $state<string | null>(null);
  let busy = $state<string | null>(null);
  let refusal = $state<Refusal | null>(null);
  let trust = $state<{ id: string; version: string | null; publisher: string; namespace: string; verified: boolean } | null>(null);
  let updates = $state<Update[] | null>(null);
  let checking = $state(false);

  const list = $derived(installed?.extensions ?? []);
  const privateName = $derived(installed?.private ?? null);
  const imposed = $derived((installed?.layers ?? []).filter((l) => l.imposed));
  const theme = $derived(chosenTheme(list, installed?.theme ?? null));

  async function refresh() {
    installed = await api.vsxInstalled(projects);
    await onlaid(installed);
  }

  onMount(() => {
    void app.run(refresh());
  });

  async function search(e?: Event) {
    e?.preventDefault();
    const q = query.trim();
    if (!q || privateName) return;
    searching = true;
    refusal = null;
    card = null;
    try {
      const out = await api.vsxSearch(projects, q);
      searched = q;
      results = out.value?.extensions ?? [];
      refusal = out.refusal;
    } finally {
      searching = false;
    }
  }

  async function openCard(id: string) {
    opening = id;
    refusal = null;
    try {
      const out = await api.vsxCard(projects, id);
      card = out.value;
      refusal = out.refusal;
    } finally {
      opening = null;
    }
  }

  const nameOf = (id: string) => list.find((e) => e.id === id)?.displayName ?? card?.detail.displayName ?? id;

  async function install(id: string, version: string | null, trusted = false) {
    busy = id;
    refusal = null;
    try {
      const before = list.find((e) => e.id.toLowerCase() === id.toLowerCase())?.version ?? null;
      const out = await api.vsxInstall(projects, id, version, trusted);
      if (out.refusal?.code === 'untrusted') {
        const d = card?.detail;
        trust = {
          id,
          version,
          publisher: d?.publisher ?? id.split('.')[0],
          namespace: d?.namespace ?? id.split('.')[0],
          verified: d?.verified ?? false,
        };
        return;
      }
      if (out.refusal || !out.value) {
        refusal = out.refusal;
        return;
      }
      const e = out.value;
      app.toast(
        before
          ? t('editor.ext.updatedOk', { name: e.displayName, version: e.version })
          : t('editor.ext.installedOk', { name: e.displayName }),
      );
      updates = updates?.filter((u) => u.id !== id) ?? null;
      await refresh();
      if (card && card.detail.id === id) card = { ...card, installed: e.version };
    } finally {
      busy = null;
    }
  }

  async function answerTrust(yes: boolean) {
    const asked = trust;
    trust = null;
    if (yes && asked) await install(asked.id, asked.version, true);
  }

  async function setEnabled(e: ExtState, on: boolean) {
    await app.run(api.vsxSetEnabled(e.id, on));
    await refresh();
  }

  async function remove(e: ExtState) {
    await app.run(api.vsxRemove(e.id));
    app.toast(t('editor.ext.removedOk', { name: e.displayName }));
    updates = updates?.filter((u) => u.id !== e.id) ?? null;
    if (card?.detail.id === e.id) card = { ...card, installed: null };
    await refresh();
  }

  async function useTheme(e: ExtState | null, id: string | null) {
    await app.run(api.vsxSetTheme(e && id ? themeKey(e, id) : null));
    await refresh();
  }

  async function checkUpdates() {
    checking = true;
    refusal = null;
    try {
      const out = await api.vsxUpdates(projects);
      updates = out.value ?? [];
      refusal = out.refusal;
    } finally {
      checking = false;
    }
  }
</script>

<aside class="ext-panel" data-testid="vscode-ext-panel" aria-label={t('editor.ext.title')}>
  <header class="head">
    <span class="title">{t('editor.ext.title')}<span class="src">{t('editor.ext.source')}</span></span>
    <button class="icon-btn" aria-label={t('editor.ext.close')} use:tip={t('editor.ext.close')} onclick={onclose}>
      <Icon name="close" size={14} />
    </button>
  </header>

  <div class="body">
    {#if privateName}
      <p class="note" data-testid="vscode-ext-private">{t('editor.ext.private', { name: privateName })}</p>
    {:else}
      <form class="search" onsubmit={search}>
        <Icon name="search" size={14} />
        <input
          class="field"
          data-testid="vscode-ext-search"
          placeholder={t('editor.ext.search')}
          aria-label={t('editor.ext.search')}
          spellcheck="false"
          bind:value={query}
        />
      </form>
    {/if}
    {#each imposed as layer (layer.by)}
      <p class="note imposed" data-testid="vscode-ext-imposed">
        <Icon name="lock" size={12} /><span>{t('editor.ext.imposed', { by: layer.by })}</span>
      </p>
    {/each}

    {#if refusal}
      <p class="refusal" role="alert" data-testid="vscode-ext-refusal" data-code={refusal.code}>{refusal.message}</p>
    {/if}

    {#if trust}
      <div class="trust" role="alertdialog" aria-labelledby="ext-trust-title" data-testid="vscode-ext-trust">
        <p id="ext-trust-title" class="trust-title">{t('editor.ext.trust.title', { publisher: trust.publisher })}</p>
        <p>
          {trust.verified
            ? t('editor.ext.trust.verified', { namespace: trust.namespace })
            : t('editor.ext.trust.unverified', { namespace: trust.namespace })}
        </p>
        <p>{t('editor.ext.trust.body')}</p>
        <div class="row">
          <button class="btn primary" data-testid="vscode-ext-trust-yes" onclick={() => void answerTrust(true)}
            >{t('editor.ext.trust.yes')}</button
          >
          <button class="btn" onclick={() => void answerTrust(false)}>{t('editor.ext.trust.no')}</button>
        </div>
      </div>
    {/if}

    {#if card}
      {@const d = card.detail}
      <section class="card" data-testid="vscode-ext-card" data-id={d.id}>
        <button class="link" onclick={() => (card = null)}><Icon name="left" size={12} />{t('editor.ext.back')}</button>
        <div class="card-head">
          <ExtIcon url={d.icon} {projects} off={!!privateName} />
          <div class="who">
            <strong class="name">{d.displayName}</strong>
            <span class="meta">{t('editor.ext.by', { publisher: d.publisher })}</span>
            <span class="meta mono">{d.id} · {t('editor.ext.version', { version: d.version })}</span>
          </div>
        </div>
        <div class="badges">
          {#if d.verified}
            <span class="badge ok" data-testid="vscode-ext-verified" use:tip={t('editor.ext.verifiedTip')}
              ><Icon name="shield" size={12} />{t('editor.ext.verified')}</span
            >
          {:else}
            <span class="badge warn" data-testid="vscode-ext-unverified" use:tip={t('editor.ext.unverifiedTip')}
              >{t('editor.ext.unverified')}</span
            >
          {/if}
          <span class="meta">{t('editor.ext.downloads', { count: downloads(d.downloads, locale.ui) })}</span>
        </div>
        <p class="desc">{d.description}</p>
        {#if card.refused}
          <p class="refusal" data-testid="vscode-ext-card-refused">{card.refused}</p>
        {/if}
        <div class="row">
          {#if card.installed === null}
            <button
              class="btn primary"
              data-testid="vscode-ext-install"
              disabled={!!card.refused || busy !== null || !d.downloadable}
              onclick={() => void install(d.id, null)}>{busy === d.id ? t('editor.ext.installing') : t('editor.ext.install')}</button
            >
          {:else if card.installed !== d.version}
            <button
              class="btn primary"
              data-testid="vscode-ext-update"
              disabled={!!card.refused || busy !== null}
              onclick={() => void install(d.id, d.version)}>{t('editor.ext.update', { version: d.version })}</button
            >
          {:else}
            <span class="badge ok"><Icon name="check" size={12} />{t('editor.ext.status.active')}</span>
          {/if}
        </div>
        <p class="fine">{t('editor.ext.thirdParty')}</p>
        <div class="readme" data-testid="vscode-ext-readme">
          {#if card.readme}<Markdown text={card.readme} />{:else}<p class="meta">{t('editor.ext.noReadme')}</p>{/if}
        </div>
      </section>
    {:else}
      {#if results}
        <section>
          <h3>{t('editor.ext.results')}</h3>
          {#if results.length === 0}
            <p class="meta">{t('editor.ext.none', { query: searched })}</p>
          {/if}
          <ul class="list">
            {#each results as r (r.id)}
              <li>
                <button
                  class="item"
                  data-testid="vscode-ext-result"
                  data-id={r.id}
                  disabled={opening !== null}
                  onclick={() => void openCard(r.id)}
                >
                  <ExtIcon url={r.icon} {projects} off={!!privateName} size={16} />
                  <span class="who">
                    <span class="name"
                      >{r.displayName}{#if r.verified}<span class="tick" aria-label={t('editor.ext.verified')}
                          ><Icon name="shield" size={11} /></span
                        >{/if}</span
                    >
                    <span class="meta">{r.description}</span>
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {:else if searching}
        <p class="meta">{t('editor.ext.searching')}</p>
      {/if}

      <section data-testid="vscode-ext-installed">
        <h3>{t('editor.ext.installed')}</h3>
        {#if theme}
          {@const owner = list.find((e) => e.key === installed?.theme?.split('/')[0])}
          <p class="note" data-testid="vscode-ext-theme-on">
            {t('editor.ext.themeOn', { theme })}
            <button class="link" data-testid="vscode-ext-theme-off" onclick={() => void useTheme(owner ?? null, null)}
              >{t('editor.ext.escouadeTheme')}</button
            >
          </p>
        {/if}
        {#if list.length === 0}
          <p class="meta">{t('editor.ext.noneInstalled')}</p>
        {/if}
        <ul class="list">
          {#each list as e (e.key)}
            {@const s = status(e)}
            <li class="installed" data-testid="vscode-ext-item" data-id={e.id} data-status={s}>
              <div class="line">
                <span class="name">{e.displayName}</span>
                <span class="meta mono">{e.version}</span>
                <span class="chip" data-status={s}>{statusLabel(s)}</span>
              </div>
              {#if e.refused}<p class="meta">{e.refused}</p>{:else if e.waits}<p class="meta">{e.waits}</p>{/if}
              {#if brings(e.contributes)}<p class="meta">{brings(e.contributes)}</p>{/if}
              <p class="fine">{t('editor.ext.signed')}</p>
              <div class="row">
                {#if s === 'active'}
                  {#each e.contributes.themes as th (th)}
                    {#if theme !== th}
                      <button class="btn" data-testid="vscode-ext-use-theme" onclick={() => void useTheme(e, th)}
                        >{t('editor.ext.useTheme', { theme: th })}</button
                      >
                    {/if}
                  {/each}
                {/if}
                <button class="btn" data-testid="vscode-ext-toggle" onclick={() => void setEnabled(e, !e.enabled)}
                  >{e.enabled ? t('editor.ext.disable') : t('editor.ext.enable')}</button
                >
                <button class="btn" data-testid="vscode-ext-remove" onclick={() => void remove(e)}>{t('editor.ext.remove')}</button>
              </div>
            </li>
          {/each}
        </ul>
        {#if list.length && !privateName}
          <div class="row">
            <button class="btn" data-testid="vscode-ext-check" disabled={checking} onclick={() => void checkUpdates()}
              >{checking ? t('editor.ext.checking') : t('editor.ext.checkUpdates')}</button
            >
          </div>
        {/if}
        {#if updates}
          {#if updates.length === 0}
            <p class="meta" data-testid="vscode-ext-no-updates">{t('editor.ext.noUpdates')}</p>
          {:else}
            <h3>{t('editor.ext.updates')}</h3>
            <ul class="list">
              {#each updates as u (u.id)}
                <li class="installed" data-testid="vscode-ext-update-item" data-id={u.id}>
                  <div class="line"><span class="name">{nameOf(u.id)}</span><span class="meta mono">{u.from} → {u.to}</span></div>
                  {#if u.refused}<p class="meta">{u.refused}</p>{/if}
                  <div class="row">
                    <button
                      class="btn primary"
                      data-testid="vscode-ext-update-one"
                      disabled={!!u.refused || busy !== null}
                      onclick={() => void install(u.id, u.to)}>{t('editor.ext.update', { version: u.to })}</button
                    >
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      </section>
      <p class="fine">{t('editor.ext.openVsxOnly')}</p>
    {/if}
  </div>
</aside>

<style>
  .ext-panel {
    flex: none;
    display: flex;
    flex-direction: column;
    width: min(340px, 45%);
    min-width: 240px;
    min-height: 0;
    border-left: 1px solid var(--line);
    background: var(--frame);
    color: var(--text);
    font-size: 12.5px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 8px 0 12px;
    border-bottom: 1px solid var(--line);
  }
  .title {
    flex: 1;
    font-weight: 600;
  }
  .src {
    margin-left: 6px;
    color: var(--muted);
    font-weight: 400;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--soft);
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--hover);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 10px 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
    color: var(--muted);
  }
  .search .field {
    flex: 1;
    min-width: 0;
    height: 28px;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    outline: none;
  }
  .search:focus-within {
    border-color: var(--p);
  }
  h3 {
    margin: 4px 0 6px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .item {
    display: flex;
    gap: 8px;
    width: 100%;
    padding: 6px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .item:hover {
    background: var(--hover);
  }
  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }
  .name {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .tick {
    color: var(--p-ink);
  }
  .meta {
    margin: 0;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .mono {
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .fine {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  .note {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 8px;
    border-radius: 8px;
    background: var(--panel);
    color: var(--soft);
  }
  .note.imposed {
    flex-wrap: nowrap;
    align-items: flex-start;
  }
  .note.imposed :global(.icon) {
    margin-top: 2px;
  }
  .refusal {
    margin: 0;
    padding: 8px;
    border-radius: 8px;
    background: color-mix(in oklab, var(--del) 14%, transparent);
    color: var(--text);
  }
  .trust {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    border: 1px solid var(--p);
    border-radius: 10px;
    background: var(--panel);
  }
  .trust p {
    margin: 0;
  }
  .trust-title {
    font-weight: 600;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card-head {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 7px;
    border-radius: 10px;
    font-size: 11.5px;
  }
  .badge.ok {
    background: color-mix(in oklab, var(--p) 16%, transparent);
    color: var(--p-ink);
  }
  .badge.warn {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--text);
  }
  .desc {
    margin: 0;
  }
  .readme {
    padding-top: 6px;
    border-top: 1px solid var(--line);
    overflow-wrap: anywhere;
  }
  .installed {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 8px;
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .chip {
    margin-left: auto;
    padding: 1px 7px;
    border-radius: 9px;
    background: var(--panel);
    color: var(--soft);
    font-size: 11px;
  }
  .chip[data-status='active'] {
    background: color-mix(in oklab, var(--p) 16%, transparent);
    color: var(--p-ink);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  /* The app's buttons (`.btn`, `.btn.primary`), smaller in the panel. */
  .row :global(.btn) {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--p-ink);
    font: inherit;
    cursor: pointer;
  }
</style>
