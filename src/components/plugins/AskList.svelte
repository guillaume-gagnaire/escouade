<script lang="ts">
  import { t } from '../../lib/i18n';
  import type { Ask } from '../../lib/plugins';
  import Icon from '../Icon.svelte';

  // What a plugin may do, a line per permission, the strongest first (§3.4): the program in bold, the agents'
  // permissions in red. `grown`: the words new since the installed version, marked. `revoked` and `onToggle`: each line
  // gets its switch (the permissions tab), but the program, which is switched off with the plugin.
  let {
    asks,
    grown = [],
    revoked,
    onToggle,
  }: {
    asks: Ask[];
    grown?: string[];
    revoked?: string[];
    onToggle?: (word: string, granted: boolean) => void;
  } = $props();
</script>

<ul class="asks">
  {#each asks as a (a.word)}
    {@const off = revoked?.includes(a.word) ?? false}
    <li class:strong={a.strong} class:red={a.red} class:off>
      <Icon name={a.icon} size={14} />
      <span class="txt"
        >{a.text}{#if a.detail}<small class="mono">{a.detail}</small>{/if}{#if a.word.startsWith('network:')}<small
            >{t('plugins.perms.networkNote')}</small
          >{/if}{#if onToggle && a.word === 'run'}<small>{t('plugins.perms.runNote')}</small>{/if}</span
      >
      {#if grown.includes(a.word)}<span class="new">{t('plugins.install.grownMark')}</span>{/if}
      {#if onToggle && a.word !== 'run'}
        <button
          class="switch"
          class:on={!off}
          role="switch"
          aria-checked={!off}
          aria-label={t('plugins.perms.granted', { text: a.detail ? `${a.text} ${a.detail}` : a.text })}
          onclick={() => onToggle(a.word, off)}
        ></button>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .asks {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 8px;
  }
  li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto auto;
    gap: 8px;
    align-items: start;
    font-size: 12.5px;
    color: var(--soft);
  }
  li :global(.icon) {
    margin-top: 2px;
    color: var(--muted);
  }
  li.strong .txt {
    color: var(--text);
    font-weight: 600;
  }
  li.red .txt,
  li.red :global(.icon) {
    color: var(--del);
  }
  li.off .txt {
    color: var(--muted);
    text-decoration: line-through;
    text-decoration-color: var(--line-3);
  }
  small {
    display: block;
    margin-top: 2px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 400;
  }
  small.mono {
    font-size: 11px;
  }
  .new {
    align-self: center;
    font-size: 10.5px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 5px;
    color: var(--warn-ink);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .switch {
    align-self: center;
  }
</style>
