<script lang="ts">
  import { FEATURE_ICONS, FEATURES, featureRow, featureSwitch, NEEDS } from '../../lib/features';
  import { reviewTour } from '../../lib/features-tour.svelte';
  import { t } from '../../lib/i18n';
  import { lockLine } from '../../lib/locks.svelte';
  import { SETTINGS_ROWS } from '../../lib/settings-pages';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Feature, Features } from '../../lib/types';
  import { loadNever, offers, restore } from '../../lib/offers.svelte';
  import Icon from '../Icon.svelte';
  import Group from './Group.svelte';
  import Switch from './Switch.svelte';

  // « Propositions écartées » (F-3): read as the page opens.
  $effect(() => {
    void loadNever();
  });

  // Réglages › Fonctionnalités (F-1, ESC-550, maquette design/features/reglages-fonctionnalites.png, direction A): the
  // essential first, never switched off; then each feature with what it does, what it takes with it when it is off (its
  // data stay), and its switch. A switch a policy imposes is greyed with who imposes it; « Tâches programmées » goes with
  // the Kanban. Saved as it changes, as every page. « Revoir la présentation » opens the assistant again (F-2, ESC-559),
  // the settings' window closed first.
  const s = $derived(settingsForm.settings);

  /** The switches of the draft, made whole the first time one is set (the core settled them: never null for long). */
  function switches(): Features {
    s.features ??= Object.fromEntries(FEATURES.map((f) => [f, true])) as Features;
    return s.features;
  }

  const lockOf = (f: Feature) => SETTINGS_ROWS.find((r) => r.id === featureRow(f))?.lockedBy ?? null;
  /** What the installation already used when the switches came (`featuresUsed`): « Utilisée sur 2 projets ». */
  const used = $derived(new Map((app.settings.featuresUsed ?? []).map((u) => [u.feature, u.projects])));
  const usedLine = (f: Feature) => {
    const n = used.get(f);
    if (n === undefined) return '';
    return n > 0 ? t('features.page.usedOn', { count: n }) : t('features.page.used');
  };
</script>

<div class="features" role="list">
  <!-- The essential: the agents and the conversation, the Git panel, the terminals, the launches. -->
  <div class="feat essential" role="listitem">
    <span class="ic" aria-hidden="true"><Icon name="agents" /></span>
    <div class="lab">
      <b class="l">{t('features.essential.name')}</b>
      <span class="d">{t('features.essential.desc')}</span>
    </div>
    <div class="ctl" aria-disabled="true">
      <Switch label={t('features.essential.name')} on={true} disabled describedby="feat-essential-desc" />
      <span class="vh" id="feat-essential-desc">{t('features.essential.always')}</span>
    </div>
  </div>
  {#each FEATURES as f (f)}
    {@const lock = lockOf(f)}
    {@const need = NEEDS[f]}
    {@const needOff = !!need && !featureSwitch(s, need)}
    {@const on = featureSwitch(s, f)}
    <div class="feat" class:off={!on || needOff} class:locked={!!lock} role="listitem" data-feature={f} data-row={featureRow(f)}>
      <span class="ic" aria-hidden="true"><Icon name={FEATURE_ICONS[f]} /></span>
      <div class="lab">
        <b class="l"
          >{t(`features.${f}.name`)}{#if usedLine(f)}<span class="used"><Icon name="check" size={12} />{usedLine(f)}</span>{/if}</b
        >
        <span class="d">{t(`features.${f}.desc`)}</span>
        <span class="takes" id="feat-{f}-takes">{t(`features.${f}.takes`)}</span>
        {#if needOff && need}
          <p class="note">{t('features.page.needs', { need: t(`features.${need}.name`) })}</p>
        {/if}
        {#if lock}<p class="lock"><Icon name="lock" size={13} />{lockLine(lock)}</p>{/if}
      </div>
      <div class="ctl" inert={!!lock || needOff} aria-disabled={lock || needOff ? true : undefined}>
        <Switch
          label={t(`features.${f}.name`)}
          describedby="feat-{f}-takes"
          disabled={!!lock || needOff}
          bind:on={() => on, (v) => (switches()[f] = v)}
        />
      </div>
    </div>
  {/each}
</div>
<p class="foot">{t('features.page.foot')}</p>
<div class="review">
  <div class="lab">
    <b class="l">{t('features.tour.review')}</b>
    <span class="d">{t('features.tour.reviewDesc')}</span>
  </div>
  <button class="btn" onclick={() => void reviewTour()}><Icon name="sparkle" size={13} />{t('features.tour.review')}</button>
</div>

<!-- « Ne plus proposer » of a proposal at the right moment (F-3, ESC-558), given back here. -->
<Group title={t('features.declined.title')} note={t('features.declined.desc')} data-testid="declined-offers">
  {#if offers.never.length}
    <div class="declined" role="list">
      {#each offers.never as f (f)}
        <div class="feat" role="listitem" data-declined={f}>
          <span class="ic" aria-hidden="true"><Icon name={FEATURE_ICONS[f]} /></span>
          <div class="lab"><b class="l">{t(`features.${f}.name`)}</b></div>
          <button
            class="btn small ghost"
            aria-label={t('features.declined.restoreLabel', { name: t(`features.${f}.name`) })}
            onclick={() => void restore(f)}>{t('features.declined.restore')}</button
          >
        </div>
      {/each}
    </div>
  {:else}
    <p class="none">{t('features.declined.none')}</p>
  {/if}
</Group>

<style>
  /* The rows of the maquette (design/features/reglages-fonctionnalites.png): a square for the icon, the name and what
     it does, the switch on the right; a hairline between two. */
  .features {
    display: grid;
  }
  .feat {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 0;
  }
  .feat + .feat {
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
  .feat.off .ic {
    color: var(--dim);
  }
  .lab {
    display: grid;
    flex: 1 1 auto;
    gap: 3px;
    min-width: 0;
  }
  .l {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    font-weight: 600;
  }
  .d,
  .takes,
  .note,
  .lock {
    max-width: 68ch;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .takes {
    color: var(--dim);
  }
  .used {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--add);
    font-size: 11.5px;
    font-weight: 500;
  }
  .note {
    color: var(--warn, var(--muted));
  }
  .lock {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .ctl {
    flex: none;
  }
  .ctl[aria-disabled='true'] {
    cursor: default;
  }
  .review {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 18px;
    padding-top: 14px;
    border-top: 1px solid rgba(var(--fg-rgb), 0.06);
  }
  .review .btn {
    flex: none;
    gap: 6px;
  }
  .foot {
    margin: 14px 0 0;
    color: var(--dim);
    font-size: 12px;
  }
  .declined .feat {
    padding: 10px 0;
  }
  .none {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
