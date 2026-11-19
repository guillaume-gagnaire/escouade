<script lang="ts">
  // A state of a connection to a server (ESC-537, docs/SERVER-UX.md § 3): what happened, what did not change, and the
  // next gesture. What ssh and the server said is shown as text, never run.
  import type { Snippet } from 'svelte';
  import type { ServerProblem } from '../../lib/connection.svelte';
  import { fDate, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import Rich from '../../lib/i18n/Rich.svelte';

  /** The command that meets the host's key once: `ssh <host>`. */
  const sshTo = (h: string) => `ssh ${h}`;
  import Icon from '../Icon.svelte';

  let {
    problem,
    host,
    name,
    target,
    actions,
  }: {
    problem: ServerProblem;
    /** The SSH host as the user knows it (`acme-escouade`). */
    host: string;
    /** The space's name (« Travail »), when it is one already. */
    name?: string;
    /** Its address (`escouade.acme.internal`). */
    target?: string;
    actions?: Snippet;
  } = $props();

  const space = $derived(name || host);

  /** The title of the state. */
  const title = $derived.by(() => {
    switch (problem.kind) {
      case 'unreachable':
        if (problem.why === 'auth') return t('servers.problem.auth.title', { host });
        if (problem.why === 'forwarding') return t('servers.problem.forwarding.title', { host });
        if (problem.why === 'noSsh') return t('servers.problem.noSsh.title');
        return t('servers.problem.unreachable.title', { host });
      case 'hostKey':
        return problem.changed ? t('servers.problem.hostKey.changedTitle', { host }) : t('servers.problem.hostKey.title');
      case 'noEscouade':
        return t('servers.problem.noEscouade.title');
      case 'version':
        return t('servers.problem.version.title', { name: space });
      case 'revoked':
        return t('servers.problem.revoked.title', { name: space });
      case 'policy':
        return t('servers.problem.policy.title');
      case 'code':
        return t('servers.dialog.code.wrong');
      default:
        return t('servers.problem.other.title');
    }
  });

  /** The domain's end of an address, for the cause of a VPN (`.internal`, `.lan`, `.corp`, `.local`). */
  const privateSuffix = $derived.by(() => {
    const m = /\.(internal|lan|corp|local|intra|home)$/i.exec(target ?? '');
    return m ? `.${m[1]}` : null;
  });

  const revokedLine = $derived.by(() => {
    if (problem.kind !== 'revoked') return '';
    const r = problem.revoked;
    if (!r) return t('servers.problem.revoked.unknown');
    const date = t('format.dateTime', { date: fDate(r.at), time: fTime(r.at) });
    return r.by ? t('servers.problem.revoked.when', { date, by: r.by }) : t('servers.problem.revoked.whenServer', { date });
  });
</script>

<div class="problem" role="alertdialog" aria-label={title} data-testid="server-problem" data-kind={problem.kind}>
  <h3 class="t" class:bad={problem.kind !== 'version' && problem.kind !== 'revoked'}>
    <Icon name={problem.kind === 'revoked' ? 'unlink' : problem.kind === 'hostKey' ? 'key' : 'warn'} size={17} />
    <span>{title}</span>
  </h3>

  {#if problem.kind === 'unreachable'}
    <p>
      {#if problem.why === 'auth'}{t('servers.problem.auth.text')}
      {:else if problem.why === 'forwarding'}{t('servers.problem.forwarding.text')}
      {:else if problem.why === 'noSsh'}{t('servers.problem.noSsh.text')}
      {:else}{t('servers.problem.unreachable.text', { host: target || host })}{/if}
      {t('servers.problem.unchanged')}
    </p>
  {:else if problem.kind === 'hostKey'}
    <p>
      {#if problem.changed}{t('servers.problem.hostKey.changedText')}{:else}<Rich k="servers.problem.hostKey.text"
          >{#snippet command()}<code class="inline">{sshTo(host)}</code>{/snippet}</Rich
        >{/if}
      {t('servers.problem.unchanged')}
    </p>
  {:else if problem.kind === 'noEscouade'}
    <p>{t('servers.problem.noEscouade.text')} {t('servers.problem.unchanged')}</p>
    <p class="hint">{t('servers.problem.noEscouade.install')}</p>
  {:else if problem.kind === 'version'}
    <p>{t('servers.problem.version.text', { client: problem.client, server: problem.server })}</p>
    <p>
      {problem.update === 'client'
        ? t('servers.problem.version.updateClient', { server: problem.server })
        : t('servers.problem.version.updateServer', { client: problem.client })}
      {t('servers.problem.agentsGoOn')}
    </p>
  {:else if problem.kind === 'revoked'}
    <p>{revokedLine} {t('servers.problem.revoked.text', { name: space })}</p>
    <p class="hint"><Icon name="info" size={12} />{t('servers.problem.revoked.hint')}</p>
  {:else if problem.kind === 'policy'}
    <p>{problem.message} {t('servers.problem.unchanged')}</p>
  {:else if problem.kind === 'refused' || problem.kind === 'code'}
    <p>{problem.message}</p>
  {/if}

  {#if 'command' in problem && (problem.command || problem.output)}
    <div class="code" aria-label={t('servers.problem.output')}>
      {#if problem.command}<span class="d">$ {problem.command}</span>{/if}
      {#if problem.output}<span class="e">{problem.output}</span>{/if}
    </div>
  {/if}

  {#if problem.kind === 'unreachable' && problem.why === 'unreachable'}
    <div class="lbl">{t('servers.problem.unreachable.causes')}</div>
    <ul class="why">
      {#if privateSuffix}<li>{t('servers.problem.unreachable.vpn', { suffix: privateSuffix })}</li>{/if}
      <li>{t('servers.problem.unreachable.firewall')}</li>
      {#if target && target !== host}<li>{t('servers.problem.unreachable.config', { target })}</li>{/if}
    </ul>
  {/if}

  {#if actions}<div class="acts">{@render actions()}</div>{/if}
</div>

<style>
  .problem {
    display: grid;
    gap: 14px;
    align-content: start;
    min-width: 0;
  }
  .t {
    display: flex;
    gap: 10px;
    align-items: center;
    margin: 0;
    font-size: 16px;
    font-weight: 640;
    letter-spacing: -0.01em;
    color: var(--text);
  }
  .t.bad {
    color: var(--del);
  }
  p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
    max-width: 62ch;
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--dim);
    font-size: 11.5px;
  }
  .code {
    display: grid;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--code-fg);
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    border-radius: 8px;
    padding: 9px 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 160px;
    overflow: auto;
  }
  .code .d {
    color: var(--muted);
  }
  .code .e {
    color: var(--del);
  }
  .lbl {
    font-size: 12px;
    font-weight: 600;
    color: var(--soft);
  }
  .why {
    display: grid;
    gap: 6px;
    margin: 0;
    padding-left: 18px;
    color: var(--soft);
    font-size: 12.5px;
  }
  .acts {
    display: flex;
    gap: 8px;
    align-items: center;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
  /* The command of the sentence, in the code font as the block of the command (ESC-539, asked of ESC-537). */
  .inline {
    font-family: var(--mono);
    font-size: 0.95em;
  }
</style>
