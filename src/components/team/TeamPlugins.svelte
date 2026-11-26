<script lang="ts">
  // « Plugins de l'équipe » (TS-14): the team's private store. The team's key and its namespace (made on an
  // administrator's device, never on the server, with its backup key); « Publier un plugin »: the store's checks played
  // on this device, the compilation twice; « À signer »: an administrator plays them again and signs only when both
  // digests are the same; « Publiés »: withdrawn by the team's revocation list. Installing stays each member's gesture,
  // from the page Plugins.
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { t } from '../../lib/i18n';
  import { fDate } from '../../lib/format';
  import { errorText, teams } from '../../lib/teams.svelte';
  import {
    namespaceOk,
    teamPluginsApi,
    type Checked,
    type ProposalRow,
    type Report,
    type TeamPluginRow,
    type TeamPluginsView,
  } from '../../lib/team-plugins';

  let { teamId, onback }: { teamId: string; onback: () => void } = $props();

  let view = $state<TeamPluginsView | null>(null);
  let busy = $state(false);
  let failure = $state('');
  let notice = $state('');

  // The key's form.
  let namespace = $state('');
  let password = $state('');
  let password2 = $state('');
  let backupDir = $state('');
  // The key's import.
  let importing = $state(false);
  let keyFile = $state('');
  let importPassword = $state('');
  // « Publier un plugin ».
  let folder = $state('');
  let tag = $state('');
  let report = $state<Report | null>(null);
  // « À signer » and « Retirer ».
  let signing = $state<ProposalRow | null>(null);
  let signPassword = $state('');
  /** The administrator agreed that this device compiles the member's plugin (ESC-648): the core refuses to sign without. */
  let compileHere = $state(false);
  let withdrawing = $state<TeamPluginRow | null>(null);
  let reason = $state('');
  let withdrawPassword = $state('');

  const teamName = $derived(view?.name || teams.byId(teamId)?.name || '');
  const nsState = $derived(namespace ? namespaceOk(namespace) : 'ok');
  const canCreate = $derived(
    !!namespace && nsState === 'ok' && password.length >= 12 && password === password2 && !!backupDir.trim() && !busy,
  );

  async function act(f: () => Promise<TeamPluginsView>) {
    busy = true;
    failure = '';
    try {
      view = await f();
      return true;
    } catch (e) {
      failure = errorText(e);
      return false;
    } finally {
      busy = false;
    }
  }

  onMount(() => void act(() => teamPluginsApi.get(teamId)));

  async function pick(target: 'backup' | 'folder' | 'key') {
    const got = await open({ directory: target !== 'key', multiple: false });
    if (typeof got !== 'string' || !got) return;
    if (target === 'backup') backupDir = got;
    else if (target === 'folder') folder = got;
    else keyFile = got;
  }

  async function createKey() {
    if (!canCreate) return;
    notice = '';
    const ok = await act(async () => {
      const made = await teamPluginsApi.createKey(teamId, namespace.trim(), password, backupDir.trim());
      notice = t('teamPlugins.key.created', { path: made.backup });
      return made.view;
    });
    if (ok) password = password2 = '';
  }

  async function importKey() {
    if (await act(() => teamPluginsApi.importKey(teamId, keyFile.trim(), importPassword))) {
      importing = false;
      importPassword = '';
    }
  }

  async function submit() {
    notice = '';
    report = null;
    await act(async () => {
      const s = await teamPluginsApi.submit(teamId, folder.trim(), tag.trim());
      report = s.report;
      if (s.proposal && s.view) {
        notice = t('teamPlugins.publish.sent', { id: s.report.id, version: s.report.version, digest: s.report.digest.slice(0, 12) });
        return s.view;
      }
      return view!;
    });
  }

  async function sign() {
    if (!signing) return;
    const id = signing.id;
    notice = '';
    const ok = await act(async () => {
      const r = await teamPluginsApi.sign(teamId, id, signPassword, compileHere);
      notice = t('teamPlugins.proposed.signed', { digest: r.report.digest.slice(0, 12) });
      return r.view;
    });
    if (ok) {
      signing = null;
      signPassword = '';
      compileHere = false;
    }
  }

  async function withdraw() {
    if (!withdrawing) return;
    const r = withdrawing;
    const ok = await act(() => teamPluginsApi.revoke(teamId, r.id, reason.trim(), withdrawPassword, [r.version]));
    if (ok) {
      withdrawing = null;
      reason = withdrawPassword = '';
    }
  }

  const checkName = (c: Checked) => t(`teamPlugins.check.${c.key}` as 'teamPlugins.check.archive');
</script>

{#snippet checks(list: Checked[])}
  <ul class="checks" aria-label={t('teamPlugins.publish.title')}>
    {#each list as c (c.key)}
      <li class:bad={!c.ok} data-testid="team-plugin-check" data-ok={c.ok}>
        <span class="mark" aria-hidden="true">{c.ok ? '✓' : '✗'}</span>
        <span class="ck">{checkName(c)}</span>
        <span class="muted small">{c.detail}</span>
      </li>
    {/each}
  </ul>
{/snippet}

<div class="tplugins">
  <p class="lead">{t('teamPlugins.lead')}</p>

  {#if !view}
    <p class="muted">{failure || t('teamPlugins.loading')}</p>
  {:else}
    {#if view.alarm}
      <div class="alarm" role="alert">
        <span>{t('teamPlugins.alarm')}</span>
        <button class="btn" disabled={busy} onclick={() => act(() => teamPluginsApi.retry(teamId))}>{t('teamPlugins.retry')}</button>
      </div>
    {/if}
    {#if view.conflict}<p class="advice small" role="alert">{t('teamPlugins.conflict', { id: view.conflict })}</p>{/if}
    {#if view.refused}<p class="advice small">{t('teamPlugins.refused', { count: view.refused })}</p>{/if}

    <section aria-label={t('teamPlugins.key.title')}>
      <h4>{t('teamPlugins.key.title')}</h4>
      {#if view.key}
        <div class="item" data-testid="team-plugin-key">
          <div class="head">
            <span class="name mono">{t('teamPlugins.key.declared', { namespace: view.key.namespace })}</span>
          </div>
          <p class="muted small mono">
            {view.key.ids.length > 1
              ? t('teamPlugins.key.ids', { daily: view.key.ids[0], backup: view.key.ids[1] })
              : t('teamPlugins.key.one', { daily: view.key.ids[0] })}
          </p>
          {#each view.key.revoked as id (id)}<p class="advice small mono">{t('teamPlugins.key.revoked', { id })}</p>{/each}
          <p class="small" class:muted={!view.key.here}>{view.key.here ? t('teamPlugins.key.here') : t('teamPlugins.key.notHere')}</p>
          {#if view.admin && !view.key.here}
            {#if importing}
              <form
                class="form"
                onsubmit={(e) => {
                  e.preventDefault();
                  void importKey();
                }}
              >
                <p class="muted small">{t('teamPlugins.key.importHint')}</p>
                <label class="fld"
                  ><span>{t('teamPlugins.key.file')}</span>
                  <span class="row"
                    ><input
                      class="field input mono"
                      bind:value={keyFile}
                      spellcheck="false"
                      aria-label={t('teamPlugins.key.file')}
                    /><button type="button" class="btn" onclick={() => pick('key')}>{t('teamPlugins.key.pick')}</button></span
                  ></label
                >
                <label class="fld"
                  ><span>{t('teamPlugins.key.password')}</span><input
                    class="field input"
                    type="password"
                    autocomplete="off"
                    bind:value={importPassword}
                  /></label
                >
                <div class="acts">
                  <button class="btn primary" disabled={busy || !keyFile || !importPassword}>{t('teamPlugins.key.importDo')}</button>
                  <button type="button" class="btn ghost" onclick={() => (importing = false)}>{t('teamPlugins.published.cancel')}</button>
                </div>
              </form>
            {:else}
              <div class="acts"><button class="btn" onclick={() => (importing = true)}>{t('teamPlugins.key.import')}</button></div>
            {/if}
          {/if}
        </div>
      {:else if view.admin}
        <form
          class="form"
          data-testid="team-plugin-key-form"
          onsubmit={(e) => {
            e.preventDefault();
            void createKey();
          }}
        >
          <p class="muted small">{t('teamPlugins.key.noneAdmin')}</p>
          <label class="fld"
            ><span>{t('teamPlugins.key.namespace')}</span><input
              class="field input mono"
              bind:value={namespace}
              spellcheck="false"
              aria-invalid={nsState !== 'ok'}
            /></label
          >
          <p class="muted small">{t('teamPlugins.key.namespaceHint')}</p>
          {#if nsState === 'form'}<p class="error small" role="alert">{t('teamPlugins.key.formBad')}</p>{/if}
          {#if nsState === 'firstParty'}<p class="error small" role="alert">{t('teamPlugins.key.firstParty')}</p>{/if}
          <div class="two">
            <label class="fld"
              ><span>{t('teamPlugins.key.password')}</span><input
                class="field input"
                type="password"
                autocomplete="new-password"
                bind:value={password}
              /></label
            >
            <label class="fld"
              ><span>{t('teamPlugins.key.password2')}</span><input
                class="field input"
                type="password"
                autocomplete="new-password"
                bind:value={password2}
              /></label
            >
          </div>
          <p class="muted small">{t('teamPlugins.key.passwordHint')}</p>
          {#if password2 && password !== password2}<p class="error small" role="alert">{t('teamPlugins.key.mismatch')}</p>{/if}
          <label class="fld"
            ><span>{t('teamPlugins.key.backupDir')}</span>
            <span class="row"
              ><input
                class="field input mono"
                bind:value={backupDir}
                spellcheck="false"
                aria-label={t('teamPlugins.key.backupDir')}
              /><button type="button" class="btn" onclick={() => pick('backup')}>{t('teamPlugins.key.pick')}</button></span
            ></label
          >
          <p class="muted small">{t('teamPlugins.key.backupHint')}</p>
          <div class="acts end"><button class="btn primary" disabled={!canCreate}>{t('teamPlugins.key.create')}</button></div>
        </form>
      {:else}
        <p class="muted">{t('teamPlugins.key.none')}</p>
      {/if}
    </section>

    {#if view.key}
      <section aria-label={t('teamPlugins.publish.title')}>
        <h4>{t('teamPlugins.publish.title')}</h4>
        <form
          class="form"
          onsubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <p class="muted small">{t('teamPlugins.publish.hint')}</p>
          <div class="pub">
            <label class="fld grow"
              ><span>{t('teamPlugins.publish.folder')}</span>
              <span class="row"
                ><input
                  class="field input mono"
                  bind:value={folder}
                  spellcheck="false"
                  aria-label={t('teamPlugins.publish.folder')}
                /><button type="button" class="btn" onclick={() => pick('folder')}>{t('teamPlugins.key.pick')}</button></span
              ></label
            >
            <label class="fld tagf"
              ><span>{t('teamPlugins.publish.tag')}</span><input
                class="field input mono"
                bind:value={tag}
                spellcheck="false"
                placeholder={t('teamPlugins.publish.tag')}
              /></label
            >
          </div>
          <div class="acts end">
            <button class="btn primary" disabled={busy || !folder.trim() || !tag.trim()} aria-busy={busy}
              >{busy ? t('teamPlugins.publish.checking') : t('teamPlugins.publish.check')}</button
            >
          </div>
          {#if report}
            {@render checks(report.checks)}
            {#if !report.ok}<p class="error small" role="alert">{t('teamPlugins.publish.failed')}</p>{/if}
          {/if}
        </form>
      </section>

      <section aria-label={t('teamPlugins.proposed.title')}>
        <h4>{t('teamPlugins.proposed.title')}</h4>
        {#if !view.proposed.length}<p class="muted small">{t('teamPlugins.proposed.empty')}</p>{/if}
        {#each view.proposed as p (p.id)}
          <article class="item" aria-label={p.plugin} data-testid="team-plugin-proposed">
            <div class="head">
              <span class="name">{p.name || p.plugin}</span>
              <span class="mono small muted">{p.plugin}</span>
            </div>
            <p class="muted small">
              {t('teamPlugins.proposed.by', {
                version: p.version,
                who: p.mine ? t('teamPlugins.proposed.me') : t('teamPlugins.proposed.other'),
              })}
              · <span class="mono">{t('teamPlugins.publish.digest', { digest: p.digest.slice(0, 12) })}</span>
            </p>
            {@render checks(p.checks)}
            {#if view.admin}
              {#if signing?.id === p.id}
                <form
                  class="form inner"
                  onsubmit={(e) => {
                    e.preventDefault();
                    void sign();
                  }}
                >
                  <p class="muted small">{t('teamPlugins.proposed.signHint')}</p>
                  <label class="check small"
                    ><input type="checkbox" bind:checked={compileHere} /><span>{t('teamPlugins.proposed.compileHere')}</span></label
                  >
                  <label class="fld"
                    ><span>{t('teamPlugins.key.password')}</span><input
                      class="field input"
                      type="password"
                      autocomplete="off"
                      bind:value={signPassword}
                    /></label
                  >
                  <div class="acts">
                    <button class="btn primary" disabled={busy || !signPassword || !compileHere} aria-busy={busy}
                      >{t('teamPlugins.proposed.sign')}</button
                    >
                    <button type="button" class="btn ghost" onclick={() => (signing = null)}>{t('teamPlugins.published.cancel')}</button>
                  </div>
                </form>
              {:else}
                <div class="acts">
                  <button class="btn primary" disabled={busy || !view.key.here} onclick={() => (signing = p)}
                    >{t('teamPlugins.proposed.sign')}</button
                  >
                  <button class="btn ghost" disabled={busy} onclick={() => act(() => teamPluginsApi.dismiss(teamId, p.id))}
                    >{t('teamPlugins.proposed.dismiss')}</button
                  >
                </div>
              {/if}
            {:else}
              <p class="muted small">{t('teamPlugins.proposed.waiting')}</p>
              {#if p.mine}
                <div class="acts">
                  <button class="btn ghost" disabled={busy} onclick={() => act(() => teamPluginsApi.dismiss(teamId, p.id))}
                    >{t('teamPlugins.proposed.dismiss')}</button
                  >
                </div>
              {/if}
            {/if}
          </article>
        {/each}
      </section>

      <section aria-label={t('teamPlugins.published.title')}>
        <h4>{t('teamPlugins.published.title')}</h4>
        {#if !view.plugins.length}<p class="muted small">{t('teamPlugins.published.empty')}</p>{/if}
        {#each view.plugins as r (r.id)}
          <article class="item" aria-label={r.id} data-testid="team-plugin-published">
            <div class="head">
              <span class="name">{r.name || r.id}</span>
              <span class="tag">{teamName} · {t('teamPlugins.store.private')}</span>
            </div>
            <p class="muted small">
              <span class="mono">{r.id}</span> · {t('teamPlugins.published.line', { version: r.version, date: fDate(r.signedAt * 1000) })}
            </p>
            {#if r.withdrawn}<p class="advice small">{t('teamPlugins.published.withdrawn', { reason: r.withdrawn })}</p>{/if}
            {#if view.admin && !r.withdrawn}
              {#if withdrawing?.id === r.id}
                <form
                  class="form inner"
                  onsubmit={(e) => {
                    e.preventDefault();
                    void withdraw();
                  }}
                >
                  <label class="fld"
                    ><span>{t('teamPlugins.published.reason')}</span><input class="field input" bind:value={reason} /></label
                  >
                  <label class="fld"
                    ><span>{t('teamPlugins.key.password')}</span><input
                      class="field input"
                      type="password"
                      autocomplete="off"
                      bind:value={withdrawPassword}
                    /></label
                  >
                  <div class="acts">
                    <button class="btn danger" disabled={busy || !reason.trim() || !withdrawPassword}
                      >{t('teamPlugins.published.withdrawDo')}</button
                    >
                    <button type="button" class="btn ghost" onclick={() => (withdrawing = null)}>{t('teamPlugins.published.cancel')}</button
                    >
                  </div>
                </form>
              {:else}
                <div class="acts">
                  <button class="btn ghost" disabled={busy || !view.key.here} onclick={() => (withdrawing = r)}
                    >{t('teamPlugins.published.withdraw')}</button
                  >
                </div>
              {/if}
            {/if}
          </article>
        {/each}
        <p class="muted small">
          {view.serial ? t('teamPlugins.published.serial', { serial: String(view.serial) }) : t('teamPlugins.published.noList')}
        </p>
      </section>
    {/if}
  {/if}

  {#if notice}<p class="ok small" role="status">{notice}</p>{/if}
  {#if failure && view}<p class="error small" role="alert">{failure}</p>{/if}

  <div class="acts end">
    <button class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
  </div>
</div>

<style>
  .tplugins {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h4 {
    margin: 0;
    font-size: 11px;
    letter-spacing: 0.06em;
    color: var(--muted);
    font-weight: 600;
    text-transform: uppercase;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
  }
  .advice {
    color: var(--warn-ink);
  }
  .error {
    color: var(--del);
  }
  .ok {
    color: var(--run);
  }
  .mono {
    font-family: var(--mono);
  }
  .alarm {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: 8px;
    color: var(--del);
    background: color-mix(in oklch, var(--del) 10%, transparent);
  }
  .item,
  .form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .form.inner {
    background: var(--bg);
  }
  .head {
    display: flex;
    gap: 10px;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .name {
    font-weight: 600;
    color: var(--text);
  }
  .tag {
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 11px;
    color: var(--soft);
    background: var(--hover);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .acts.end {
    justify-content: flex-end;
  }
  .fld {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .check {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    color: var(--soft);
  }
  .check input {
    margin-top: 2px;
  }
  .fld > span:first-child {
    color: var(--soft);
    font-size: 12px;
    font-weight: 560;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .row .field {
    flex: 1;
    min-width: 0;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .pub {
    display: flex;
    gap: 10px;
    align-items: flex-end;
  }
  .pub .grow {
    flex: 1;
  }
  .pub .tagf {
    width: 120px;
  }
  .checks {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .checks li {
    display: grid;
    grid-template-columns: 16px 150px 1fr;
    gap: 6px;
    align-items: baseline;
    font-size: 12px;
  }
  .checks .mark {
    color: var(--run);
    font-weight: 700;
  }
  .checks li.bad .mark,
  .checks li.bad .ck {
    color: var(--del);
  }
  .ck {
    color: var(--text);
  }
</style>
