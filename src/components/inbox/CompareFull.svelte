<script lang="ts">
  import { trapFocus } from '../../lib/focus';
  import type { ViewedImage } from '../../lib/gallery';
  import { t } from '../../lib/i18n';
  import { imagesOf, inbox, isLight, letter } from '../../lib/inbox/inbox.svelte';
  import type { WindowAsk } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import ImageViewer from '../conv/ImageViewer.svelte';
  import ChoicePlates from './ChoicePlates.svelte';

  // « Comparer en grand » (docs/INBOX.md § 8.2, `a-choix.png`): full window, the options side by side on their plates, the
  // seal of the recommendation; « Sombre · Clair · Les deux » when the agent joined both; 1 2 3 choose, ← → move from one
  // option to the next, Espace shows the one in focus alone (the viewer and its zoom), Échap closes. « Un mot pour
  // l'agent » and « Choisir B » stay at the bottom: one answers without going back.
  let {
    ask,
    focus = $bindable(),
    note = $bindable(''),
    onchoose,
    onclose,
  }: { ask: WindowAsk; focus?: string; note?: string; onchoose: (option: string) => void; onclose: () => void } = $props();

  const both = $derived(ask.options.some((o) => imagesOf(ask, o.id).some(isLight) && imagesOf(ask, o.id).some((f) => !isLight(f))));
  let theme = $state<'dark' | 'light' | 'both'>('dark');
  let viewing = $state<{ images: ViewedImage[]; index: number } | null>(null);
  const chosen = $derived(ask.options.find((o) => o.id === focus));

  async function alone(id: string) {
    const files = imagesOf(ask, id);
    if (!files.length) return;
    const images = await Promise.all(
      files.map(async (f) => ({
        name: f.name,
        url: await inbox.imageUrl(ask, f),
        source: { kind: 'ask' as const, id: ask.id, file: f.file },
      })),
    );
    viewing = { images, index: 0 };
  }

  function onKey(e: KeyboardEvent) {
    if (viewing) return;
    const inField = (e.target as HTMLElement).closest('input, textarea');
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
      return;
    }
    if (e.key === 'Enter' && !e.shiftKey && focus) {
      e.preventDefault();
      e.stopPropagation();
      onchoose(focus);
      return;
    }
    if (inField || e.ctrlKey || e.metaKey || e.altKey) return;
    const i = ask.options.findIndex((o) => o.id === focus);
    const n = Number(e.key);
    if (n >= 1 && n <= ask.options.length) focus = ask.options[n - 1].id;
    else if (e.key === 'ArrowRight') focus = ask.options[Math.min(ask.options.length - 1, i + 1)].id;
    else if (e.key === 'ArrowLeft') focus = ask.options[Math.max(0, i - 1)].id;
    else if (e.key === ' ' && focus) void alone(focus);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<div class="compare" role="presentation">
  <div
    class="box"
    role="dialog"
    aria-modal="true"
    aria-label={t('inbox.compare.title')}
    tabindex="-1"
    use:trapFocus
    onkeydown={onKey}
    data-testid="compare"
  >
    <header class="ch">
      <Icon name="choice" />
      <h2 class="ell">{ask.title}</h2>
      <span class="sp"></span>
      {#if both}
        <div class="seg" role="radiogroup" aria-label={t('inbox.compare.theme')}>
          {#each ['dark', 'light', 'both'] as const as v (v)}
            <button role="radio" aria-checked={theme === v} class:on={theme === v} onclick={() => (theme = v)}
              >{t(`inbox.compare.${v}`)}</button
            >
          {/each}
        </div>
      {/if}
      <button class="btn icon-only" aria-label={t('inbox.compare.close')} onclick={onclose} data-testid="compare-close"
        ><Icon name="close" /></button
      >
    </header>
    <div class="cb">
      <ChoicePlates {ask} bind:focus full {theme} onpick={onchoose} />
    </div>
    <footer class="cf">
      <p class="keys">{t('inbox.compare.keys')}</p>
      <input
        class="field"
        placeholder={t('inbox.answer.noteFor', { agent: ask.author.name })}
        aria-label={t('inbox.answer.note')}
        bind:value={note}
      />
      <button class="btn primary" disabled={!focus} onclick={() => focus && onchoose(focus)} data-testid="compare-choose">
        {t('inbox.answer.choose', { option: chosen ? letter(chosen.id) : '' })}<kbd>{t('inbox.answer.enter')}</kbd>
      </button>
    </footer>
  </div>
</div>

{#if viewing}
  <ImageViewer images={viewing.images} bind:index={viewing.index} onclose={() => (viewing = null)} />
{/if}

<style>
  .compare {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    padding: 22px;
    background: var(--scrim);
  }
  .box {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    min-height: 0;
    overflow: hidden;
    border-radius: 14px;
    background: var(--panel);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 24px 64px -12px rgba(0, 0, 0, calc(0.75 * var(--shade)));
    outline: none;
  }
  .ch {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 12px 14px 12px 18px;
    border-bottom: 1px solid var(--line);
  }
  .ch h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 640;
  }
  .sp {
    flex: 1;
  }
  .seg {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .seg button {
    height: 24px;
    padding: 0 9px;
    border: none;
    border-radius: 7px;
    background: none;
    color: var(--muted);
    font: 550 12px/1 var(--font);
    cursor: pointer;
  }
  .seg button.on {
    background: var(--raise-2);
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .cb {
    min-height: 0;
    padding: 26px 22px 12px;
    overflow: auto;
  }
  .cf {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 12px 14px;
    border-top: 1px solid var(--line);
  }
  .keys {
    flex: 0 1 auto;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    color: var(--dim);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .field {
    flex: 1;
    min-width: 160px;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: 12.5px var(--font);
  }
  .btn kbd {
    margin-left: 6px;
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (max-width: 1100px) {
    .compare {
      padding: 12px;
    }
    .keys {
      display: none;
    }
  }
</style>
