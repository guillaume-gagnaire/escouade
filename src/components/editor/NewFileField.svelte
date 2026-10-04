<script lang="ts">
  import FileIcon from './FileIcon.svelte';

  let {
    check,
    oncreate,
    oncancel,
  }: {
    /** Why `name` cannot be created, or null. */
    check: (name: string) => string | null;
    /** Creates the file `name`: null once done, else what refused it. */
    oncreate: (name: string) => Promise<string | null>;
    oncancel: () => void;
  } = $props();

  let input = $state<HTMLInputElement>();
  let name = $state('');
  /** What refused the creation of `name`: shown until another name is typed; Enter tries it again. */
  let refused = $state<{ name: string; error: string } | null>(null);
  let busy = false;
  /** Given up or created: the focus lost as the field goes away decides nothing. */
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
    const error = await oncreate(typed).catch((e) => String(e));
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

  function take(node: HTMLInputElement) {
    node.focus();
    node.scrollIntoView?.({ block: 'nearest' });
  }
</script>

<FileIcon path={name.replaceAll('\\', '/')} />
<input
  use:take
  bind:this={input}
  bind:value={name}
  aria-label="Nom du nouveau fichier"
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
