<script lang="ts">
  import { onMount } from 'svelte';
  import { accountName, principalId } from '../../lib/accounts';
  import { speakerOf } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { maskToken, mcp } from '../../lib/mcp.svelte';
  import { IS_MAC } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import type { McpDeclaration } from '../../lib/types';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Icon from '../Icon.svelte';

  // « Piloter Escouade »: one switch, « Les agents peuvent piloter Escouade » (ESC-119), gives the server to Escouade's agents
  // and declares it in every harness installed, in each of its active accounts; under it, each declaration, with its
  // file and what to paste by hand when it failed. The switch goes to
  // the backend itself (`mcp_set_enabled`), not through the fields of the window saved as they change: the declaration
  // writes into the config of each account's harness.

  const on = $derived(app.settings.mcpEnabled);
  let busy = $state(false);

  onMount(() => {
    void mcp.load().catch(() => {});
  });

  async function toggle() {
    if (busy) return;
    busy = true;
    const next = !on;
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
  /* As `Switch` has it: shown busy while the change is saved. */
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
