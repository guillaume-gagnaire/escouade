<script lang="ts">
  import { onMount } from 'svelte';
  import { accountName } from '../../lib/accounts';
  import { fDateTime, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { maskToken, mcp } from '../../lib/mcp.svelte';
  import { IS_MAC } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';

  // « Escouade dans Claude »: Claude Code drives Escouade through its MCP server, declared in the Claude Code of each
  // active account; and « Activité MCP »: what the server was asked. The switch is saved at once, not through the
  // modal's draft: the declaration writes into the config of each account's Claude Code, which « Annuler » could not
  // take back.

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

  /** The account's name as shown (its id when the window does not know it any more). */
  function nameOf(id: string): string {
    const account = app.settings.accounts.find((a) => a.id === id);
    return account ? accountName(account) : id;
  }

  const declared = $derived(mcp.declared.filter((d) => d.ok));
  const failed = $derived(mcp.declared.filter((d) => !d.ok));
  /** The most recent first. */
  const calls = $derived([...mcp.activity].reverse());

  /** The shell the command is written for (the backend writes PowerShell for Windows, `sh` for macOS). */
  const shell = $derived(IS_MAC ? 'sh' : 'PowerShell');

  /** Copies the command for the account: the real one, asked of the backend now, which never sent it with the state. */
  async function copy(account: string) {
    const command = await app.run(api.mcpManualCommand(account));
    if (!command) return;
    try {
      await navigator.clipboard.writeText(command);
      app.toast(t('mcp.section.copied'), 'ok');
    } catch (e) {
      app.toast(t('mcp.section.copyFailed', { error: String(e) }), 'error');
    }
  }
</script>

<Group title={t('mcp.section.title')} note={t('mcp.section.note')}>
  <Row label={t('mcp.section.enable')} desc={t('mcp.section.enableDesc')}>
    <button class="switch" class:on role="switch" aria-checked={on} aria-label={t('mcp.section.enable')} disabled={busy} onclick={toggle}
    ></button>
  </Row>
  {#if on}
    <div class="state">
      {#if mcp.status.error}
        <p class="bad" role="alert">{t('mcp.section.serverFailed', { error: mcp.status.error })}</p>
      {:else if !mcp.declared.length}
        <p class="muted" role="status">{t('mcp.section.working')}</p>
      {:else}
        {#if declared.length}
          <p class="good" role="status">
            {t('mcp.section.declared', { accounts: declared.map((d) => t('mcp.section.account', { name: nameOf(d.account) })).join(', ') })}
          </p>
        {/if}
        {#each failed as d (d.account)}
          <div class="fail">
            <p class="bad" role="alert">{t('mcp.section.notDeclared', { name: nameOf(d.account), error: maskToken(d.error ?? '') })}</p>
            <span class="how">{t('mcp.section.command', { shell })}</span>
            <code class="cmd mono">{maskToken(d.command)}</code>
            <div><button class="btn" onclick={() => copy(d.account)}>{t('mcp.section.copy')}</button></div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</Group>

<Group title={t('mcp.activity.title')} plain>
  {#if calls.length}
    <!-- A list that scrolls is reached with the keyboard to be read. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <ul class="log" tabindex="0" aria-label={t('mcp.activity.list')}>
      {#each calls as e}
        <li class:bad={e.outcome !== 'ok'}>
          <span class="at mono" title={fDateTime(e.at)}>{fTime(e.at)}</span>
          <span class="who">{e.caller}</span>
          <span class="tool mono">{e.tool || '—'}</span>
          <span class="res">{t(`mcp.activity.outcome.${e.outcome}`)}</span>
          {#if e.summary}<span class="sum mono">{e.summary}</span>{/if}
          {#if e.message}<span class="msg">{e.message}</span>{/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">{t('mcp.activity.empty')}</p>
  {/if}
  <div>
    <button class="btn" disabled={!calls.length} onclick={() => mcp.clear()}>{t('common.clear')}</button>
  </div>
</Group>

<style>
  .state {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--line);
  }
  p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
  }
  .good {
    color: var(--add);
  }
  .muted {
    color: var(--muted);
  }
  .bad {
    color: var(--del);
  }
  .fail {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  .how {
    font-size: 11.5px;
    color: var(--dim);
  }
  .cmd {
    max-width: 100%;
    padding: 8px 10px;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--panel);
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--muted);
    overflow-wrap: anywhere;
    user-select: text;
  }
  .btn {
    height: 28px;
    font-size: 12px;
  }
  /* As `Switch` has it: shown busy while the change is saved. */
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .log {
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 320px;
    overflow-y: auto;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--bg);
  }
  .log:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .log li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 10px;
    padding: 8px 14px;
    font-size: 12px;
    border-left: 2px solid transparent;
  }
  .log li:not(:first-child) {
    border-top: 1px solid var(--line);
  }
  .log li.bad {
    border-left-color: var(--del);
  }
  .at {
    color: var(--dim);
    font-size: 11.5px;
  }
  .who {
    font-weight: 600;
  }
  .tool {
    color: var(--muted);
    font-size: 11.5px;
  }
  .res {
    margin-left: auto;
    font-size: 11.5px;
    color: var(--dim);
  }
  .bad .res,
  .bad .msg {
    color: var(--del);
  }
  .sum,
  .msg {
    flex-basis: 100%;
    min-width: 0;
    font-size: 11.5px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .empty {
    color: var(--dim);
  }
</style>
