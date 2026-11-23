<script lang="ts">
  // « Se connecter à un serveur » (ESC-537, docs/SERVER-UX.md § 2, design/server-2-3 `x-hote`, `*-connexion`,
  // `*-erreur`): the SSH host (from ~/.ssh/config, or typed), the connection (ssh, then Escouade), the pairing the first
  // time (the code typed, or got « d'ici par SSH »), then the space opens. Every failure has its state (§ 3).
  import { onDestroy, onMount, tick } from 'svelte';
  import type { ServerProblem as Problem } from '../../lib/connection.svelte';
  import { trapFocus } from '../../lib/focus';
  import { t } from '../../lib/i18n';
  import { goToServer, servers, type Outcome, type Reached, type SshHost } from '../../lib/servers.svelte';
  import { app } from '../../lib/state.svelte';
  import { local } from '../../lib/transport';
  import Icon from '../Icon.svelte';
  import ServerProblem from './ServerProblem.svelte';

  let { existing }: { existing?: string } = $props();

  type Step = 'host' | 'connecting' | 'problem' | 'pair' | 'space';
  let step = $state<Step>('host');
  let hosts = $state<SshHost[]>([]);
  let chosen = $state<string>('');
  let typed = $state('');
  let serverPort = $state('');
  let reached = $state<Reached | null>(null);
  let problem = $state<Problem | null>(null);
  const LENGTH = 8;
  let code = $state<string[]>(Array(LENGTH).fill(''));
  let codeError = $state('');
  let name = $state('');
  let device = $state('');
  let busy = $state(false);
  let overSsh = $state(false);
  let copied = $state(false);
  let cells = $state<HTMLInputElement[]>([]);
  let typedField = $state<HTMLInputElement>();

  const known = $derived(existing ? servers.view?.servers.find((s) => s.id === existing) : undefined);
  /** The host given to ssh: the one chosen in the list, else the one typed. */
  const host = $derived(known ? known.ssh : typed.trim() || chosen);
  const target = $derived(known?.host ?? hosts.find((h) => h.alias === host)?.hostName ?? host.split('@').pop()?.split(':')[0] ?? host);
  const ALPHABET = /[A-HJ-NP-Z2-9]/;
  /** The command that shows a code, on the server. */
  const PAIR = 'escouade-server pair';

  onMount(async () => {
    if (!servers.view) void import('../../lib/servers.svelte').then((m) => m.loadServers());
    if (existing) {
      void connect();
      return;
    }
    try {
      hosts = await local<SshHost[]>('servers_ssh_hosts');
      if (hosts.length && !chosen) chosen = hosts[0].alias;
    } catch {
      hosts = [];
    }
  });

  onDestroy(() => {
    if (reached && step !== 'space') void local('server_cancel', { attempt: reached.attempt });
  });

  function close() {
    app.modal = null;
  }

  function describe(h: SshHost): string {
    const at = h.user ? `${h.user}@${h.hostName}` : h.hostName;
    const via = h.proxyJump ? ` · ${t('servers.dialog.host.via', { jump: h.proxyJump })}` : '';
    return `${at}${h.port ? ` · ${t('servers.dialog.host.port', { port: h.port })}` : ''}${via}`;
  }

  async function connect() {
    if (!host || busy) return;
    busy = true;
    problem = null;
    step = 'connecting';
    if (reached) void local('server_cancel', { attempt: reached.attempt });
    reached = null;
    const port = Number(serverPort) || (known ? known.remotePort : null);
    try {
      const r = await local<Outcome<Reached>>('server_attempt', { host, remotePort: port, existing: existing ?? null });
      if (r.outcome === 'problem') {
        problem = r.problem;
        step = 'problem';
      } else {
        reached = r.value;
        name = known?.name ?? r.value.name;
        device = known?.device ?? r.value.device;
        step = 'pair';
        await tick();
        cells[0]?.focus();
      }
    } catch (e) {
      problem = { kind: 'refused', code: 'error', message: String(e) };
      step = 'problem';
    } finally {
      busy = false;
    }
  }

  /** A letter typed in the cell `i`: kept when it is one of the code's, the next cell then. */
  async function typeAt(i: number, e: Event) {
    const input = e.target as HTMLInputElement;
    const letters = input.value.toUpperCase().replace(/[^A-Z0-9]/g, '');
    codeError = '';
    if (letters.length > 1) {
      fill(letters, i);
      return;
    }
    const c = letters.slice(-1);
    code[i] = c && ALPHABET.test(c) ? c : '';
    input.value = code[i];
    if (code[i] && i < LENGTH - 1) cells[i + 1]?.focus();
    if (code.every(Boolean)) await tick();
  }

  /** A code pasted whole (with or without its dash). */
  function fill(text: string, from = 0) {
    const letters = [...text.toUpperCase().replace(/[^A-Z0-9]/g, '')].filter((c) => ALPHABET.test(c));
    for (let k = 0; k < letters.length && from + k < LENGTH; k++) code[from + k] = letters[k];
    const next = Math.min(from + letters.length, LENGTH - 1);
    cells[next]?.focus();
  }

  function onPaste(i: number, e: ClipboardEvent) {
    const text = e.clipboardData?.getData('text') ?? '';
    if (!text) return;
    e.preventDefault();
    fill(text, text.replace(/[^A-Za-z0-9]/g, '').length >= LENGTH ? 0 : i);
  }

  function onKey(i: number, e: KeyboardEvent) {
    if (e.key === 'Backspace' && !code[i] && i > 0) {
      e.preventDefault();
      code[i - 1] = '';
      cells[i - 1]?.focus();
    } else if (e.key === 'ArrowLeft' && i > 0) cells[i - 1]?.focus();
    else if (e.key === 'ArrowRight' && i < LENGTH - 1) cells[i + 1]?.focus();
    else if (e.key === 'Enter') void pair();
  }

  const complete = $derived(code.every(Boolean));

  async function getOverSsh() {
    if (!reached || overSsh) return;
    overSsh = true;
    codeError = '';
    try {
      const r = await local<Outcome<string>>('server_code_over_ssh', { attempt: reached.attempt });
      if (r.outcome === 'ok') fill(r.value, 0);
      else {
        problem = r.problem;
        step = 'problem';
      }
    } finally {
      overSsh = false;
    }
  }

  async function pair() {
    if (!reached || !complete || busy) return;
    busy = true;
    try {
      const joined = `${code.slice(0, 4).join('')}-${code.slice(4).join('')}`;
      const r = await local<Outcome<string>>('server_pair', { attempt: reached.attempt, code: joined, name, device });
      if (r.outcome === 'problem') {
        if (r.problem.kind === 'code') {
          const reason = r.problem.reason;
          codeError =
            reason === 'wrong'
              ? t('servers.dialog.code.wrong')
              : reason === 'wait'
                ? t('servers.dialog.code.wait')
                : reason === 'tries'
                  ? t('servers.dialog.code.tries')
                  : t('servers.dialog.code.expired');
          code = Array(LENGTH).fill('');
          await tick();
          cells[0]?.focus();
        } else {
          problem = r.problem;
          step = 'problem';
        }
        return;
      }
      const id = r.value;
      reached = null;
      step = 'space';
      const failed = await goToServer(id);
      if (failed) {
        problem = failed;
        step = 'problem';
      }
    } finally {
      busy = false;
    }
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      // The clipboard refused: the text stays on screen.
    }
  }

  function editHost() {
    problem = null;
    step = 'host';
    void tick().then(() => typedField?.focus());
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      close();
    }
  }

  type State = 'done' | 'cur' | 'bad' | '';
  const steps = $derived.by((): [string, string, State][] => {
    const connectState: State = step === 'problem' ? 'bad' : step === 'connecting' ? 'cur' : step === 'host' ? '' : 'done';
    return [
      [t('servers.dialog.steps.host'), step === 'host' ? t('servers.dialog.steps.hostHint') : host, step === 'host' ? 'cur' : 'done'],
      [
        t('servers.dialog.steps.connect'),
        reached ? `SSH · Escouade ${reached.version}` : t('servers.dialog.steps.connectHint'),
        connectState,
      ],
      [t('servers.dialog.steps.pair'), t('servers.dialog.steps.pairHint'), step === 'pair' ? 'cur' : step === 'space' ? 'done' : ''],
      [t('servers.dialog.steps.space'), t('servers.dialog.steps.spaceHint'), step === 'space' ? 'cur' : ''],
    ];
  });
</script>

<svelte:window {onkeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay">
  <div class="dialog" use:trapFocus role="dialog" aria-modal="true" aria-label={t('servers.dialog.title')} tabindex="-1">
    <div class="steps">
      <h2>{t('servers.dialog.title')}</h2>
      <ol>
        {#each steps as [label, sub, state], i (i)}
          <li class="step {state}" aria-current={state === 'cur' ? 'step' : undefined}>
            <span class="n"
              >{#if state === 'done'}<Icon name="check" size={11} />{:else if state === 'bad'}<Icon name="close" size={11} />{:else}{i +
                  1}{/if}</span
            >
            <b>{label}</b><span class="s">{sub}</span>
          </li>
        {/each}
      </ol>
    </div>

    <div class="cbody">
      {#if step === 'host'}
        <h3>{t('servers.dialog.host.title')}</h3>
        <p>{t('servers.dialog.host.intro')}</p>
        {#if hosts.length}
          <div class="hosts" role="radiogroup" aria-label={t('servers.dialog.host.fromConfig')}>
            {#each hosts as h (h.alias)}
              <button
                class="hostrow"
                class:on={chosen === h.alias && !typed.trim()}
                role="radio"
                aria-checked={chosen === h.alias && !typed.trim()}
                onclick={() => {
                  chosen = h.alias;
                  typed = '';
                }}
                ondblclick={() => void connect()}
              >
                <span class="radio" class:on={chosen === h.alias && !typed.trim()}></span>
                <b class="mono">{h.alias}</b>
                <span class="cfg">{t('servers.dialog.host.fromConfig')}</span>
                <span class="s">{describe(h)}</span>
              </button>
            {/each}
          </div>
        {:else}
          <p class="hint">{t('servers.dialog.host.none')}</p>
        {/if}
        <label class="fieldset">
          <span class="lbl">{t('servers.dialog.host.other')}</span>
          <input
            bind:this={typedField}
            class="field mono"
            bind:value={typed}
            placeholder={t('servers.dialog.host.otherPlaceholder')}
            spellcheck="false"
            autocomplete="off"
            onkeydown={(e) => e.key === 'Enter' && void connect()}
          />
        </label>
        <label class="fieldset port">
          <span class="lbl">{t('servers.dialog.host.serverPort')}</span>
          <input class="field mono" bind:value={serverPort} placeholder="46480" inputmode="numeric" spellcheck="false" />
        </label>
        <div class="acts">
          <span class="hint"><Icon name="info" size={12} />{t('servers.dialog.host.hint')}</span>
          <span class="sp"></span>
          <button class="btn" onclick={close}>{t('common.cancel')}</button>
          <button class="btn primary" disabled={!host} onclick={() => void connect()}>
            {t('servers.dialog.host.submit')}<kbd>{t('format.keys.enter')}</kbd>
          </button>
        </div>
      {:else if step === 'connecting'}
        <h3>{t('servers.dialog.connecting.title', { host })}</h3>
        <p class="wait">
          <span class="spin" aria-hidden="true"><Icon name="refresh" size={14} /></span>{t('servers.dialog.connecting.ssh')}
        </p>
        <div class="acts"><span class="sp"></span><button class="btn" onclick={close}>{t('common.cancel')}</button></div>
      {:else if step === 'problem' && problem}
        <ServerProblem {problem} {host} {target} name={known?.name}>
          {#snippet actions()}
            {#if problem && problem.kind !== 'hostKey' && 'command' in problem && problem.command}
              <button class="btn ghost" onclick={() => problem && 'command' in problem && void copy(problem.command)}>
                <Icon name={copied ? 'check' : 'term'} size={12} />{t('servers.problem.tryInTerminal')}
              </button>
            {/if}
            {#if problem?.kind === 'hostKey'}
              <button class="btn" onclick={() => void copy(`ssh ${host}`)}>
                <Icon name={copied ? 'check' : 'copy'} size={12} />{t('servers.problem.hostKey.copy')}
              </button>
            {/if}
            <span class="sp"></span>
            {#if !existing}<button class="btn" onclick={editHost}>{t('servers.problem.editHost')}</button>{/if}
            <button class="btn primary" onclick={() => void connect()}><Icon name="refresh" size={12} />{t('servers.problem.retry')}</button
            >
          {/snippet}
        </ServerProblem>
      {:else if step === 'pair' && reached}
        <h3>{t('servers.dialog.pair.title', { host })}</h3>
        <p>{t('servers.dialog.pair.intro', { version: reached.version })}</p>
        <div class="instr">
          <div class="r">
            <span class="k">{t('servers.dialog.pair.run')}</span>
            <span class="cmd"
              >{PAIR}<button class="icon-btn" aria-label={t('servers.dialog.pair.copy')} onclick={() => void copy(PAIR)}
                ><Icon name={copied ? 'check' : 'copy'} size={12} /></button
              ></span
            >
            <span class="hint"
              >{t('servers.dialog.pair.or')}
              <button class="link" disabled={overSsh} onclick={() => void getOverSsh()} data-testid="pair-over-ssh"
                >{overSsh ? t('servers.dialog.pair.overSshRunning') : t('servers.dialog.pair.overSsh')}</button
              ></span
            >
          </div>
          <div class="r"><span class="k">{t('servers.dialog.pair.type')}</span></div>
          <div class="pin" role="group" aria-label={t('servers.dialog.pair.codeLabel')}>
            {#each code as c, i (i)}
              {#if i === 4}<i aria-hidden="true"></i>{/if}
              <input
                bind:this={cells[i]}
                value={c}
                class:bad={!!codeError}
                maxlength={LENGTH}
                aria-label={`${i + 1}`}
                autocomplete="off"
                spellcheck="false"
                oninput={(e) => void typeAt(i, e)}
                onpaste={(e) => onPaste(i, e)}
                onkeydown={(e) => onKey(i, e)}
              />
            {/each}
          </div>
          {#if codeError}
            <span class="err" role="alert">{codeError}</span>
          {:else}
            <span class="hint"><Icon name="clock" size={12} />{t('servers.dialog.pair.lasts')}</span>
          {/if}
        </div>
        <div class="grid2">
          <label class="fieldset">
            <span class="lbl">{t('servers.dialog.pair.spaceName')}</span>
            <span class="field icon"><Icon name="server" size={13} /><input bind:value={name} spellcheck="false" maxlength="48" /></span>
          </label>
          <label class="fieldset">
            <span class="lbl">{t('servers.dialog.pair.device')}</span>
            <span class="field icon"><Icon name="monitor" size={13} /><input bind:value={device} spellcheck="false" maxlength="64" /></span>
          </label>
        </div>
        <div class="acts">
          {#if !existing}<button class="btn ghost" onclick={editHost}>{t('servers.dialog.pair.back')}</button>{/if}
          <span class="sp"></span>
          <button class="btn" onclick={close}>{t('common.cancel')}</button>
          <button class="btn primary" disabled={!complete || busy} onclick={() => void pair()}
            >{t('servers.dialog.pair.submit')}<kbd>{t('format.keys.enter')}</kbd></button
          >
        </div>
      {:else if step === 'space'}
        <h3>{t('servers.dialog.space.title', { name })}</h3>
        <p class="wait">
          <span class="spin" aria-hidden="true"><Icon name="refresh" size={14} /></span>{t('servers.dialog.space.text', { name })}
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--scrim);
  }
  .dialog {
    width: min(760px, 100%);
    max-height: 100%;
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    overflow: hidden;
    border-radius: 14px;
    background: var(--panel);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 24px 64px -12px rgba(0, 0, 0, calc(0.75 * var(--shade)));
  }
  .dialog:focus {
    outline: none;
  }
  .steps {
    display: grid;
    align-content: start;
    gap: 2px;
    padding: 18px 12px;
    border-right: 1px solid var(--line);
    background: rgba(var(--fg-rgb), 0.015);
  }
  .steps h2 {
    margin: 0;
    padding: 0 8px 12px;
    font-size: 15px;
    font-weight: 640;
    color: var(--text);
  }
  ol {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .step {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 2px 9px;
    align-items: center;
    padding: 8px;
    border-radius: 9px;
  }
  .step .n {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    font-size: 11px;
    font-weight: 700;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .step b {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--muted);
  }
  .step .s {
    grid-column: 2;
    overflow: hidden;
    font-size: 11.5px;
    color: var(--dim);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .step.done .n {
    background: color-mix(in oklch, var(--run) 14%, transparent);
    color: var(--run);
    box-shadow: none;
  }
  .step.done b {
    color: var(--soft);
  }
  .step.cur {
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .step.cur .n {
    background: var(--primary);
    color: var(--on-primary);
    box-shadow: none;
  }
  .step.cur b {
    color: var(--text);
  }
  .step.bad .n {
    background: color-mix(in oklch, var(--del) 12%, transparent);
    color: var(--del);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 40%, transparent);
  }
  .step.bad b {
    color: var(--del);
  }
  .cbody {
    display: grid;
    gap: 14px;
    align-content: start;
    min-width: 0;
    max-height: calc(100vh - 48px);
    padding: 22px 24px 18px;
    overflow: auto;
  }
  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 640;
    letter-spacing: -0.01em;
    color: var(--text);
  }
  p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
    max-width: 62ch;
  }
  .wait {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .spin {
    display: inline-grid;
    animation: spin 1.2s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
  .hosts {
    display: grid;
    gap: 4px;
  }
  .hostrow {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) auto;
    gap: 2px 10px;
    align-items: center;
    padding: 9px 10px;
    border-radius: 9px;
    text-align: left;
    background: none;
    color: var(--text);
  }
  .hostrow:hover {
    background: var(--hover);
  }
  .hostrow.on {
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--brand);
  }
  .hostrow b {
    font-size: 12.5px;
    font-weight: 500;
  }
  .hostrow .s {
    grid-column: 2;
    font-size: 12px;
    color: var(--muted);
  }
  .radio {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--line-3);
  }
  .radio.on {
    box-shadow: inset 0 0 0 5px var(--brand);
  }
  .cfg {
    display: inline-flex;
    align-items: center;
    height: 21px;
    padding: 0 7px;
    border-radius: 6px;
    font-size: 11.5px;
    font-weight: 550;
    color: var(--muted);
    white-space: nowrap;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .fieldset {
    display: grid;
    gap: 6px;
    min-width: 0;
  }
  .fieldset.port {
    max-width: 220px;
  }
  .lbl {
    font-size: 12px;
    font-weight: 600;
    color: var(--soft);
  }
  .field {
    height: 32px;
    min-width: 0;
    padding: 0 10px;
    border: 0;
    border-radius: 7px;
    background: var(--well);
    color: var(--text);
    font-size: 12.5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .field:focus,
  .field:focus-within {
    outline: none;
    box-shadow:
      inset 0 0 0 1px var(--brand),
      0 0 0 3px color-mix(in oklch, var(--brand) 18%, transparent);
  }
  .field.icon {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .field.icon input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    outline: none;
  }
  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .instr {
    display: grid;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .instr .r {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .instr .k {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--soft);
  }
  .cmd {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 4px 0 10px;
    border-radius: 7px;
    background: var(--well);
    color: var(--code-fg);
    font-family: var(--mono);
    font-size: 12.5px;
    white-space: nowrap;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .cmd .btn {
    height: 22px;
    padding: 0 6px;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--soft);
    font: inherit;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .link:disabled {
    cursor: progress;
    text-decoration: none;
  }
  .pin {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .pin input {
    width: 34px;
    height: 40px;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: var(--well);
    color: var(--text);
    font-family: var(--mono);
    font-size: 18px;
    font-weight: 500;
    text-align: center;
    text-transform: uppercase;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .pin input:focus {
    outline: none;
    box-shadow:
      inset 0 0 0 1px var(--brand),
      0 0 0 3px color-mix(in oklch, var(--brand) 18%, transparent);
  }
  .pin input.bad {
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 60%, transparent);
  }
  .pin i {
    width: 10px;
    height: 2px;
    border-radius: 1px;
    background: var(--line-3);
  }
  .hint {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--dim);
    font-size: 11.5px;
  }
  .err {
    color: var(--del);
    font-size: 12px;
    font-weight: 550;
  }
  .acts {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 4px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
  .sp {
    flex: 1;
  }
  @media (max-width: 1100px) {
    .dialog {
      grid-template-columns: 190px minmax(0, 1fr);
    }
  }
</style>
