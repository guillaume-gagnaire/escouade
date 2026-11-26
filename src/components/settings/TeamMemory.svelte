<script lang="ts">
  import { fDate, fDateTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import {
    author,
    dayStart,
    short,
    teamMemoryApi,
    type QuarantinePreview,
    type QuarantineRule,
    type TeamMemoryEntry,
    type TeamMemoryView,
  } from '../../lib/team-memory';
  import { tip } from '../../lib/tip';
  import type { Project } from '../../lib/types';

  // « Mémoire partagée » of a project whose memory is on the team server (TS-10, docs/TEAM-SERVER.md §8): what this
  // member's agents read, each memory with its provenance; the proposals of the agents to keep or set aside (C14-B, the
  // review on by default, imposable by the team); what was withdrawn, kept in the journal; the quarantine of an
  // administrator, after a preview of what goes. Read again at each change the backend tells.
  let { project, view = $bindable() }: { project: Project; view: TeamMemoryView } = $props();

  let busy = $state(false);
  let armed = $state<string | null>(null);
  let ticket = $state('');
  let who = $state('');
  let since = $state('');
  let preview = $state<QuarantinePreview | null>(null);

  function how(e: TeamMemoryEntry): string {
    const a = author(e.provenance);
    const by = a.agent ? t('memory.team.byAgent', { agent: a.agent }) : t('memory.team.byHand');
    return t('memory.team.by', { who: a.who, how: by, date: fDateTime(e.provenance.at) });
  }

  async function act(f: () => Promise<TeamMemoryView>, said?: string) {
    busy = true;
    try {
      const v = await app.run(f());
      if (v) {
        view = v;
        if (said) app.toast(said);
      }
    } finally {
      busy = false;
    }
  }

  const decide = (e: TeamMemoryEntry, keep: boolean) =>
    act(() => teamMemoryApi.decide(project.id, e.id, keep), t(keep ? 'memory.team.kept' : 'memory.team.rejected', { name: e.name }));

  /** « Retirer » or « Garder » a memory whose removal a tool proposed (ESC-647). */
  const settleRemoval = (e: TeamMemoryEntry, remove: boolean) =>
    act(
      () => teamMemoryApi.decideRemoval(project.id, e.id, remove),
      t(remove ? 'memory.team.removedAll' : 'memory.team.removalKept', { name: e.name }),
    );

  /** Who asked a removal: the agent (or the outside client) and its member, when. */
  function askedBy(e: TeamMemoryEntry): string {
    const r = e.removal;
    if (!r) return '';
    const who = r.agent ? `${r.agent.name}${r.agent.ticket ? ` (${r.agent.ticket})` : ''} · ${r.name}` : r.name;
    return t('memory.team.removalBy', { who, date: fDateTime(r.at) });
  }

  async function forget(name: string) {
    if (armed !== name) {
      armed = name;
      return;
    }
    busy = true;
    const done = await app.run(api.memoryForget(project.id, name).then(() => true));
    busy = false;
    armed = null;
    if (!done) return;
    app.toast(t('memory.settings.forgotten', { name }));
    const v = await app.run(teamMemoryApi.view(project.id));
    if (v) view = v;
  }

  async function setReview(on: boolean) {
    busy = true;
    try {
      await app.run(teamMemoryApi.setReview(project.id, on));
      const v = await app.run(teamMemoryApi.view(project.id));
      if (v) view = v;
    } finally {
      busy = false;
    }
  }

  const rule = $derived.by((): QuarantineRule => {
    const author = view.authors.find((a) => a.device === who);
    return {
      ticket: ticket || null,
      member: author ? author.member : null,
      device: author ? author.device : null,
      since: dayStart(since),
    };
  });
  const ruleSet = $derived(!!(rule.ticket || rule.device || rule.since != null));

  /** What the rule is, as a key: the preview read before goes when it changes, not when the view is read again. */
  const ruleKey = $derived(JSON.stringify(rule));
  let previewOf = '';
  $effect(() => {
    if (ruleKey !== previewOf) preview = null;
  });

  async function look(apply: boolean) {
    busy = true;
    try {
      const p = await app.run(teamMemoryApi.quarantine(project.id, rule, apply));
      if (!p) return;
      if (apply) {
        app.toast(t('memory.team.applied'));
        ticket = '';
        who = '';
        since = '';
        const v = await app.run(teamMemoryApi.view(project.id));
        if (v) view = v;
        preview = null;
      } else {
        previewOf = ruleKey;
        preview = p;
      }
    } finally {
      busy = false;
    }
  }

  function ruleWhat(r: QuarantineRule): string {
    const parts: string[] = [];
    if (r.ticket) parts.push(t('memory.team.ruleTicket', { ticket: r.ticket }));
    if (r.device || r.member) {
      const a = view.authors.find((x) => x.device === r.device) ?? view.authors.find((x) => x.member === r.member);
      parts.push(t('memory.team.ruleAuthor', { who: a?.name || short(r.device ?? r.member ?? '') }));
    }
    if (r.since != null) parts.push(t('memory.team.ruleSince', { date: fDate(r.since) }));
    return parts.join(', ');
  }

  const lift = (id: string) => act(() => teamMemoryApi.lift(project.id, id), t('memory.team.lifted'));
</script>

<div class="team-memory" data-testid="team-memory">
  <label class="review">
    <input
      type="checkbox"
      checked={view.review}
      disabled={busy || !!view.reviewLockedBy}
      onchange={(e) => setReview(e.currentTarget.checked)}
      data-testid="team-memory-review"
    />
    <span>{t('memory.team.review')}</span>
  </label>
  <p class="note">
    {view.review ? t('memory.team.reviewNote') : t('memory.team.reviewOff')}
    {#if view.reviewLockedBy}<span class="locked">{t('memory.team.reviewLocked', { team: view.reviewLockedBy })}</span>{/if}
  </p>

  {#if view.refused}<p class="note warn" role="alert">{t('memory.team.refused', { count: view.refused })}</p>{/if}

  {#if view.proposals.length}
    <section class="proposals" aria-label={t('memory.team.proposals', { count: view.proposals.length })}>
      <h4>{t('memory.team.proposals', { count: view.proposals.length })}</h4>
      <p class="note">{t('memory.team.proposalsNote')}</p>
      <ul class="list">
        {#each view.proposals as e (e.id)}
          <li data-testid="team-memory-proposal" data-name={e.name}>
            <details>
              <summary>
                <span class="name mono">{e.name}</span>
                <span class="pill data" use:tip={t('memory.team.untrustedTitle')}>{t('memory.team.proposed')}</span>
                {#if e.provenance.thirdParty}<span class="pill warn">{t('memory.team.thirdParty')}</span>{/if}
                <span class="desc">{e.description}</span>
                <span class="who">{how(e)}</span>
              </summary>
              <pre class="body">{e.body}</pre>
            </details>
            {#if e.unknown.length}<p class="note warn" data-testid="team-memory-unknown">
                {t('memory.team.unknown', { hosts: e.unknown.join(', ') })}
              </p>{/if}
            <div class="acts">
              <button class="btn small primary" disabled={busy} onclick={() => decide(e, true)} data-testid="team-memory-keep"
                >{t('memory.team.keep')}</button
              >
              <button class="btn small ghost" disabled={busy} onclick={() => decide(e, false)} data-testid="team-memory-reject"
                >{t('memory.team.reject')}</button
              >
            </div>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if view.removals?.length}
    <section class="proposals removals" aria-label={t('memory.team.removals', { count: view.removals.length })}>
      <h4>{t('memory.team.removals', { count: view.removals.length })}</h4>
      <p class="note">{t('memory.team.removalsNote')}</p>
      <ul class="list">
        {#each view.removals as e (e.id)}
          <li data-testid="team-memory-removal" data-name={e.name}>
            <details>
              <summary>
                <span class="name mono">{e.name}</span>
                <span class="desc">{e.description}</span>
                <span class="who">{askedBy(e)}</span>
              </summary>
              <pre class="body">{e.body}</pre>
            </details>
            <div class="acts">
              <button class="btn small" disabled={busy} onclick={() => settleRemoval(e, true)} data-testid="team-memory-remove"
                >{t('memory.team.remove')}</button
              >
              <button
                class="btn small primary"
                disabled={busy}
                onclick={() => settleRemoval(e, false)}
                data-testid="team-memory-keep-removal">{t('memory.team.keepIt')}</button
              >
            </div>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if !view.memories.length}
    <p class="note">{t('memory.settings.empty')}</p>
  {:else}
    <ul class="list" aria-label={t('memory.settings.title')}>
      {#each view.memories as e (e.id)}
        <li data-testid="team-memory-item" data-name={e.name} data-standing={e.standing}>
          <details>
            <summary>
              <span class="name mono">{e.name}</span>
              <span class="kind">{t(`memory.kinds.${e.kind}`)}</span>
              {#if e.standing === 'proposed'}<span class="pill data">{t('memory.team.proposed')}</span>{/if}
              {#if e.untrusted}<span class="pill data" use:tip={t('memory.team.untrustedTitle')}>{t('memory.team.untrusted')}</span>{/if}
              {#if e.provenance.imported}<span class="pill">{t('memory.team.imported')}</span>{/if}
              <span class="desc">{e.description}</span>
              <span class="who">{how(e)}{e.provenance.thirdParty ? ` · ${t('memory.team.thirdParty')}` : ''}</span>
            </summary>
            <pre class="body">{e.body}</pre>
          </details>
          <button
            class="btn ghost small"
            class:danger={armed === e.name}
            aria-label={armed === e.name ? t('memory.settings.confirm') : t('memory.settings.forgetLabel', { name: e.name })}
            disabled={busy}
            onclick={() => forget(e.name)}
            onblur={() => armed === e.name && (armed = null)}
            >{armed === e.name ? t('memory.settings.confirm') : t('memory.settings.forget')}</button
          >
        </li>
      {/each}
    </ul>
  {/if}

  {#if view.withdrawn.length}
    <details class="withdrawn" data-testid="team-memory-withdrawn">
      <summary>{t('memory.team.withdrawn', { count: view.withdrawn.length })}</summary>
      <ul class="list">
        {#each view.withdrawn as e (e.id)}
          <li class="past">
            <span class="name mono">{e.name}</span>
            <span class="pill"
              >{e.quarantined
                ? t('memory.team.quarantined')
                : e.standing === 'rejected'
                  ? t('memory.team.rejectedState')
                  : t('memory.team.removed')}</span
            >
            <span class="who">{how(e)}</span>
          </li>
        {/each}
      </ul>
    </details>
  {/if}

  <section class="quarantine" aria-label={t('memory.team.quarantine')}>
    <h4>{t('memory.team.quarantine')}</h4>
    <p class="note">{t('memory.team.quarantineNote')}</p>
    {#if !view.admin}
      <p class="note">{t('memory.team.quarantineAdminOnly')}</p>
    {:else}
      <div class="form">
        <label>
          <span>{t('memory.team.ofTicket')}</span>
          <select class="field" bind:value={ticket} data-testid="team-memory-q-ticket">
            <option value="">{t('memory.team.anyTicket')}</option>
            {#each view.tickets as k (k)}<option value={k}>{k}</option>{/each}
          </select>
        </label>
        <label>
          <span>{t('memory.team.ofAuthor')}</span>
          <select class="field" bind:value={who} data-testid="team-memory-q-author">
            <option value="">{t('memory.team.anyAuthor')}</option>
            {#each view.authors as a (a.device)}<option value={a.device}>{a.name || short(a.member)} · {short(a.device)}</option>{/each}
          </select>
        </label>
        <label>
          <span>{t('memory.team.since')}</span>
          <input class="field" type="date" bind:value={since} data-testid="team-memory-q-since" />
        </label>
        <button class="btn small" disabled={busy || !ruleSet} onclick={() => look(false)} data-testid="team-memory-q-preview"
          >{t('memory.team.preview')}</button
        >
      </div>
      {#if preview}
        <div class="preview" data-testid="team-memory-q-list">
          <p class="note">
            {preview.entries.length ? t('memory.team.previewCount', { count: preview.entries.length }) : t('memory.team.previewNone')}
          </p>
          <ul class="list">
            {#each preview.entries as e (e.id)}<li class="past">
                <span class="name mono">{e.name}</span> <span class="who">{how(e)}</span>
              </li>{/each}
          </ul>
          <button class="btn small danger" disabled={busy} onclick={() => look(true)} data-testid="team-memory-q-apply"
            >{t('memory.team.apply')}</button
          >
        </div>
      {/if}
    {/if}
    {#if view.rules.length}
      <h5>{t('memory.team.rules')}</h5>
      <ul class="list">
        {#each view.rules as r (r.id)}
          <li class="past" data-testid="team-memory-rule">
            <span
              >{t('memory.team.rule', {
                what: ruleWhat(r),
                by: r.byName || short(r.by),
                date: fDate(r.at),
                count: t('memory.settings.count', { count: r.count }),
              })}</span
            >
            {#if view.admin}<button class="btn ghost small" disabled={busy} onclick={() => lift(r.id)}>{t('memory.team.lift')}</button>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .team-memory {
    display: grid;
    gap: 8px;
    margin: 4px 0 12px;
    min-width: 0;
  }
  .note {
    max-width: 68ch;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .note.warn {
    color: var(--warn, var(--del));
  }
  .locked {
    margin-left: 4px;
    color: var(--soft);
  }
  .review {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
  }
  h4,
  h5 {
    margin: 6px 0 0;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
  }
  h5 {
    font-size: 12px;
    color: var(--soft);
  }
  .list {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .list > li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: start;
    gap: 6px 10px;
    padding: 7px 0;
    border-top: 1px solid var(--line);
  }
  .list > li:first-child {
    border-top: none;
  }
  .list > li.past {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 8px;
    color: var(--muted);
    font-size: 12px;
  }
  .proposals .list > li {
    grid-template-columns: minmax(0, 1fr);
  }
  summary {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 8px;
    cursor: pointer;
    list-style: none;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  .name {
    color: var(--text);
    font-size: 12.5px;
    font-weight: 550;
  }
  .kind,
  .pill {
    padding: 1px 6px;
    border-radius: 999px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--muted);
    font-size: 11px;
  }
  .pill.data {
    background: rgba(var(--fg-rgb), 0.1);
    color: var(--soft);
  }
  .pill.warn {
    color: var(--warn, var(--del));
  }
  .desc,
  .who {
    flex-basis: 100%;
    color: var(--soft);
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .who {
    color: var(--muted);
    font-size: 11.5px;
  }
  .body {
    max-height: 240px;
    margin: 8px 0 0;
    padding: 9px 11px;
    overflow: auto;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.035);
    color: var(--soft);
    font: 12px/1.5 var(--mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    gap: 6px;
  }
  .withdrawn summary {
    color: var(--muted);
    font-size: 12px;
  }
  .form {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 8px 12px;
  }
  .form label {
    display: grid;
    gap: 3px;
    color: var(--muted);
    font-size: 11.5px;
  }
  .preview {
    display: grid;
    gap: 6px;
    justify-items: start;
  }
  .danger {
    color: var(--del);
  }
</style>
