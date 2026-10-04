<script lang="ts">
  import type { Report } from '../../lib/escouade';
  import { app } from '../../lib/state.svelte';
  import { canTest, testAgent } from '../../lib/test-launch.svelte';
  import type { Agent, Criterion } from '../../lib/types';

  let { report, criteria = [], agent }: { report: Report; criteria?: Criterion[]; agent: Agent } = $props();

  const met = $derived(report.criteria?.filter((c) => c.ok).length ?? 0);
  const project = $derived(app.projects.find((p) => p.id === agent.projectId));
</script>

<div class="report">
  {#if report.criteria}
    <div class="head">
      <span class="t">Bilan des critères</span>
      <span class="n mono">{met}/{report.criteria.length}</span>
    </div>
    <ul aria-label="Bilan des critères">
      {#each report.criteria as c, i (i)}
        <li class:ok={c.ok}>
          <span class="mark">{c.ok ? '✓' : '○'}</span><span class="text">{criteria[c.n - 1]?.text ?? `Critère ${c.n}`}</span
          >{#if c.note}<span class="note">{c.note}</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}
  {#if report.progress}
    <div class="head"><span class="t">Ce qui a été fait</span></div>
    <ul class="steps" aria-label="Avancement">
      {#each report.progress as p, i (i)}
        <li>{p}</li>
      {/each}
    </ul>
  {/if}
  {#if report.recipe}
    <div class="head"><span class="t">Lancement de test</span></div>
    <!-- A recipe has a step at least: a preparation alone shows as such. -->
    <ul aria-label="Lancement de test">
      {#each report.recipe.prepare as s, i (i)}
        <li><span class="mono name">Préparation</span><span class="mono cmd">{s.command}</span></li>
      {/each}
      {#each report.recipe.processes as p, i (i)}
        <li>
          <span class="mono name">{p.name || `processus ${i + 1}`}</span><span class="mono cmd">{p.command}</span>{#if p.url}<span
              class="mono url">{p.url}</span
            >{/if}
        </li>
      {/each}
    </ul>
    {#if project && canTest(agent)}
      <button class="btn test" onclick={() => testAgent(agent, project)}>▶ Tester</button>
    {/if}
  {/if}
</div>

<style>
  .report {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .t {
    font-size: 12px;
    font-weight: 700;
  }
  .n {
    font-size: 11px;
    color: var(--dim);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
    font-size: 12.5px;
    line-height: 1.45;
    /* Long commands, addresses and notes wrap rather than widen the card. */
    overflow-wrap: anywhere;
  }
  .mark {
    width: 12px;
    flex: none;
    color: var(--dim);
  }
  li.ok .mark {
    color: var(--ok);
  }
  .note {
    color: var(--muted);
    font-size: 11.5px;
  }
  .note::before {
    content: '— ';
  }
  .steps li {
    flex-wrap: nowrap;
    color: var(--muted);
  }
  .steps li::before {
    content: '·';
    width: 12px;
    flex: none;
    text-align: center;
    color: var(--dim);
  }
  .name {
    font-weight: 600;
  }
  .cmd,
  .url {
    color: var(--muted);
    font-size: 11.5px;
  }
  .test {
    align-self: flex-start;
    height: 28px;
    font-size: 12px;
  }
</style>
