<script lang="ts">
  // The viewer of a request's renders (MO-5, `docs/MOBILE.md` § 3.6): one at a time, full width, on a dark ground;
  // « Sombre · Clair » shows the same option in the other theme, the arrows and a swipe go from one to the next, a
  // double touch zooms (the system's pinch too). Its option's letter and label under it.
  import Icon from '$shared/components/Icon.svelte';
  import { isLight, letter } from '$shared/lib/inbox/rules';
  import { t } from '$shared/lib/i18n';
  import type { AskFile } from '$shared/lib/types';
  import { overlay } from '../lib/feed.svelte';
  import { closeSheet } from '../lib/nav.svelte';
  import AskImg from './AskImg.svelte';
  import Screen from './Screen.svelte';

  const v = overlay.viewer!;
  let index = $state(v.index);
  let zoomed = $state(false);
  let startX = 0;

  const file = $derived(v.files[index]);
  const option = $derived(v.ask.options.find((o) => o.id === file?.option) ?? null);
  /** The same option's render in the other theme, when there is one. */
  const twin = $derived(v.files.findIndex((f) => f !== file && f.option === file?.option && isLight(f) !== isLight(file)));

  function go(by: number) {
    zoomed = false;
    index = (index + by + v.files.length) % v.files.length;
  }

  function theme(light: boolean) {
    if (isLight(file) !== light && twin >= 0) index = twin;
  }

  const caption = (f: AskFile | undefined) => (option ? `${letter(option.id)} · ${option.label}` : (f?.name ?? ''));
</script>

<Screen
  title={caption(file)}
  sub={t('mobile.viewer.nth', { n: index + 1, total: v.files.length })}
  label={t('mobile.viewer.label')}
  onClose={closeSheet}
  dark
>
  {#snippet side()}
    {#if twin >= 0}
      <span class="seg" role="group">
        <button type="button" aria-pressed={!isLight(file)} onclick={() => theme(false)}>{t('mobile.viewer.dark')}</button>
        <button type="button" aria-pressed={isLight(file)} onclick={() => theme(true)}>{t('mobile.viewer.light')}</button>
      </span>
    {/if}
  {/snippet}
  <div
    class="stage"
    class:zoomed
    role="img"
    aria-label={caption(file)}
    title={t('mobile.viewer.zoom')}
    ondblclick={() => (zoomed = !zoomed)}
    ontouchstart={(e) => (startX = e.touches[0].clientX)}
    ontouchend={(e) => {
      const dx = e.changedTouches[0].clientX - startX;
      if (!zoomed && Math.abs(dx) > 60) go(dx < 0 ? 1 : -1);
    }}
  >
    {#key file?.file}{#if file}<AskImg space={v.space} ask={v.ask} {file} alt={caption(file)} />{/if}{/key}
  </div>
  {#if option?.description}<p class="desc">{option.description}</p>{/if}
  {#snippet foot()}
    <div class="nav">
      <button type="button" aria-label={t('mobile.viewer.prev')} disabled={v.files.length < 2} onclick={() => go(-1)}
        ><Icon name="left" size={22} /></button
      >
      <button type="button" aria-label={t('mobile.viewer.next')} disabled={v.files.length < 2} onclick={() => go(1)}
        ><Icon name="right" size={22} /></button
      >
    </div>
  {/snippet}
</Screen>

<style>
  .stage {
    margin-top: 8px;
    border-radius: 10px;
    overflow: auto;
    touch-action: pan-y pinch-zoom;
  }
  .stage :global(img) {
    object-fit: contain;
    height: auto;
  }
  .stage.zoomed :global(img) {
    width: 220%;
    max-width: none;
  }
  .desc {
    margin: 10px 0 0;
    font-size: 14px;
    color: #c9cdd3;
  }
  .seg {
    display: inline-flex;
    padding: 3px;
    border-radius: 999px;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.16);
  }
  .seg button {
    min-height: 36px;
    padding: 0 12px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: #c9cdd3;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
  }
  .seg button[aria-pressed='true'] {
    background: #f7f8f8;
    color: #0b0c0e;
  }
  .nav {
    display: flex;
    justify-content: space-between;
  }
  .nav button {
    width: var(--target);
    height: var(--target);
    border: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.08);
    color: #f7f8f8;
  }
  .nav button:disabled {
    opacity: 0.3;
  }
</style>
