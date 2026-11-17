<script lang="ts">
  import { t } from '../../lib/i18n';
  import { imagesOf, isLight, letter } from '../../lib/inbox/inbox.svelte';
  import type { AskFile, AskOption, WindowAsk } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import AskImage from './AskImage.svelte';

  // The options of a choice on their plates (docs/INBOX.md § 8.2, the « planches » of an annual's section): each its
  // render on its own ground, its line of credits under it (letter, name, description, how many images), and the
  // agent's recommendation as a seal set half out of its plate, never chosen for the user. `full`: the comparison, the
  // renders of the theme chosen (« Sombre · Clair · Les deux »).
  let {
    ask,
    focus = $bindable(),
    chosen = [],
    full = false,
    theme = 'dark',
    onzoom,
    onpick,
  }: {
    ask: WindowAsk;
    focus?: string;
    chosen?: string[];
    full?: boolean;
    theme?: 'dark' | 'light' | 'both';
    onzoom?: (option: string) => void;
    onpick?: (option: string) => void;
  } = $props();

  /** The renders shown for an option: the dark one (or the first), the light one, or both, by their names. */
  function shown(o: AskOption): AskFile[] {
    const all = imagesOf(ask, o.id);
    if (!all.length) return [];
    const light = all.filter(isLight);
    const dark = all.filter((f) => !isLight(f));
    if (!full) return [dark[0] ?? all[0]];
    if (theme === 'both') return [dark[0], light[0]].filter((x): x is AskFile => !!x);
    if (theme === 'light') return [light[0] ?? dark[0]];
    return [dark[0] ?? light[0]];
  }
  const sealText = $derived(`${t('inbox.ask.seal').toUpperCase()} · `);
</script>

<div
  class="plates"
  class:full
  style:--n={Math.min(ask.options.length, full ? 6 : 3)}
  role="radiogroup"
  aria-label={t('inbox.answer.options')}
>
  {#each ask.options as o (o.id)}
    {@const imgs = shown(o)}
    {@const count = imagesOf(ask, o.id).length}
    {@const rec = ask.recommend?.option === o.id}
    <div class="plate" class:on={focus === o.id || chosen.includes(o.id)} data-option={o.id} data-testid="ask-plate">
      {#if rec}
        <svg class="seal" viewBox="0 0 62 62" role="img" aria-label={t('inbox.ask.seal')}>
          <defs><path id="sealp-{ask.id}-{full ? 'f' : 'p'}" d="M31 31 m-21 0 a21 21 0 1 1 42 0 a21 21 0 1 1 -42 0" /></defs>
          <circle class="bg" cx="31" cy="31" r="29" /><circle class="in" cx="31" cy="31" r="14" />
          <text aria-hidden="true"><textPath href="#sealp-{ask.id}-{full ? 'f' : 'p'}">{sealText}</textPath></text>
          <path class="ck" d="M25 31.5l4 4 8-9" />
        </svg>
      {/if}
      <button
        class="shot"
        class:both={imgs.length > 1}
        role="radio"
        aria-checked={focus === o.id}
        aria-label={`${letter(o.id)} · ${o.label}${rec ? ` · ${t('inbox.ask.recommended')}` : ''}`}
        onclick={() => (focus = o.id)}
        ondblclick={() => onpick?.(o.id)}
      >
        {#if imgs.length}
          {#each imgs as f (f.file)}<AskImage {ask} file={f} alt={f.name} />{/each}
        {:else}
          <span class="none">{t('inbox.compare.noImage')}</span>
        {/if}
      </button>
      {#if onzoom && count}
        <button class="zoom" aria-label={t('inbox.ask.zoom', { label: `${letter(o.id)} · ${o.label}` })} onclick={() => onzoom?.(o.id)}
          ><Icon name="expand" size={12} /></button
        >
      {/if}
      <div class="cred">
        <span class="l">{letter(o.id)}</span>
        <span class="ell"
          ><b>{o.label}</b>{#if o.description}<span class="d"> · {o.description}</span>{/if}</span
        >
        {#if count}<span class="n">{t('inbox.ask.optionImages', { count })}</span>{/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .plates {
    display: grid;
    grid-template-columns: repeat(var(--n), minmax(0, 1fr));
    gap: 14px;
    min-width: 0;
  }
  .plates.full {
    gap: 18px;
    align-content: start;
  }
  .plate {
    position: relative;
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    min-width: 0;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    transition: box-shadow 0.15s;
  }
  .plate.on {
    box-shadow: inset 0 0 0 1.5px var(--brand);
  }
  .shot {
    display: grid;
    gap: 6px;
    align-content: start;
    min-width: 0;
    padding: 8px;
    border: none;
    border-radius: 10px 10px 0 0;
    background: none;
    cursor: pointer;
  }
  .shot:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .full .shot {
    padding: 10px;
  }
  .full .shot.both {
    gap: 8px;
  }
  .shot :global(img) {
    display: block;
    width: 100%;
    height: auto;
    max-height: 260px;
    object-fit: contain;
    border-radius: 6px;
    box-shadow: 0 0 0 1px var(--line);
  }
  .full .shot :global(img) {
    max-height: calc(100vh - 290px);
  }
  .full .shot.both :global(img) {
    max-height: calc((100vh - 300px) / 2);
  }
  .none {
    display: grid;
    place-items: center;
    min-height: 90px;
    color: var(--dim);
    font-size: 12px;
  }
  .zoom {
    position: absolute;
    top: 14px;
    left: 14px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: rgba(8, 9, 10, 0.55);
    color: #fff;
    cursor: pointer;
  }
  .zoom:hover {
    background: rgba(8, 9, 10, 0.75);
  }
  .cred {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 8px 10px 9px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .cred .l {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    flex: none;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--line-3);
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 700;
  }
  .plate.on .cred .l {
    background: var(--brand);
    color: var(--frame);
    box-shadow: none;
  }
  .cred b {
    color: var(--text);
    font-weight: 620;
  }
  .cred .n {
    margin-left: auto;
    flex: none;
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .seal {
    position: absolute;
    top: -16px;
    right: -12px;
    z-index: 2;
    width: 62px;
    height: 62px;
    pointer-events: none;
  }
  .seal .bg {
    fill: var(--panel);
    stroke: var(--brand);
    stroke-width: 1.2;
  }
  .seal .in {
    fill: none;
    stroke: oklch(from var(--brand) l c h / 0.45);
    stroke-width: 0.8;
  }
  .seal text {
    font: 700 6.3px var(--font);
    letter-spacing: 0.16em;
    fill: var(--teal-ink);
  }
  .seal .ck {
    fill: none;
    stroke: var(--teal-ink);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  @media (max-width: 1100px) {
    .plates {
      gap: 10px;
    }
  }
</style>
