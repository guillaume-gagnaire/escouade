<script lang="ts">
  import { addAccount, defaultEngine } from '../lib/accounts';
  import { t } from '../lib/i18n';
  import Rich from '../lib/i18n/Rich.svelte';
  import { api } from '../lib/ipc';
  import { OS } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { EngineId, MachineAccount } from '../lib/types';
  import Icon from './Icon.svelte';

  // The panel with no project (the 2.0 design, `#welcome` and `#welcome-account`). Escouade works as soon as one harness
  // is found and one account is added. With no harness: each one Escouade knows, with the command that installs it, and
  // « Chercher à nouveau ». A harness found and no account (ESC-128): each harness found, what the machine's own account
  // is (signed in, as whom), and « Ajouter un compte… » (choice 19). Then: the way to a first project.
  const none = $derived(app.enginesFound.length === 0);
  /** A harness found, no account added (ESC-128): the first thing to do. */
  const noAccount = $derived(!none && !(app.settings.accounts ?? []).length);
  /** The harnesses found, as « Ordre des harness » would list them: the default one first. */
  const found = $derived(app.engines.filter((e) => app.enginesFound.includes(e.id)));

  /** What the machine's own account of each harness found is, read once there is no account. */
  let machine = $state<Partial<Record<EngineId, MachineAccount | null>>>({});
  $effect(() => {
    if (!noAccount) return;
    for (const e of app.enginesFound)
      if (!(e in machine))
        api
          .accountMachine(e)
          .then((m) => (machine[e] = m))
          .catch(() => (machine[e] = null));
  });
  /** The harness whose « Ajouter un compte… » is the main button: the first signed in on the machine, else the default one. */
  const lead = $derived(
    app.enginesFound.find((e) => machine[e]?.connected) ??
      (app.enginesFound.includes(defaultEngine()) ? defaultEngine() : app.enginesFound[0]),
  );

  let searching = $state(false);
  /** « Chercher à nouveau »: the harnesses looked for again (one may have been installed since). */
  async function searchAgain() {
    searching = true;
    try {
      const f = await api.claudeFound();
      app.claudeFound = f.claudeFound;
      app.claudePathFound = f.claudePathFound;
      app.enginesFound = f.enginesFound ?? (f.claudeFound ? ['claude'] : []);
      machine = {};
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      searching = false;
    }
  }

  const install = (e: (typeof app.engines)[number]) => (OS === 'windows' ? e.install.windows : e.install.unix);

  async function copy(text: string) {
    await navigator.clipboard?.writeText(text).catch(() => {});
    app.toast(t('shell.welcome.copied'), 'ok');
  }
</script>

<div class="welcome">
  <div class="box">
    {#if none}
      <section class="part" aria-labelledby="no-engine">
        <h1 id="no-engine">{t('shell.welcome.noEngine')}</h1>
        <p>
          <Rich k="shell.welcome.noEngineDesc">{#snippet where()}<b>{t('shell.welcome.harnessSettings')}</b>{/snippet}</Rich>
        </p>
        <ul class="rows">
          {#each app.engines as e (e.id)}
            <li>
              <b class="name">{e.name}</b>
              <code class="cmd mono">{install(e)}</code>
              <button
                class="btn icon-only"
                aria-label={t('shell.welcome.copy', { harness: e.name })}
                use:tip={t('shell.welcome.copy', { harness: e.name })}
                onclick={() => copy(install(e))}><Icon name="import" size={14} /></button
              >
            </li>
          {/each}
        </ul>
      </section>
    {:else if noAccount}
      <section class="part" aria-labelledby="no-account">
        <h1 id="no-account">{t('shell.welcome.noAccount')}</h1>
        <p>{t('shell.welcome.noAccountDesc')}</p>
        <ul class="rows">
          {#each found as e (e.id)}
            {@const m = machine[e.id]}
            <li>
              <b class="name">{e.name}</b>
              {#if m?.connected}
                <span class="state ok"
                  ><Icon name="check" size={14} /><span
                    >{m.email ? t('shell.welcome.machineAs', { email: m.email }) : t('shell.welcome.machine')}</span
                  ></span
                >
              {:else}
                <span class="state">{m === undefined ? '' : t('shell.welcome.notConnected')}</span>
              {/if}
              <button class="btn" class:primary={e.id === lead} onclick={() => addAccount(e.id)}>{t('accounts.tab.add')}</button>
            </li>
          {/each}
        </ul>
      </section>
    {:else}
      <section class="part" aria-labelledby="first-project">
        <h1 id="first-project">{t('shell.welcome.title')}</h1>
        <p>{t('shell.welcome.intro')}</p>
      </section>
    {/if}
    <div class="actions">
      {#if none || noAccount}
        <button class="btn" class:primary={none} disabled={searching} onclick={searchAgain}
          ><Icon name="restart" size={14} />{t('shell.welcome.searchAgain')}</button
        >
        <button class="btn" onclick={() => (app.modal = { kind: 'settings', page: 'accounts' })}
          >{t('shell.welcome.harnessSettingsButton')}</button
        >
        <span class="where"
          >{app.settings.wslDistro ? t('shell.welcome.searchedWsl', { distro: app.settings.wslDistro }) : t('shell.welcome.searched')}</span
        >
      {:else}
        <button class="btn primary" onclick={() => (app.modal = { kind: 'newProject' })}
          ><Icon name="plus" size={14} />{t('shell.welcome.addProject')}</button
        >
      {/if}
    </div>
  </div>
</div>

<style>
  .welcome {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 40px;
    overflow: auto;
  }
  .box {
    display: grid;
    gap: 18px;
    width: min(560px, 100%);
  }
  .part {
    display: grid;
    gap: 18px;
  }
  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 640;
    letter-spacing: -0.015em;
  }
  p {
    margin: 0;
    color: var(--muted);
    line-height: 1.55;
  }
  p b {
    color: var(--soft);
    font-weight: 600;
  }
  .rows {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
    border-top: 1px solid var(--line);
  }
  .rows li {
    display: grid;
    grid-template-columns: 110px minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    padding: 12px 0;
    border-bottom: 1px solid var(--line);
  }
  .name {
    font-weight: 600;
  }
  .cmd {
    padding: 6px 9px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--code-fg);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    user-select: all;
  }
  .state {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }
  .state.ok :global(.icon) {
    margin-top: 1px;
    color: var(--add);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .where {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
  }
</style>
