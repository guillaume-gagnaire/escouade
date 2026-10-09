<script lang="ts" module>
  import type { DraftAttachment } from '../lib/attachments';

  const drafts = new Map<string, { text: string; files: DraftAttachment[] }>();
  const commandCache = new Map<string, { name: string; description: string; argumentHint?: string }[]>();
</script>

<script lang="ts">
  import { untrack } from 'svelte';
  import { ACCEPT, MAX_TOTAL, readAttachment, sizeLabel } from '../lib/attachments';
  import { applyCompletion, detectTrigger, filterCommands, type Trigger } from '../lib/complete';
  import { conversationOf } from '../lib/conversations.svelte';
  import { injectedSource } from '../lib/events';
  import { basename, dirname } from '../lib/format';
  import { api } from '../lib/ipc';
  import { EFFORTS, MODES, modelOptions, supportsAuto, supportsEffort } from '../lib/models';
  import { observeWidth } from '../lib/resize';
  import { app } from '../lib/state.svelte';
  import type { Agent, QuestionItem } from '../lib/types';
  import Dropdown from './Dropdown.svelte';

  let { agent }: { agent: Agent } = $props();

  // The agent object is replaced on every backend update: only its id may drive the draft.
  const agentId = $derived(agent.id);
  let draftFor = untrack(() => agent.id);
  const initial = drafts.get(draftFor);
  let text = $state(initial?.text ?? '');
  let files = $state<DraftAttachment[]>(initial?.files ?? []);
  let ta = $state<HTMLTextAreaElement>();
  let trigger = $state<Trigger | null>(null);
  let suggestions = $state<{ label: string; detail: string; value: string }[]>([]);
  let sel = $state(0);
  let sending = $state(false);
  /** Files being read, not attached yet. */
  let reading = $state(0);
  let dragOver = $state(false);
  let menu = $state<'model' | 'effort' | 'mode' | null>(null);
  let width = $state(0);
  let fileInput = $state<HTMLInputElement>();
  let reqSeq = 0;

  const conv = $derived(conversationOf(agent.id));
  const pendingItem = $derived(agent.pending.length ? conv.items.find((i) => i.id === agent.pending[0]) : undefined);
  const busy = $derived(agent.status === 'running' || agent.status === 'waiting');
  const effortOk = $derived(supportsEffort(agent.model));
  const modeOptions = $derived(
    MODES.map((md) => {
      const unavailable = md.value === 'auto' && !supportsAuto(agent.model);
      return {
        value: md.value,
        label: md.label,
        detail: unavailable ? 'indisponible avec Haiku' : md.title,
        title: unavailable ? "Le mode Auto n'est pas disponible avec Haiku" : md.title,
        disabled: unavailable,
      };
    }),
  );
  // Keeps the send button on the same line in a narrow column (split layout): drop the captions.
  const tight = $derived(width > 0 && width < (busy ? 640 : 540));
  const placeholder = $derived(
    pendingItem
      ? pendingItem.kind === 'permission'
        ? 'Explique à Claude quoi faire à la place (refuse la demande)…'
        : 'Réponds à la question ou écris une réponse libre…'
      : conv.items.length
        ? `Envoyer un message à ${agent.name}…`
        : 'Décris la tâche à confier à Claude…',
  );

  // Reused for another agent (the id itself changed): switch drafts.
  $effect(() => {
    const id = agentId;
    if (id === draftFor) return;
    untrack(() => {
      draftFor = id;
      const d = drafts.get(id);
      text = d?.text ?? '';
      files = d?.files ?? [];
      trigger = null;
      suggestions = [];
      queueMicrotask(autosize);
    });
  });

  // Drafts are saved as they are typed, so they survive agent switches.
  $effect(() => {
    const draft = { text, files: files.slice() };
    untrack(() => {
      if (draft.text || draft.files.length) drafts.set(draftFor, draft);
      else drafts.delete(draftFor);
    });
  });

  $effect(() => {
    queueMicrotask(autosize);
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
    const t = trigger;
    if (!t) {
      suggestions = [];
      return;
    }
    const seq = ++reqSeq;
    if (t.kind === 'file') {
      const files = await api.fileSuggestions(agent.id, t.query).catch(() => [] as string[]);
      if (seq !== reqSeq) return;
      suggestions = files.slice(0, 12).map((f) => ({ label: basename(f), detail: dirname(f), value: f }));
    } else {
      let cmds = commandCache.get(agent.id);
      if (!cmds) {
        cmds = await api.getCommands(agent.id).catch(() => []);
        if (cmds.length) commandCache.set(agent.id, cmds);
      }
      if (seq !== reqSeq) return;
      suggestions = filterCommands(cmds, t.query).map((c) => ({
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
      if (it.kind === 'user' && !injectedSource(it)) return it.text;
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
  // the files being read, and one read while switching agents goes to the draft it was added to.
  async function addFiles(list: Iterable<File>) {
    const owner = draftFor;
    reading++;
    try {
      for (const f of list) {
        try {
          const a = await readAttachment(f);
          const target = owner === draftFor ? files : (drafts.get(owner)?.files ?? []);
          if (target.reduce((n, x) => n + x.size, a.size) > MAX_TOTAL) {
            throw new Error(`${f.name} n'est pas joint : les fichiers d'un message sont limités à ${sizeLabel(MAX_TOTAL)} en tout.`);
          }
          if (owner === draftFor) files.push(a);
          else drafts.set(owner, { text: drafts.get(owner)?.text ?? '', files: [...target, a] });
        } catch (e) {
          app.toast(e instanceof Error ? e.message : String(e), 'error');
        }
      }
    } finally {
      reading--;
    }
  }

  function onPaste(e: ClipboardEvent) {
    const list = [...(e.clipboardData?.files ?? [])];
    if (list.length) {
      e.preventDefault();
      addFiles(list);
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

  async function send() {
    const body = text.trim();
    if ((!body && !files.length) || sending || reading) return;
    // Text typed while Claude waits answers the question / refuses the permission.
    const answering = body ? pendingItem : undefined;
    sending = true;
    const prevText = text;
    const prevFiles = files;
    text = '';
    if (!answering) files = [];
    try {
      if (answering?.kind === 'question') {
        const q = answering as QuestionItem;
        await api.answerQuestion(agent.id, q.id, Object.fromEntries(q.questions.map((x) => [x.question, body])));
      } else if (answering?.kind === 'permission') {
        await api.answerPermission(agent.id, answering.id, 'deny', body);
      } else {
        await api.sendMessage(
          agent.id,
          body,
          prevFiles.map(({ name, mediaType, data }) => ({ name, mediaType, data })),
        );
      }
      if (answering && prevFiles.length) {
        app.toast("Les fichiers joints n'accompagnent pas une réponse : ils restent prêts pour ton prochain message.");
      }
    } catch (e) {
      text = prevText;
      files = prevFiles;
      app.toast(String(e), 'error');
    }
    sending = false;
  }

  function stop() {
    app.run(api.interrupt(agent.id));
  }

  function toggleMenu(m: 'model' | 'effort' | 'mode') {
    menu = menu === m ? null : m;
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

  function setOption(o: { model?: string; effort?: string; mode?: string }) {
    const a = app.agents[agent.id];
    if (a) Object.assign(a, o);
    if (o.mode === 'bypassPermissions') app.toast('Mode Bypass : Claude agira sans aucune demande de permission.', 'info');
    app.run(api.setAgentOptions(agent.id, o));
  }
</script>

<div class="composer-wrap">
  <div
    class="composer"
    class:pending={!!pendingItem}
    class:drag={dragOver}
    role="group"
    ondragover={(e) => {
      if (e.dataTransfer?.types.includes('Files')) {
        e.preventDefault();
        dragOver = true;
      }
    }}
    ondragleave={() => (dragOver = false)}
    ondrop={onDrop}
  >
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
    {#if files.length}
      <div class="atts">
        {#each files as f, i (i)}
          {#if f.url}
            <div class="img">
              <img src={f.url} alt={f.name} />
              <button class="rm" aria-label="Retirer {f.name}" onclick={() => files.splice(i, 1)}>×</button>
            </div>
          {:else}
            <div class="file" title={f.name}>
              <span aria-hidden="true">📄</span>
              <span class="fname">{f.name}</span>
              <button class="rm" aria-label="Retirer {f.name}" onclick={() => files.splice(i, 1)}>×</button>
            </div>
          {/if}
        {/each}
      </div>
    {/if}
    <textarea
      bind:this={ta}
      bind:value={text}
      {placeholder}
      rows="2"
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
        caption="Modèle"
        value={agent.model}
        options={modelOptions(app.models)}
        open={menu === 'model'}
        showCaption={!tight}
        onToggle={() => toggleMenu('model')}
        onPick={(v) => pick({ model: v })}
      />
      <Dropdown
        caption="Effort"
        value={effortOk ? agent.effort : ''}
        options={EFFORTS.map((ef) => ({ value: ef.value, label: ef.label, detail: ef.title }))}
        open={menu === 'effort'}
        showCaption={!tight}
        disabled={!effortOk}
        title={effortOk ? 'Effort de réflexion' : "Haiku ne gère pas l'effort"}
        onToggle={() => toggleMenu('effort')}
        onPick={(v) => pick({ effort: v })}
      />
      <Dropdown
        caption="Mode"
        value={agent.mode}
        options={modeOptions}
        open={menu === 'mode'}
        showCaption={!tight}
        danger={agent.mode === 'bypassPermissions'}
        onToggle={() => toggleMenu('mode')}
        onPick={(v) => pick({ mode: v })}
      />
      <div style="flex:1"></div>
      {#if busy}
        <button class="btn ghost stop" class:icon={tight} onclick={stop} title="Interrompre (Échap)" aria-label="Stop"
          >{tight ? '■' : '■ Stop'}</button
        >
      {/if}
      <input
        bind:this={fileInput}
        type="file"
        accept={ACCEPT}
        multiple
        hidden
        aria-label="Joindre un fichier"
        onchange={(e) => {
          addFiles(e.currentTarget.files ?? []);
          e.currentTarget.value = '';
        }}
      />
      <button
        class="icon-btn attach"
        title="Joindre une image, un PDF ou un fichier texte (ou colle / glisse-le)"
        aria-label="Joindre un fichier"
        onclick={() => fileInput?.click()}
      >
        <svg
          width="15"
          height="15"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          ><path d="M21.4 11.1l-9.2 9.2a6 6 0 0 1-8.5-8.5l9.2-9.2a4 4 0 0 1 5.7 5.7l-9.2 9.2a2 2 0 0 1-2.8-2.8l8.5-8.5" /></svg
        >
      </button>
      <button
        class="btn primary"
        title={busy && !pendingItem
          ? 'Claude en tiendra compte dès sa prochaine étape · Entrée pour envoyer'
          : 'Entrée pour envoyer · Maj+Entrée pour aller à la ligne'}
        disabled={sending || reading > 0 || (!text.trim() && !files.length)}
        onclick={send}
      >
        Envoyer
      </button>
    </div>
  </div>
</div>

<style>
  .composer-wrap {
    flex: none;
    padding: 0 28px 20px;
  }
  .composer {
    position: relative;
    max-width: 780px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
    transition: border-color 0.15s;
  }
  .composer:focus-within {
    border-color: color-mix(in oklch, var(--accent) 55%, var(--line2));
  }
  .composer.pending {
    border-color: var(--wait);
  }
  .composer.drag {
    border-style: dashed;
    border-color: var(--accent);
  }
  textarea {
    resize: none;
    border: none;
    outline: none;
    background: transparent;
    font-size: 14px;
    line-height: 1.5;
    padding: 14px 16px 6px;
    min-height: 58px;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 10px 14px;
    min-width: 0;
  }
  .bar > :global(button) {
    flex: none;
  }
  .stop {
    height: 30px;
    color: var(--del);
  }
  .stop.icon {
    padding: 0 10px;
  }
  .atts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 12px 14px 0;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 260px;
    height: 30px;
    padding: 0 5px 0 10px;
    border-radius: 99px;
    border: 1px solid var(--line2);
    background: var(--elev2);
    font-size: 12.5px;
    color: var(--muted);
  }
  .fname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file .rm {
    position: static;
    flex: none;
  }
  .img {
    position: relative;
    width: 64px;
    height: 64px;
    border-radius: var(--r-sm);
    overflow: hidden;
    border: 1px solid var(--line2);
  }
  .img img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .rm {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 18px;
    height: 18px;
    border: none;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.7);
    color: #fff;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
  }
  .suggest {
    position: absolute;
    left: 0;
    right: 0;
    bottom: calc(100% + 6px);
    z-index: 30;
    display: flex;
    flex-direction: column;
    padding: 5px;
    max-height: 320px;
    overflow: auto;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
  }
  .sug {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 7px 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .sug.on {
    background: var(--elev2);
  }
  .sl {
    font-size: 12.5px;
    flex: none;
  }
  .sd {
    font-size: 12px;
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
</style>
