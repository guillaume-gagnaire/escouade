<script lang="ts">
  // A request of the box, unfolded on the spot (MO-5, `docs/MOBILE.md` § 3.6, `docs/INBOX.md`): its text, then by its
  // sort: a choice (its renders in a strip, the agent's seal, the viewer « En grand », its options and « Choisir B »;
  // no lock: the agent acts with its own safeguards), a go-ahead (« Oui » under the lock, « Non » without), an answer
  // (its field, the text proposed, the dictation), a task (its steps, « C'est fait »). Its files; « Plus tard »,
  // « Un mot », leaving it to the squad lead (never a go-ahead). Answered: what was said, and « Revenir sur ma réponse »
  // while its agent has not read it. Put off: until when, and « La remettre maintenant ».
  import { untrack } from 'svelte';
  import Icon from '$shared/components/Icon.svelte';
  import { fLater, imagesOf, isLight, letter, startOption } from '$shared/lib/inbox/rules';
  import { t } from '$shared/lib/i18n';
  import { renderMarkdown } from '$shared/lib/markdown';
  import type { AskAnswerIn, WindowAsk } from '$shared/lib/types';
  import { answerAsk, delegateAsk, reopenAsk, snooze, wakeAsk, type Outcome } from '../lib/act';
  import { showImages } from '../lib/feed.svelte';
  import { keepDraft } from '../lib/kept';
  import type { SpaceRef } from '../lib/thread';
  import AskFiles from './AskFiles.svelte';
  import AskImg from './AskImg.svelte';
  import LaterMenu from './LaterMenu.svelte';
  import Word from './Word.svelte';

  let {
    space,
    ask,
    cut,
    onSettled,
    onAnswered,
  }: { space: SpaceRef; ask: WindowAsk; cut: boolean; onSettled: () => void; onAnswered: (a: WindowAsk) => void } = $props();

  const open = $derived(ask.state === 'open');
  const light = () => document.documentElement.dataset.theme === 'light';
  // What the answer starts on, read once when the line unfolds.
  let chosen = $state<string[]>(untrack(() => (ask.options.length ? [startOption(ask)] : [])));
  let steps = $state<boolean[]>(untrack(() => ask.steps.map(() => false)));
  let text = $state(untrack(() => ask.recommend?.text ?? ''));
  let word = $state('');
  let wording = $state(false);
  let later = $state(false);
  let busy = $state(false);
  let said = $state<string | null>(null);
  const draftKey = $derived(`${space.id}:ask:${ask.id}`);

  /** The render of an option shown in the strip: the one of the phone's theme, else its first. */
  function renderOf(option: string) {
    const all = imagesOf(ask, option);
    return all.find((f) => isLight(f) === light()) ?? all[0] ?? null;
  }
  /** Every render of the options, in the phone's theme: what the viewer goes through. */
  const strip = $derived(ask.options.map((o) => renderOf(o.id)).filter((f) => f !== null));

  function big(option?: string) {
    const files = ask.options.flatMap((o) => imagesOf(ask, o.id));
    const at = option ? files.indexOf(renderOf(option) ?? files[0]) : 0;
    showImages(space.id, ask, files, Math.max(0, at));
  }

  async function finish(o: Outcome, answered = false) {
    busy = false;
    if (o.ok) {
      keepDraft(draftKey, '');
      keepDraft(`${draftKey}:word`, '');
      if (answered) onAnswered(ask);
      onSettled();
    } else said = 'locked' in o ? t('mobile.lock.cancelled') : o.error;
  }

  async function send(answer: AskAnswerIn) {
    busy = true;
    said = null;
    const note = word.trim();
    await finish(await answerAsk(space, ask, note ? { ...answer, note } : answer), true);
  }

  function choose() {
    if (!chosen.length) {
      said = t('mobile.answer.pickFirst');
      return;
    }
    void send({ options: chosen });
  }

  function toggle(id: string) {
    if (ask.multi) chosen = chosen.includes(id) ? chosen.filter((x) => x !== id) : [...chosen, id];
    else chosen = [id];
  }

  async function putOff(at: number) {
    busy = true;
    later = false;
    await finish(await snooze(space, { ask }, at));
  }

  const lead = $derived(ask.delegates[0] ?? null);
</script>

<div class="fold">
  {#if ask.body}
    <div class="md body">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- renderMarkdown sanitizes (DOMPurify) -->
      {@html renderMarkdown(ask.body)}
    </div>
  {/if}

  {#if ask.kind === 'choice' && strip.length}
    <div class="plates">
      {#each ask.options as o (o.id)}
        {@const f = renderOf(o.id)}
        <button
          type="button"
          class="render"
          class:on={chosen.includes(o.id)}
          aria-label={t('inbox.ask.zoom', { label: `${letter(o.id)} · ${o.label}` })}
          onclick={() => big(o.id)}
        >
          <span class="img"
            >{#if f}<AskImg space={space.id} {ask} file={f} />{/if}</span
          >
          <span class="cap"><b>{letter(o.id)}</b><span>{o.label}</span></span>
          {#if ask.recommend?.option === o.id}<span class="seal" title={t('inbox.ask.seal')}><Icon name="check" size={14} /></span>{/if}
        </button>
      {/each}
    </div>
  {/if}

  {#if ask.recommend && (ask.recommend.option || ask.recommend.why)}
    <p class="rec">
      {#if ask.recommend.option}
        {@const r = ask.options.find((o) => o.id === ask.recommend?.option)}
        <b>{t('inbox.ask.recommendedLine', { option: `${letter(ask.recommend.option)} · ${r?.label ?? ''}` })}</b>
      {/if}
      {ask.recommend.why}
      {#if ask.unblocks}{t('inbox.ask.unblocks')} {ask.unblocks}.{/if}
    </p>
  {:else if ask.unblocks}
    <p class="rec">{t('inbox.ask.unblocks')} {ask.unblocks}.</p>
  {/if}

  <AskFiles space={space.id} {ask} />

  {#if open}
    {#if cut}<p class="note warn" role="status">{t('mobile.answer.offline')}</p>{/if}

    {#if ask.kind === 'choice'}
      {#if !strip.length}
        <ul class="optlist">
          {#each ask.options as o (o.id)}
            <li>
              <b>{letter(o.id)}</b>
              {o.label}{#if o.description}<span> · {o.description}</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
      <div class="choose">
        {#each ask.options as o (o.id)}
          <button
            type="button"
            class="letter"
            class:on={chosen.includes(o.id)}
            role={ask.multi ? 'checkbox' : 'radio'}
            aria-checked={chosen.includes(o.id)}
            aria-label={`${letter(o.id)} · ${o.label}`}
            disabled={busy || cut}
            onclick={() => toggle(o.id)}>{letter(o.id)}</button
          >
        {/each}
        <button type="button" class="b go grow" disabled={busy || cut} onclick={choose}>
          {ask.multi
            ? t('mobile.answer.chooseMany', { count: chosen.length })
            : t('mobile.answer.choose', { option: chosen[0] ? letter(chosen[0]) : '' })}
        </button>
      </div>
    {:else if ask.kind === 'confirm'}
      <div class="acts">
        <button type="button" class="b" disabled={busy || cut} onclick={() => send({ yes: false })}
          >{ask.noLabel || t('inbox.answer.no')}</button
        >
        <button type="button" class="b go" disabled={busy || cut} onclick={() => send({ yes: true })}>
          {#if ask.gate}<Icon name="lock" size={16} />{/if}{ask.yesLabel || t('inbox.answer.yes')}
        </button>
      </div>
      {#if ask.gate}<p class="note">{t('mobile.lock.hint')}</p>{/if}
    {:else if ask.kind === 'answer'}
      <Word bind:value={text} label={t('inbox.answer.text')} draft={draftKey} placeholder="" rows={3} />
      <div class="acts one">
        <button type="button" class="b go" disabled={busy || cut || !text.trim()} onclick={() => send({ text: text.trim() })}>
          {t('inbox.answer.send')}
        </button>
      </div>
    {:else if ask.kind === 'task'}
      {#if ask.steps.length}
        <ul class="steps" aria-label={t('inbox.answer.steps')}>
          {#each ask.steps as step, i (i)}
            <li>
              <label><input type="checkbox" bind:checked={steps[i]} disabled={busy || cut} />{step}</label>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="acts">
        <button type="button" class="b" disabled={busy || cut} onclick={() => send({ done: false, steps })}
          >{t('inbox.answer.wontDo')}</button
        >
        <button type="button" class="b go" disabled={busy || cut} onclick={() => send({ done: true, steps })}
          >{t('inbox.answer.done')}</button
        >
      </div>
    {/if}

    {#if wording}
      <Word bind:value={word} label={t('inbox.answer.note')} draft={`${draftKey}:word`} />
    {/if}
    {#if later}
      <LaterMenu disabled={busy || cut} onPick={putOff} onCancel={() => (later = false)} />
    {/if}

    <div class="more">
      <button type="button" class="m" disabled={cut} onclick={() => (later = !later)}
        ><Icon name="clock" size={16} />{t('mobile.answer.later')}</button
      >
      <button type="button" class="m" onclick={() => (wording = !wording)}><Icon name="mic" size={16} />{t('mobile.answer.word')}</button>
      {#if ask.kind === 'choice' && strip.length}
        <button type="button" class="m end" onclick={() => big()}><Icon name="expand" size={16} />{t('mobile.answer.big')}</button>
      {/if}
    </div>
    {#if lead && !ask.gate}
      <button type="button" class="m lead" disabled={busy || cut} onclick={async () => finish(await delegateAsk(space, ask, lead[0]))}>
        {t('inbox.answer.delegate', { name: lead[1] })}
      </button>
    {/if}
  {:else if ask.state === 'snoozed'}
    <p class="note">{t('inbox.ask.snoozedUntil', { when: fLater(ask.snoozedUntil ?? 0) })}</p>
    <button type="button" class="b" disabled={busy || cut} onclick={async () => finish(await wakeAsk(space, ask))}
      >{t('mobile.answer.wake')}</button
    >
  {:else if ask.answer}
    <p class="note">
      {t('mobile.thread.recentAnswer', { when: fLater(ask.answer.at) })} · {ask.readAt
        ? t('mobile.thread.read')
        : t('mobile.thread.unread')}
    </p>
    {#if !ask.readAt}
      <button type="button" class="b" disabled={busy || cut} onclick={async () => finish(await reopenAsk(space, ask))}>
        <Icon name="undo" size={16} />{t('mobile.answer.reopen')}
      </button>
    {/if}
  {/if}

  {#if said}<p class="said" role="status">{said}</p>{/if}
</div>

<style>
  .fold {
    display: grid;
    gap: 12px;
    padding: 2px 0 4px;
    min-width: 0;
  }
  .body {
    font-size: 14px;
    color: var(--soft);
    overflow-wrap: anywhere;
  }
  .body :global(p) {
    margin: 0 0 6px;
  }
  .plates {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(120px, 1fr);
    gap: 8px;
    overflow-x: auto;
    scrollbar-width: none;
    padding: 10px 2px 2px;
  }
  .render {
    position: relative;
    display: grid;
    padding: 0;
    border: 0;
    border-radius: 12px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    text-align: left;
    overflow: visible;
  }
  .render.on {
    box-shadow: inset 0 0 0 2px var(--teal-ink);
  }
  .img {
    display: block;
    height: 84px;
    border-radius: 12px 12px 0 0;
    overflow: hidden;
  }
  .cap {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 36px;
    padding: 4px 8px;
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
  }
  .cap span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cap b {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--line-3);
    font-size: 12px;
  }
  .render.on .cap b {
    background: var(--teal-ink);
    color: var(--panel);
    box-shadow: none;
  }
  .seal {
    position: absolute;
    top: -8px;
    right: -4px;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--panel);
    color: var(--teal-ink);
    box-shadow: 0 0 0 2px var(--teal-ink);
  }
  .rec {
    margin: 0;
    font-size: 14px;
    color: var(--soft);
  }
  .rec b {
    color: var(--text);
  }
  .optlist {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 4px;
    font-size: 14px;
  }
  .optlist span {
    color: var(--muted);
  }
  .choose {
    display: flex;
    gap: var(--gap-target);
  }
  .letter {
    flex: none;
    width: calc(var(--target) + 4px);
    min-height: calc(var(--target) + 2px);
    border: 0;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-weight: 700;
    font-size: 17px;
  }
  .letter.on {
    background: var(--teal-ink);
    color: var(--panel);
    box-shadow: none;
  }
  .grow {
    flex: 1;
  }
  .acts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--gap-target);
  }
  .acts.one {
    grid-template-columns: 1fr;
  }
  .b {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: calc(var(--target) + 2px);
    padding: 0 14px;
    border: 0;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-weight: 650;
    font-size: 16px;
  }
  .b.go {
    background: var(--text);
    color: var(--panel);
    box-shadow: none;
  }
  .b:disabled,
  .letter:disabled,
  .m:disabled {
    opacity: 0.45;
  }
  .steps {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 4px;
  }
  .steps label {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: var(--target);
    font-size: 14px;
  }
  .steps input {
    width: 20px;
    height: 20px;
    accent-color: var(--teal-ink);
  }
  .more {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .m {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: var(--target);
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-m);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-weight: 600;
    font-size: 15px;
  }
  .m.end {
    margin-left: auto;
  }
  .lead {
    justify-self: start;
    color: var(--soft);
    font-size: 14px;
  }
  .note {
    margin: 0;
    font-size: var(--small);
    color: var(--muted);
  }
  .warn {
    color: var(--wait);
  }
  .said {
    margin: 0;
    font-size: var(--small);
    color: var(--soft);
  }
</style>
