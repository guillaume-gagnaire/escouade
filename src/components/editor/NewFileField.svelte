<script lang="ts">
  import { untrack } from 'svelte';
  import type { EntryKind } from '../../lib/editor/create';
  import FileIcon from './FileIcon.svelte';

  let {
    check,
    onsubmit,
    oncancel,
    value = '',
    label = 'Nom du nouveau fichier',
    kind = 'file',
  }: {
    /** Why `name` cannot be taken, or null. */
    check: (name: string) => string | null;
    /** Creates (or renames to) `name`: null once done, else what refused it. */
    onsubmit: (name: string) => Promise<string | null>;
    oncancel: () => void;
    /** The name to start from: a file or folder renamed has its own. */
    value?: string;
    label?: string;
    /** A folder's name has no file icon. */
    kind?: EntryKind;
  } = $props();

  let input = $state<HTMLInputElement>();
  let name = $state(untrack(() => value));
  /** What refused `name`: shown until another name is typed; Enter tries it again. */
  let refused = $state<{ name: string; error: string } | null>(null);
  let busy = false;
  /** Given up or done: the focus lost as the field goes away decides nothing. */
  let done = false;

  const invalid = $derived(name.trim() ? check(name) : null);
  const problem = $derived(refused?.name === name ? refused.error : invalid);

  function cancel() {
    done = true;
    oncancel();
  }

  async function commit() {
    if (busy || done) return;
    if (!name.trim()) return cancel();
    if (invalid) return;
    busy = true;
    const typed = name;
    const error = await onsubmit(typed).catch((e) => String(e));
    busy = false;
    if (!error) {
      done = true;
      return;
    }
    refused = { name: typed, error };
    // Refused after the field was left: it takes the focus back for its error to be read.
    if (input?.isConnected) input.focus();
  }

  // As VS Code does: leaving the field creates the file named, or gives it up when there is none to create (or the
  // disk refused it). Decided once the blur is over: a field the app takes away (its source changed) is out of the page
  // by then, and one whose window was left still holds the focus, to get it back with the window.
  function onblur() {
    queueMicrotask(() => {
      if (busy || done || !input?.isConnected || document.activeElement === input) return;
      if (!name.trim() || problem) oncancel();
      else commit();
    });
  }

  // A name given is selected but its extension, as VS Code does: what is typed replaces the name, `.ts` stays. A folder's,
  // or a name with nothing before its dot (`.env`), is selected whole.
  function take(node: HTMLInputElement) {
    node.focus();
    // Given before the binding does, for the selection to be made in it.
    node.value = name;
    const dot = kind === 'file' ? name.lastIndexOf('.') : -1;
    node.setSelectionRange(0, dot > 0 ? dot : name.length);
    node.scrollIntoView?.({ block: 'nearest' });
  }
</script>

{#if kind === 'file'}<FileIcon path={name.replaceAll('\\', '/')} />{/if}
<input
  use:take
  bind:this={input}
  bind:value={name}
  aria-label={label}
  aria-invalid={!!problem}
  spellcheck="false"
  autocomplete="off"
  onkeydown={(e) => {
    e.stopPropagation();
    if (e.key === 'Enter') commit();
    if (e.key === 'Escape') cancel();
  }}
  {onblur}
/>
{#if problem}<div class="problem" role="alert">{problem}</div>{/if}

<style>
  input {
    flex: 1;
    min-width: 0;
    height: 20px;
    padding: 0 4px;
    border: 1px solid var(--accent);
    border-radius: 2px;
    outline: none;
    background: var(--term);
    font-size: 13px;
  }
  input[aria-invalid='true'] {
    border-color: var(--del);
  }
  .problem {
    position: absolute;
    z-index: 2;
    top: 100%;
    left: var(--field-left);
    right: 8px;
    padding: 5px 8px;
    border: 1px solid var(--del);
    border-top: none;
    background: oklch(0.3 0.06 25);
    color: var(--text);
    font-size: 12px;
    line-height: 1.4;
    white-space: normal;
  }
</style>
