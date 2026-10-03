<script lang="ts">
  let { onsubmit, oncancel }: { onsubmit: (comment: string) => void | Promise<void>; oncancel: () => void } = $props();

  let comment = $state('');
  let busy = $state(false);

  async function submit() {
    if (!comment.trim() || busy) return;
    busy = true;
    try {
      await onsubmit(comment.trim());
    } finally {
      busy = false;
    }
  }
</script>

<!-- The card under it opens its agent on a click: the form keeps its clicks. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="reject" onclick={(e) => e.stopPropagation()}>
  <!-- svelte-ignore a11y_autofocus -->
  <textarea
    rows="3"
    placeholder="Ce qui ne va pas"
    aria-label="Ce qui ne va pas"
    bind:value={comment}
    autofocus
    onkeydown={(e) => {
      e.stopPropagation();
      if (e.key === 'Escape') oncancel();
    }}
  ></textarea>
  <div class="row">
    <button class="btn ghost" onclick={oncancel}>Annuler</button>
    <button class="btn primary" disabled={!comment.trim() || busy} onclick={submit}>Renvoyer</button>
  </div>
</div>

<style>
  .reject {
    display: flex;
    flex-direction: column;
    gap: 6px;
    cursor: default;
  }
  textarea {
    resize: none;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    line-height: 1.5;
    outline: none;
  }
  textarea:focus {
    border-color: var(--accent);
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  .btn {
    height: 28px;
    font-size: 12px;
  }
</style>
