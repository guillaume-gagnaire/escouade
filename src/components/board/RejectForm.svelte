<script lang="ts">
  import { t } from '../../lib/i18n';

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
    class="field"
    rows="3"
    placeholder={t('board.reject.comment')}
    aria-label={t('board.reject.comment')}
    bind:value={comment}
    autofocus
    onkeydown={(e) => {
      e.stopPropagation();
      if (e.key === 'Escape') oncancel();
    }}></textarea>
  <div class="row">
    <button class="btn ghost" onclick={oncancel}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!comment.trim() || busy} onclick={submit}>{t('board.reject.submit')}</button>
  </div>
</div>

<style>
  .reject {
    display: grid;
    gap: 6px;
    cursor: default;
  }
  textarea {
    resize: none;
    font-size: 12.5px;
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  .btn {
    height: 28px;
    font-size: 12.5px;
  }
</style>
