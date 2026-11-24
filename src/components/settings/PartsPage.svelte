<script lang="ts">
  import { featureOn } from '../../lib/features';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { PART_FEATURE, PART_ICONS, PARTS, partKey, partState } from '../../lib/parts';
  import { app } from '../../lib/state.svelte';
  import type { Part, Project } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Switch from './Switch.svelte';

  // Réglages › Projets › Parties du projet (ESC-658), in the line of Réglages › Fonctionnalités: each part of the project
  // with what it does, what turning it off changes (its data stay), its switch, and who says it: this device, the
  // repository's .escouade.yml (« proposé par le dépôt »), the team that recommends or imposes it. The device's choice is
  // saved as it changes, by its own command; « Reprendre le défaut » forgets it. A part whose feature is off for the whole
  // app is greyed with where to turn it on.
  let { project }: { project: Project } = $props();

  /** The project as the core last told it: the page follows a .escouade.yml read again. */
  const live = $derived(app.projects.find((p) => p.id === project.id) ?? project);
  /** What the repository proposes for `part`, when it does. */
  const repoSays = (part: Part) => typeof live.shared?.file?.values?.[partKey(part)] === 'boolean';

  let busy = $state(false);
  async function set(part: Part, on: boolean | null) {
    if (busy) return;
    busy = true;
    try {
      const p = await app.run(api.projectSetPart(project.id, part, on));
      if (p) app.replaceProject(p);
    } finally {
      busy = false;
    }
  }

  /** Who says it, in a line; empty when nobody does (on, as every project). */
  function whoLine(part: Part): string {
    const s = partState(live, part);
    switch (s.by) {
      case 'repo':
        return t('parts.page.repo');
      case 'device':
        return repoSays(part) ? t('parts.page.deviceOverRepo') : t('parts.page.device');
      case 'teamDefault':
        return t('parts.page.teamDefault', { team: s.team ?? '' });
      case 'teamEnforced':
        return t('parts.page.enforced', { team: s.team ?? '' });
      default:
        return '';
    }
  }
</script>

<div class="parts" role="list">
  {#each PARTS as part (part)}
    {@const s = partState(live, part)}
    {@const feature = PART_FEATURE[part]}
    {@const featureOff = !!feature && !featureOn(app.settings, feature)}
    {@const locked = s.by === 'teamEnforced'}
    {@const who = whoLine(part)}
    <div class="part" class:off={!s.on || featureOff} class:locked role="listitem" data-part={part} data-by={s.by}>
      <span class="ic" aria-hidden="true"><Icon name={PART_ICONS[part]} /></span>
      <div class="lab">
        <b class="l">{t(`parts.${part}.name`)}</b>
        <span class="d">{t(`parts.${part}.desc`)}</span>
        <span class="takes" id="part-{part}-takes">{t(`parts.${part}.takes`)}</span>
        {#if featureOff}
          <p class="note">{t('parts.page.feature')}</p>
        {/if}
        {#if who}
          <p class="who" class:lock={locked} data-testid="part-{part}-by">
            {#if locked}<Icon name="lock" size={13} />{/if}<span>{who}</span>
            {#if s.by === 'device'}
              <button
                class="link"
                aria-label={t('parts.page.resetLabel', { name: t(`parts.${part}.name`) })}
                disabled={busy}
                onclick={() => void set(part, null)}>{t('parts.page.reset')}</button
              >
            {/if}
          </p>
        {/if}
      </div>
      <div class="ctl" inert={locked || featureOff} aria-disabled={locked || featureOff ? true : undefined}>
        <Switch
          label={t(`parts.${part}.name`)}
          describedby="part-{part}-takes"
          disabled={locked || featureOff || busy}
          on={s.on && !featureOff}
          onchange={(on) => void set(part, on)}
        />
      </div>
    </div>
  {/each}
</div>
<p class="foot">{t('parts.page.foot')}</p>

<style>
  /* The rows of Réglages › Fonctionnalités: a square for the icon, the name and what it does, the switch on the right. */
  .parts {
    display: grid;
  }
  .part {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 0;
  }
  .part + .part {
    border-top: 1px solid rgba(var(--fg-rgb), 0.06);
  }
  .ic {
    display: grid;
    flex: none;
    place-items: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--panel-2, var(--hover));
    color: var(--text);
  }
  .part.off .ic {
    color: var(--dim);
  }
  .lab {
    display: grid;
    flex: 1 1 auto;
    gap: 3px;
    min-width: 0;
  }
  .l {
    font-weight: 600;
  }
  .d,
  .takes,
  .note,
  .who {
    max-width: 68ch;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .takes {
    color: var(--dim);
  }
  .note {
    color: var(--warn, var(--muted));
  }
  .who {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    color: var(--text);
  }
  .who.lock {
    color: var(--muted);
  }
  .ctl {
    flex: none;
  }
  .ctl[aria-disabled='true'] {
    cursor: default;
  }
  .foot {
    margin: 14px 0 0;
    color: var(--dim);
    font-size: 12px;
  }
</style>
