<script lang="ts">
  import { openUrl } from '../../lib/opener';
  import { accountName, newAgentDefaults, pauseOf } from '../../lib/accounts';
  import {
    APPROVE_LABEL,
    canStart,
    doneMeta,
    launchAnyway,
    outcomeShown,
    reviewMeta,
    ticketSpent,
    waitingFor,
    waitLabel,
  } from '../../lib/board';
  import { buffers, lossNotice } from '../../lib/editor/buffers.svelte';
  import { fTime, fWhen } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { SERVICES, shortName } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { EFFORTS, modelLabel, supportsEffort } from '../../lib/models';
  import { openAddress } from '../../lib/recipe';
  import { estimateHint, fSpentUsd } from '../../lib/spend';
  import { app } from '../../lib/state.svelte';
  import { anyRunning, flows, stopTests, testAgent } from '../../lib/test-launch.svelte';
  import type { Project, Ticket } from '../../lib/types';
  import { cardActivity } from '../../lib/sidebar';
  import type { IconName } from '../../lib/icons';
  import { pushedCi, redTargetHolds, targetCiOf } from '../../lib/branch-ci';
  import CiPill from '../CiPill.svelte';
  import EngineMark from '../EngineMark.svelte';
  import Icon from '../Icon.svelte';
  import RejectForm from './RejectForm.svelte';
  import { tip } from '../../lib/tip';
  import { isMulti, repoLabel, touchedRepos } from '../../lib/repos';
  import { alsoOn, authorName, hueOf, initialOf, isMine, isTheirs, sharedFile, theirsLine } from '../../lib/sharing';
  import { shareTicket } from '../../lib/share-actions';
  import { docsStore } from '../../lib/docs.svelte';
  import { pageTitle } from '../../lib/planning';
  import { prIcon, prTone } from '../../lib/prs';
  import {
    asPull,
    canFixCi,
    canFixReviews,
    canUpdateBranch,
    ciLine,
    fixesSpent,
    fixingLine,
    mergeActions,
    mergeHowText,
    mergeWaitLine,
    noLogLine,
    prErrorLine,
    prFacts,
    prStateKey,
    prStateWord,
  } from '../../lib/ticket-prs';
  import { cardEnvironments } from '../../lib/environments';

  const uid = $props.id();

  /** Items of the agent's progress a ticket under way shows: the last ones. */
  const SHOWN = 3;

  let {
    ticket,
    project,
    queueIndex,
    busyCount,
    quota,
    onedit,
  }: { ticket: Ticket; project: Project; queueIndex: number; busyCount: number; quota: number | null; onedit: () => void } = $props();

  const s = $derived(project.board);
  /** Why no ticket of the board starts now (its target branch), if so. */
  const issue = $derived(app.boardIssues[project.id] ?? null);
  /** Why none starts for now (a quota, a usage limit): that of the accounts the project goes to. */
  const pause = $derived(pauseOf(project));
  /** « Pause quand la CI de la cible est rouge » (ESC-63) holds the autopilot's tickets back now. */
  const redTarget = $derived(redTargetHolds(s, targetCiOf(project)));
  /** The branch it pushed without a pull request (« Valider et pousser »), and its CI. */
  const pushed = $derived(pushedCi(ticket));
  const agent = $derived(ticket.agentId ? app.agents[ticket.agentId] : undefined);
  /** Of several repositories (ESC-169): those its agent changed, while it is tested. */
  const changedRepos = $derived(ticket.column === 'review' && agent && isMulti(project) ? touchedRepos(project, agent).map(repoLabel) : []);
  /** What every agent of the ticket used so far: the card of a finished ticket keeps the cost it was closed with. */
  const used = $derived(ticket.column === 'doing' || ticket.column === 'review' ? ticketSpent(ticket, app.agents) : null);
  /**
   * Its own model and effort, which its agent runs over the Kanban's (empty: the Kanban's): the chip beside the key. The
   * effort is left out when the model its agent runs (the ticket's, else the Kanban's, else its account's) takes none:
   * Haiku starts without one.
   */
  const own = $derived.by(() => {
    const runs = ticket.model || s.model || newAgentDefaults(project, ticket.engine ?? undefined).model;
    const model = ticket.model ? modelLabel(ticket.model, app.models) : '';
    const effort = ticket.effort && supportsEffort(runs) ? (EFFORTS.find((e) => e.value === ticket.effort)?.label ?? ticket.effort) : '';
    if (!model && !effort) return null;
    const label = !effort
      ? t('board.card.ownModel', { model })
      : !model
        ? t('board.card.ownEffort', { effort })
        : t('board.card.ownBoth', { model, effort });
    return { text: [model, effort].filter(Boolean).join(' · '), label };
  });
  /** The name of the account it is set to (ESC-153), its id when the settings no longer have it; null on « Auto ». */
  const ownAccount = $derived.by(() => {
    if (!ticket.account) return null;
    const a = (app.settings.accounts ?? []).find((x) => x.id === ticket.account);
    return a ? accountName(a) : ticket.account;
  });
  /** What the paperclip counts: its images and its files, said apart (« 1 image jointe, 2 fichiers joints »). */
  const attached = $derived(ticket.images.length + ticket.files.length);
  const said = $derived(
    [
      ticket.images.length ? t('board.card.images', { count: ticket.images.length }) : '',
      ticket.files.length ? t('board.card.files', { count: ticket.files.length }) : '',
    ]
      .filter(Boolean)
      .join(', '),
  );
  /** The harness its agent runs on, or the one set on a ticket to do (« Harness »): the brand beside its key. */
  const engine = $derived(
    ticket.column === 'doing' && agent ? (agent.engine ?? 'claude') : ticket.column === 'todo' ? ticket.engine : null,
  );
  /** What its agent does right now, as its card in the list says it (« Lit src/db.ts »: the verb, then what it acts on). */
  const activity = $derived(agent && ticket.column === 'doing' ? cardActivity(agent, 0) : null);
  /** The words and the icon of a finished ticket's outcome. */
  const outcome = $derived(ticket.outcome ? outcomeShown(ticket.outcome) : null);
  /** What validating does, drawn on « Valider »: its words are in the button's name. */
  const APPROVE_ICON: Record<string, IconName> = { merge: 'merge', pr: 'pr', push: 'push', keep: 'check' };
  const waiting = $derived(ticket.column === 'doing' && agent?.status === 'waiting');
  // Under way, the last items; to test, all of them.
  const steps = $derived(ticket.column === 'doing' ? ticket.progress.slice(-SHOWN) : ticket.progress);
  const hidden = $derived(ticket.progress.length - steps.length);
  /** The keys of the tickets it comes after that are not done yet: the autopilot leaves it until they are. */
  const awaited = $derived(ticket.column === 'todo' ? waitingFor(ticket, app.tickets) : []);
  /** Shared with the team (ESC-190): another one's ticket, the line it says, someone else on it too. */
  const theirs = $derived(isTheirs(ticket));
  const theirLine = $derived(theirsLine(ticket));
  const other = $derived(alsoOn(ticket, app.settings.userId));
  let rejecting = $state(false);
  /** The ⚠ of a failed sync shows its reason and « Resynchroniser ». */
  let syncOpen = $state(false);
  let resyncing = $state(false);
  let card = $state<HTMLDivElement>();
  let warn = $state<HTMLButtonElement>();
  let panel = $state<HTMLDivElement>();
  // The error gone (a retry went through, by itself or not), the ⚠ and its panel go: before they do, the focus they
  // hold moves to the card, not to the window. A later failure starts closed again.
  $effect.pre(() => {
    if (ticket.external?.error) return;
    if ([warn, panel].some((el) => el?.contains(document.activeElement))) card?.focus();
    syncOpen = false;
  });

  /** « Resynchroniser »: the failed syncs go again at once; their success takes the ⚠ away (the ticket comes back without its error). */
  async function resync() {
    if (resyncing) return;
    resyncing = true;
    const ok = await app.run(api.integrationResync(ticket.id).then(() => true));
    resyncing = false;
    if (!ok) return;
    syncOpen = false;
    // Its button goes with the ⚠: the focus stays on the card.
    card?.focus();
  }

  /** "Lancer": a ticket that waits for others starts without them only once the user agrees. */
  function launch() {
    const start = () => app.run(api.ticketStart(ticket.id));
    if (!awaited.length) {
      start();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: t('board.card.launchTitle', { key: ticket.key }),
      body: launchAnyway(ticket.key, awaited),
      confirm: t('board.card.launchAnyway'),
      onConfirm: start,
    };
  }

  function open() {
    if (ticket.column === 'todo') onedit();
    else if (ticket.agentId) app.selectAgent(ticket.agentId);
  }

  async function remove() {
    // Its agent is archived with it, which stops its test launches: stopped first, their ends are no crashes.
    if (ticket.column !== 'todo' && ticket.agentId) stopTests(ticket.agentId);
    const gone = await app.run(api.ticketDelete(ticket.id).then(() => true));
    if (gone) delete app.tickets[ticket.id];
  }

  /**
   * A ticket « À faire » goes with its description, its images and its files, which the confirmation says (with files, as
   * attachments, images included); an imported one is not brought back by the next import.
   */
  function confirmRemove() {
    const count = ticket.images.length;
    const attachments = count + ticket.files.length;
    const service = ticket.external ? shortName(ticket.external.service) : null;
    let body: string;
    if (ticket.files.length) {
      body =
        service === null
          ? t('board.card.removeBodyFiles', { count: attachments })
          : t('board.card.removeBodyImportedFiles', { count: attachments, service });
    } else if (service === null) body = count ? t('board.card.removeBodyImages', { count }) : t('board.card.removeBody');
    else body = count ? t('board.card.removeBodyImportedImages', { count, service }) : t('board.card.removeBodyImported', { service });
    if (isMine(ticket)) body += ' ' + t('share.menu.removeShared');
    app.modal = {
      kind: 'confirm',
      title: t('board.card.removeTitle', { key: ticket.key }),
      body,
      confirm: t('common.delete'),
      danger: true,
      onConfirm: remove,
    };
  }

  /** « Supprimer pour l'équipe… » of another one's ticket (ESC-190): its file goes, to be committed. */
  function confirmRemoveTheirs() {
    app.modal = {
      kind: 'confirm',
      title: t('share.menu.removeTheirsTitle', { key: ticket.key }),
      body: t('share.menu.removeTheirsBody', { name: authorName(ticket) }),
      confirm: t('common.delete'),
      danger: true,
      onConfirm: remove,
    };
  }

  /** « Partager avec l'équipe », « Arrêter le partage » (ESC-190), for a ticket of ours. */
  function shareItems(): MenuItem[] {
    if (theirs) return [];
    const on = !!ticket.shared && !ticket.shared.gone;
    return [
      on
        ? { label: t('share.menu.unshare'), onClick: () => shareTicket(ticket, project, false) }
        : { label: t('share.menu.share'), onClick: () => shareTicket(ticket, project, true) },
    ];
  }

  function contextMenu(e: MouseEvent) {
    // A text field keeps the browser's menu (copy, paste), as in main.ts.
    if ((e.target as Element).closest('input, textarea')) return;
    if (theirs) {
      // Another one's ticket (ESC-190, choice 118): launched by hand, changed for the team, hidden here or deleted for all.
      const run = ticket.column === 'todo' && canStart(ticket, s, busyCount, quota, issue, pause, redTarget);
      menu.show(e, [
        ...(run ? [{ label: t('board.card.launch'), onClick: launch }] : []),
        ...(ticket.column === 'todo' ? [{ label: t('common.edit'), onClick: onedit }] : []),
        ...(ticket.column !== 'todo' ? [{ label: t('board.card.openAgent'), onClick: open, disabled: !agent }] : []),
        { label: '', separator: true },
        ticket.shared?.hidden
          ? { label: t('share.board.show'), onClick: () => app.run(api.ticketHide(ticket.id, false)) }
          : { label: t('share.menu.hide'), onClick: () => app.run(api.ticketHide(ticket.id, true)) },
        { label: t('share.menu.deleteForTeam'), danger: true, onClick: confirmRemoveTheirs },
      ]);
      return;
    }
    const items: MenuItem[] =
      ticket.column === 'todo'
        ? [
            { label: t('common.edit'), onClick: onedit },
            { label: t('board.card.prioritize'), onClick: () => app.run(api.ticketPrioritize(ticket.id)) },
            { label: '', separator: true },
            ...shareItems(),
            { label: '', separator: true },
            { label: t('common.delete'), danger: true, onClick: confirmRemove },
          ]
        : [
            { label: t('board.card.openAgent'), onClick: open, disabled: !agent },
            { label: '', separator: true },
            ...shareItems(),
            { label: '', separator: true },
            {
              label: t('common.delete'),
              danger: true,
              onClick: () => {
                if (ticket.column === 'done') return remove();
                app.modal = {
                  kind: 'confirm',
                  title: t('board.card.removeRunningTitle', { key: ticket.key }),
                  // Its worktree goes too with the board's cleanup, but what holds work (ESC-143).
                  body: t(s.cleanup ? 'board.card.removeRunningCleanup' : 'board.card.removeRunningBody'),
                  confirm: t('common.delete'),
                  danger: true,
                  onConfirm: remove,
                };
              },
            },
          ];
    menu.show(e, items);
  }

  /** The card opens its agent from the keyboard when it has the focus itself: its buttons keep their keys. */
  function key(e: KeyboardEvent) {
    if (e.target !== e.currentTarget || (e.key !== 'Enter' && e.key !== ' ')) return;
    // Space would scroll the column.
    e.preventDefault();
    open();
  }

  /** The card opens its agent on a click: its buttons only act. */
  function act(e: Event, f: () => unknown) {
    e.stopPropagation();
    f();
  }

  /** « Merger » / « Merger quand c'est vert » (ESC-62): what it did said by a toast. */
  async function merge(whenGreen: boolean) {
    const p = ticket.pr;
    if (!p) return;
    const how = await app.run(api.ticketPrMerge(ticket.id, whenGreen));
    if (how) app.toast(mergeHowText(how, p), how === 'merged' ? 'ok' : 'info');
  }

  function approve() {
    const send = () => {
      // The test launches stop first (the backend does it too): their ends are no crashes.
      if (ticket.agentId) stopTests(ticket.agentId);
      return app.run(api.ticketApprove(ticket.id));
    };
    // A merge that removes the worktree takes the files of the agent open in the editor with it.
    const lost = s.action === 'merge' && s.cleanup && ticket.agentId ? buffers.unsavedIn(project.id, ticket.agentId) : 0;
    if (!lost) {
      send();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: t('board.card.approveTitle', { key: ticket.key }),
      body: t('board.card.approveBody') + lossNotice(lost),
      confirm: APPROVE_LABEL.merge,
      danger: true,
      onConfirm: send,
    };
  }

  async function reject(comment: string) {
    const sent = await app.run(api.ticketReject(ticket.id, comment).then(() => true));
    if (sent) rejecting = false;
  }
</script>

<div
  bind:this={card}
  class="card"
  class:attn={ticket.column === 'review'}
  class:past={ticket.column === 'done'}
  class:waiting
  role="button"
  tabindex="0"
  aria-label={`${ticket.key} ${ticket.title}`}
  aria-describedby={attached ? `${uid}-images` : undefined}
  onclick={open}
  onkeydown={key}
  oncontextmenu={contextMenu}
>
  <div class="top">
    {#if ticket.external}
      {@const svc = SERVICES[ticket.external.service]}
      <button
        class="ext"
        use:tip={t('board.card.openExternal', { key: ticket.external.key, service: svc.name })}
        aria-label={t('board.card.openExternal', { key: ticket.external.key, service: svc.name })}
        onclick={(e) => act(e, () => app.run(openUrl(ticket.external!.url)))}
        ><span class="svc" style:background={svc.color} style:color={svc.ink} aria-hidden="true">{svc.letter}</span><span
          >{ticket.external.key}</span
        ></button
      >
      {#if ticket.external.error}<button
          bind:this={warn}
          class="sync-err"
          aria-label={t('board.card.syncError', { service: svc.name, error: ticket.external.error })}
          aria-expanded={syncOpen}
          use:tip={ticket.external.error}
          onclick={(e) => act(e, () => (syncOpen = !syncOpen))}><Icon name="info" size={14} /></button
        >{/if}
    {/if}
    <span class="key">{ticket.key.split('@')[0]}</span>
    {#if isMine(ticket) && !ticket.shared?.gone}
      <!-- Shared with the team by this machine's user (ESC-190, choice 116): a discreet mark, its file in the tooltip. -->
      <span
        class="mark"
        role="img"
        aria-label={t('share.card.mark', { file: sharedFile(ticket) ?? '' })}
        use:tip={t('share.card.mark', { file: sharedFile(ticket) ?? '' })}><Icon name="team" size={14} /></span
      >
    {/if}
    {#if engine}<EngineMark {engine} />{/if}
    {#if attached}
      <!-- The images and the files attached in its form, which its agent gets with its first message. -->
      <span class="clip" id="{uid}-images" use:tip={said}
        ><Icon name="clip" size={13} /><span class="n" aria-hidden="true">{attached}</span><span class="sr">{said}</span></span
      >
    {/if}
    {#if ownAccount}<span
        class="own"
        role="img"
        aria-label={t('board.card.ownAccount', { account: ownAccount })}
        use:tip={t('board.card.ownAccount', { account: ownAccount })}>{ownAccount}</span
      >{/if}
    {#if own}<span class="own" role="img" aria-label={own.label} use:tip={own.label}>{own.text}</span>{/if}
    {#if theirs && ticket.shared?.author.name}
      <!-- Another one's ticket (choice 117): their name, a disc of a hue drawn from their id. -->
      <span class="who" aria-label={t('share.card.by', { name: authorName(ticket) })}
        ><i style:--h={hueOf(ticket.shared.author.id || ticket.shared.author.name)} aria-hidden="true"
          >{initialOf(ticket.shared.author.name)}</i
        ><span aria-hidden="true">{ticket.shared.author.name}</span></span
      >
    {/if}
    {#if ticket.column === 'doing'}
      <span class="loop"
        >{t('board.card.loop', { iteration: ticket.iteration, max: ticket.maxLoops })}<span class="loops" aria-hidden="true"
          >{#each { length: Math.min(ticket.maxLoops, 10) } as _, i (i)}<i class:on={i < ticket.iteration}></i>{/each}</span
        ></span
      >
    {:else if ticket.column === 'review'}
      <span class="sum" use:tip={used?.estimated ? estimateHint(used) : undefined}
        ><span class="sr">{t('board.card.costLabel')}{' '}</span><span>{reviewMeta(ticket, used)}</span></span
      >
    {:else if ticket.column === 'pr' && ticket.pr}
      <span class="sum">{t('prs.to', { base: ticket.pr.target })}</span>
    {:else if ticket.column === 'done'}
      <span class="sum">{doneMeta(ticket)}</span>
    {/if}
  </div>
  {#if ticket.external?.error && syncOpen}
    <div class="sync" bind:this={panel}>
      <span class="why">{ticket.external.error}</span>
      <div class="row">
        <!-- Not disabled while it runs: the focus stays on it (resync asks once). -->
        <button class="btn small" aria-disabled={resyncing} onclick={(e) => act(e, resync)}>{t('board.card.resync')}</button>
      </div>
    </div>
  {/if}
  <h2 class="title">{ticket.shared?.problem && !ticket.shared.author.name ? t('share.card.unreadable') : ticket.title}</h2>
  {#if ticket.shared?.problem}
    <!-- Its file is in conflict, or cannot be read (choice 125): the ticket keeps its last state, its file is left alone. -->
    <div class="clash" role="status">
      <Icon name="shield" size={14} /><span>{ticket.shared.problem}</span>
      {#if ticket.shared.conflict}
        <div class="row">
          <button class="btn small" onclick={(e) => act(e, () => app.run(api.sharedResolve(ticket.id, true)))}
            >{t('share.card.keepMine')}</button
          >
          <button class="btn small" onclick={(e) => act(e, () => app.run(api.sharedResolve(ticket.id, false)))}
            >{t('share.card.takeTheirs')}</button
          >
        </div>
      {/if}
    </div>
  {:else if other}
    <div class="clash" role="status"><Icon name="team" size={14} /><span>{t('share.card.alsoOn', { name: other })}</span></div>
  {:else if ticket.shared?.left}
    <div class="clash quiet" role="status">
      <Icon name="info" size={14} /><span>{t('share.card.left', { name: authorName(ticket) })}</span>
    </div>
  {:else if ticket.shared?.gone && ticket.shared.mine}
    <div class="clash quiet" role="status">
      <Icon name="info" size={14} /><span>{t('share.card.gone')}</span>
      <div class="row">
        <button class="btn small" onclick={(e) => act(e, () => shareTicket(ticket, project, true))}>{t('share.card.shareAgain')}</button>
        <button class="btn small" onclick={(e) => act(e, () => shareTicket(ticket, project, false))}>{t('share.menu.unshare')}</button>
      </div>
    </div>
  {:else if ticket.shared?.newer}
    <div class="clash quiet" role="status"><Icon name="info" size={14} /><span>{t('share.card.newer')}</span></div>
  {/if}
  {#if changedRepos.length}
    <span class="repos-line"
      ><Icon name="branch" size={14} /><span>{t('repos.card.changed', { repos: changedRepos.join(', ') })}</span></span
    >
  {/if}
  {#if ticket.pages?.length}
    <!-- The pages of the spec it implements (« Planifier », ESC-262): each opens in the Docs tab. -->
    <span class="spec-line" data-ticket-pages
      ><Icon name="file" size={14} /><span class="pages"
        >{#each ticket.pages as path, i (path)}{#if i}{', '}{/if}<button
            class="page"
            aria-label={t('docs.plan.openPage', { name: pageTitle(path, docsStore.tree[ticket.projectId]) })}
            onclick={(e) => {
              e.stopPropagation();
              app.openDocs(ticket.projectId, 'spec', path);
            }}
            onkeydown={(e) => e.stopPropagation()}>{pageTitle(path, docsStore.tree[ticket.projectId])}</button
          >{/each}</span
      ></span
    >
  {/if}
  {#if ticket.partial && (ticket.column === 'review' || ticket.column === 'done')}<span class="partial">{t('board.card.partial')}</span
    >{/if}

  {#if ticket.column === 'pr' && ticket.pr}
    <!-- « En revue » (ESC-59): its pull request as its host last told it; its number opens it in the browser. -->
    {@const p = ticket.pr}
    {@const pull = asPull(p)}
    {@const ci = ciLine(p)}
    {@const facts = prFacts(p)}
    {@const down = prErrorLine(p, app.now)}
    <div class="prline" data-ticket-pr={p.number}>
      <button
        class="prnum {prTone(pull)}"
        aria-label={t('prs.chipLabel', { word: p.word, number: p.number, state: prStateWord(p) })}
        onclick={(e) => act(e, () => app.run(openUrl(p.url)))}><Icon name={prIcon(pull)} size={12} />{p.word} #{p.number}</button
      >
      <span class="prstate {prStateKey(p)}">{prStateWord(p)}</span>
      {#if ci}<button
          class="prci {p.checks}"
          data-ticket-ci
          use:tip={t('board.ci.pillTitle', { branch: p.branch, state: ci })}
          onclick={(e) => act(e, () => (app.modal = { kind: 'ci', of: { kind: 'ticketPr', id: ticket.id }, projectId: project.id }))}
          >{ci}</button
        >{/if}
    </div>
    {#if facts.length}
      <!-- The conflict with its target is always the last, in red: what to fix first. -->
      <span class="prfacts"
        >{#each facts as f, i (f)}{#if i}{' · '}{/if}<span class:conflicting={p.mergeable === false && i === facts.length - 1}>{f}</span
          >{/each}</span
      >
    {/if}
    {#if down}<span class="lost" role="status" data-pr-error use:tip={{ text: down, cut: true }}>{down}</span>{/if}
    <!-- Its CI failed (ESC-60): its agent fixes it, by itself with the autopilot (within its loops), else on « L'agent corrige ». -->
    {@const noLog = noLogLine(p)}
    {#if noLog}<span class="prfacts" data-pr-nolog use:tip={{ text: noLog, cut: true }}>{noLog}</span>{/if}
    {#if p.fixing}
      <span class="fixing" role="status" data-pr-fixing><i class="dot"></i>{fixingLine(p)}</span>
    {:else if p.fixError}
      <span class="lost" role="status" data-pr-fix-error use:tip={{ text: p.fixError, cut: true }}>{p.fixError}</span>
    {/if}
    {#if canFixCi(ticket)}
      <div class="row">
        <button
          class="btn small"
          data-pr-fix
          use:tip={t('board.pr.fixCiTitle', { word: p.word })}
          onclick={(e) => act(e, () => app.run(api.ticketPrFixCi(ticket.id)))}>{t('board.pr.fixCi')}</button
        >
        {#if fixesSpent(ticket, s.autopilot)}<span class="prfacts"
            >{t('board.pr.loopsSpent', { count: p.fixes, max: ticket.maxLoops })}</span
          >{/if}
      </div>
    {/if}
    <!-- Its review's new comments (ESC-61): counted, and given to its agent on this click only, the autopilot's too. -->
    {#if canFixReviews(ticket)}
      <div class="row">
        <span class="prfacts" data-pr-feedback={p.feedback}>{t('board.pr.feedback', { count: p.feedback })}</span>
        <button
          class="btn small"
          data-pr-fix-reviews
          use:tip={t('board.pr.fixReviewsTitle')}
          onclick={(e) => act(e, () => app.run(api.ticketPrFixReviews(ticket.id)))}>{t('board.pr.fixReviews')}</button
        >
      </div>
    {/if}
    <!-- In conflict with its target (ESC-61): merged in (or rebased onto, by the board's settings), its conflicts to its agent. -->
    {#if canUpdateBranch(ticket)}
      <div class="row">
        <button
          class="btn small"
          data-pr-update-branch
          use:tip={t(`board.pr.updateBranchTitle.${s.prUpdate === 'rebase' ? 'rebase' : 'merge'}`, { target: p.target })}
          onclick={(e) => act(e, () => app.run(api.ticketPrUpdateBranch(ticket.id)))}>{t('board.pr.updateBranch')}</button
        >
      </div>
    {/if}
    <!-- Merging from Escouade (ESC-62): never by itself, a button. « Merger » approved and green, with a strategy the
         repository allows; « Merger quand c'est vert » while its CI runs (the host's auto-merge, else Escouade). -->
    {@const acts = mergeActions(p, s.strategy)}
    {@const waits = mergeWaitLine(p)}
    {#if waits}
      <span class="prwait" role="status" data-pr-wait>{waits}</span>
    {/if}
    {#if p.mergeNote}<span class="lost" role="status" data-pr-merge-note use:tip={{ text: p.mergeNote, cut: true }}>{p.mergeNote}</span
      >{/if}
    {#if acts.none}<span class="prfacts" data-pr-no-strategy>{t('board.pr.merge.none')}</span>{/if}
    {#if acts.merge || acts.whenGreen || p.mergeWhenGreen || p.mergeNote}
      <div class="row prmerge">
        {#if acts.merge}
          {@const strategy = t(`board.pr.merge.strategies.${acts.merge}`)}
          <button
            class="btn small primary"
            data-pr-merge
            use:tip={t('board.pr.merge.nowTitle', { word: p.word, number: p.number, target: p.target, strategy })}
            onclick={(e) => act(e, () => merge(false))}><Icon name="merge" size={12} />{t('board.pr.merge.now', { strategy })}</button
          >
        {/if}
        {#if acts.whenGreen}
          <button
            class="btn small"
            data-pr-merge-green
            use:tip={t('board.pr.merge.whenGreenTitle')}
            onclick={(e) => act(e, () => merge(true))}>{t('board.pr.merge.whenGreen')}</button
          >
        {/if}
        {#if p.mergeWhenGreen || p.mergeNote}
          <button class="btn small" data-pr-stop-waiting onclick={(e) => act(e, () => app.run(api.ticketPrStopWaiting(ticket.id)))}
            >{p.mergeNote ? t('common.close') : t('board.pr.merge.stopWaiting')}</button
          >
        {/if}
      </div>
    {/if}
    {#if p.preview && !ticket.blocked}
      <!-- Its preview (ESC-67): Vercel, Netlify, Cloudflare Pages… put it on the pull request. -->
      {@const preview = p.preview}
      <div class="row">
        <button
          class="btn small"
          data-ticket-preview
          use:tip={t('envs.card.previewTitle', { word: p.word, url: preview })}
          onclick={(e) => act(e, () => app.run(openUrl(preview)))}><Icon name="ext" size={14} />{t('envs.card.preview')}</button
        >
      </div>
    {/if}
  {/if}

  {#if ticket.column === 'done' && outcome}
    {#if ticket.outcomeUrl}
      <button class="outcome link" onclick={(e) => act(e, () => app.run(openUrl(ticket.outcomeUrl!)))}
        >{#if outcome.icon}<Icon name={outcome.icon} size={14} />{/if}{outcome.text}</button
      >
    {:else}
      <span class="outcome"
        >{#if outcome.icon}<Icon name={outcome.icon} size={14} />{/if}{outcome.text}</span
      >
    {/if}
    {#if pushed}
      <!-- Pushed without a pull request (ESC-63): its branch's CI, its jobs on a click. -->
      <div class="cirow" data-ticket-pushed-ci><CiPill ci={pushed} of={{ kind: 'pushed', id: ticket.id }} projectId={project.id} /></div>
    {/if}
  {/if}

  {#if cardEnvironments(ticket).length}
    <!-- Where it is deployed (ESC-67): « staging ✓ · production ⏳ » drawn with icons, each name opens its environment's address. -->
    <div class="dep-envs" data-ticket-envs aria-label={t('envs.card.label')}>
      {#each cardEnvironments(ticket) as e, i (e.name)}
        {@const label = t(e.deployed ? 'envs.card.deployed' : 'envs.card.waiting', { name: e.name })}
        {#if i}<span class="dep-sep" aria-hidden="true">{' · '}</span>{/if}
        {#if e.url}
          {@const url = e.url}
          <button
            class="dep-env"
            class:deployed={e.deployed}
            aria-label={label}
            use:tip={`${label} · ${url}`}
            onclick={(ev) => act(ev, () => app.run(openUrl(url)))}>{e.name}<Icon name={e.deployed ? 'check' : 'clock'} size={12} /></button
          >
        {:else}
          <span class="dep-env" class:deployed={e.deployed} aria-label={label} use:tip={label}
            >{e.name}<Icon name={e.deployed ? 'check' : 'clock'} size={12} /></span
          >
        {/if}
      {/each}
    </div>
  {/if}

  {#if ticket.column !== 'done' && ticket.column !== 'pr' && ticket.criteria.length}
    <ul class="crit" aria-label={t('board.card.criteriaLabel')}>
      {#each ticket.criteria as c, i (i)}
        <li class:met={c.ok} use:tip={c.note || undefined}>
          <Icon name={c.ok ? 'check-c' : 'circle'} size={14} /><span
            >{c.text}{#if c.ok}<span class="sr">{' '}{t('board.card.met')}</span>{/if}</span
          >
        </li>
      {/each}
    </ul>
  {/if}

  {#snippet stepList()}
    <ul class="steps" aria-label={t('board.card.progressLabel')}>
      {#each steps as p, i (i)}<li>{p}</li>{/each}
    </ul>
  {/snippet}

  {#if ticket.column === 'review' && steps.length}
    <div class="prog">
      <span class="h">{t('board.card.doneHeading')}</span>
      {@render stepList()}
    </div>
  {/if}

  {#if ticket.column === 'doing'}
    {#if steps.length}
      <div class="prog">
        {@render stepList()}
        {#if hidden > 0}<span class="more" use:tip={t('board.card.moreSteps', { count: hidden })}>+{hidden}</span>{/if}
      </div>
    {/if}
    {#if agent?.progressLine}
      <!-- What the agent reported through Escouade's MCP server, until its next report. -->
      <div class="said" use:tip={{ text: agent.progressLine, cut: true }}>
        <span class="sr">{t('mcp.progress.label')}{' '}</span>{agent.progressLine}
      </div>
    {/if}
  {/if}

  {#if ticket.accountLost && ticket.column !== 'done'}
    <!-- Its account removed, switched off or signed out: back on « Auto », until it is saved again. -->
    <span class="lost" role="status">{ticket.accountLost}</span>
  {/if}

  {#if ticket.blocked}
    <div class="blocked" class:conflict={ticket.conflict}>
      <span class="why">{ticket.blocked}</span>
      {#if ticket.column === 'doing'}
        <div class="row">
          <button class="btn small" onclick={(e) => act(e, () => app.run(api.ticketResume(ticket.id)))}>{t('board.card.resume')}</button>
        </div>
      {:else if ticket.column === 'pr' && ticket.pr?.closed}
        <!-- Its pull request closed without a merge (ESC-59): back to do (its branch taken up), or done all the same. -->
        <div class="row">
          <button class="btn small" onclick={(e) => act(e, () => app.run(api.ticketPrBackToTodo(ticket.id)))}
            >{t('board.pr.backToTodo')}</button
          >
          <button class="btn small" onclick={(e) => act(e, () => app.run(api.ticketFinishAnyway(ticket.id)))}
            >{t('board.pr.finishAnyway')}</button
          >
        </div>
      {:else if ticket.conflict}
        <div class="row">
          <button class="btn small" onclick={(e) => act(e, () => app.run(api.ticketResolveConflict(ticket.id)))}
            >{t('board.card.agentResolves')}</button
          >
          <button class="btn small" onclick={(e) => act(e, () => app.run(api.ticketDismiss(ticket.id)))}>{t('common.cancel')}</button>
        </div>
      {/if}
    </div>
  {/if}

  <!-- The card's last line: why a ticket to do waits, or what the agent of one under way does and what it cost. -->
  {#if ticket.column === 'todo'}
    {@const why = waitLabel(ticket, queueIndex, s, busyCount, issue, pause, awaited, redTarget)}
    <div class="act" class:theirs={theirs && !ticket.forced}>
      <Icon name={theirs && !ticket.forced ? 'team' : awaited.length && !ticket.forced ? 'link' : 'clock'} size={14} /><span
        class="grow"
        use:tip={{ text: why, cut: true }}>{why}</span
      >
      {#if canStart(ticket, s, busyCount, quota, issue, pause, redTarget)}
        <button class="btn small" onclick={(e) => act(e, launch)}><Icon name="play" size={14} />{t('board.card.launch')}</button>
      {/if}
    </div>
  {:else if theirLine}
    <!-- Another one's ticket, under way, to test or done at theirs: who has it (choice 117). -->
    <div class="act theirs"><Icon name="team" size={14} /><span class="grow">{theirLine}</span></div>
  {:else if ticket.column === 'doing' && !ticket.blocked && (waiting || agent?.resumeAt || agent?.setup || activity || used)}
    <div class="act" class:wait={waiting}>
      {#if waiting}
        <i class="dot"></i><span class="grow">{t('board.card.questionWaiting')}</span>
      {:else if agent?.resumeAt}
        <!-- Set on its account, it waits for that one's reset, never going elsewhere (ESC-153). -->
        {@const when = ownAccount
          ? t('board.card.waitsAccount', { account: ownAccount, at: fTime(agent.resumeAt) })
          : t('board.card.resumes', { when: fWhen(agent.resumeAt, app.now) })}
        <Icon name="clock" size={14} /><span class="grow" use:tip={{ text: when, cut: true }}>{when}</span>
      {:else if agent?.setup}
        <i class="dot"></i><span class="grow" use:tip={agent.setup}>{t('board.card.settingUp', { step: agent.setup })}</span>
      {:else if activity?.kind === 'doing'}
        <i class="dot"></i><span class="grow"
          ><b>{activity.verb}</b>{#if activity.what}{' '}<span class="mono">{activity.what}</span>{/if}</span
        >
      {:else if activity?.kind === 'thinking'}
        <i class="dot"></i><span class="grow">{t('board.card.thinking')}</span>
      {:else}
        <!-- Between two turns: its cost alone, at the right. -->
        <span class="grow"></span>
      {/if}
      {#if used}
        <span class="num cost" use:tip={used.estimated ? estimateHint(used) : undefined}
          ><span class="sr">{t('board.card.costLabel')}{' '}</span>{fSpentUsd(used)}</span
        >
      {/if}
    </div>
  {/if}

  {#if ticket.step}
    <div class="act"><i class="dot"></i><span class="grow" use:tip={{ text: ticket.step, cut: true }}>{ticket.step}</span></div>
  {/if}

  {#if (agent?.recipe || agent?.isola) && (ticket.column === 'doing' || ticket.column === 'review') && anyRunning(agent.id)}
    {@const a = agent}
    {@const address = flows.all[a.id]?.opened ?? openAddress(a)}
    <div class="row">
      <button class="btn small" onclick={(e) => act(e, () => stopTests(a.id))}
        ><Icon name="stop" size={14} />{t('board.card.stopTests')}</button
      >
      {#if address}<button class="btn small" onclick={(e) => act(e, () => app.run(openUrl(address)))}
          ><Icon name="ext" size={14} />{t('common.open')}</button
        >{/if}
    </div>
  {/if}

  {#if ticket.column === 'review' && !ticket.step && !ticket.conflict && !theirLine}
    {#if rejecting}
      <RejectForm onsubmit={reject} oncancel={() => (rejecting = false)} />
    {:else}
      <!-- « Valider » draws what validating does, said whole in its name; « Tester » once its launch is ready. -->
      <div class="row">
        {#if ticket.blocked}
          <button class="btn primary small" onclick={(e) => act(e, approve)}><Icon name="restart" size={14} />{t('common.retry')}</button>
        {:else}
          <button
            class="btn primary small"
            aria-label={APPROVE_LABEL[s.action]}
            use:tip={APPROVE_LABEL[s.action]}
            onclick={(e) => act(e, approve)}
            ><Icon name={APPROVE_ICON[s.action] ?? 'check'} size={14} />{t('boardSettings.approve.keep')}</button
          >
        {/if}
        {#if (agent?.recipe || agent?.isola) && !anyRunning(agent.id)}
          {@const a = agent}
          <button class="btn small" onclick={(e) => act(e, () => testAgent(a, project))}
            ><Icon name="play" size={14} />{t('board.card.test')}</button
          >
        {/if}
        <button class="btn ghost small" onclick={(e) => act(e, () => (rejecting = true))}>{t('board.card.sendBack')}</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  /* A card on the board (2.0): raised, its key and loop, its title, its criteria, what its agent does and costs. */
  .card {
    flex: none;
    display: grid;
    gap: 9px;
    min-width: 0;
    /* Its top line gives way by its own width (the column's), never the window's: see the end. */
    container: tcard / inline-size;
    padding: 12px 12px 11px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line),
      0 1px 2px rgba(0, 0, 0, calc(0.3 * var(--shade)));
    text-align: left;
    cursor: pointer;
    outline: none;
    transition: box-shadow 0.15s;
  }
  .card:hover {
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 4px 12px -4px rgba(0, 0, 0, calc(0.5 * var(--shade)));
  }
  .card:focus-visible {
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--p) 60%, transparent),
      0 0 0 3px color-mix(in oklch, var(--p) 14%, transparent);
  }
  /* To test, or its agent waiting for an answer: amber, the card to act on. */
  .card.attn,
  .card.waiting {
    background: color-mix(in oklch, var(--wait) 5%, var(--raise));
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .card.past {
    background: transparent;
  }
  .top,
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .top {
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .row {
    flex-wrap: wrap;
    gap: 6px;
  }
  .key {
    flex: none;
    white-space: nowrap;
  }
  .clip {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--muted);
    font-size: 11.5px;
  }
  .ext {
    flex: 0 1 auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }
  .ext:hover {
    color: var(--text);
  }
  .ext > span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .svc {
    min-width: 15px;
    height: 15px;
    padding: 0 2px;
    flex: none;
    border-radius: 4px;
    font-size: 8.5px;
    font-weight: 800;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .sync-err {
    flex: none;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--wait);
    cursor: pointer;
  }
  .sync-err[aria-expanded='true'] {
    background: var(--wait-bg);
  }
  .sync {
    display: grid;
    gap: 6px;
    padding: 7px 9px;
    border-radius: 7px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12px;
  }
  .sync .why {
    overflow-wrap: anywhere;
  }
  /* Its own account, model and effort, in a hairline as the harness's brand; cut before they push the loop out. */
  .own {
    min-width: 0;
    height: 16px;
    padding: 0 5px;
    border-radius: 4px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--soft);
    font-size: 10.5px;
    font-weight: 560;
    line-height: 16px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The loop and its squares, or the loops and the cost of a ticket to test or done: at the right of the line. */
  .loop,
  .sum {
    flex: none;
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    white-space: nowrap;
  }
  .loop,
  .sum {
    font-weight: 550;
  }
  .loop {
    color: var(--p-ink);
  }
  .loops {
    display: inline-flex;
    gap: 2px;
  }
  .loops i {
    width: 7px;
    height: 3px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.12);
  }
  .loops i.on {
    background: var(--p-ink);
  }
  .title {
    margin: 0;
    font-size: 13.5px;
    font-weight: 590;
    letter-spacing: -0.01em;
    line-height: 1.35;
    text-wrap: pretty;
    overflow-wrap: anywhere;
  }
  .past .title {
    color: var(--soft);
    font-weight: 550;
  }
  .partial {
    justify-self: start;
    height: 18px;
    padding: 0 7px;
    border-radius: 9px;
    background: oklch(from var(--wait) l c h / 0.1);
    color: var(--wait);
    font-size: 11.5px;
    font-weight: 550;
    line-height: 18px;
  }
  .repos-line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .repos-line span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spec-line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  .spec-line .pages {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spec-line .page {
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    text-decoration: underline;
    text-decoration-color: var(--line);
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .spec-line .page:hover {
    color: var(--text);
  }
  .outcome {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    text-align: left;
  }
  .outcome > :global(.icon) {
    color: var(--add);
  }
  .outcome.link {
    cursor: pointer;
  }
  .outcome.link:hover {
    color: var(--text);
  }
  /* The environments a done ticket is in (ESC-67): green once deployed, amber while it waits. */
  .dep-envs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    row-gap: 2px;
    min-width: 0;
    font-size: 12px;
  }
  .dep-sep {
    color: var(--muted);
    white-space: pre;
  }
  .dep-env {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 0;
    border: none;
    background: none;
    color: var(--wait);
    font: inherit;
    white-space: nowrap;
  }
  .dep-env.deployed {
    color: var(--add);
  }
  button.dep-env {
    cursor: pointer;
  }
  button.dep-env:hover {
    text-decoration: underline;
  }
  .crit {
    display: grid;
    gap: 5px;
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.4;
  }
  .crit li {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr);
    gap: 8px;
    overflow-wrap: anywhere;
  }
  .crit li > :global(.icon) {
    margin-top: 1px;
    color: var(--idle);
  }
  .crit li.met {
    color: var(--muted);
  }
  .crit li.met > :global(.icon) {
    color: var(--add);
  }
  .prog {
    display: grid;
    gap: 4px;
  }
  .h {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
  }
  .steps {
    display: grid;
    gap: 3px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .steps li {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr);
    gap: 8px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  /* A small dot in the criteria's column, drawn. */
  .steps li::before {
    content: '';
    justify-self: center;
    width: 3px;
    height: 3px;
    margin-top: 7px;
    border-radius: 50%;
    background: var(--idle);
  }
  .more {
    justify-self: start;
    color: var(--dim);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .said {
    color: var(--soft);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .lost {
    color: var(--wait);
    font-size: 12px;
  }
  /* « En revue » (ESC-59): its pull request's number as the sidebar draws an agent's, then its state and its CI. */
  .prline {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 8px;
    min-width: 0;
    font-size: 12px;
  }
  .prnum {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 6px;
    border: none;
    border-radius: 5px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font: 11.5px var(--mono);
    white-space: nowrap;
    cursor: pointer;
  }
  .prnum:hover {
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--line-3, var(--line-2));
  }
  .prnum.ok :global(.icon) {
    color: var(--run);
  }
  .prnum.fail :global(.icon) {
    color: var(--del);
  }
  .prnum.wait :global(.icon) {
    color: var(--wait);
  }
  .prnum.merged :global(.icon) {
    color: var(--syn-k);
  }
  .prnum.dim {
    color: var(--muted);
  }
  .prstate {
    color: var(--soft);
  }
  .prstate.approved {
    color: var(--run);
  }
  .prstate.changes {
    color: var(--wait);
  }
  .prstate.draft,
  .prstate.closed {
    color: var(--muted);
  }
  .prci {
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }
  .prci:hover {
    text-decoration: underline;
  }
  .cirow {
    display: flex;
    min-width: 0;
  }
  .prci.success {
    color: var(--run);
  }
  .prci.failure {
    color: var(--del);
  }
  .prci.pending {
    color: var(--wait);
  }
  .prwait {
    color: var(--muted);
    font-size: 12px;
  }
  .prmerge {
    margin-top: 2px;
  }
  .prfacts {
    color: var(--muted);
    font-size: 12px;
  }
  .prfacts .conflicting {
    color: var(--del);
  }
  .fixing {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--run);
    font-size: 12px;
  }
  /* The last line, under a hairline: a dot of the agent's state, what it does, its cost. */
  .act {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding-top: 9px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 12px;
  }
  .act > :global(.icon) {
    flex: none;
  }
  .act .grow {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .act b {
    color: var(--soft);
    font-weight: 550;
  }
  .act .mono {
    font-size: 0.92em;
  }
  .dot {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--run);
  }
  .act.wait {
    color: var(--wait);
  }
  .act.wait .dot {
    background: var(--wait);
  }
  .cost {
    flex: none;
    white-space: nowrap;
  }
  .act .btn {
    flex: none;
  }
  /* Another one's ticket (ESC-190): its line wraps rather than cuts, the button stays at the end. */
  .act.theirs {
    align-items: flex-start;
    line-height: 1.4;
  }
  .act.theirs .grow {
    white-space: normal;
  }
  .act.theirs > :global(.icon) {
    margin-top: 1px;
  }
  .mark {
    flex: none;
    display: inline-flex;
    color: var(--p-ink);
  }
  .who {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    margin-left: auto;
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .who + :global(*) {
    margin-left: 0;
  }
  .who > span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .who > i {
    flex: none;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    /* The theme's own: dark ink on a pale disc in the light one. */
    background: light-dark(oklch(0.9 0.05 var(--h)), oklch(0.42 0.08 var(--h)));
    color: light-dark(oklch(0.36 0.1 var(--h)), oklch(0.97 0.02 var(--h)));
    font: normal 600 9.5px/1 var(--font);
  }
  /* What a shared file asks of the user (ESC-190): a conflict, someone else on it, a ticket no longer shared. */
  .clash {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr);
    gap: 6px 8px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .clash > :global(.icon) {
    margin-top: 1px;
    color: var(--wait);
  }
  .clash .row {
    grid-column: 1 / -1;
    flex-wrap: wrap;
  }
  .clash.quiet {
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .clash.quiet > :global(.icon) {
    color: var(--muted);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .blocked {
    display: grid;
    gap: 6px;
    padding: 7px 9px;
    border-radius: 7px;
    background: oklch(from var(--del) l c h / 0.08);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.32);
    color: var(--del);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .blocked.conflict {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--wait);
  }
  /* The card's buttons, 28 pixels high as the preview's « Valider » and « Tester ». */
  .btn.small {
    height: 28px;
    font-size: 12.5px;
  }
  .btn[aria-disabled='true'] {
    opacity: 0.5;
    cursor: default;
  }
  /* A narrow card (a column of 220 px at 1028 px) with its harness, its attachments and its loop: the squares of the
     loop go (« Boucle 1/5 » says it), then an account's chip shortens; the key never wraps nor shrinks. */
  @container tcard (max-width: 270px) {
    .loops {
      display: none;
    }
  }
</style>
