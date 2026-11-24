// The width of the side panel (ESC-659): its left edge dragged as the sidebar's right edge is, kept in the settings,
// beside the conversation and docked side by side each its own, held to the room its row leaves the conversation.
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import {
  chosenPanel,
  PANEL_MAX,
  PANEL_MIN,
  PANEL_USUAL,
  panelRoom,
  panelShown,
  panelUsual,
  ROOM_FOR_CONVERSATION,
} from '../lib/panel-width.svelte';
import { railWidth } from '../lib/rail-width.svelte';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import '../test/pointer';
import SidePanel from './SidePanel.svelte';

const edge = () => screen.getByRole('separator', { name: 'Largeur du panneau' });
const panel = () => document.querySelector<HTMLElement>('aside.git-side')!;
const saved = (backend: ReturnType<typeof fakeBackend>, key: 'panelWidth' | 'splitPanelWidth') =>
  backend.called('save_settings').at(-1)?.args.settings[key];

describe('the side panel’s width', () => {
  it('is 340 px beside the conversation, 44 % of the work docked, held to what the conversation keeps', () => {
    expect(ROOM_FOR_CONVERSATION).toBe(1024 - 52 - 276 - 340);
    expect(panelShown(null, false, 1112)).toBe(PANEL_USUAL);
    // 44 % exactly, as the design lays it out: not rounded.
    expect(panelShown(null, true, 1112)).toBeCloseTo(489.28);
    expect(panelUsual(true, 600)).toBe(PANEL_MIN);
    expect(panelShown(600, false, 1112)).toBe(600);
    expect(panelShown(900, false, 1112)).toBe(1112 - ROOM_FOR_CONVERSATION);
    expect(panelShown(10, true, 1112)).toBe(PANEL_MIN);
    // Never under its narrowest, whatever the room: its tabs hold.
    expect(panelRoom(500)).toBe(PANEL_MIN);
    expect(panelShown(600, false, 696)).toBe(PANEL_MIN);
    expect(panelRoom(4000)).toBe(PANEL_MAX);
  });

  it('reads the width kept in the settings, and nothing that is not one', () => {
    for (const [kept, read] of [
      [420, 420],
      [400.6, 401],
      [9000, PANEL_MAX],
      [10, PANEL_MIN],
      [0, null],
      [undefined, null],
      [NaN, null],
      [-4, null],
    ] as const)
      expect(chosenPanel(kept)).toBe(read);
  });
});

describe('the side panel’s left edge', () => {
  beforeEach(() => {
    railWidth.window = 1440;
    resetApp({ projects: [project()], agents: [agent()] });
  });

  it('is dragged left to widen, moved by the keyboard, kept, and a double click gives its usual width back', async () => {
    const backend = fakeBackend();
    render(SidePanel, { project: project(), agent: app.agents.a1 });
    const e = edge();
    // The work beside the rail (52) and the sidebar (276): 1 112 px, the conversation keeping its 356.
    const widest = 1112 - ROOM_FOR_CONVERSATION;
    expect(panel().style.width).toBe('340px');
    expect(e).toHaveAttribute('aria-orientation', 'vertical');
    expect(e).toHaveAttribute('aria-valuenow', '340');
    expect(e).toHaveAttribute('aria-valuemin', String(PANEL_MIN));
    expect(e).toHaveAttribute('aria-valuemax', String(widest));
    e.focus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(panel().style.width).toBe('356px');
    await waitFor(() => expect(saved(backend, 'panelWidth')).toBe(356));
    await userEvent.keyboard('{End}');
    expect(panel().style.width).toBe(`${widest}px`);
    await userEvent.keyboard('{Home}');
    expect(panel().style.width).toBe('340px');
    // Dragged left: it grows, kept once let go, once.
    const saves = backend.called('save_settings').length;
    await fireEvent.pointerDown(e, { button: 0, pointerId: 1, clientX: 1100 });
    await fireEvent.pointerMove(e, { pointerId: 1, buttons: 1, clientX: 1000 });
    expect(panel().style.width).toBe('440px');
    await fireEvent.pointerUp(e, { pointerId: 1 });
    await waitFor(() => expect(saved(backend, 'panelWidth')).toBe(440));
    expect(backend.called('save_settings')).toHaveLength(saves + 1);
    // A narrow window draws it narrower, the width chosen kept and given back with the room.
    railWidth.window = 1024;
    await waitFor(() => expect(panel().style.width).toBe('340px'));
    expect(app.settings.panelWidth).toBe(440);
    railWidth.window = 1440;
    await waitFor(() => expect(panel().style.width).toBe('440px'));
    // Double click: the usual width.
    await fireEvent.dblClick(e);
    expect(panel().style.width).toBe('340px');
    await waitFor(() => expect(saved(backend, 'panelWidth')).toBe(0));
  });

  it('docked side by side, has its own width, read back from the settings', async () => {
    fakeBackend();
    app.settings.panelWidth = 400;
    app.settings.splitPanelWidth = 600;
    const { unmount } = render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(panel().style.width).toBe('600px');
    await fireEvent.dblClick(edge());
    expect(parseFloat(panel().style.width)).toBeCloseTo(489.28);
    expect(app.settings.splitPanelWidth).toBe(0);
    unmount();
    render(SidePanel, { project: project(), agent: app.agents.a1 });
    expect(panel().style.width).toBe('400px');
  });
});
