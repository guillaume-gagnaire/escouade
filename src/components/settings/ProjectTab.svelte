<script lang="ts">
  import { openPath } from '@tauri-apps/plugin-opener';
  import { tick, type Snippet } from 'svelte';
  import { askCloseProject } from '../../lib/project-actions';
  import { revealHidden } from '../../lib/recipe';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { PROJECT_COLORS } from '../../lib/theme';
  import type { CommitMode, Project, WorktreeStep } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import StepList from './StepList.svelte';
  import Switch from './Switch.svelte';

  // A project's own settings: who it is, its worktrees and what runs in them, its launch commands (its draft: the
  // modal's project).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);
  /** Who writes the commits of the files panel's « Commit… » and « Commit tout… ». */
  const COMMIT_MODES: { value: CommitMode; label: string }[] = [
    { value: 'agent', label: "Rédigé par l'agent" },
    { value: 'direct', label: 'Direct, avec un message proposé' },
  ];

  /** Asks before closing it, then comes back to these settings, their draft as it was. */
  function askClose() {
    // Taken now: the tab is gone while the confirmation is open.
    const projectId = project.id;
    askCloseProject(project, () => {
      app.modal = { kind: 'settings', tab: 'projects', projectId, resume: true };
    });
  }

  // What Claude proposed, read here in full before it replaces anything of the draft.
  const proposal = $derived(settingsForm.proposal[project.id]);
  const launchProposal = $derived(settingsForm.launchProposal[project.id]);
  let suggestButton = $state<HTMLButtonElement>();
  let launchButton = $state<HTMLButtonElement>();

  /** A shell by its name, or its id when this machine has none such. */
  const shellName = (id: string) => app.shells.find((s) => s.id === id)?.label ?? `${revealHidden(id)} (introuvable)`;

  /**
   * Asks Claude with `button`, then hands its proposal the focus, to be read from the keyboard: only when the focus is
   * still where the button left it (the reading takes minutes, the user may have gone on elsewhere).
   */
  async function ask(read: () => Promise<void>, button: HTMLButtonElement | undefined, proposalId: string) {
    await read();
    await tick();
    const at = document.activeElement;
    if (!at || at === document.body || at === button) document.getElementById(proposalId)?.focus();
  }

  /** Takes or ignores a proposal, the focus back on the button that asked for it. */
  async function settle(run: () => void, button: HTMLButtonElement | undefined) {
    run();
    await tick();
    button?.focus();
  }
</script>

{#snippet facts(shell: string, dir: string)}
  <dl class="facts">
    <dt>Shell</dt>
    <dd>{shellName(shell)}</dd>
    <dt>Dossier</dt>
    <dd class="mono">{dir}</dd>
  </dl>
{/snippet}

{#snippet proposed(id: string, label: string, notice: string, list: Snippet, take: () => void, ignore: () => void)}
  <div {id} class="proposal" role="region" aria-label={label} tabindex="-1">
    <p class="notice">{notice}</p>
    {@render list()}
    <div class="actions">
      <button class="btn ghost" onclick={ignore}>Ignorer</button>
      <button class="btn primary" onclick={take}>Remplacer les commandes</button>
    </div>
  </div>
{/snippet}

{#snippet stepsOf(title: string, steps: WorktreeStep[])}
  <section class="part">
    <h4 class="section-label">{title}</h4>
    {#if steps.length}
      <ol class="steps">
        {#each steps as s (s.id)}
          <li>
            <pre class="command mono">{revealHidden(s.command)}</pre>
            {@render facts(s.shell, s.cwd.trim() && s.cwd.trim() !== '.' ? revealHidden(s.cwd) : 'la racine du worktree')}
          </li>
        {/each}
      </ol>
    {:else}
      <p class="none">Aucune commande.</p>
    {/if}
  </section>
{/snippet}

{#snippet proposedSteps()}
  {@render stepsOf("À l'ouverture d'un worktree", proposal!.setup)}
  {@render stepsOf('Avant sa suppression', proposal!.teardown)}
{/snippet}

{#snippet proposedLaunch()}
  <ul class="steps">
    {#each launchProposal! as c (c.id)}
      <li>
        <span class="name">{revealHidden(c.name.trim())}</span>
        <pre class="command mono">{revealHidden(c.command)}</pre>
        {@render facts(c.shell, c.cwd.trim() ? revealHidden(c.cwd) : 'le dossier du projet')}
      </li>
    {/each}
  </ul>
{/snippet}

<Group title="Identité">
  <Row label="Nom">
    <input class="field input" aria-label="Nom du projet" spellcheck="false" bind:value={draft.name} />
  </Row>
  <Row label="Dossier">
    <span class="path mono" title={project.path}>{project.path}</span>
    <button class="btn" onclick={() => openPath(project.path).catch((err) => app.toast(String(err), 'error'))}>Ouvrir</button>
  </Row>
  <Row label="Couleur">
    <div class="colors" role="group" aria-label="Couleur">
      {#each PROJECT_COLORS as c, i (c)}
        <button
          class="swatch"
          aria-label="Couleur {i + 1}"
          aria-pressed={draft.color === c}
          style:background={c}
          style:box-shadow={draft.color === c ? '0 0 0 2px var(--bg), 0 0 0 4px var(--text)' : 'none'}
          onclick={() => (draft.color = c)}
        ></button>
      {/each}
    </div>
  </Row>
</Group>

<Group title="Git">
  <Row label="Un worktree par agent" desc="Isole les modifications de chaque nouvel agent dans sa propre branche.">
    <Switch label="Un worktree par agent" bind:on={draft.worktreePerAgent} />
  </Row>
  <Row
    label="Fichiers copiés dans les worktrees"
    desc="Seuls ceux que git ignore, jamais commités ; un motif par ligne : .env* à la racine, **/.env* partout."
    descId="copy-desc"
    wide
  >
    <textarea
      class="field mono copy"
      rows="3"
      placeholder=".env*"
      aria-label="Fichiers copiés dans les worktrees"
      aria-describedby="copy-desc"
      bind:value={draft.copy}
    ></textarea>
  </Row>
  <Row label="Commit" desc="Direct : Escouade propose un message, tu le relis et tu commites toi-même.">
    <Chips label="Commit" options={COMMIT_MODES} bind:value={draft.commitMode} />
  </Row>
</Group>

<Group
  title="Worktrees"
  anchor="worktrees"
  note="Variables disponibles : ESCOUADE_PROJECT_DIR (le projet), ESCOUADE_WORKTREE_DIR, ESCOUADE_BRANCH, et les ports réservés d'un ticket (ESCOUADE_PORT_BASE, ESCOUADE_PORT_END). Un échec est signalé dans la conversation de l'agent."
>
  <Row
    label="Remplir automatiquement"
    desc="Claude lit le projet (manifestes, lockfiles, README…) sans rien modifier et propose les commandes. Relis-les avant d'enregistrer."
    descId="worktrees-suggest-desc"
  >
    <!-- Two buttons of this tab read the same: what each row says of itself tells them apart. -->
    <button
      class="btn suggest"
      aria-describedby="worktrees-suggest-desc"
      disabled={settingsForm.suggesting[project.id]}
      bind:this={suggestButton}
      onclick={() => ask(() => settingsForm.suggest(project.id), suggestButton, 'worktrees-proposal')}
    >
      {settingsForm.suggesting[project.id] ? 'Claude lit le projet…' : '✦ Remplir automatiquement'}
    </button>
  </Row>
  {#if proposal}
    {@render proposed(
      'worktrees-proposal',
      'Commandes de worktree proposées',
      'Proposition de Claude : relis chaque commande en entier. Elles remplacent les deux listes et, une fois enregistrées, tournent seules dans chaque nouveau worktree.',
      proposedSteps,
      () => settle(() => settingsForm.takeProposal(project.id), suggestButton),
      () => settle(() => delete settingsForm.proposal[project.id], suggestButton),
    )}
  {/if}
  <Row
    label="À l'ouverture d'un worktree"
    desc="Dans l'ordre, avant le premier message de son agent : dépendances, code généré… Les messages attendent la fin."
    wide
  >
    <StepList bind:steps={draft.worktreeSetup} label="Commande de préparation" onadd={() => settingsForm.addStep('setup')} />
  </Row>
  <Row
    label="Avant sa suppression"
    desc="Ce que la préparation a créé hors du worktree (base de données, conteneurs…) ; souvent rien."
    wide
  >
    <StepList bind:steps={draft.worktreeTeardown} label="Commande de démontage" onadd={() => settingsForm.addStep('teardown')} />
  </Row>
</Group>

<Group
  title="Lancement"
  anchor="launch"
  note="Chaque commande tourne dans son propre terminal, en lecture seule. Lance-les depuis la section « Lancement » de la barre latérale."
>
  <div class="fill">
    <Row
      label="Remplir automatiquement"
      desc="Claude lit le projet (manifestes, scripts, docker-compose, README…) sans rien modifier et propose les commandes à lancer. Relis-les avant d'enregistrer."
      descId="launch-suggest-desc"
    >
      <button
        class="btn suggest"
        aria-describedby="launch-suggest-desc"
        disabled={settingsForm.suggestingLaunch[project.id]}
        bind:this={launchButton}
        onclick={() => ask(() => settingsForm.suggestLaunch(project.id), launchButton, 'launch-proposal')}
      >
        {settingsForm.suggestingLaunch[project.id] ? 'Claude lit le projet…' : '✦ Remplir automatiquement'}
      </button>
    </Row>
    {#if launchProposal}
      {@render proposed(
        'launch-proposal',
        'Commandes de lancement proposées',
        'Proposition de Claude : relis chaque commande en entier. Elles remplacent celles de la liste.',
        proposedLaunch,
        () => settle(() => settingsForm.takeLaunchProposal(project.id), launchButton),
        () => settle(() => delete settingsForm.launchProposal[project.id], launchButton),
      )}
    {/if}
  </div>
  {#each draft.runCommands as c, i (c.id)}
    <fieldset class="cmd">
      <legend class="sr">Commande {i + 1}</legend>
      <div class="line">
        <label class="f grow"><span>Nom</span><input class="field" bind:value={c.name} placeholder="ex. Front" /></label>
        <label class="f"
          ><span>Shell</span>
          <select class="field" bind:value={c.shell}>
            {#each app.shells as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
            {#if !app.shells.some((s) => s.id === c.shell)}<option value={c.shell}>{c.shell} (introuvable)</option>{/if}
          </select>
        </label>
        <button class="del" title="Supprimer" aria-label="Supprimer" onclick={() => draft.runCommands.splice(i, 1)}>×</button>
      </div>
      <label class="f"><span>Commande</span><input class="field mono" bind:value={c.command} placeholder="ex. npm run dev" /></label>
      <label class="f"
        ><span>Sous-dossier <em>(vide = dossier du projet)</em></span><input
          class="field mono"
          bind:value={c.cwd}
          placeholder="ex. apps/web"
        /></label
      >
    </fieldset>
  {:else}
    <div class="empty">Aucune commande pour l'instant.</div>
  {/each}
  <div class="add"><button class="btn" onclick={() => settingsForm.addCommand()}>+ Ajouter une commande</button></div>
</Group>

<Group title="Zone sensible">
  <Row
    label="Fermer le projet"
    desc="Le projet et ses agents sont retirés de l'application, conversations comprises. Les fichiers et les worktrees sur le disque ne sont pas touchés."
  >
    <button class="btn danger" onclick={askClose}>Fermer le projet…</button>
  </Row>
</Group>

<style>
  .input {
    flex: 1;
    min-width: 0;
    max-width: 340px;
    height: 32px;
    padding: 0 10px;
    background: var(--panel);
    font-weight: 600;
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .colors {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 9px;
    max-width: 60%;
  }
  .swatch {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 7px;
    padding: 0;
    cursor: pointer;
  }
  .copy {
    height: auto;
    resize: vertical;
    padding: 8px 10px;
    background: var(--panel);
    font-size: 12px;
  }
  .cmd {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 14px 16px;
    border: none;
    border-bottom: 1px solid var(--line);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .line {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  em {
    color: var(--dim);
    font-style: normal;
  }
  .cmd .field {
    height: 32px;
    background: var(--panel);
    font-size: 12.5px;
  }
  select.field {
    min-width: 170px;
  }
  .del {
    width: 32px;
    height: 32px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 15px;
    cursor: pointer;
  }
  .del:hover {
    border-color: var(--del);
    color: var(--del);
  }
  .empty {
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
    color: var(--dim);
  }
  .add {
    padding: 12px 16px;
  }
  .suggest {
    flex-shrink: 0;
  }
  /* Its row stands apart from the commands below, whose boxes have no border above. */
  .fill {
    border-bottom: 1px solid var(--line);
  }
  .proposal {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--line);
  }
  .notice {
    margin: 0;
    padding: 8px 12px;
    border-left: 2px solid var(--wait);
    background: var(--wait-soft);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text);
  }
  .part {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h4 {
    margin: 0;
  }
  .none {
    margin: 0;
    font-size: 12px;
    color: var(--dim);
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .steps li {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  /* What is read here is what the shell reads, in that order: right-to-left letters must not reorder a « ; » or a « | ». */
  .name,
  .command,
  .facts dd {
    unicode-bidi: bidi-override;
    direction: ltr;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
  }
  /* The whole command, as written: never cut, never scrolled out of sight, however long. */
  .command {
    margin: 0;
    padding: 6px 10px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--elev);
    font-size: 11.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .facts {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 10px;
    font-size: 11.5px;
  }
  .facts dt {
    color: var(--dim);
  }
  .facts dd {
    margin: 0;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
