<script lang="ts">
  import { conversationOf } from '../lib/conversations.svelte';
  import { trapFocus } from '../lib/focus';
  import { fAgo, plural } from '../lib/format';
  import { api } from '../lib/ipc';
  import { app } from '../lib/state.svelte';
  import type { ConvHit, ConvSearchResult } from '../lib/types';

  // « Rechercher dans les conversations » (Ctrl+K): what the agents' conversations said, found by the backend in their
  // logs; a result opens its agent at the message.

  /** Characters typed before a search starts, and the pause after the last key. */
  const MIN_CHARS = 2;
  const PAUSE_MS = 150;

  let field = $state<HTMLInputElement>();
  let list = $state<HTMLDivElement>();
  let text = $state('');
  // Without a project on screen, there is no "this project".
  let everywhere = $state(!app.project);
  let archived = $state(true);
  let result = $state<ConvSearchResult | null>(null);
  let loading = $state(false);
  let failure = $state<string | null>(null);
  /** The result Enter opens, by its rank among all of them. */
  let active = $state(0);
  /** Only the last search's answer counts. */
  let asked = 0;

  const query = $derived(text.trim());
  const short = $derived([...query].length < MIN_CHARS);
  const hits = $derived(short ? [] : (result?.hits ?? []));

  /** The results agent by agent, in the backend's order (agents are not mixed in it), each with its rank. */
  const groups = $derived.by(() => {
    const out: { agentId: string; name: string; project: string; archived: boolean; hits: { hit: ConvHit; rank: number }[] }[] = [];
    hits.forEach((hit, rank) => {
      let g = out.at(-1);
      if (g?.agentId !== hit.agentId) {
        const a = app.agents[hit.agentId];
        g = {
          agentId: hit.agentId,
          name: a?.name ?? hit.agentName,
          project: app.projects.find((p) => p.id === hit.projectId)?.name ?? '',
          archived: a?.archived ?? hit.archived,
          hits: [],
        };
        out.push(g);
      }
      g.hits.push({ hit, rank });
    });
    return out;
  });

  const status = $derived.by(() => {
    if (short) return 'Cherche dans les messages, les commandes et les fichiers des conversations de tes agents.';
    if (loading && !result) return 'Recherche…';
    if (!hits.length) return 'Aucun message ne correspond.';
    const n = plural(hits.length, 'résultat', 'résultats');
    if (result?.capped) return `Les ${hits.length} premiers résultats : précise ta recherche.`;
    if (result?.timedOut) return `${n}, recherche arrêtée au bout de 5 s : précise ta recherche.`;
    return n;
  });

  $effect(() => {
    const q = query;
    const projectId = everywhere ? null : (app.project?.id ?? null);
    const withArchived = archived;
    const n = ++asked;
    if (short) {
      loading = false;
      return;
    }
    loading = true;
    const timer = setTimeout(() => {
      api
        .searchConversations(q, projectId, withArchived)
        .then((r) => {
          if (n !== asked) return;
          result = r;
          failure = null;
          loading = false;
          active = 0;
        })
        .catch((e) => {
          if (n !== asked) return;
          result = null;
          failure = String(e);
          loading = false;
        });
    }, PAUSE_MS);
    return () => clearTimeout(timer);
  });

  // The chosen result stays in view as the arrows move through a long list.
  $effect(() => {
    void hits;
    list?.querySelector(`#${optionId(active)}`)?.scrollIntoView({ block: 'nearest' });
  });

  function optionId(rank: number) {
    return `conv-hit-${rank}`;
  }

  function close() {
    app.modal = null;
  }

  function move(by: number) {
    const n = hits.length;
    if (n) active = (active + by + n) % n;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      move(e.key === 'ArrowDown' ? 1 : -1);
    } else if (e.key === 'Enter' && !e.isComposing) {
      e.preventDefault();
      const hit = hits[active];
      if (hit) open(hit);
    }
  }

  /**
   * Shows the agent of a result (an archived one in reading, as its card opens it), its conversation rather than the
   * editor, scrolled to the message: asked for before its view is made, which shows it once loaded.
   */
  function open(hit: ConvHit) {
    close();
    const a = app.agents[hit.agentId];
    if (!a) {
      app.toast('Cet agent n’existe plus.');
      return;
    }
    if (a.archived) app.showArchived = true;
    conversationOf(a.id).reveal(hit.itemId);
    app.selectAgent(a.id);
    app.closeEditor(a.projectId);
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={close}>
  <div
    class="palette"
    use:trapFocus
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label="Rechercher dans les conversations"
  >
    <div class="search">
      <span class="ic" aria-hidden="true">⌕</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        bind:this={field}
        bind:value={text}
        autofocus
        spellcheck="false"
        placeholder="Rechercher dans les conversations"
        role="combobox"
        aria-label="Rechercher dans les conversations"
        aria-expanded={hits.length > 0}
        aria-controls="conv-hits"
        aria-autocomplete="list"
        aria-activedescendant={hits.length ? optionId(active) : undefined}
        onkeydown={onKeydown}
      />
      <button class="icon-btn" style="width:28px;height:28px;font-size:16px" onclick={close} aria-label="Fermer">×</button>
    </div>
    <div class="scope">
      <div class="segmented" role="group" aria-label="Où chercher">
        <button class:on={!everywhere} aria-pressed={!everywhere} disabled={!app.project} onclick={() => (everywhere = false)}
          >Ce projet</button
        >
        <button class:on={everywhere} aria-pressed={everywhere} onclick={() => (everywhere = true)}>Tous les projets</button>
      </div>
      <label class="check"><input type="checkbox" bind:checked={archived} />Agents archivés</label>
    </div>
    <div class="list" id="conv-hits" role="listbox" aria-label="Résultats" aria-busy={loading} bind:this={list}>
      {#each groups as g (g.agentId)}
        <div class="group" role="group" aria-label={[g.name, g.project, g.archived ? 'archivé' : ''].filter(Boolean).join(', ')}>
          <div class="who" aria-hidden="true">
            <span class="name">{g.name}</span>
            {#if g.project}<span class="proj">{g.project}</span>{/if}
            {#if g.archived}<span class="tag">archivé</span>{/if}
          </div>
          {#each g.hits as { hit, rank } (rank)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="hit"
              class:on={rank === active}
              id={optionId(rank)}
              role="option"
              tabindex="-1"
              aria-selected={rank === active}
              onclick={() => open(hit)}
              onmousemove={() => (active = rank)}
            >
              <span class="snip"
                >{hit.snippet.slice(0, hit.mark[0])}<mark>{hit.snippet.slice(hit.mark[0], hit.mark[1])}</mark>{hit.snippet.slice(
                  hit.mark[1],
                )}</span
              >
              <span class="when">{fAgo(hit.at / 1000, app.now)}</span>
            </div>
          {/each}
        </div>
      {/each}
    </div>
    <div class="foot">
      {#if failure}
        <span class="status error" role="alert">{failure}</span>
      {:else}
        <span class="status" role="status">{status}</span>
      {/if}
      <div style="flex:1"></div>
      <span class="keys" aria-hidden="true"
        ><kbd>↑</kbd><kbd>↓</kbd> choisir <span class="sep">·</span> <kbd>Entrée</kbd> ouvrir <span class="sep">·</span>
        <kbd>Échap</kbd> fermer</span
      >
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    background: rgba(12, 10, 8, 0.6);
    backdrop-filter: blur(3px);
    animation: fade 0.12s ease-out;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .palette {
    width: 680px;
    max-width: calc(100vw - 40px);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 14px;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55);
    overflow: hidden;
    animation: ccFadeIn 0.15s ease-out;
  }
  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .palette {
      animation: none;
    }
  }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 12px 12px 18px;
    border-bottom: 1px solid var(--line);
  }
  .ic {
    color: var(--dim);
    font-size: 16px;
  }
  .search input {
    flex: 1;
    min-width: 0;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 15px;
    outline: none;
  }
  .search input::placeholder {
    color: var(--dim);
  }
  .scope {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 18px;
    border-bottom: 1px solid var(--line);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: var(--muted);
    cursor: pointer;
  }
  .check input {
    margin: 0;
    accent-color: var(--accent);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 6px 8px;
  }
  .list:empty {
    display: none;
  }
  .group + .group {
    margin-top: 6px;
  }
  .who {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 8px 10px 4px;
    font-size: 11.5px;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
  }
  .who .name {
    font-weight: 700;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    padding: 0 6px;
    border-radius: 99px;
    border: 1px solid var(--line2);
    font-size: 10.5px;
  }
  .hit {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 7px 10px;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .hit.on {
    background: var(--elev2);
  }
  .snip {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hit.on .snip {
    color: var(--text);
  }
  mark {
    background: color-mix(in oklch, var(--wait) 28%, transparent);
    color: var(--text);
    border-radius: 2px;
  }
  .when {
    flex: none;
    font-size: 11px;
    color: var(--dim);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 18px;
    border-top: 1px solid var(--line);
    font-size: 11.5px;
    color: var(--dim);
  }
  .status {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status.error {
    color: var(--del);
  }
  .keys {
    flex: none;
    white-space: nowrap;
  }
  kbd {
    font-family: var(--mono);
    font-size: 10.5px;
    padding: 0 4px;
    margin-right: 2px;
    border: 1px solid var(--line2);
    border-radius: 4px;
    color: var(--muted);
  }
  .sep {
    margin: 0 4px;
  }
</style>
