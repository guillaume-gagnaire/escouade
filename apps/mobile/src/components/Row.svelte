<script lang="ts">
  // A line of the thread (MO-1, direction B): its kind in a square (amber for an agent that waits), what it is, and
  // where (the kind, the space's plate and name, the project's dot and name). Touched (MO-5), it opens what it names.
  import Icon from '$shared/components/Icon.svelte';
  import SpacePlate from '$shared/components/servers/SpacePlate.svelte';
  import type { IconName } from '$shared/lib/icons';
  import type { GlanceProject } from '$shared/lib/spaces.svelte';
  import type { PhoneSpace } from '../lib/phone-spaces.svelte';

  let {
    icon,
    title,
    kind,
    space,
    project,
    waits = false,
    onOpen,
  }: {
    icon: IconName;
    title: string;
    kind: string;
    space: Pick<PhoneSpace, 'name' | 'here'>;
    project: GlanceProject | null;
    waits?: boolean;
    onOpen?: () => void;
  } = $props();
</script>

<li class="row" class:waits>
  {#if onOpen}<button type="button" class="hit" aria-label={title} onclick={onOpen}></button>{/if}
  <span class="k"><Icon name={icon} size={16} /></span>
  <span class="tt">{title}</span>
  <span class="mt">
    <span class="kind">{kind}</span>
    <span class="s" aria-hidden="true">·</span>
    <SpacePlate name={space.name} here={space.here} size={20} />
    <span>{space.name}</span>
    {#if project}
      <span class="s" aria-hidden="true">·</span>
      <i class="pdot" style:background={project.color} aria-hidden="true"></i>
      <span class="pj">{project.name}</span>
    {/if}
  </span>
</li>

<style>
  .row {
    position: relative;
    display: grid;
    grid-template-columns: 30px minmax(0, 1fr);
    gap: 4px 12px;
    padding: 12px var(--gut);
    align-items: start;
    min-height: var(--target);
  }
  .row + :global(.row) {
    box-shadow: inset 0 1px 0 var(--line);
  }
  .hit {
    position: absolute;
    inset: 0;
    border: 0;
    background: transparent;
  }
  .hit:active {
    background: var(--hover);
  }
  .k {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    color: var(--soft);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .waits .k {
    color: var(--wait);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .tt {
    font-weight: 620;
    font-size: var(--body);
    line-height: 1.32;
    color: var(--text);
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .mt {
    grid-column: 2;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: var(--small);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .s {
    opacity: 0.6;
  }
  .pdot {
    width: 8px;
    height: 8px;
    border-radius: 3px;
    flex: none;
  }
  .pj {
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
