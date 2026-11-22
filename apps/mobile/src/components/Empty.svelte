<script lang="ts">
  // An empty state of the phone (MO-1): its icon, what is empty, and what will show there.
  import Icon from '$shared/components/Icon.svelte';
  import type { IconName } from '$shared/lib/icons';

  let { icon, title, text }: { icon: IconName; title: string; text: string } = $props();
</script>

<div class="empty">
  <span class="mark"><Icon name={icon} size={26} /></span>
  <h2>{title}</h2>
  <p>{text}</p>
</div>

<style>
  .empty {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 56px calc(var(--gut) + 12px) 32px;
    text-align: center;
  }
  .mark {
    width: 56px;
    height: 56px;
    border-radius: var(--r-l);
    display: grid;
    place-items: center;
    color: var(--muted);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  h2 {
    margin: 6px 0 0;
    font-size: 17px;
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--text);
  }
  p {
    margin: 0;
    max-width: 34ch;
    font-size: var(--small);
    line-height: 1.5;
    color: var(--muted);
  }
</style>
