<script lang="ts">
  // The window on a server that does not answer it (ESC-537, design/server-2-3 `x-jeton`, `x-version`): the whole space
  // says why — this computer's token revoked (when, by whom), another version, a host key, the policy — or that it is
  // still being reached; with the way back to this computer always there.
  import { connection, type ServerProblem as Problem } from '../../lib/connection.svelte';
  import { t } from '../../lib/i18n';
  import { forgetServer, goHome, nth, openConnect, plateOf, retryNow } from '../../lib/servers.svelte';
  import { app } from '../../lib/state.svelte';
  import { local } from '../../lib/transport';
  import Icon from '../Icon.svelte';
  import ServerProblem from './ServerProblem.svelte';

  const server = $derived(connection.server!);

  /** What stopped the tries, as a state of § 3. */
  const problem = $derived.by((): Problem | null => {
    if (connection.problem) return connection.problem;
    const r = connection.refusal;
    if (!r) return connection.state === 'refused' ? { kind: 'refused', code: 'error', message: connection.reason } : null;
    if (r.code === 'token') return { kind: 'revoked', revoked: r.revoked };
    if (r.code === 'version')
      return {
        kind: 'version',
        server: r.serverVersion ?? '?',
        client: r.clientVersion ?? app.version,
        update: r.update ?? 'server',
        serverUpdate: r.serverUpdate,
      };
    if (r.code === 'policy') return { kind: 'policy', message: r.error };
    return { kind: 'refused', code: r.code, message: r.error };
  });

  // Noted on this computer, for Réglages › Serveurs once back here.
  $effect(() => {
    const p = problem;
    if (p && connection.state === 'refused') void local('server_note', { id: server.id, problem: $state.snapshot(p) }).catch(() => {});
  });

  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });
  const seconds = $derived(connection.retryAt ? Math.max(0, Math.ceil((connection.retryAt - now) / 1000)) : 0);

  async function pairAgain() {
    // A new code, through a new pairing of the same server: its line kept, its token replaced.
    await local('server_leave');
    openConnect(server.id);
  }

  async function remove() {
    await forgetServer(server.id);
  }

  async function updateClient() {
    app.modal = { kind: 'settings', page: 'about' };
  }
</script>

<div class="screen" data-testid="server-screen">
  <div class="card">
    <span class="plate" aria-hidden="true">{plateOf(server.name)}</span>
    {#if problem}
      <ServerProblem {problem} host={server.ssh} name={server.name} target={server.host}>
        {#snippet actions()}
          {#if problem?.kind === 'revoked'}
            <button class="btn primary" onclick={() => void pairAgain()}
              ><Icon name="key" size={12} />{t('servers.problem.revoked.again')}</button
            >
            <button class="btn" onclick={() => void goHome()}>{t('servers.backHome')}</button>
            <button class="btn ghost danger" onclick={() => void remove()}
              >{t('servers.problem.revoked.remove', { name: server.name })}</button
            >
          {:else if problem?.kind === 'version' && problem.update === 'client'}
            <button class="btn primary" onclick={() => void updateClient()}>{t('servers.problem.version.updateClientButton')}</button>
            <button class="btn" onclick={() => void goHome()}>{t('servers.backHome')}</button>
          {:else}
            <button class="btn primary" onclick={() => void retryNow()}
              ><Icon name="refresh" size={12} />{t('servers.problem.retry')}</button
            >
            <button class="btn" onclick={() => void goHome()}>{t('servers.backHome')}</button>
          {/if}
        {/snippet}
      </ServerProblem>
    {:else}
      <h3>{t('servers.status.connecting', { name: server.name })}</h3>
      <p>
        {connection.retryAt ? t('servers.band.retryIn', { seconds, nth: nth(connection.tries) }) : t('servers.band.retrying')}
        {t('servers.problem.agentsGoOn')}
      </p>
      <div class="acts">
        <button class="btn primary" onclick={() => void retryNow()}><Icon name="refresh" size={12} />{t('servers.band.retry')}</button>
        <button class="btn" onclick={() => void goHome()}>{t('servers.backHome')}</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .screen {
    flex: 1;
    display: grid;
    place-items: center;
    min-width: 0;
    padding: 24px;
    overflow: auto;
  }
  .card {
    display: grid;
    gap: 16px;
    width: min(520px, 100%);
  }
  .plate {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 12px;
    background: var(--raise);
    color: var(--wait);
    font-size: 22px;
    font-weight: 700;
    box-shadow: inset 0 0 0 1.5px var(--wait-line);
  }
  h3 {
    margin: 0;
    font-size: 17px;
    font-weight: 640;
    color: var(--text);
  }
  p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
  }
  .acts {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .danger {
    color: var(--del);
  }
</style>
