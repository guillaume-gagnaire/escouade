<script lang="ts">
  // « 1 environnement sur 2 » (ESC-390, direction A of design/kanban-envs-2-2): in the board's header, the menu that
  // checks the environments the view shows and orders them (dragged, or Alt+↑ and Alt+↓), kept for the project on
  // this machine. Shown by default: those where a deployment once succeeded.
  import { arrange, hiddenWhy, isShown, lanes, moveBefore, moveBy, setShown } from '../../lib/environments';
  import { envsView } from '../../lib/envs-view.svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Environment, Project } from '../../lib/types';
  import Icon from '../Icon.svelte';

  let { project, narrow = false }: { project: Project; narrow?: boolean } = $props();

  let open = $state(false);
  let box = $state<HTMLElement>();
  /** The environment being dragged. */
  let dragging = $state<string | null>(null);

  const prefs = $derived(envsView.of(project.id));
  const envs = $derived(app.environments[project.id]?.environments ?? []);
  const set = $derived(arrange(envs, prefs));
  const tickets = $derived(Object.values(app.tickets).filter((x) => x.projectId === project.id));
  /** How many tickets each shown environment holds, as its lane counts them. */
  const counts = $derived(new Map(lanes(tickets, set.shown, app.now).lanes.map((l) => [l.env.name, l.tickets.length])));
  const label = $derived(
    narrow
      ? t('envs.menu.short', { count: set.shown.length, total: envs.length })
      : t('envs.menu.button', { count: set.shown.length, total: envs.length }),
  );

  function toggle(e: Environment, on: boolean) {
    envsView.set(project.id, setShown(prefs, e, on));
  }

  function key(ev: KeyboardEvent, e: Environment) {
    if (!ev.altKey || (ev.key !== 'ArrowUp' && ev.key !== 'ArrowDown')) return;
    ev.preventDefault();
    envsView.set(project.id, moveBy(prefs, set.all, e.name, ev.key === 'ArrowUp' ? -1 : 1));
    // The focus follows the row moved.
    queueMicrotask(() => box?.querySelector<HTMLInputElement>(`input[data-env="${CSS.escape(e.name)}"]`)?.focus());
  }

  function drop(ev: DragEvent, before: string | null) {
    ev.preventDefault();
    if (dragging && dragging !== before) envsView.set(project.id, moveBefore(prefs, set.all, dragging, before));
    dragging = null;
  }

  function why(e: Environment): string {
    const w = hiddenWhy(e, prefs);
    if (w) return t(w === 'byYou' ? 'envs.menu.byYou' : 'envs.menu.neverRan');
    if (e.production) return t('envs.view.production');
    return t('envs.menu.tickets', { count: counts.get(e.name) ?? 0 });
  }

  function outside(ev: PointerEvent) {
    if (open && box && !box.contains(ev.target as Node)) open = false;
  }
</script>

<svelte:window
  onpointerdown={outside}
  onkeydown={(e) => {
    if (open && e.key === 'Escape') {
      e.stopPropagation();
      open = false;
    }
  }}
/>

<div class="em" bind:this={box}>
  <button
    class="em-b"
    aria-haspopup="dialog"
    aria-expanded={open}
    use:tip={t('envs.menu.tip')}
    disabled={!envs.length}
    onclick={() => (open = !open)}><Icon name="sliders" size={14} /><b>{label}</b><Icon name="down" size={12} /></button
  >
  {#if open}
    <div
      class="pop"
      role="dialog"
      aria-label={t('envs.menu.title')}
      tabindex="-1"
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation();
          open = false;
          box?.querySelector<HTMLButtonElement>('.em-b')?.focus();
        }
      }}
    >
      <h4>{t('envs.menu.title')}</h4>
      <ul ondragover={(e) => e.preventDefault()} ondrop={(e) => drop(e, null)}>
        {#each set.all as e (e.name)}
          <li
            draggable="true"
            class:dragging={dragging === e.name}
            data-env-row={e.name}
            ondragstart={(ev) => {
              dragging = e.name;
              ev.dataTransfer?.setData('text/plain', e.name);
            }}
            ondragend={() => (dragging = null)}
            ondragover={(ev) => ev.preventDefault()}
            ondrop={(ev) => {
              ev.stopPropagation();
              drop(ev, e.name);
            }}
          >
            <span class="grip" aria-hidden="true" use:tip={t('envs.menu.move')}><Icon name="grip" size={14} /></span>
            <label>
              <input
                type="checkbox"
                data-env={e.name}
                checked={isShown(e, prefs)}
                aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown"
                onchange={(ev) => toggle(e, ev.currentTarget.checked)}
                onkeydown={(ev) => key(ev, e)}
              />
              <span class="n">{e.name}</span>
            </label>
            <span class="why">{why(e)}</span>
          </li>
        {/each}
      </ul>
      <p class="foot">{t('envs.menu.foot')}</p>
    </div>
  {/if}
</div>

<style>
  .em {
    position: relative;
    min-width: 0;
    display: flex;
  }
  .em-b {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }
  .em-b:hover:not(:disabled) {
    background: var(--hover);
  }
  .em-b:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .em-b b {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
    font-weight: 550;
  }
  .pop {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 30;
    width: 300px;
    padding: 6px;
    border-radius: 12px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 18px 40px rgba(0, 0, 0, calc(0.35 * var(--shade)));
    white-space: normal;
  }
  :global([data-theme='light']) .pop {
    background: var(--panel);
  }
  h4 {
    margin: 4px 8px 6px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 4px;
    border-radius: 7px;
    font-size: 12.5px;
  }
  li:hover,
  li:focus-within {
    background: var(--hover);
  }
  li.dragging {
    opacity: 0.5;
  }
  .grip {
    display: grid;
    place-items: center;
    color: var(--muted);
    cursor: grab;
  }
  label {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 7px 0;
    cursor: pointer;
  }
  input {
    flex: none;
    width: 15px;
    height: 15px;
    margin: 0;
    accent-color: var(--text);
  }
  .n {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .why {
    flex: none;
    color: var(--muted);
    font-size: 11.5px;
  }
  .foot {
    margin: 4px 0 0;
    padding: 8px 8px 4px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.4;
  }
</style>
