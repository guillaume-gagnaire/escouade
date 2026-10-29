<script lang="ts">
  import { logoUrl } from '../../lib/project-logo.svelte';
  import { tileText } from '../../lib/project-rail';
  import type { Project } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import TileFace from '../TileFace.svelte';
  import { tip } from '../../lib/tip';

  // A project's small tile in the view Plugins (choice 218): its colour and letters, faded where the plugin is off; a
  // private project's is a lock on no colour. Says its state through `label`.
  let { project, on, size = 20, label }: { project: Project; on: boolean; size?: number; label?: string } = $props();
</script>

<span
  class="chip"
  class:off={!on}
  class:lock={project.private}
  style:--c={project.color}
  style:--s="{size}px"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  use:tip={label ? { text: label, aria: false } : undefined}
>
  {#if project.private}<Icon name="lock" size={Math.round(size * 0.55)} />{:else}<TileFace
      text={tileText(project)}
      logo={logoUrl(project)}
    />{/if}
</span>

<style>
  .chip {
    position: relative;
    display: grid;
    flex: none;
    place-items: center;
    width: var(--s);
    height: var(--s);
    border-radius: calc(var(--s) * 0.26);
    background: var(--c);
    color: var(--on-tile);
    font: 700 calc(var(--s) * 0.45) / 1 var(--font);
    overflow: hidden;
  }
  .chip.off {
    opacity: 0.25;
  }
  .chip.lock {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
  }
  .chip.lock.off {
    opacity: 1;
  }
  .chip.lock:not(.off) {
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
</style>
