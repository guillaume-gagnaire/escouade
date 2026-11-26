<script lang="ts">
  import { onMount } from 'svelte';
  import { accountName, principalId } from '../../lib/accounts';
  import { speakerOf } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { maskToken, mcp } from '../../lib/mcp.svelte';
  import { IS_MAC } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import { settingsForm } from '../../lib/settings.svelte';
  import type { AgentsAnswer, McpDeclaration } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';
  import Icon from '../Icon.svelte';

  // « Piloter Escouade »: one switch, « Les agents peuvent piloter Escouade » (ESC-119), gives the server to Escouade's agents
  // and declares it in every harness installed, in each of its active accounts; under it, each declaration, with its
  // file and what to paste by hand when it failed. The switch goes to
  // the backend itself (`mcp_set_enabled`), not through the fields of the window saved as they change: the declaration
  // writes into the config of each account's harness.

  const on = $derived(app.settings.mcpEnabled);

  /**
   * « Les agents peuvent répondre aux demandes des autres agents » (ESC-423, audit SEC-03): saved with
   * the window's draft. Nobody by default, settings saved before it too.
   */
  const answerOptions = $derived(
    (['nobody', 'chef', 'all'] as const).map((value) => ({
      value,
      label: t(value === 'nobody' ? 'mcp.section.answerNobody' : value === 'chef' ? 'mcp.section.answerChef' : 'mcp.section.answerAll'),
    })),
  );
  const answer = $derived<AgentsAnswer>(settingsForm.settings.agentsAnswer ?? 'nobody');
  let busy = $state(false);

  /** « Garder les fichiers des demandes closes » (ESC-510): 7, 30 or 90 days, 30 by default; saved with the draft. */
  const keepOptions = $derived(
    ([7, 30, 90] as const).map((value) => ({ value, label: t('inbox.settings.days', { count: value }), disabled: !on })),
  );
  const keep = $derived(
    settingsForm.settings.inboxKeepDays === 7 || settingsForm.settings.inboxKeepDays === 90 ? settingsForm.settings.inboxKeepDays : 30,
  );

  /** « Ouvrir la boîte »: the settings close on it. */
  function openBox() {
    app.modal = null;
    app.openInbox('settings');
  }

  onMount(() => {
    void mcp.load().catch(() => {});
  });

  async function toggle() {
    if (busy) return;
    const next = !on;
    // Turned off with requests of the box « À toi » waiting (ESC-510, docs/INBOX.md § 6.2): asked first; they are kept,
    // frozen, until it is on again.
    if (!next) {
      const closing = (await api.inboxClosing().catch(() => null)) ?? { open: 0, titles: [] as string[] };
      if (closing.open > 0) {
        // Back to this page of the settings, whichever page they were opened on.
        const back = app.modal?.kind === 'settings' ? { ...app.modal, page: 'agents' as const, at: undefined } : app.modal;
        const list = closing.titles.map((title) => t('inbox.closing.item', { title })).join('\n');
        const more =
          closing.open > closing.titles.length ? `\n${t('inbox.closing.more', { count: closing.open - closing.titles.length })}` : '';
        app.modal = {
          kind: 'confirm',
          icon: 'inbox',
          title: t('inbox.closing.title'),
          body: [t('inbox.closing.body', { count: closing.open }), `${list}${more}`, t('inbox.closing.rest')].join('\n\n'),
          confirm: t('inbox.closing.confirm'),
          danger: true,
          alt: {
            label: t('inbox.settings.open'),
            onClick: () => {
              app.modal = null;
              app.openInbox('settings');
            },
          },
          onConfirm: () => {
            app.modal = back;
            void turn(false);
          },
          onCancel: () => (app.modal = back),
        };
        return;
      }
    }
    await turn(next);
  }

  async function turn(next: boolean) {
    busy = true;
    const saved = await app.run(api.mcpSetEnabled(next).then(() => true));
    busy = false;
    if (saved) app.settings.mcpEnabled = next;
  }

  /**
   * The account's name as shown: a harness's Principal not saved yet (made at its first agent) as Principal, any other
   * by its id when the window does not know it any more.
   */
  function nameOf(d: McpDeclaration): string {
    const account = app.settings.accounts.find((a) => a.id === d.account);
    if (account) return accountName(account);
    return d.account === principalId(d.engine) ? t('accounts.principal') : d.account;
  }

  /** The agents already running whose next start gives them the tools (ESC-91). */
  const nextStart = $derived(Object.values(app.agents).filter((a) => a.escouadeNext).length);

  /** The shell the command is written for (the backend writes PowerShell for Windows, `sh` for macOS). */
  const shell = $derived(IS_MAC ? 'sh' : 'PowerShell');

  /** Copies the command (or the entry) for the account: the real one, asked of the backend now, which never sent it with the state. */
  async function copy(d: McpDeclaration) {
    const command = await app.run(api.mcpManualCommand(d.account));
    if (!command) return;
    try {
      await navigator.clipboard.writeText(command);
      app.toast(t(d.paste ? 'mcp.section.copiedEntry' : 'mcp.section.copied'), 'ok');
    } catch (e) {
      app.toast(t('mcp.section.copyFailed', { error: String(e) }), 'error');
    }
  }
</script>

<!-- Agents › « Piloter Escouade » (PROPOSITION.md §2, from the 2.0 tab « Serveur MCP »): the switch, where it is declared.
     What the server was asked is in the page's « Avancé » (McpActivity). -->
<Group title={t('settings.groups.pilot')}>
  <Row label={t('mcp.section.enable')} desc={t('mcp.section.enableDesc')}>
    <button class="switch" class:on role="switch" aria-checked={on} aria-label={t('mcp.section.enable')} disabled={busy} onclick={toggle}
    ></button>
  </Row>
  <!-- « Agents et navigateurs » (ESC-669): the tools of the browsers, under the server they depend on; saved with the draft. -->
  <Row label={t('mcp.section.browsers')} desc={t('mcp.section.browsersDesc')}>
    <Switch
      label={t('mcp.section.browsers')}
      disabled={!on}
      bind:on={() => settingsForm.settings.agentBrowsers !== false, (v) => (settingsForm.settings.agentBrowsers = v)}
    />
  </Row>
  <Row label={t('mcp.section.answer')} desc={t('mcp.section.answerDesc')}>
    <Chips
      label={t('mcp.section.answer')}
      options={answerOptions}
      bind:value={() => answer, (v) => (settingsForm.settings.agentsAnswer = v)}
    />
    {#snippet extra()}
      {#if answer === 'all'}
        <p class="warn" role="alert" data-testid="agents-answer-warning">{t('mcp.section.answerAllWarn')}</p>
      {:else if answer === 'chef'}
        <p class="more">{t('mcp.section.answerChefDesc')}</p>
      {/if}
    {/snippet}
  </Row>
  <!-- The box « À toi » (ESC-510, docs/INBOX.md § 6.3, § 10.5): the server's own, greyed with it. -->
  <div class="box" class:off={!on} data-testid="inbox-settings">
    <Row label={t('inbox.settings.box')} desc={on ? t('inbox.settings.boxOn') : t('inbox.settings.boxOff')}>
      {#if on}<button class="btn" onclick={openBox}><Icon name="inbox" size={14} />{t('inbox.settings.open')}</button>{/if}
    </Row>
    <Row label={t('inbox.settings.keep')} desc={t('inbox.settings.keepDesc')}>
      <Chips
        label={t('inbox.settings.keep')}
        options={keepOptions}
        bind:value={() => keep, (v) => (settingsForm.settings.inboxKeepDays = v)}
      />
    </Row>
  </div>
  {#if on}
    <Row label={t('mcp.section.list')} wide>
      {#snippet extra()}
        {#if mcp.status.error}
          <p class="bad" role="alert">{t('mcp.section.serverFailed', { error: mcp.status.error })}</p>
        {:else if !mcp.declared.length}
          <p class="more" role="status">{t('mcp.section.working')}</p>
        {:else}
          <ul class="decls" aria-label={t('mcp.section.list')}>
            {#each mcp.declared as d (d.account)}
              <li class:failed={!d.ok}>
                <div class="head">
                  <span class="ic" class:good={d.ok} class:bad={!d.ok}><Icon name={d.ok ? 'check' : 'info'} size={14} /></span>
                  <span class="where">{t('mcp.section.where', { harness: speakerOf(d.engine), name: nameOf(d) })}</span>
                  <span class="mark" class:good={d.ok} class:bad={!d.ok}>{t(d.ok ? 'mcp.section.declared' : 'mcp.section.failed')}</span>
                </div>
                <span class="file mono">{d.file}</span>
                {#if !d.ok}
                  <p class="bad" role="alert">{t('mcp.section.notDeclared', { name: nameOf(d), error: maskToken(d.error ?? '') })}</p>
                  <span class="how">{d.paste ? t('mcp.section.paste', { file: d.file }) : t('mcp.section.command', { shell })}</span>
                  <code class="cmd mono">{maskToken(d.command)}</code>
                  <div>
                    <button class="btn" onclick={() => copy(d)}>{t(d.paste ? 'mcp.section.copyEntry' : 'mcp.section.copy')}</button>
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
        {#if nextStart}
          <p class="warn">{t('mcp.section.nextStart', { count: nextStart })}</p>
        {/if}
      {/snippet}
    </Row>
  {/if}
  <p class="note">{t('mcp.section.note')}</p>
</Group>

<style>
  p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
  }
  .note {
    max-width: 64ch;
    margin: 4px 0 12px;
    color: var(--muted);
  }
  .good {
    color: var(--add);
  }
  .warn,
  .more {
    max-width: 64ch;
    margin-top: 6px;
    color: var(--muted);
  }
  .warn {
    color: var(--warn-ink);
  }
  .bad {
    color: var(--del);
  }
  .decls {
    display: grid;
    gap: 8px;
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }
  .decls li {
    display: grid;
    justify-items: start;
    gap: 4px;
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    font-size: 12px;
  }
  .ic {
    display: inline-flex;
  }
  .where {
    color: var(--soft);
    font-weight: 550;
  }
  .mark {
    margin-left: auto;
    font-size: 11.5px;
  }
  .file {
    max-width: 100%;
    padding-left: 22px;
    color: var(--muted);
    font-size: 11.5px;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .how {
    color: var(--muted);
    font-size: 12px;
  }
  .cmd {
    max-width: 100%;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 11.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .box.off {
    opacity: 0.55;
  }
  /* As `Switch` has it: shown busy while the change is saved. */
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
