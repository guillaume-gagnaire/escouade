<script lang="ts">
  import { api } from '../../lib/ipc';
  import { EFFORTS, MODES, modelOptions } from '../../lib/models';
  import { IS_MAC } from '../../lib/platform';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { checkForUpdate } from '../../lib/updater';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // The tabs of the app's own settings.
  let { tab }: { tab: 'claude' | 'notifications' | 'terminals' | 'network' | 'about' } = $props();

  const s = $derived(settingsForm.settings);
  let checking = $state(false);

  async function check() {
    checking = true;
    const found = await checkForUpdate(true);
    checking = false;
    if (!found) app.toast('Aucune mise à jour disponible.', 'info');
  }
</script>

{#snippet text(label: string, hint: string, value: string, set: (v: string) => void, placeholder = '')}
  <Row {label} {hint}>
    <input
      class="field mono input"
      aria-label={label}
      spellcheck="false"
      {placeholder}
      {value}
      oninput={(e) => set(e.currentTarget.value)}
    />
  </Row>
{/snippet}

{#if tab === 'claude'}
  <Group title="Exécutable">
    {@render text(
      "Chemin de l'exécutable",
      'vide = détection automatique',
      s.claudePath,
      (v) => (s.claudePath = v),
      app.claudeFound ? 'claude (trouvé dans le PATH)' : 'introuvable — indique le chemin de claude.exe',
    )}
  </Group>
  <Group title="Nouveaux agents">
    <Row label="Modèle par défaut">
      <Chips label="Modèle par défaut" options={modelOptions(app.models)} bind:value={s.defaultModel} />
    </Row>
    <Row label="Effort par défaut">
      <Chips label="Effort par défaut" options={EFFORTS} bind:value={s.defaultEffort} />
    </Row>
    <Row label="Mode de permission par défaut">
      <Chips label="Mode de permission par défaut" options={MODES} bind:value={s.defaultMode} />
    </Row>
  </Group>
  <Group title="Processus">
    <Row label="Reprise automatique après la limite d’usage" desc="« continue » envoyé une fois le quota réinitialisé">
      <Switch label="Reprise automatique après la limite d’usage" bind:on={s.autoResume} />
    </Row>
    <Row label="Arrêter les processus Claude inactifs après" hint="minutes, 0 = jamais">
      <input
        class="field mono input short"
        type="number"
        min="0"
        aria-label="Arrêter les processus Claude inactifs après (minutes)"
        bind:value={s.idleStopMinutes}
      />
    </Row>
  </Group>
{:else if tab === 'notifications'}
  <Group
    title="Me prévenir pour"
    note="Décoché, un type de notification ne joue plus de son et n’affiche plus de notification système. L’onglet et la carte de l’agent clignotent quand même."
  >
    <Row label="Questions et autorisations">
      <Switch label="Questions et autorisations" bind:on={s.notifyFor.questions} />
    </Row>
    <Row label="Tâches terminées">
      <Switch label="Tâches terminées" bind:on={s.notifyFor.done} />
    </Row>
    <Row label="Erreurs">
      <Switch label="Erreurs" bind:on={s.notifyFor.errors} />
    </Row>
    <Row label="Tickets (prêt à tester, bloqué)">
      <Switch label="Tickets (prêt à tester, bloqué)" bind:on={s.notifyFor.tickets} />
    </Row>
  </Group>
  <Group title="Canaux">
    <Row label="Notifications {IS_MAC ? 'macOS' : 'Windows'} quand l'app n'est pas au premier plan">
      <Switch label="Notifications système" bind:on={s.osNotifications} />
    </Row>
  </Group>
  <Group title="Son">
    <Row label="Son activé" desc="Question de Claude, fin de tour">
      <Switch label="Son activé" bind:on={s.sound} />
    </Row>
    <Row label="Tester le son">
      <button class="btn" onclick={() => api.playChime()}>▶ Tester</button>
    </Row>
  </Group>
{:else if tab === 'terminals'}
  <Group title="Chemins" note="Détectés : {app.shells.map((x) => x.label).join(', ') || 'aucun'}">
    {#if IS_MAC}
      {@render text('bash', 'vide = auto', s.bashPath, (v) => (s.bashPath = v))}
    {:else}
      {@render text('PowerShell 7', 'vide = auto', s.pwshPath, (v) => (s.pwshPath = v))}
      {@render text('Git Bash', 'vide = auto', s.bashPath, (v) => (s.bashPath = v))}
      {@render text('Distribution WSL', 'vide = distribution par défaut', s.wslDistro, (v) => (s.wslDistro = v), 'Ubuntu')}
    {/if}
  </Group>
{:else if tab === 'network'}
  <Group
    title="Proxy"
    note="Le proxy est transmis aux processus Claude Code, à la lecture des quotas, aux intégrations et aux mises à jour. Il s'applique aux agents au prochain (re)démarrage de leur processus."
  >
    {@render text('Proxy HTTP(S)', 'ex. http://utilisateur:motdepasse@proxy:3128', s.proxyUrl, (v) => (s.proxyUrl = v), 'aucun')}
    {@render text('Exclusions', 'NO_PROXY, séparées par des virgules', s.noProxy, (v) => (s.noProxy = v))}
    <Row label="Appliquer aussi le proxy aux terminaux intégrés">
      <Switch label="Appliquer aussi le proxy aux terminaux intégrés" bind:on={s.proxyTerminals} />
    </Row>
  </Group>
  <Group
    title="Certificats"
    note="Les intégrations, les quotas et les mises à jour font confiance aux certificats reconnus et à ceux installés sur ce système (celui d'un proxy d'entreprise, le plus souvent). Claude Code, lui, a sa propre liste."
  >
    <Row
      label="Ignorer la vérification des certificats TLS"
      desc="Pour un proxy qui déchiffre le trafic avec un certificat non reconnu (erreur « UnknownIssuer »). S'applique aux intégrations, aux quotas, aux mises à jour, et aux processus Claude Code (au prochain démarrage de leur processus) avec les commandes que lancent les agents (npm, node…). À réserver à un réseau de confiance : une connexion interceptée, jetons compris, ne serait plus détectée."
    >
      <Switch label="Ignorer la vérification des certificats TLS" bind:on={s.insecureTls} />
    </Row>
  </Group>
{:else}
  <Group title="Escouade">
    <Row label="Escouade {app.version}" desc={app.claudeFound ? 'Claude Code détecté' : 'Claude Code introuvable'}>
      <button class="btn" disabled={checking} onclick={check}>{checking ? 'Recherche…' : 'Rechercher une mise à jour'}</button>
    </Row>
    <Row label="Données locales" desc="~/.escouade/" />
  </Group>
{/if}

<style>
  .input {
    width: 340px;
    max-width: 50%;
    flex: none;
    height: 32px;
    padding: 0 10px;
    background: var(--panel);
    font-size: 12px;
  }
  .input.short {
    width: 110px;
  }
</style>
