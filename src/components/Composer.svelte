<script lang="ts" module>
  const commandCache = new Map<string, { name: string; description: string; argumentHint?: string }[]>();
</script>

<script lang="ts">
  import { untrack } from 'svelte';
  import {
    MAX_TOTAL,
    attachByPath,
    clipboardImage,
    inMessageBytes,
    planAttachment,
    planByPath,
    readForMessage,
    type DraftAttachment,
  } from '../lib/attachments';
  import { accountName, accountsOf, newAgentAccount, serves } from '../lib/accounts';
  import { applyCompletion, detectTrigger, filterCommands, type Trigger } from '../lib/complete';
  import { conversationOf } from '../lib/conversations.svelte';
  import { getDraft, giveBack, setDraft } from '../lib/drafts';
  import { hold, holdModeOf, settled } from '../lib/outbox.svelte';
  import { injectedSource } from '../lib/events';
  import { parseLocalCommand } from '../lib/localcmd';
  import { reportRefusal, sendDraft } from '../lib/send.svelte';
  import { fileOrigins } from '../lib/file-origin';
  import { basename, dirname, fSize } from '../lib/format';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { EFFORTS, MODES, modeTitle, modelOptions } from '../lib/models';
  import {
    can,
    capsOf,
    effortsOf,
    modelCost,
    modelMenu,
    modelsOf,
    modelTakes,
    modesOf,
    fromConfig,
    readingModels,
    refreshModels,
    searchesModels,
    setFavorite,
    speakerOf,
    engineName,
  } from '../lib/engines';
  import { keyLabel } from '../lib/platform';
  import { observeWidth } from '../lib/resize';
  import { app, atWork } from '../lib/state.svelte';
  import type { Agent, QuestionItem } from '../lib/types';
  import { insertAt } from '../lib/dictation';
  import type { DictationTarget } from '../lib/dictation.svelte';
  import AttachmentChip from './AttachmentChip.svelte';
  import DictationBand from './DictationBand.svelte';
  import Icon from './Icon.svelte';
  import ConversationButton from './ConversationButton.svelte';
  import MicButton from './MicButton.svelte';
  import Dropdown from './Dropdown.svelte';
  import { tip } from '../lib/tip';

  let { agent }: { agent: Agent } = $props();

  // The agent object is replaced on every backend update: only its id may drive the draft.
  const agentId = $derived(agent.id);
  let draftFor = untrack(() => agent.id);
  const initial = getDraft(draftFor);
  let text = $state(initial.text);
  let files = $state<DraftAttachment[]>(initial.files);
  let ta = $state<HTMLTextAreaElement>();
  let trigger = $state<Trigger | null>(null);
  let suggestions = $state<{ label: string; detail: string; value: string }[]>([]);
  let sel = $state(0);
  let sending = $state(false);
  /** The agent whose message being sent the backend holds until its worktree is prepared. */
  let heldFor: string | null = null;
  /** Files being read, not attached yet. */
  let reading = $state(0);
  /** A file is being copied: its chip is there, but its path is not known yet. */
  const copying = $derived(files.some((f) => f.copying));
  let dragOver = $state(false);
  let menu = $state<'model' | 'effort' | 'mode' | 'account' | null>(null);
  let width = $state(0);
  let fileInput = $state<HTMLInputElement>();
  let reqSeq = 0;
  let wrap = $state<HTMLDivElement>();

  // The dictation puts its text where the cursor is (the selection, if any, replaced): the button keeps the focus in the
  // field, and the text stays there to be read, changed and sent by hand.
  const dictated: DictationTarget = {
    kind: 'composer',
    id: `composer-${Math.random().toString(36).slice(2)}`,
    el: () => wrap,
    insert: (said) => {
      const r = insertAt(text, ta?.selectionStart ?? text.length, ta?.selectionEnd ?? text.length, said);
      text = r.text;
      queueMicrotask(() => {
        ta?.focus();
        ta?.setSelectionRange(r.caret, r.caret);
        autosize();
      });
    },
  };

  const conv = $derived(conversationOf(agent.id));
  const pendingItem = $derived(agent.pending.length ? conv.items.find((i) => i.id === agent.pending[0]) : undefined);
  const busy = $derived(atWork(agent));
  const sendTitle = $derived.by(() => {
    const engine = speakerOf(agent.engine);
    const enter = keyLabel('Enter');
    const mode = pendingItem ? null : holdModeOf(agent);
    if (mode === 'delay') return t('composer.send.titleHeld', { engine, s: app.settings.midTurnDelaySecs, enter });
    if (mode === 'turnEnd') return t('composer.send.titleTurnEnd', { engine, enter });
    if (busy && !pendingItem) return t('composer.send.titleQueued', { engine, enter });
    return t('composer.send.title', { enter, newline: keyLabel('Shift+Enter') });
  });
  // The menus are those of the agent's harness: its models, its efforts, its modes.
  const effortOk = $derived(modelTakes(agent, agent.model, 'effort'));
  // A harness that lists many models, or those of its providers (OpenCode): a search, the favorites first, then a group by
  // family (by provider); a favorite says its provider, out of its group.
  const manyModels = $derived(searchesModels(agent));
  const modelChoices = $derived(
    manyModels
      ? modelMenu(agent).map((m) => {
          const cost = modelCost(agent, m.id);
          const detail =
            (m.favorite ? m.provider : undefined) ?? cost?.label ?? (fromConfig(agent, m.id) ? t('composer.model.fromConfig') : undefined);
          return { value: m.id, label: m.label, detail, title: cost?.title, favorite: m.favorite, group: m.group };
        })
      : // Claude Code's aliases (`default`, `opus`…) are read for its own agents only: Cursor's Auto is `default` too.
        modelOptions((agent.engine ?? 'claude') === 'claude' ? app.models : [], modelsOf(agent)).map((o) => {
          // What it costs beside the others, when its harness bills by model (Copilot: `×3`).
          const cost = modelCost(agent, o.value);
          if (cost) return { ...o, detail: cost.label, title: cost.title };
          // Codex's model of its config.toml, which it does not list (ESC-186).
          if (fromConfig(agent, o.value)) return { ...o, detail: t('composer.model.fromConfig') };
          // Codex's stand-in while its account's list is being read (ESC-186).
          return o.value && readingModels(agent) ? { ...o, detail: t('composer.model.reading') } : o;
        }),
  );
  // A harness with no stand-in (OpenCode) while its account's list is being read: one greyed line that says so (ESC-186).
  const modelOptionsShown = $derived(
    modelChoices.length || !readingModels(agent) ? modelChoices : [{ value: '', label: t('composer.model.reading'), disabled: true }],
  );
  // The modes its harness takes, each with what it means there when that is not what Escouade's menu says.
  const caps = $derived(capsOf(agent.engine));
  // The five modes keep their places: one its harness lacks is greyed, with why (choice 17 of the 2.0 design).
  const takes = $derived(new Set(modesOf(agent, MODES).map((m) => m.value)));
  const modeOptions = $derived(
    MODES.map((md) => {
      if (!takes.has(md.value)) {
        const harness = engineName(agent.engine);
        const reason =
          md.value === 'auto' ? t('composer.mode.missingAuto', { harness }) : t('composer.mode.missing', { harness, mode: md.label });
        return { value: md.value, label: md.label, disabled: true, reason };
      }
      const unavailable = md.value === 'auto' && !modelTakes(agent, agent.model, 'auto');
      const meaning = caps?.modes.find((m) => m.id === md.value)?.meaning ?? modeTitle(md.value, speakerOf(agent.engine));
      return {
        value: md.value,
        label: md.label,
        detail: unavailable ? t('composer.mode.autoUnavailableDetail') : meaning,
        title: unavailable ? t('composer.mode.autoUnavailableTitle') : meaning,
        disabled: unavailable,
      };
    }),
  );
  // The account chip: with several accounts of the agent's harness, beside the model. It changes the account only while
  // the agent has no session (nothing said to it yet, no copy's original behind it): then its conversation is filed in that
  // account. Another harness's accounts are never offered: the agent runs on its own harness.
  const project = $derived(app.projects.find((p) => p.id === agent.projectId));
  const accounts = $derived(accountsOf(agent.engine ?? undefined));
  const accountLocked = $derived(!!agent.sessionId || !!agent.forkOf || agent.prompts > 0 || busy);
  // « Automatique » names the account a new agent of the project would go to: none when every one is reserved to others.
  const autoAccount = $derived(newAgentAccount(project, agent.engine ?? undefined));
  const accountOptions = $derived(
    accounts.length < 2
      ? []
      : [
          {
            value: '',
            label: autoAccount ? t('accounts.composer.auto', { name: accountName(autoAccount) }) : t('accounts.composer.autoNone'),
            detail: t('accounts.composer.autoDetail'),
          },
          // The agent's own, even switched off since: it is where it is.
          ...accounts
            .filter((a) => (a.active && serves(a, agent.projectId)) || a.id === agent.account)
            .map((a) => ({ value: a.id, label: a.active ? accountName(a) : t('accounts.composer.inactive', { name: accountName(a) }) })),
        ],
  );
  // A narrow composer (a small window, the split layout) drops the chips' captions, as the preview does under 760 px of
  // conversation; a narrower one, Stop's word too; the narrowest (the panel open beside a widened projects' rail in a
  // window of 1028 px, ESC-168), the key of « Envoyer ».
  const tight = $derived(width > 0 && width < 696);
  const narrow = $derived(width > 0 && width < 520);
  const cramped = $derived(width > 0 && width < 300);
  const placeholder = $derived(
    pendingItem
      ? pendingItem.kind === 'permission'
        ? t('composer.placeholder.denyPermission', { engine: speakerOf(agent.engine) })
        : t('composer.placeholder.answerQuestion')
      : conv.items.length
        ? t('composer.placeholder.sendTo', { name: agent.name })
        : t('composer.placeholder.describe', { engine: speakerOf(agent.engine) }),
  );

  // Reused for another agent (the id itself changed): switch drafts.
  $effect(() => {
    const id = agentId;
    if (id === draftFor) return;
    untrack(() => {
      draftFor = id;
      const d = getDraft(id);
      text = d.text;
      files = d.files;
      trigger = null;
      suggestions = [];
      queueMicrotask(autosize);
    });
  });

  // Drafts are saved as they are typed, so they survive agent switches (and, for their text, a restart).
  $effect(() => {
    const draft = { text, files: files.slice() };
    untrack(() => setDraft(draftFor, draft));
  });

  $effect(() => {
    queueMicrotask(autosize);
  });

  // A message given back to this draft from elsewhere (`giveBack`).
  $effect(() => {
    if (!app.draftReturned[agentId]) return;
    untrack(() => {
      const d = getDraft(draftFor);
      text = d.text;
      files = d.files;
      queueMicrotask(autosize);
    });
  });

  // A door of the Workflow tab (ESC-194): its command at the head of the message, in place of a command already there;
  // nothing is sent.
  $effect(() => {
    const put = app.composerPut;
    if (!put || put.agentId !== agentId) return;
    untrack(() => {
      app.composerPut = null;
      const rest = text.replace(/^\/\S*\s*/, '');
      text = rest ? `${put.text} ${rest}` : `${put.text} `;
      queueMicrotask(() => {
        autosize();
        ta?.focus();
        ta?.setSelectionRange(text.length, text.length);
      });
    });
  });

  $effect(() => {
    void app.focusComposer;
    queueMicrotask(() => ta?.focus());
  });

  function autosize() {
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = Math.min(ta.scrollHeight, window.innerHeight * 0.4) + 'px';
  }

  // Also when the text changes without typing: cleared once sent, another agent's draft, recall.
  $effect(() => {
    void text;
    autosize();
  });

  async function refreshSuggestions() {
    if (!ta) return;
    trigger = detectTrigger(text, ta.selectionStart);
    const trig = trigger;
    if (!trig) {
      suggestions = [];
      return;
    }
    const seq = ++reqSeq;
    if (trig.kind === 'file') {
      const files = await api.fileSuggestions(agent.id, trig.query).catch(() => [] as string[]);
      if (seq !== reqSeq) return;
      suggestions = files.slice(0, 12).map((f) => ({ label: basename(f), detail: dirname(f), value: f }));
    } else if (!can(agent, 'slashCommands')) {
      // Its harness offers no `/` command to a client.
      suggestions = [];
    } else {
      let cmds = commandCache.get(agent.id);
      if (!cmds) {
        cmds = await api.getCommands(agent.id).catch(() => []);
        if (cmds.length) commandCache.set(agent.id, cmds);
      }
      if (seq !== reqSeq) return;
      suggestions = filterCommands(cmds, trig.query).map((c) => ({
        label: '/' + c.name + (c.argumentHint ? ' ' + c.argumentHint : ''),
        detail: c.description,
        value: c.name,
      }));
    }
    sel = 0;
  }

  function accept(i: number) {
    const s = suggestions[i];
    if (!s || !trigger || !ta) return;
    const r = applyCompletion(text, trigger, s.value);
    text = r.text;
    suggestions = [];
    trigger = null;
    queueMicrotask(() => {
      ta?.setSelectionRange(r.caret, r.caret);
      ta?.focus();
      autosize();
    });
  }

  function lastUserMessage(): string | null {
    for (let i = conv.items.length - 1; i >= 0; i--) {
      const it = conv.items[i];
      if (it.kind !== 'user' || injectedSource(it)) continue;
      // A local command kept with its tags (ESC-154): as typed; its output is no message.
      const parts = parseLocalCommand(it.text);
      if (!parts) return it.text;
      const typed = parts.find((p) => p.kind === 'typed');
      if (typed) return typed.text;
    }
    return null;
  }

  function onKeydown(e: KeyboardEvent) {
    // A dialog is open above the composer: its keys (Escape…) are not for the agent.
    if (app.modal) return;
    if (suggestions.length) {
      if (e.key === 'ArrowDown') {
        sel = (sel + 1) % suggestions.length;
        e.preventDefault();
        return;
      }
      if (e.key === 'ArrowUp') {
        sel = (sel - 1 + suggestions.length) % suggestions.length;
        e.preventDefault();
        return;
      }
      if (e.key === 'Enter' || e.key === 'Tab') {
        accept(sel);
        e.preventDefault();
        return;
      }
      if (e.key === 'Escape') {
        suggestions = [];
        trigger = null;
        e.preventDefault();
        e.stopPropagation();
        return;
      }
    }
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send();
      return;
    }
    if (e.key === 'Escape' && busy) {
      e.preventDefault();
      e.stopPropagation();
      stop();
      return;
    }
    if (e.key === 'ArrowUp' && !text) {
      const last = lastUserMessage();
      if (last) {
        text = last;
        e.preventDefault();
        queueMicrotask(autosize);
      }
    }
  }

  // A file that cannot be attached is named in a toast, never silently dropped. Sending waits for
  // the files being read or copied, and one read while switching agents goes to the draft it was added to.
  async function addFiles(source: File[] | Promise<File[]>) {
    const owner = draftFor;
    // Asked at once, while the files are read: what the host holds for each of them is an id, never a path the page could give.
    const origins = app.settings.attachOrigin && Array.isArray(source) ? fileOrigins(source) : null;
    reading++;
    try {
      const list = await source;
      for (const [i, f] of list.entries()) {
        try {
          await attach(f, owner, origins?.then((o) => o[i] ?? null) ?? null);
        } catch (e) {
          failed(e);
        }
      }
    } catch (e) {
      failed(e);
    } finally {
      reading--;
    }
  }

  const draftFiles = (owner: string) => (owner === draftFor ? files : getDraft(owner).files);

  function add(owner: string, a: DraftAttachment) {
    if (owner === draftFor) files.push(a);
    else setDraft(owner, { text: getDraft(owner).text, files: [...getDraft(owner).files, a] });
  }

  /** Takes an attachment out of the draft of `owner`, if it is still there (a copy that failed). */
  function leave(owner: string, a: DraftAttachment) {
    const list = draftFiles(owner);
    const i = list.indexOf(a);
    if (i >= 0) list.splice(i, 1);
  }

  /**
   * Attaches a file to the draft of `owner`: in the message when Claude reads it and the message has room, else by its path
   * (the chip is there at once, while the file is copied or its path checked). `origin`: the id the host holds for the file
   * when it lies on the disk, if it knows it.
   */
  async function attach(f: File, owner: string, origin: Promise<string | null> | null) {
    const settings = app.settings;
    let plan = planAttachment(f, { settings, messageBytes: inMessageBytes(draftFiles(owner)) });
    if ('refusal' in plan) throw new Error(plan.refusal);
    if (plan.kind !== 'file') {
      const a = await readForMessage(f, plan.kind);
      // Another file may have taken the room while this one was read.
      if (a && inMessageBytes(draftFiles(owner)) + a.size <= MAX_TOTAL) return add(owner, a);
      // Not text after all, or no room left: its path carries it.
      plan = planByPath(f, settings);
      if ('refusal' in plan) throw new Error(plan.refusal);
    }
    const abort = new AbortController();
    const chip = $state<DraftAttachment>({
      kind: 'file',
      name: f.name,
      mediaType: '',
      data: '',
      size: f.size,
      warn: plan.warn,
      copying: true,
      progress: 0,
      cancel: () => abort.abort(),
    });
    add(owner, chip);
    // Not waited for: the next files of a drop do not wait for the copy of a big one, and sending waits for the chip itself.
    void attachChip(f, owner, chip, abort.signal, origin);
  }

  /** The copy of a file (or the check of its path) that its chip stands for, until its path is known. */
  async function attachChip(f: File, owner: string, chip: DraftAttachment, signal: AbortSignal, origin: Promise<string | null> | null) {
    try {
      chip.file = await attachByPath(f, owner, {
        origin: await origin,
        signal,
        onProgress: (done) => (chip.progress = done / f.size),
      });
      chip.copying = false;
      chip.cancel = undefined;
    } catch (e) {
      leave(owner, chip);
      // Taken away by the user: nothing to say.
      if (!signal.aborted) failed(e);
    }
  }

  /** Takes an attachment away: a copy still going stops, one finished is erased (never a file attached where it is). */
  function remove(i: number) {
    const [a] = files.splice(i, 1);
    if (a.copying) a.cancel?.();
    else if (a.file && !a.file.origin) api.attachDiscard(draftFor, a.file.path).catch(() => {});
  }

  function failed(e: unknown) {
    app.toast(e instanceof Error ? e.message : String(e), 'error');
  }

  function onPaste(e: ClipboardEvent) {
    const list = [...(e.clipboardData?.files ?? [])];
    if (list.length) {
      e.preventDefault();
      addFiles(list);
    } else if (!e.clipboardData?.types.includes('text/plain')) {
      addFiles(clipboardImage().then((f) => (f ? [f] : [])));
    }
  }

  function onDrop(e: DragEvent) {
    dragOver = false;
    const list = [...(e.dataTransfer?.files ?? [])];
    if (list.length) {
      // Left to the WebView, a dropped file would replace the app.
      e.preventDefault();
      addFiles(list);
    }
  }

  /** The send under way: a message sent before it is taken goes right after it, never dropped. */
  let inflight: Promise<void> | null = null;

  async function send() {
    const body = text.trim();
    if ((!body && !files.length) || reading || copying) return;
    // The one before waits for the worktree, which can take minutes: this one is not dropped without a word.
    if (sending && heldFor === agent.id) {
      app.toast(t('composer.send.alreadyWaiting'));
      return;
    }
    // Text typed while Claude waits answers the question / refuses the permission.
    const answering = body ? pendingItem : undefined;
    const mode = answering ? null : holdModeOf(agent);
    if (mode) {
      hold(agent.id, body, files.slice(), mode);
      text = '';
      files = [];
      return;
    }
    const message = { id: agent.id, conv, owner: draftFor, body, answering, prevText: text, prevFiles: files };
    // Another one may still be on its way (its answer can show before its call returns): this one leaves the field at
    // once, as it would, and goes right after it.
    const before = inflight;
    text = '';
    if (!answering) files = [];
    const mine = (async () => {
      if (before) await before;
      // Nor before a held message on its way.
      await settled(message.id);
      await deliver(message);
    })();
    inflight = mine;
    await mine;
    if (inflight === mine) inflight = null;
  }

  async function deliver({
    id,
    conv,
    owner,
    body,
    answering,
    prevText,
    prevFiles,
  }: {
    id: string;
    conv: ReturnType<typeof conversationOf>;
    owner: string;
    body: string;
    answering: typeof pendingItem;
    prevText: string;
    prevFiles: DraftAttachment[];
  }) {
    sending = true;
    const agent = app.agents[id];
    // A new worktree is prepared first (made first, when it waits for the first message: ESC-156): the backend holds the
    // message until then, and it shows meanwhile.
    const unhold =
      !answering && (agent?.setup || agent?.worktreePending)
        ? conv.holdMessage({
            text: body,
            images: prevFiles.filter((f) => f.kind === 'image').length,
            files: prevFiles.filter((f) => f.kind !== 'image').map((f) => f.name),
          })
        : undefined;
    if (unhold) heldFor = id;
    try {
      if (answering?.kind === 'question') {
        const q = answering as QuestionItem;
        await api.answerQuestion(id, q.id, Object.fromEntries(q.questions.map((x) => [x.question, body])));
      } else if (answering?.kind === 'permission') {
        await api.answerPermission(id, answering.id, 'deny', body);
      } else {
        await sendDraft(id, body, prevFiles);
      }
      if (answering && prevFiles.length) {
        app.toast(t('composer.send.filesKept'));
      }
      conv.noAccount = null;
    } catch (e) {
      // The files of an answer never left the field.
      giveBack(owner, prevText, answering ? [] : prevFiles);
      // For want of an account of its harness: said in the conversation, with the way out (choice 19).
      reportRefusal(owner, e);
    }
    // Recorded by now, or given back to the field above.
    unhold?.();
    heldFor = null;
    sending = false;
  }

  function stop() {
    app.run(api.interrupt(agent.id));
  }

  function toggleMenu(m: 'model' | 'effort' | 'mode' | 'account') {
    menu = menu === m ? null : m;
    // Its account's models read again when a day old (ESC-186).
    if (menu === 'model') refreshModels(agent);
  }

  function pick(o: { model?: string; effort?: string; mode?: string }) {
    menu = null;
    setOption(o);
    ta?.focus();
  }

  // A dialog opening over an open menu would leave it hidden, still catching Escape.
  $effect(() => {
    if (app.modal) menu = null;
  });

  /** Empty: « Automatique », which account that is is the backend's to say (it tells the agent back). */
  async function pickAccount(id: string) {
    menu = null;
    const a = app.agents[agent.id];
    const before = a?.account;
    if (a && id) a.account = id;
    ta?.focus();
    try {
      await api.setAgentAccount(agent.id, id);
    } catch (e) {
      // Refused: the agent is where it was, as the chip must say.
      if (a && before !== undefined) a.account = before;
      app.toast(String(e), 'error');
    }
  }

  function setOption(o: { model?: string; effort?: string; mode?: string }) {
    const a = app.agents[agent.id];
    if (a) Object.assign(a, o);
    if (o.mode === 'bypassPermissions') app.toast(t('composer.mode.bypassToast', { engine: speakerOf(agent.engine) }), 'info');
    app.run(api.setAgentOptions(agent.id, o));
  }
</script>

<div class="composer-wrap" bind:this={wrap}>
  <div
    class="composer"
    class:pending={!!pendingItem}
    class:dragging={dragOver}
    role="group"
    ondragover={(e) => {
      if (e.dataTransfer?.types.includes('Files')) {
        e.preventDefault();
        dragOver = true;
      }
    }}
    ondragleave={(e) => {
      // Leaving for one of its own children is no leaving.
      if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node | null)) dragOver = false;
    }}
    ondrop={onDrop}
  >
    <!-- Dropped files: the whole field takes them. -->
    {#if dragOver}
      <div class="drop" aria-hidden="true">
        <div>
          <span><Icon name="clip" size={16} />{t('composer.drop.title')}</span>
          <small>{t('composer.drop.detail')}</small>
        </div>
      </div>
    {/if}
    {#if suggestions.length}
      <div class="suggest" role="listbox">
        {#each suggestions as s, i (s.value)}
          <button
            class="sug"
            class:on={i === sel}
            role="option"
            aria-selected={i === sel}
            onmousedown={(e) => {
              e.preventDefault();
              accept(i);
            }}
            onmouseenter={() => (sel = i)}
          >
            <span class="sl mono">{s.label}</span>
            <span class="sd">{s.detail}</span>
          </button>
        {/each}
      </div>
    {/if}
    <!-- The dictation, in the field itself: listening, the model's offer, its download. -->
    <DictationBand targetId={dictated.id} />
    {#if files.length}
      <div class="atts">
        {#each files as f, i (i)}
          {#if f.url}
            <span class="att img" use:tip={{ text: f.name, cut: true }}>
              <img class="th" src={f.url} alt={f.name} />
              <span class="nm2"><b>{f.name}</b><span>{fSize(f.size)}</span></span>
              <button class="x" aria-label={t('composer.attachments.remove', { name: f.name })} onclick={() => remove(i)}
                ><Icon name="close" size={14} /></button
              >
            </span>
          {:else}
            <AttachmentChip attachment={f} onremove={() => remove(i)} />
          {/if}
        {/each}
      </div>
    {/if}
    <textarea
      bind:this={ta}
      bind:value={text}
      {placeholder}
      rows="1"
      spellcheck="false"
      oninput={() => {
        autosize();
        refreshSuggestions();
      }}
      onkeydown={onKeydown}
      onclick={refreshSuggestions}
      onpaste={onPaste}
      onblur={() => setTimeout(() => (suggestions = []), 120)}
    ></textarea>
    <div class="bar" use:observeWidth={(w) => (width = w)}>
      <Dropdown
        caption={t('common.model')}
        value={agent.model}
        options={modelOptionsShown}
        searchable={manyModels}
        onFavorite={manyModels ? (v, fav) => void setFavorite(agent.engine ?? 'claude', v, fav) : undefined}
        open={menu === 'model'}
        showCaption={!tight}
        onToggle={() => toggleMenu('model')}
        onPick={(v) => pick({ model: v })}
      />
      {#if accounts.length > 1}
        <Dropdown
          caption={t('accounts.composer.caption')}
          value={agent.account}
          options={accountOptions}
          open={menu === 'account'}
          showCaption={!tight}
          readonly={accountLocked}
          title={accountLocked ? t('accounts.composer.locked') : undefined}
          onToggle={() => toggleMenu('account')}
          onPick={pickAccount}
        />
      {/if}
      {#if caps?.effort}
        <Dropdown
          caption={t('composer.bar.effort')}
          value={effortOk ? agent.effort : ''}
          options={effortsOf(agent, EFFORTS).map((ef) => ({ value: ef.value, label: ef.label, detail: ef.title }))}
          shownLabel={EFFORTS.find((ef) => ef.value === agent.effort)?.label}
          open={menu === 'effort'}
          showCaption={!tight}
          disabled={!effortOk}
          title={effortOk ? t('composer.bar.effortTitle') : t('composer.bar.effortUnsupported')}
          onToggle={() => toggleMenu('effort')}
          onPick={(v) => pick({ effort: v })}
        />
      {/if}
      <Dropdown
        caption={t('composer.bar.mode')}
        value={agent.mode}
        options={modeOptions}
        open={menu === 'mode'}
        showCaption={!tight}
        danger={agent.mode === 'bypassPermissions'}
        onToggle={() => toggleMenu('mode')}
        onPick={(v) => pick({ mode: v })}
      />
      <div class="end">
        <input
          bind:this={fileInput}
          type="file"
          multiple
          hidden
          aria-label={t('composer.attach.label')}
          onchange={(e) => {
            // Copied first: the list is the input's own, emptied with its value below, before `addFiles` reads it.
            addFiles([...(e.currentTarget.files ?? [])]);
            e.currentTarget.value = '';
          }}
        />
        <button
          class="icon-btn attach"
          use:tip={t('composer.attach.title')}
          aria-label={t('composer.attach.label')}
          onclick={() => fileInput?.click()}><Icon name="clip" size={14} /></button
        >
        <MicButton target={dictated} panel={false} />
        <ConversationButton {agent} />
        <span class="c-sep" aria-hidden="true"></span>
        {#if busy}
          <button
            class="btn stop"
            class:icon={narrow}
            onclick={stop}
            use:tip={t('composer.bar.interrupt', { key: keyLabel('Esc') })}
            aria-label={t('composer.bar.stop')}
            ><Icon name="stop" size={14} />{#if !narrow}{t('composer.bar.stop')}{#if !tight}<kbd aria-hidden="true">{keyLabel('Esc')}</kbd
                >{/if}{/if}</button
          >
        {/if}
        <button
          class="btn primary send"
          use:tip={sendTitle}
          disabled={sending || reading > 0 || copying || (!text.trim() && !files.length)}
          onclick={send}
        >
          {t('composer.send.label')}{#if !cramped}<kbd aria-hidden="true"><Icon name="enter" size={12} /></kbd>{/if}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .composer-wrap {
    flex: none;
  }
  /* The composer floats above the panel, the width of the thread (2.0 design). */
  .composer {
    position: relative;
    width: min(780px, 100% - 64px);
    margin: 0 auto 18px;
    border-radius: 14px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -12px rgba(0, 0, 0, calc(0.6 * var(--shade)));
    transition: box-shadow 0.2s;
  }
  .composer:focus-within {
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--p) 55%, transparent),
      0 0 0 3px color-mix(in oklch, var(--p) 14%, transparent),
      0 12px 32px -12px rgba(0, 0, 0, calc(0.6 * var(--shade)));
  }
  /* A question or a permission waits: what is typed answers it, in amber. */
  .composer.pending {
    box-shadow:
      inset 0 0 0 1px var(--wait-line),
      0 12px 32px -12px rgba(0, 0, 0, calc(0.6 * var(--shade)));
  }
  .composer.dragging {
    box-shadow:
      inset 0 0 0 1.5px var(--p-ink),
      0 0 0 4px color-mix(in oklch, var(--p) 18%, transparent);
  }
  .drop {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: color-mix(in oklch, var(--p) 14%, var(--drop-base));
    color: var(--text);
    font-weight: 550;
    pointer-events: none;
  }
  .drop span {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .drop small {
    display: block;
    margin-top: 3px;
    color: var(--muted);
    font-weight: 450;
    text-align: center;
  }
  textarea {
    display: block;
    width: 100%;
    min-height: 52px;
    padding: 14px 16px 6px;
    border: 0;
    outline: 0;
    resize: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 13.5px;
    line-height: 1.5;
  }
  textarea::placeholder {
    color: var(--muted);
  }
  /* Model, effort and mode in chips at the left; join, dictate, converse, then Stop and Send at the right. The bar
     wraps when there is no room. */
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    row-gap: 6px;
    padding: 6px 8px 8px 10px;
    min-width: 0;
  }
  .end {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
  }
  .end :global(.icon-btn) {
    width: 30px;
    height: 28px;
    padding: 0;
    border-radius: 7px;
    font-size: 12.5px;
    font-weight: 500;
  }
  .c-sep {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: var(--line-2);
  }
  .stop :global(.icon) {
    color: var(--muted);
  }
  .stop kbd {
    height: 16px;
    font-size: 10.5px;
  }
  .stop.icon {
    width: 30px;
    padding: 0;
    justify-content: center;
  }
  .send {
    height: 32px;
    padding: 0 10px 0 12px;
    border-radius: 9px;
  }
  .send kbd {
    border-color: var(--on-primary-kbd);
    color: var(--on-primary-dim);
  }
  @container conv (max-width: 760px) {
    .stop kbd {
      display: none;
    }
  }
  /* Files attached, in a row of chips above the text. */
  .atts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 10px 10px 0;
  }
  .att {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 260px;
    height: 40px;
    padding: 0 6px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 12px;
  }
  .att .th {
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    object-fit: cover;
  }
  .nm2 {
    display: grid;
    min-width: 0;
    line-height: 1.25;
  }
  .nm2 b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }
  .nm2 span {
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .x {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
  /* The `/` commands and the files of `@`, above the field. */
  .suggest {
    position: absolute;
    left: 0;
    right: 0;
    bottom: calc(100% + 6px);
    z-index: 30;
    display: flex;
    flex-direction: column;
    padding: 4px;
    max-height: 320px;
    overflow: auto;
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 16px 40px -12px rgba(0, 0, 0, calc(0.7 * var(--shade)));
  }
  .sug {
    display: flex;
    align-items: baseline;
    gap: 12px;
    min-width: 0;
    padding: 7px 9px;
    border: none;
    border-radius: 7px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .sug.on {
    background: var(--hover);
  }
  .sl {
    flex: none;
    font-size: 12.5px;
  }
  .sd {
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (prefers-reduced-motion: reduce) {
    .composer {
      transition: none;
    }
  }
</style>
