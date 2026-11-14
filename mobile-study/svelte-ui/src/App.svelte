<script lang="ts">
  // La maquette de l'app mobile (ESC-445), la même pour Tauri 2 et Capacitor : la liste des agents qui attendent, la
  // conversation longue, l'autorisation sous verrou biométrique, l'appairage par QR code, le push et le pont Rust.
  import { onMount, tick } from 'svelte';
  import { t } from '../../../src/lib/i18n';
  import Message from './Message.svelte';
  import type { Platform } from './platform';
  import { getJson, postJson, mark, pairing, parsePairUri, savePairing, type MobileItem, type WaitingAgent } from './server';
  import { mobile } from './texts';

  let { platform }: { platform: Platform } = $props();

  let screen = $state<'list' | 'conv' | 'pair' | 'status'>('list');
  let agents = $state<WaitingAgent[]>([]);
  let items = $state<MobileItem[]>([]);
  let current = $state<WaitingAgent | null>(null);
  let toast = $state('');
  let error = $state('');
  let rustVersion = $state('');
  let bridge = $state('');
  let pushState = $state('');
  let paired = $state(pairing());
  let pairResult = $state('');
  let list = $state<HTMLElement>();

  const say = (s: string) => {
    toast = s;
    setTimeout(() => (toast === s ? (toast = '') : null), 4000);
  };

  async function openAgent(a: WaitingAgent) {
    current = a;
    screen = 'conv';
    items = await getJson<MobileItem[]>(`/api/agents/${a.id}/conversation`);
    await tick();
    // content-visibility estime la hauteur des messages hors de l'écran : la hauteur réelle arrive après quelques
    // images ; on redescend jusqu'à ce qu'elle ne bouge plus (comme la conversation du bureau, Conversation.svelte).
    let last = -1;
    for (let i = 0; i < 20 && list && list.scrollHeight !== last; i++) {
      last = list.scrollHeight;
      list.scrollTo({ top: last });
      await new Promise((r) => requestAnimationFrame(r));
    }
    requestAnimationFrame(() => mark(platform.name, 'conv-ready', items.length));
  }

  async function answer(decision: 'allow' | 'deny') {
    const lock = await platform.biometric(mobile.lockReason);
    if (!lock.ok) {
      say(`${mobile.lockFailed} (${lock.detail})`);
      mark(platform.name, 'perm-locked', lock.detail);
      return;
    }
    await postJson(`/api/agents/${current?.id ?? 'a1'}/permission`, { decision, app: platform.name, biometric: lock.detail });
    mark(platform.name, 'perm-sent', decision);
    say(`${mobile.sent} : ${decision === 'allow' ? t('composer.permission.allowed') : t('composer.permission.denied')}`);
  }

  async function scan() {
    const text = await platform.scanQr();
    if (!text) return;
    const p = parsePairUri(text);
    if (!p) {
      pairResult = `QR inconnu : ${text}`;
      return;
    }
    savePairing(p);
    paired = p;
    // Le serveur scelle un texte avec la clé du QR code : l'ouvrir par le Rust prouve l'appairage et le pont.
    const { sealed } = await getJson<{ sealed: string }>('/api/challenge');
    pairResult = await platform.rust.open(p.key, sealed);
    mark(platform.name, 'paired', pairResult.length);
  }

  async function measureBridge() {
    const key = paired.key || import.meta.env.VITE_MS_KEY;
    const sealed = await platform.rust.seal(key, 'x'.repeat(256));
    const n = 1000;
    const start = performance.now();
    for (let i = 0; i < n; i++) await platform.rust.open(key, sealed);
    const perCall = ((performance.now() - start) * 1000) / n;
    const inRust = (await platform.rust.benchInRust(n, 256)) / n;
    bridge = `${perCall.toFixed(1)} µs par appel (dont ${inRust.toFixed(1)} µs de Rust pour seal + open)`;
    mark(platform.name, 'bridge-us', perCall.toFixed(1));
  }

  async function registerPush() {
    const r = await platform.push.register();
    pushState = r.token ? `jeton ${r.token.slice(0, 24)}…` : r.detail;
    mark(platform.name, 'push-register', r.token ? 'token' : r.detail.slice(0, 80));
  }

  onMount(async () => {
    platform.push.onOpened(async (p) => {
      let text = '(sans contenu scellé)';
      const key = paired.key || import.meta.env.VITE_MS_KEY;
      if (p.sealed) {
        try {
          text = await platform.rust.open(key, p.sealed);
        } catch (e) {
          text = `ne s'ouvre pas : ${e}`;
        }
      }
      say(`${mobile.pushOpened} : ${text}`);
      mark(platform.name, 'push-open', p.sealed ? 'decrypted' : 'plain');
    });
    try {
      agents = await getJson<WaitingAgent[]>('/api/agents');
      await tick();
      requestAnimationFrame(() => mark(platform.name, 'ready', agents.length));
    } catch (e) {
      error = String(e);
    }
    rustVersion = await platform.rust.version().catch((e) => `erreur : ${e}`);
    setTimeout(() => measureBridge().catch((e) => (bridge = `erreur : ${e}`)), 1500);
  });
</script>

<div class="shell">
  <header>
    {#if screen !== 'list'}
      <button class="back" onclick={() => (screen = 'list')} aria-label={mobile.back}>‹</button>
    {/if}
    <h1>{screen === 'conv' ? current?.name : screen === 'pair' ? mobile.pairTitle : screen === 'status' ? mobile.status : t('nav.titleBar.waiting')}</h1>
    {#if screen === 'list'}
      <button class="chip" onclick={() => (screen = 'pair')}>{mobile.pair}</button>
      <button class="chip" onclick={() => (screen = 'status')}>{mobile.status}</button>
    {/if}
  </header>

  {#if screen === 'list'}
    <main class="agents">
      {#if error}<p class="err">{error}</p>{/if}
      {#each agents as a (a.id)}
        <button class="agent" onclick={() => openAgent(a)}>
          <span class="dot" class:wait={a.waiting}></span>
          <span class="who"><b>{a.name}</b> <small>{a.project}</small><br /><span class="prev">{a.preview}</span></span>
          <span class="badge">{a.waiting === 'permission' ? mobile.permission : a.waiting === 'question' ? mobile.question : mobile.noWait}</span>
        </button>
      {/each}
    </main>
  {:else if screen === 'conv'}
    <main class="conv" bind:this={list} data-testid="conversation">
      {#each items as item (item.id)}
        <div class="row"><Message {item} onAnswer={item.kind === 'permission' ? answer : undefined} /></div>
      {/each}
    </main>
  {:else if screen === 'pair'}
    <main class="pane">
      <p>{mobile.pairHelp}</p>
      <button class="primary" onclick={scan}>{mobile.scan}</button>
      <p>{paired.name ? `${mobile.paired} : ${paired.name} (${paired.url})` : mobile.notPaired}</p>
      {#if pairResult}<p class="ok">{pairResult}</p>{/if}
    </main>
  {:else}
    <main class="pane">
      <dl>
        <dt>{mobile.rust}</dt><dd>{rustVersion}</dd>
        <dt>{mobile.bridge}</dt><dd>{bridge || '…'}</dd>
        <dt>{mobile.push}</dt><dd>{pushState || '—'}</dd>
        <dt>Serveur</dt><dd>{paired.url}</dd>
      </dl>
      <button class="primary" onclick={registerPush}>{mobile.pushRegister}</button>
    </main>
  {/if}

  {#if toast}<div class="toast" role="status">{toast}</div>{/if}
</div>
