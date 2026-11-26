import { tick } from 'svelte';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { dropConversation } from '../../lib/conversations.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, pullRequest, resetApp } from '../../test/ipc';
import { bg, check, cmd, kit, planWith, planWithRunning, prDetail, questionItem, req, sub } from '../../test/workspace';
import WorkspaceView from './WorkspaceView.svelte';
import { HUD_H } from '../../lib/workspace-camera';

const show = () => render(WorkspaceView, { project: app.projects[0] });
/** The zoom the bar shows (the canvas opens at 100 %). */
const zoomShown = () => document.querySelector('.bar .pct')?.textContent;
/** The world's translation and scale. */
const camOf = () =>
  (document.querySelector('.world') as HTMLElement).style.transform
    .match(/translate\((.+)px, (.+)px\) scale\((.+)\)/)!
    .slice(1)
    .map(Number);

/** Clicks « Zoomer » or « Dézoomer » until the bar says `z` (from 100 %: 80, 64, 51, 41, 33, 30). */
async function zoomTo(z: number) {
  const want = `${Math.round(z * 100)} %`;
  for (let i = 0; i < 20 && zoomShown() !== want; i++) {
    const shown = parseInt(zoomShown() ?? '0');
    await fireEvent.click(screen.getByRole('button', { name: shown < z * 100 ? 'Zoomer' : 'Dézoomer' }));
  }
  expect(zoomShown()).toBe(want);
}

/** The centre on screen of an agent's card, from the world's transform (positions written in `left`/`top` of the world). */
function centreOf(container: HTMLElement, agentId: string): [number, number] {
  const [x, y, z] = camOf();
  const frame = container.querySelector(`div[data-agent="${agentId}"]`) as HTMLElement;
  const slot = frame.querySelector('[data-side="centre"]')!.parentElement as HTMLElement;
  const px = (el: HTMLElement, k: 'left' | 'top' | 'width' | 'height') => parseFloat(el.style[k]);
  const wx = px(frame, 'left') + px(slot, 'left') + px(slot, 'width') / 2;
  const wy = px(frame, 'top') + px(slot, 'top') + px(slot, 'height') / 2;
  return [wx * z + x, wy * z + y];
}

/** The agents' pieces in the order of the DOM, with their place in the world. */
const pieces = (container: HTMLElement) =>
  [...container.querySelectorAll<HTMLElement>('.world [data-agent]')].map((e) => ({
    id: e.dataset.agent,
    x: parseFloat(e.style.left),
    y: parseFloat(e.style.top),
  }));

/** A view of 1200 × 800 for the test that needs one (jsdom lays out nothing). */
function sized(w = 1200, h = 800) {
  const spy = vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(() => w);
  const spyH = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(() => h);
  return () => {
    spy.mockRestore();
    spyH.mockRestore();
  };
}

describe('the workspace', () => {
  beforeEach(() => {
    setLang('fr');
    fakeBackend({ get_conversation: () => [] });
  });
  // An agent that waits has its conversation read: none is kept from a test to the next.
  afterEach(() => ['a1', 'a2', 'a3'].forEach(dropConversation));

  it('draws a compass per agent of the project, archived ones aside', () => {
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'autre' })],
      agents: [
        agent({ id: 'a1', name: 'un' }),
        agent({ id: 'a2', name: 'deux', archived: true }),
        agent({ id: 'a3', name: 'trois', projectId: 'p2' }),
      ],
    });
    const { container } = show();
    expect(pieces(container).map((p) => p.id)).toEqual(['a1']);
  });

  it('hides the agents ready or done on « Masquer les inactifs », and shows them again', async () => {
    resetApp({
      agents: [
        agent({ id: 'a1', status: 'idle' }),
        agent({ id: 'a2', status: 'running' }),
        agent({ id: 'a3', status: 'done' }),
        agent({ id: 'a4', status: 'error' }),
      ],
    });
    const { container } = show();
    const toggle = screen.getByRole('button', { name: 'Masquer les inactifs' });
    await fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-pressed')).toBe('true');
    expect(pieces(container).map((p) => p.id)).toEqual(['a2', 'a4']);
    await fireEvent.click(toggle);
    expect(pieces(container)).toHaveLength(4);
  });

  it('draws two agents running the same tool, in two projects on the canvas of every project', () => {
    const canvas = { tools: [kit('plugin', 'escouade', 1, 1)], subs: [] };
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'autre' })],
      agents: [agent({ id: 'a1', canvas }), agent({ id: 'a2', canvas }), agent({ id: 'a3', projectId: 'p2', canvas })],
    });
    const { container } = render(WorkspaceView, {});
    expect(pieces(container).map((p) => p.id)).toEqual(['a1', 'a2', 'a3']);
    expect(container.querySelectorAll('path.link.run')).toHaveLength(3);
  });

  it('reads its pull request again every minute while a check or a deployment runs, never once all is done', async () => {
    vi.useFakeTimers();
    try {
      const pending = { ...prDetail(), checks: [check('lint', 'pending')] };
      const backend = fakeBackend({ get_conversation: () => [], agent_pr_detail: () => ({ data: pending }) });
      resetApp({ agents: [agent({ id: 'a1', prs: [pullRequest({ number: 41, state: 'open', checks: 'pending' })] })] });
      show();
      await vi.advanceTimersByTimeAsync(0);
      expect(backend.called('agent_pr_detail')).toHaveLength(1);
      await vi.advanceTimersByTimeAsync(60_000);
      expect(backend.called('agent_pr_detail')).toHaveLength(2);

      const deploying = { ...prDetail(), deployments: [{ environment: 'Preview', state: 'in_progress', url: null }] };
      const b2 = fakeBackend({ get_conversation: () => [], agent_pr_detail: () => ({ data: deploying }) });
      resetApp({ agents: [agent({ id: 'a2', prs: [pullRequest({ number: 42, state: 'open', checks: 'success' })] })] });
      show();
      await vi.advanceTimersByTimeAsync(0);
      await vi.advanceTimersByTimeAsync(60_000);
      expect(b2.called('agent_pr_detail').filter((c) => c.args.number === 42)).toHaveLength(2);

      const done = fakeBackend({ get_conversation: () => [], agent_pr_detail: () => ({ data: prDetail() }) });
      resetApp({ agents: [agent({ id: 'a3', prs: [pullRequest({ number: 43, state: 'open', checks: 'success' })] })] });
      show();
      await vi.advanceTimersByTimeAsync(0);
      await vi.advanceTimersByTimeAsync(180_000);
      expect(done.called('agent_pr_detail').filter((c) => c.args.number === 43)).toHaveLength(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it('says when there is no agent', () => {
    resetApp({ agents: [] });
    show();
    expect(screen.getByText('Aucun agent dans ce projet')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Quitter le workspace' })).toBeTruthy();
  });

  it('counts the agents and the questions', () => {
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2', status: 'waiting', requests: [req('q')] })] });
    show();
    expect(screen.getByText('2 agents')).toBeTruthy();
    expect(screen.getByText('1 question')).toBeTruthy();
  });

  it("leaves the wheel to the plan's list while it can scroll that way, else pans", async () => {
    resetApp({ agents: [agent({ id: 'a1', plan: planWith(Array(12).fill('pending')) })] });
    const { container } = show();
    const list = container.querySelector('[data-side="north"] .list') as HTMLElement;
    list.style.overflowY = 'auto'; // jsdom reads no stylesheet
    Object.defineProperty(list, 'scrollHeight', { value: 400 });
    Object.defineProperty(list, 'clientHeight', { value: 232 });
    const row = list.firstElementChild as HTMLElement;
    const wheel = (deltaY: number) => {
      const e = new WheelEvent('wheel', { deltaY, bubbles: true, cancelable: true });
      row.dispatchEvent(e);
      return e.defaultPrevented;
    };
    const before = camOf();
    expect(wheel(50)).toBe(false); // down, room below: the list scrolls
    await tick();
    expect(camOf()).toEqual(before);
    expect(wheel(-50)).toBe(true); // up, at the top: the canvas pans
    await tick();
    expect(camOf()).not.toEqual(before);
  });

  it("places the card under the plan's banner at the height it has, the reader's included", async () => {
    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function (this: HTMLElement) {
      return this.dataset.side === 'north' ? 500 : 0;
    });
    resetApp({ agents: [agent({ id: 'a1', plan: planWith(Array(12).fill('pending')) })] });
    const { container } = show();
    await tick();
    const slot = (side: string) => container.querySelector(`[data-side="${side}"]`)!.parentElement as HTMLElement;
    expect(slot('north').style.height).toBe('500px');
    expect(slot('centre').style.top).toBe(`${500 + 48}px`);
  });

  describe('with the keyboard and a screen reader', () => {
    const FOCUSABLE = 'button:not([disabled]), a[href], input, textarea, select, [tabindex]:not([tabindex="-1"])';

    it('is a region, « Quitter » first, and reaches every card: tools, background tasks, ended subagents, the zone', async () => {
      resetApp({
        agents: [
          agent({
            id: 'a1',
            name: 'un',
            plan: planWith([], { agents: [sub('s1', 'failed')], bgTasks: [bg('b1')] }),
            canvas: { tools: [kit('skill', 'tdd')], subs: [] },
          }),
        ],
      });
      const { container } = show();
      expect(screen.getByRole('region', { name: 'Toile des agents' })).toBeTruthy();
      expect(container.querySelector(FOCUSABLE)!.textContent).toContain('Quitter le workspace');
      for (const sel of ['article.tool', 'article.card.bg', 'article.card.sub', '[data-side="south"] h3'])
        expect(container.querySelector(sel)!.matches(FOCUSABLE), sel).toBe(true);
    });

    it('opens a conversation on Enter, as on a double click', async () => {
      resetApp({ agents: [agent({ id: 'a1', name: 'un', plan: planWith([], { agents: [sub('s1', 'failed')], bgTasks: [bg('b1')] }) })] });
      const { container } = show();
      const at = (sel: string) => container.querySelector(sel) as HTMLElement;
      await fireEvent.keyDown(at('article.card.sub'), { key: 'Enter' });
      expect(app.workspaceOn).toBe(false);
      expect(app.subView.a1).toBe('s1');
      app.openWorkspace('p1');
      await tick();
      await fireEvent.keyDown(at('article.card.bg'), { key: 'Enter' });
      expect(app.taskView.a1).toBe('b1');
      app.openWorkspace('p1');
      await tick();
      await fireEvent.keyDown(at('article.agent'), { key: 'Enter' });
      expect(app.workspaceOn).toBe(false);
      expect(app.ui.selectedAgent.p1).toBe('a1');
    });

    it('says in words who launched an agent, not only on its dotted link', () => {
      const b = agent({ id: 'a2', name: 'deux', status: 'running' });
      const a = agent({ id: 'a1', name: 'un', plan: planWith([], { driven: [{ agent: 'a2', at: 1 }] }) });
      resetApp({ agents: [a, b] });
      const { container } = show();
      expect(container.querySelector('[data-agent="a2"]')!.textContent).toContain('lancé par un');
    });
  });

  it('zooms with its buttons, within bounds', async () => {
    resetApp({ agents: [agent()] });
    show();
    for (let i = 0; i < 20; i++) await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' }));
    expect(screen.getByText('250 %')).toBeTruthy();
    for (let i = 0; i < 30; i++) await fireEvent.click(screen.getByRole('button', { name: 'Dézoomer' }));
    expect(screen.getByText('30 %')).toBeTruthy();
  });

  it('keeps its camera when an agent changes', async () => {
    resetApp({ agents: [agent({ id: 'a1', status: 'running' })] });
    show();
    const opened = zoomShown();
    await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' }));
    const zoomed = zoomShown();
    expect(zoomed).not.toBe(opened);
    app.agents.a1 = { ...app.agents.a1, status: 'waiting' };
    await new Promise((r) => setTimeout(r, 0));
    expect(zoomShown()).toBe(zoomed);
  });

  it('leaves Ctrl and Cmd shortcuts to the app', async () => {
    resetApp({ agents: [agent()] });
    show();
    const opened = zoomShown();
    await fireEvent.keyDown(window, { key: '+', metaKey: true });
    await fireEvent.keyDown(window, { key: '=', ctrlKey: true });
    expect(zoomShown()).toBe(opened);
    await fireEvent.keyDown(window, { key: '+' });
    expect(zoomShown()).not.toBe(opened);
  });

  it('pans with the main button only', async () => {
    resetApp({ agents: [agent()] });
    show();
    const ws = screen.getByRole('region', { name: 'Toile des agents' });
    const world = () => (document.querySelector('.world') as HTMLElement).style.transform;
    // jsdom's pointer events carry no coordinates: mouse events under their names do.
    const ptr = (type: string, init: MouseEventInit) => fireEvent(ws, new MouseEvent(type, { bubbles: true, ...init }));
    const before = world();
    await ptr('pointerdown', { button: 2, clientX: 0, clientY: 0 });
    await ptr('pointermove', { clientX: 50, clientY: 50 });
    await ptr('pointerup', {});
    expect(world()).toBe(before);
    await ptr('pointerdown', { button: 0, clientX: 0, clientY: 0 });
    await ptr('pointermove', { clientX: 50, clientY: 50 });
    expect(world()).not.toBe(before);
  });

  it('fits below its floating bar on « Ajuster », which never covers a card', async () => {
    const restore = sized(1200, 300);
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2' })] });
    show();
    await fireEvent.click(screen.getByRole('button', { name: 'Ajuster' }));
    expect(zoomShown()).not.toBe('100 %');
    expect(camOf()[1]).toBeGreaterThanOrEqual(HUD_H);
    restore();
  });

  it('opens at 100 %, its first agent top left under its bar, and a resize moves nothing', async () => {
    let width = 1200;
    const spy = vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(() => width);
    const spyH = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(() => 800);
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2' })] });
    show();
    expect(zoomShown()).toBe('100 %');
    expect(camOf()).toEqual([32, HUD_H + 32, 1]);
    width = 400;
    await fireEvent(window, new Event('resize'));
    expect(camOf()).toEqual([32, HUD_H + 32, 1]);
    spy.mockRestore();
    spyH.mockRestore();
  });

  it("stacks the agents one under the other, in the sidebar's order", () => {
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2' }), agent({ id: 'a3' })] });
    const { container } = show();
    const ps = pieces(container);
    expect(ps.map((p) => p.id)).toEqual(app.projectAgents.map((a) => a.id));
    expect(new Set(ps.map((p) => p.x))).toEqual(new Set([0]));
    expect(ps.map((p) => p.y)).toEqual([...ps.map((p) => p.y)].sort((a, b) => a - b));
  });
  it('does not close on Escape while a dialog or a popover is open', async () => {
    resetApp({ agents: [agent()] });
    app.openWorkspace('p1');
    show();
    for (const role of ['dialog', 'listbox']) {
      const pop = document.body.appendChild(document.createElement('div'));
      pop.setAttribute('role', role);
      await fireEvent.keyDown(window, { key: 'Escape' });
      expect(app.workspaceOn).toBe(true);
      pop.remove();
    }
  });

  it('closes on « Quitter »', async () => {
    resetApp({ agents: [agent()] });
    app.openWorkspace('p1');
    show();
    await fireEvent.click(screen.getByRole('button', { name: 'Quitter le workspace' }));
    expect(app.workspaceOn).toBe(false);
  });

  it('closes on Escape, but not on Escape in a field', async () => {
    resetApp({ agents: [agent()] });
    app.openWorkspace('p1');
    show();
    const field = document.body.appendChild(document.createElement('input'));
    field.focus();
    await fireEvent.keyDown(field, { key: 'Escape' });
    expect(app.workspaceOn).toBe(true);
    field.remove();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(app.workspaceOn).toBe(false);
  });

  it("shows the compasses in piles below 60 %, down to 30 %, the agent's card kept", async () => {
    resetApp({ agents: [agent({ id: 'a1', name: 'un', canvas: { tools: [kit('skill', 'a')], subs: [] } })] });
    const { container } = show();
    expect(screen.getByRole('article', { name: /Skill a/ })).toBeTruthy();
    await zoomTo(0.3);
    expect(screen.getByRole('button', { name: 'Déplier les outils (1)' })).toBeTruthy();
    expect(screen.getByRole('textbox', { name: 'Écrire à un' })).toBeTruthy();
    expect(container.querySelector('article.tool')).toBeNull();
  });
  it('keeps the agent in the middle where it was when the altitude changes', async () => {
    const restore = sized();
    resetApp({ agents: [agent({ id: 'a1', canvas: { tools: [kit('skill', 'a'), kit('skill', 'b')], subs: [] } }), agent({ id: 'a2' })] });
    const { container } = show();
    await zoomTo(0.64);
    // The middle of the canvas under the bar: the zoom keeps it, and the agent nearest it.
    const [mx, my] = [600, HUD_H + (800 - HUD_H) / 2];
    const near = (id: string) => Math.hypot(...centreOf(container, id).map((v, i) => v - [mx, my][i]));
    const id = near('a1') < near('a2') ? 'a1' : 'a2';
    const [bx, by] = centreOf(container, id);
    await zoomTo(0.51);
    const k = 0.512 / 0.64;
    const [ax, ay] = centreOf(container, id);
    expect(ax).toBeCloseTo(mx + (bx - mx) * k, 0);
    expect(ay).toBeCloseTo(my + (by - my) * k, 0);
    restore();
  });
  it('shows piles at mid-height and the cards up close', async () => {
    resetApp({ agents: [agent({ id: 'a1', canvas: { tools: [kit('skill', 'a'), kit('plugin', 'b')], subs: [] } })] });
    show();
    await zoomTo(0.51);
    expect(screen.getByRole('button', { name: 'Déplier les outils (2)' })).toBeTruthy();
    expect(screen.queryByRole('article', { name: /Skill a/ })).toBeNull();
    await zoomTo(0.8);
    expect(screen.queryByRole('button', { name: 'Déplier les outils (2)' })).toBeNull();
    expect(screen.getByRole('article', { name: /Skill a/ })).toBeTruthy();
  });

  it('unfolds a pile without moving the camera', async () => {
    resetApp({ agents: [agent({ id: 'a1', canvas: { tools: [kit('skill', 'a')], subs: [] } }), agent({ id: 'a2' })] });
    const { container } = show();
    await zoomTo(0.51);
    const before = centreOf(container, 'a1');
    await fireEvent.click(screen.getByRole('button', { name: 'Déplier les outils (1)' }));
    expect(zoomShown()).toBe('51 %');
    // The focus on what came out of the pile, not lost on the body.
    await tick();
    expect(document.activeElement?.closest('[data-agent="a1"] [data-side="west"]')).toBeTruthy();
    expect(screen.getByRole('article', { name: /Skill a/ })).toBeTruthy();
    expect(centreOf(container, 'a1')).toEqual(before);
  });

  it("leaves the card's field alone", async () => {
    resetApp({ agents: [agent({ id: 'a1', name: 'un' })] });
    app.openWorkspace('p1');
    show();
    await zoomTo(1);
    const field = screen.getByRole('textbox', { name: 'Écrire à un' });
    field.focus();
    for (const key of ['+', '-', '0', 'Escape']) await fireEvent.keyDown(field, { key });
    expect(zoomShown()).toBe('100 %');
    expect(app.workspaceOn).toBe(true);
    await fireEvent.pointerDown(field, { clientX: 0, clientY: 0 });
    expect(document.querySelector('.ws.pan')).toBeNull();
  });

  it('animates only the links of what runs', async () => {
    resetApp({ agents: [agent({ id: 'a1', plan: planWith([], { agents: [sub('s1'), sub('s2', 'done')] }) })] });
    const { container } = show();
    await zoomTo(1);
    await fireEvent.click(screen.getByRole('button', { name: /terminé/ }));
    expect(container.querySelectorAll('path.link.run')).toHaveLength(1);
    expect(container.querySelectorAll('path.link.done')).toHaveLength(1);
  });

  it("reaches the tools before the agent's card with the keyboard, and draws a flow only while one runs", async () => {
    resetApp({ agents: [agent({ id: 'a1', name: 'un', canvas: { tools: [kit('skill', 'tdd'), kit('plugin', 'gh', 1, 1)], subs: [] } })] });
    const { container } = show();
    await zoomTo(1);
    const sides = [...container.querySelectorAll('[data-side]')].map((e) => e.getAttribute('data-side'));
    expect(sides.indexOf('west')).toBeGreaterThanOrEqual(0);
    expect(sides.indexOf('west')).toBeLessThan(sides.indexOf('centre'));
    expect(container.querySelectorAll('path.link.run')).toHaveLength(1);
  });

  it('folds the tools past the most recent in a pile that unfolds and folds back', async () => {
    const tools = Array.from({ length: 11 }, (_, i) => ({ ...kit('skill', `t${i}`), lastAt: i }));
    resetApp({ agents: [agent({ id: 'a1', canvas: { tools, subs: [] } })] });
    const { container } = show();
    await zoomTo(1);
    expect(container.querySelectorAll('article.tool')).toHaveLength(8);
    await fireEvent.click(screen.getByRole('button', { name: '+ 3 outils' }));
    expect(container.querySelectorAll('article.tool')).toHaveLength(11);
    await fireEvent.click(screen.getByRole('button', { name: 'Replier les outils' }));
    expect(container.querySelectorAll('article.tool')).toHaveLength(8);
  });

  it('shows the tools of a subagent as tags in its own card', async () => {
    resetApp({
      agents: [
        agent({
          id: 'a1',
          plan: planWith([], { agents: [sub('s1')] }),
          canvas: { tools: [], subs: [{ call: 's1', tools: [kit('skill', 'tdd', 1, 1)] }] },
        }),
      ],
    });
    show();
    await zoomTo(1);
    const tags = screen.getByRole('group', { name: 'Outils de sous-agent s1' });
    expect(tags.textContent).toContain('tdd');
    expect(tags.textContent).toContain('en cours');
    // jsdom lays out nothing: the stylesheet says the tags wrap, the card's height being measured.
    const source = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), 'TeamCard.svelte'), 'utf8');
    expect(source.slice(source.indexOf('  .kits {'), source.indexOf('}', source.indexOf('  .kits {')))).toContain('flex-wrap: wrap');
  });

  it('paints every card and pile of the canvas with one surface, opaque over the links', () => {
    const here = dirname(fileURLToPath(import.meta.url));
    for (const file of readdirSync(here).filter((f) => f.endsWith('.svelte') && f !== 'WorkspaceView.svelte')) {
      const source = readFileSync(resolve(here, file), 'utf8');
      expect(source, file).not.toMatch(/rgba\(var\(--fg-rgb\), 0\.04\)|color-mix\(in oklch, var\(--text\) 6%/);
    }
    expect(readFileSync(resolve(here, 'WorkspaceView.svelte'), 'utf8')).toContain(
      '--ws-card: color-mix(in oklch, var(--text) 6%, var(--panel));',
    );
  });

  it('keeps the touch for the canvas on its background alone: an inset or a list scrolls under a finger', () => {
    const source = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), 'WorkspaceView.svelte'), 'utf8');
    const rule = (sel: string) => source.match(new RegExp(`\\n  ${sel.replace('.', '\\.')} \\{([^}]*)\\}`))?.[1] ?? '';
    expect(rule('.ws')).not.toContain('touch-action');
    expect(rule('.pad')).toContain('touch-action: none');
    resetApp({ agents: [agent()] });
    const { container } = show();
    const pad = container.querySelector('.ws > .pad')!;
    expect(pad.nextElementSibling?.matches('.world')).toBe(true);
  });

  describe('an agent that waits', () => {
    it('is unfolded at every zoom, the others in piles', async () => {
      fakeBackend({ get_conversation: () => [questionItem('q1')] });
      const tools = { tools: [kit('skill', 'a')], subs: [] };
      resetApp({
        agents: [
          agent({ id: 'a1', name: 'un', canvas: tools }),
          agent({ id: 'a2', name: 'deux', status: 'waiting', pending: ['q1'], requests: [req('q1')], canvas: tools }),
        ],
      });
      show();
      await zoomTo(0.3);
      expect(await screen.findByText('Question q1 ?')).toBeTruthy();
      expect(screen.getAllByRole('button', { name: 'Déplier les outils (1)' })).toHaveLength(1);
      expect(screen.getAllByRole('article', { name: /Skill a/ })).toHaveLength(1);
      expect(screen.getByRole('textbox', { name: 'Écrire à deux' })).toBeTruthy();
    });

    it('has its conversation read, the others not', async () => {
      const backend = fakeBackend({ get_conversation: () => [questionItem('q1')] });
      resetApp({
        agents: [agent({ id: 'a1', status: 'running' }), agent({ id: 'a2', status: 'waiting', pending: ['q1'], requests: [req('q1')] })],
      });
      show();
      await screen.findByText('Question q1 ?');
      await zoomTo(1);
      await screen.findByText('Question q1 ?');
      expect(new Set(backend.called('get_conversation').map((c) => c.args.id))).toEqual(new Set(['a2']));
    });

    it("counts its subagents' questions too", () => {
      fakeBackend({ get_conversation: () => [questionItem('q1', { sub: 's1' })] });
      resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] });
      show();
      expect(screen.getByText('1 question')).toBeTruthy();
    });

    it('is folded back once answered', async () => {
      fakeBackend({ get_conversation: () => [questionItem('q1')] });
      const tools = { tools: [kit('skill', 'a')], subs: [] };
      resetApp({
        agents: [agent({ id: 'a2', name: 'deux', status: 'waiting', pending: ['q1'], requests: [req('q1')], canvas: tools })],
      });
      show();
      await zoomTo(0.3);
      await screen.findByText('Question q1 ?');
      expect(screen.queryByRole('button', { name: 'Déplier les outils (1)' })).toBeNull();
      app.agents.a2 = { ...app.agents.a2, status: 'running', pending: [], requests: [] };
      await new Promise((r) => setTimeout(r, 0));
      expect(screen.getByRole('button', { name: 'Déplier les outils (1)' })).toBeTruthy();
    });

    it('shows the request of a subagent folded in the pile on the agent card', async () => {
      fakeBackend({ get_conversation: () => [questionItem('q1', { sub: 's1' })] });
      const plan = planWith([], { agents: [{ ...sub('s1', 'done'), kind: 'Explore' }] });
      resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting', pending: ['q1'], requests: [req('q1')], plan })] });
      show();
      expect(await screen.findByText('Question q1 ?')).toBeTruthy();
      expect(screen.getByText('question de Explore')).toBeTruthy();
    });

    it('caps its inset on screen: zooming in lowers it, and it holds still', async () => {
      fakeBackend({ get_conversation: () => [questionItem('q1')] });
      resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] });
      const { container } = show();
      await screen.findByText('Question q1 ?');
      const capAt = () => parseFloat((container.querySelector('.ask') as HTMLElement).style.maxHeight);
      const scale = () => Number((container.querySelector('.world') as HTMLElement).style.transform.match(/scale\((.+)\)/)![1]);
      const want = () => Math.max(80, Math.round(Math.round(0.7 * window.innerHeight) / scale()) - 128);
      await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' }));
      const before = capAt();
      expect(before).toBe(want());
      await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' }));
      expect(capAt()).toBe(want());
      expect(capAt()).toBeLessThan(before);
      const zoom = zoomShown();
      await new Promise((r) => setTimeout(r, 20));
      expect(zoomShown()).toBe(zoom);
      expect(capAt()).toBe(want());
    });

    it('is tabbed to in its place in the column', async () => {
      fakeBackend({ get_conversation: () => [questionItem('q1')] });
      resetApp({
        agents: [
          agent({ id: 'a1', name: 'un' }),
          agent({ id: 'a2', name: 'deux', status: 'waiting', pending: ['q1'], requests: [req('q1')] }),
          agent({ id: 'a3', name: 'trois' }),
        ],
      });
      const { container } = show();
      await zoomTo(0.3);
      await screen.findByText('Question q1 ?');
      // The order of the DOM is the order on screen: top to bottom.
      const ps = pieces(container);
      expect(ps).toHaveLength(3);
      expect(ps.map((p) => p.id)).toEqual([...ps].sort((p, q) => p.y - q.y).map((p) => p.id));
    });
  });

  describe('the test cards', () => {
    it('draws the test card right after its card, in the keyboard order', async () => {
      resetApp({ agents: [agent({ id: 'a1', canvas: { tools: [], subs: [], commands: [cmd('c0', 'cargo test')] } })] });
      const { container } = show();
      await zoomTo(1);
      const cards = [...container.querySelectorAll('[data-agent="a1"] [data-side]')];
      const at = cards.findIndex((e) => e.getAttribute('aria-label')?.startsWith('Tests'));
      expect(at).toBeGreaterThan(0);
      expect(cards[at - 1].getAttribute('data-side')).toBe('centre');
    });
  });

  describe('the landing zone', () => {
    it('shows the landing zone south of every compass', async () => {
      resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2' })] });
      const { container } = show();
      await zoomTo(1);
      expect(screen.getAllByText('Rien n’a encore atterri')).toHaveLength(2);
      const sides = [...container.querySelectorAll('[data-agent="a1"] [data-side]')].map((e) => e.getAttribute('data-side'));
      expect(sides.at(-1)).toBe('south');
    });
  });

  describe('of every project', () => {
    const showAll = () => render(WorkspaceView);

    it('sets the projects side by side in the order of the rail, each under its name, archived agents aside', () => {
      resetApp({
        projects: [project(), project({ id: 'p2', name: 'studio-web' }), project({ id: 'p3', name: 'vide' })],
        agents: [
          agent({ id: 'a1', name: 'un' }),
          agent({ id: 'a2', name: 'deux', projectId: 'p2' }),
          agent({ id: 'a3', name: 'trois', projectId: 'p2' }),
          agent({ id: 'a4', name: 'quatre', projectId: 'p2', archived: true }),
        ],
      });
      const { container } = showAll();
      expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent?.trim())).toEqual(['demo-api', 'studio-web']);
      const ps = pieces(container);
      expect(ps.map((p) => p.id)).toEqual(['a1', 'a2', 'a3']);
      expect(ps[1].x).toBeGreaterThan(ps[0].x);
      expect(ps[2].x).toBe(ps[1].x);
      expect(ps[2].y).toBeGreaterThan(ps[1].y);
      expect(screen.getByText('3 agents')).toBeTruthy();
      expect(zoomShown()).toBe('100 %');
    });

    it('goes back to « Vue d’ensemble » on « Quitter » and on Escape', async () => {
      resetApp({ agents: [agent()] });
      app.openOverviewCanvas();
      showAll();
      await fireEvent.click(screen.getByRole('button', { name: 'Quitter le workspace' }));
      expect(app.overviewCanvasOn).toBe(false);
      expect(app.ui.view).toBe('overview');
      app.openOverviewCanvas();
      await fireEvent.keyDown(window, { key: 'Escape' });
      expect(app.overviewCanvasOn).toBe(false);
    });

    it("follows each project's deployments, and runs each agent's tests by its own project's command", async () => {
      const backend = fakeBackend({ get_conversation: () => [] });
      resetApp({
        projects: [project(), project({ id: 'p2', name: 'studio-web', board: { ...project().board, testCommand: 'make verif' } })],
        agents: [
          agent({ id: 'a1' }),
          agent({ id: 'a2', projectId: 'p2', canvas: { tools: [], subs: [], commands: [cmd('c0', 'make verif')] } }),
        ],
      });
      const { container } = showAll();
      await new Promise((r) => setTimeout(r, 0));
      expect(new Set(backend.called('project_environments').map((c) => c.args.projectId))).toEqual(new Set(['p1', 'p2']));
      expect(container.querySelector('[data-agent="a2"] [aria-label^="Tests"]')).toBeTruthy();
    });
  });
});
