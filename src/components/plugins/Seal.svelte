<script lang="ts">
  import { t } from '../../lib/i18n';

  // The seal « Vérifié par Escouade » (choices 160 to 162): the logo's spark in a small teal tile, the words, then the
  // version it covers in mono. `long`: « Vérifié par Escouade » (a page's head); `bare`: the version alone (a row of the
  // installed ones). The same on the store and in the app.
  let { version, long = false, bare = false }: { version: string; long?: boolean; bare?: boolean } = $props();
</script>

<span class="seal" aria-label={t('plugins.seal.label', { version })}
  ><i aria-hidden="true">✦</i>{#if !bare}<span aria-hidden="true">{long ? t('plugins.seal.long') : t('plugins.seal.short')}</span>{/if}<code
    aria-hidden="true">{t('plugins.version', { version })}</code
  ></span
>

<style>
  .seal {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 600;
    color: var(--teal-ink);
    padding: 1px 7px 1px 2px;
    border-radius: 5px;
    background: oklch(from var(--brand) l c h / 0.1);
    box-shadow: inset 0 0 0 1px oklch(from var(--brand) l c h / 0.35);
    white-space: nowrap;
    vertical-align: middle;
  }
  .seal i {
    width: 14px;
    height: 14px;
    border-radius: 4px;
    background: var(--brand);
    color: var(--frame);
    font-style: normal;
    font-size: 9px;
    display: grid;
    place-items: center;
  }
  code {
    font:
      10.5px ui-monospace,
      Consolas,
      monospace;
    color: var(--soft);
  }
</style>
