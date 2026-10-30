<script lang="ts">
  import { locale, t } from '../../lib/i18n';
  import { asksOf, contributionsOf, originOf, shortPrint, textOf } from '../../lib/plugins';
  import { plugins, type InstallStep } from '../../lib/plugins-view.svelte';
  import Icon from '../Icon.svelte';
  import Modal from '../modals/Modal.svelte';
  import PluginTile from './PluginTile.svelte';
  import AskList from './AskList.svelte';
  import Seal from './Seal.svelte';
  import { tip } from '../../lib/tip';

  // The install window (PLG-4; §3.4 of docs/PLUGINS.md, choices 177 and 178): who made the plugin and where it comes from
  // (its origin, its version, its fingerprint, no signature outside the store), what it adds, and what it will be able to
  // do, permission by permission, the strongest first. An update says what it asks for more than the version installed,
  // and asks again. Nothing is installed before « Installer ».
  let { step }: { step: InstallStep } = $props();

  const lang = $derived(locale.ui);
  const p = $derived(step.kind === 'preview' ? step.preview : null);
  const m = $derived(p?.manifest ?? null);
  const dev = $derived(p?.origin.kind === 'dev');
  const update = $derived(!!p?.previous);
  // From the store (STORE-9, choices 177 and 178): its fingerprints checked, its seal checked here with the app's keys.
  const store = $derived(p?.origin.kind === 'store' ? p.origin : null);
  const title = $derived(
    !p
      ? t('plugins.install.gitTitle')
      : dev
        ? t('plugins.install.devTitle', { name: p.name })
        : update
          ? t('plugins.install.updateTitle', { name: p.name })
          : t('plugins.install.title', { name: p.name }),
  );
  const adds = $derived(contributionsOf(m, lang));
  const asks = $derived(p ? asksOf(p.asks, m, lang) : []);
  const confirmLabel = $derived(
    dev
      ? t('plugins.install.load')
      : update
        ? t('plugins.install.update')
        : p?.verified
          ? t('plugins.install.install')
          : t('plugins.install.installAnyway'),
  );
  /** « v1.2.0 → v1.3.0 → 3f9c2e1 »: the version installed (an update), the one read, its commit. */
  const versionLine = $derived.by(() => {
    if (!p) return '';
    let line = t('plugins.version', { version: p.version });
    if (p.previous) line = t('plugins.versionTo', { from: t('plugins.version', { version: p.previous.version }), to: line });
    if (p.origin.kind === 'git') line = t('plugins.versionTo', { from: line, to: p.fingerprint.slice(4, 11) });
    if (p.origin.kind === 'store' && p.origin.commit) line = t('plugins.versionTo', { from: line, to: p.origin.commit.slice(0, 7) });
    return line;
  });
  const originLabel = $derived(p ? t(`plugins.origin.${p.origin.kind}`) : '');
  const print = $derived(
    !p
      ? ''
      : dev
        ? t('plugins.origin.devPrint')
        : p.origin.kind === 'folder'
          ? t('plugins.origin.folderPrint', { print: p.fingerprint })
          : p.origin.kind === 'store'
            ? t('plugins.origin.storePrint', { print: shortPrint(p.fingerprint) })
            : p.fingerprint,
  );
</script>

<Modal {title} width={560} onclose={() => plugins.cancel()}>
  {#snippet lead()}<Icon name="puzzle" />{/snippet}
  {#if step.kind === 'git'}
    <form
      class="git"
      onsubmit={(e) => {
        e.preventDefault();
        void plugins.readGit();
      }}
    >
      <label class="w-fld">
        <span>{t('plugins.install.gitUrl')}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field mono" bind:value={step.url} placeholder={t('plugins.install.gitUrlPlaceholder')} autofocus spellcheck="false" />
      </label>
      <label class="w-fld">
        <span>{t('plugins.install.gitTag')}</span>
        <input class="field mono" bind:value={step.tag} placeholder={t('plugins.install.gitTagPlaceholder')} spellcheck="false" />
      </label>
      <p class="hint">{t('plugins.install.gitHint')}</p>
      {#if step.error}<p class="err" role="alert">{step.error}</p>{/if}
    </form>
  {:else if p}
    <div class="body">
      <div class="who">
        <PluginTile id={p.id} name={p.name} src={p.icon} size={44} />
        <div>
          <h3>
            {p.name}
            {#if p.verified}<Seal version={p.version} long />{:else}<span class="ptag"
                >{dev ? t('plugins.tag.dev') : t('plugins.tag.unverified')}</span
              >{/if}
          </h3>
          <div class="ln">
            {[m?.author, t('plugins.version', { version: p.version }), m?.license].filter(Boolean).join(' · ')}<span class="mono id"
              >{p.id}</span
            >
          </div>
        </div>
      </div>
      {#if p.description || m?.description}
        <p class="desc">{p.description || textOf(m?.description, lang)}</p>
      {/if}
      {#if store && !p.verified}
        <p class="notice">
          <Icon name="info" size={14} /><span
            ><b>{t('plugins.install.storeUnverified')}</b>
            {p.unverified ? t('plugins.install.sealWithdrawn', { reason: p.unverified }) : t('plugins.install.storeUnverifiedBody')}</span
          >
        </p>
      {:else if !dev && !store}
        <p class="notice">
          <Icon name="info" size={14} /><span><b>{t('plugins.install.unverified')}</b> {t('plugins.install.unverifiedBody')}</span>
        </p>
      {/if}
      <div class="facts">
        <span class="k">{t('plugins.origin.from')}</span><span class="v" use:tip={{ text: originOf(p.origin), aria: false }}
          >{originLabel} · {originOf(p.origin)}</span
        >
        <span class="k">{t('plugins.origin.version')}</span><span class="v mono">{versionLine}</span>
        <span class="k">{t('plugins.origin.fingerprint')}</span><span class="v mono" use:tip={{ text: p.fingerprint, aria: false }}
          >{print}</span
        >
        <span class="k">{t('plugins.origin.signature')}</span>{#if p.verified}<span class="v ok">{t('plugins.origin.signedHere')}</span
          >{:else if store}<span class="v no">{t('plugins.origin.storeUnsigned')}</span>{:else}<span class="v no"
            >{t('plugins.origin.unsigned')}</span
          >{/if}
      </div>
      {#if p.previous}
        {#if p.grown.length}
          <p class="grown" role="status">
            <Icon name="up" size={14} /><span>{t('plugins.install.grown', { version: p.previous.version })}</span>
          </p>
        {:else}
          <p class="same">{t('plugins.install.same', { version: p.previous.version })}</p>
        {/if}
      {/if}
      {#if p.refusal}<p class="err">{t('plugins.install.refusal', { reason: p.refusal })}</p>{/if}
      <section>
        <h4>{t('plugins.page.adds')}</h4>
        {#if adds.length}
          <ul class="adds">
            {#each adds as line (line)}<li><Icon name="sparkle" size={14} /><span>{line}</span></li>{/each}
          </ul>
        {:else}
          <p class="hint">{t('plugins.page.addsNothing')}</p>
        {/if}
      </section>
      <section>
        <h4>{t('plugins.install.canDo')}</h4>
        {#if asks.length}
          <AskList {asks} grown={p.grown} />
        {:else}
          <p class="hint">{p.declarative ? t('plugins.install.declarative') : t('plugins.perms.none')}</p>
        {/if}
      </section>
      {#if step.error}<p class="err" role="alert">{step.error}</p>{/if}
    </div>
  {/if}
  {#snippet footer()}
    <button class="btn" onclick={() => plugins.cancel()}>{t('common.cancel')}</button>
    {#if step.kind === 'git'}
      <button class="btn primary" disabled={step.busy || !step.url.trim() || !step.tag.trim()} onclick={() => plugins.readGit()}
        >{step.busy ? t('plugins.install.reading') : t('plugins.install.read')}</button
      >
    {:else}
      <button class="btn primary" disabled={step.busy} onclick={() => plugins.confirm()}>{confirmLabel}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .git,
  .body {
    display: grid;
    gap: 14px;
  }
  .who {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 12px;
    align-items: center;
  }
  h3 {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 620;
  }
  .ln {
    margin-top: 3px;
    display: flex;
    gap: 10px;
    color: var(--dim);
    font-size: 12px;
  }
  .ln .id {
    font-size: 11px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .desc {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
  }
  .ptag {
    font-size: 11px;
    font-weight: 500;
    padding: 1px 7px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    white-space: nowrap;
  }
  .notice,
  .grown {
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 9px 11px;
    border-radius: 9px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12px;
    line-height: 1.5;
  }
  .notice :global(.icon),
  .grown :global(.icon) {
    margin-top: 2px;
    color: var(--warn-ink);
  }
  .notice b {
    color: var(--text);
    font-weight: 600;
  }
  .same {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .facts {
    display: grid;
    grid-template-columns: 110px minmax(0, 1fr);
    gap: 6px 12px;
    padding: 10px 12px;
    border-radius: 9px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12px;
  }
  .facts .k {
    color: var(--dim);
  }
  .facts .v {
    color: var(--soft);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .facts .v.mono {
    font-size: 11px;
  }
  .facts .v.ok {
    color: var(--teal-ink);
    font-weight: 560;
  }
  .facts .v.no {
    color: var(--warn-ink);
    font-weight: 560;
  }
  section {
    display: grid;
    gap: 8px;
  }
  h4 {
    margin: 0;
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 600;
  }
  .adds {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 6px;
  }
  .adds li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    font-size: 12.5px;
    color: var(--soft);
  }
  .adds :global(.icon) {
    margin-top: 2px;
    color: var(--muted);
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .err {
    margin: 0;
    color: var(--del);
    font-size: 12px;
    white-space: pre-wrap;
  }
</style>
