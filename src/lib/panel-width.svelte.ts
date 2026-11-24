// The width of the side panel (ESC-659: Non commités, Historique, Workflow, PR et CI): its left edge dragged, as the
// rail's and the sidebar's right edges are, the same for all projects, kept in the app's settings. Two widths, as it has
// two forms: opened beside the conversation (`panelWidth`, usually 340 px), and docked side by side (`splitPanelWidth`,
// usually 44 % of the work). Never under 340 px, which holds its tabs; never so wide that the conversation beside it has
// less than a window of 1 024 px left it before ESC-659. A narrower window draws it narrower, the width chosen kept.

import { api } from './ipc';
import { railWidth } from './rail-width.svelte';
import { clamp } from './resize';
import { sideWidth } from './side-width.svelte';
import { app } from './state.svelte';

/** The narrowest: its four tabs and its × (`@container git-side` packs them under 420 px). */
export const PANEL_MIN = 340;
/** The widest kept, whatever the screen. */
export const PANEL_MAX = 1200;
/** Its usual width beside the conversation. */
export const PANEL_USUAL = 340;
/** Docked side by side, its usual share of the work, as the design lays it out. */
export const SPLIT_SHARE = 0.44;
/** What the conversation keeps beside it: what a window of 1 024 px left it (1024 − 52 − 276 − 340). */
export const ROOM_FOR_CONVERSATION = 356;
/** How long after the edge is let go the width is saved: a few presses of an arrow are one save. */
export const SAVE_DELAY_MS = 400;

/** The widest the panel is drawn in a work `room` px wide. */
export const panelRoom = (room: number) => clamp(room - ROOM_FOR_CONVERSATION, PANEL_MIN, PANEL_MAX);

/** Its usual width in a work `room` px wide, docked or not. */
export const panelUsual = (docked: boolean, room: number) => (docked ? Math.max(PANEL_MIN, room * SPLIT_SHARE) : PANEL_USUAL);

/** The width drawn: the one chosen (else the usual one), narrower when the work has no room for it. */
export const panelShown = (chosen: number | null, docked: boolean, room: number) =>
  clamp(chosen ?? panelUsual(docked, room), PANEL_MIN, panelRoom(room));

/** The width the settings keep, null for the usual one (0, absent, or not a width). */
export function chosenPanel(kept: number | undefined): number | null {
  return kept && Number.isFinite(kept) && kept > 0 ? clamp(Math.round(kept), PANEL_MIN, PANEL_MAX) : null;
}

/** The work's width the window gives, before the panel's row is measured: the window less the rail and the sidebar. */
export const workGuess = () => railWidth.window - railWidth.shown - sideWidth.shown;

const key = (docked: boolean) => (docked ? 'splitPanelWidth' : 'panelWidth');

let saveTimer: ReturnType<typeof setTimeout> | undefined;

class PanelWidth {
  /** Chosen by its edge, docked or not; null for the usual width. */
  chosen(docked: boolean) {
    return chosenPanel(app.settings[key(docked)]);
  }

  /** The edge moving: the width follows, saved once it is let go. */
  set(docked: boolean, width: number) {
    app.settings[key(docked)] = Math.round(width);
  }
  commit(docked: boolean, width: number) {
    this.set(docked, width);
    this.#save();
  }
  /** A double click on the edge: the usual width again. */
  reset(docked: boolean) {
    app.settings[key(docked)] = 0;
    this.#save();
  }

  #save() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = undefined;
      app.run(api.saveSettings($state.snapshot(app.settings)));
    }, SAVE_DELAY_MS);
  }
}

export const panelWidth = new PanelWidth();
