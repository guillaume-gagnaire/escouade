<script lang="ts" module>
  /** Where the window of the teams is (TS-3). */
  export type TeamStep =
    | 'home'
    | 'create'
    | 'words'
    | 'created'
    | 'invite'
    | 'join'
    | 'device'
    | 'pairJoin'
    | 'recover'
    | 'settings'
    | 'projects'
    | 'costs'
    | 'plugins'
    | 'told';
</script>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { teams, type Created, type TeamView } from '../../lib/teams.svelte';
  import Modal from '../modals/Modal.svelte';
  import RecoveryWords from './RecoveryWords.svelte';
  import TeamCreate from './TeamCreate.svelte';
  import TeamCreated from './TeamCreated.svelte';
  import TeamDevice from './TeamDevice.svelte';
  import TeamHome from './TeamHome.svelte';
  import TeamInvite from './TeamInvite.svelte';
  import TeamJoin from './TeamJoin.svelte';
  import TeamPairJoin from './TeamPairJoin.svelte';
  import TeamRecover from './TeamRecover.svelte';
  import TeamProjects from './TeamProjects.svelte';
  import TeamSettings from './TeamSettings.svelte';
  import TeamCosts from './TeamCosts.svelte';
  import TeamCostsTold from './TeamCostsTold.svelte';
  import TeamPlugins from './TeamPlugins.svelte';

  let {
    step: first = 'home',
    link = '',
    teamId: firstTeam = null,
    expected = '',
  }: { step?: TeamStep; link?: string; teamId?: string | null; expected?: string } = $props();

  let step = $state<TeamStep>(untrack(() => first));
  let teamId = $state<string | null>(untrack(() => firstTeam));
  /** The 24 words of a team just created, recovered or given a new key: shown once, then let go. */
  let words = $state<string[]>([]);
  /** The team just created: its summary follows its words. */
  let made = $state<TeamView | null>(null);
  let fromCreate = $state(false);

  onMount(() => void teams.load());

  function go(next: TeamStep, id: string | null = teamId) {
    teamId = id;
    step = next;
  }

  /** A team made (created, recovered): its words to note, then its summary. */
  function took(c: Created, created: boolean) {
    teams.put(c.team);
    made = c.team;
    fromCreate = created;
    words = c.words;
    go('words', c.team.id);
  }

  function close() {
    words = [];
    app.modal = null;
  }

  const team = $derived(teams.byId(teamId));

  function titleOf(s: TeamStep): string {
    if (s === 'projects') return t('teamProjects.title');
    if (s === 'costs') return t('teamCosts.title');
    if (s === 'plugins') return t('teamPlugins.title');
    if (s === 'told') return t('teamCosts.told.title');
    return t(`team.title.${s}`);
  }
  const wide = $derived(
    step === 'home' ||
      step === 'words' ||
      step === 'device' ||
      step === 'invite' ||
      step === 'settings' ||
      step === 'projects' ||
      step === 'plugins' ||
      step === 'costs',
  );
</script>

<Modal title={titleOf(step)} width={step === 'costs' ? 760 : wide ? 640 : 520} onclose={close}>
  {#if step === 'home'}
    <TeamHome
      onstep={(s, id) => go(s, id ?? null)}
      onwords={(id, w) => {
        words = w;
        fromCreate = false;
        go('words', id);
      }}
    />
  {:else if step === 'create'}
    <TeamCreate onmade={(c) => took(c, true)} onback={() => go('home')} />
  {:else if step === 'recover'}
    <TeamRecover onmade={(c) => took(c, false)} onback={() => go('home')} />
  {:else if step === 'words' && teamId}
    <RecoveryWords
      {teamId}
      {words}
      ondone={() => {
        words = [];
        go(fromCreate ? 'created' : 'home');
        fromCreate = false;
      }}
    />
  {:else if step === 'created' && (team ?? made)}
    <TeamCreated team={(team ?? made)!} oninvite={() => go('invite')} ondone={close} />
  {:else if step === 'invite' && teamId}
    <TeamInvite {teamId} onback={() => go('home')} />
  {:else if step === 'projects' && teamId}
    <TeamProjects {teamId} onback={() => go('home')} />
  {:else if step === 'plugins' && teamId}
    <TeamPlugins {teamId} onback={() => go('home')} />
  {:else if step === 'costs' && teamId}
    <TeamCosts {teamId} onback={() => go('home')} />
  {:else if step === 'told' && teamId}
    <TeamCostsTold {teamId} name={team?.name ?? ''} ondone={close} onsee={() => go('costs')} />
  {:else if step === 'settings' && teamId}
    <TeamSettings {teamId} onback={() => go('home')} />
  {:else if step === 'device' && teamId}
    <TeamDevice {teamId} onback={() => go('home')} />
  {:else if step === 'join'}
    <TeamJoin {link} {expected} onback={() => go('home')} ondone={close} />
  {:else if step === 'pairJoin'}
    <TeamPairJoin onback={() => go('home')} ondone={close} />
  {:else}
    <TeamHome onstep={(s, id) => go(s, id ?? null)} onwords={(id, w) => ((words = w), go('words', id))} />
  {/if}
</Modal>
