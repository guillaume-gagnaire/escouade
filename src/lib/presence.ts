// What an automatic restart for an update waits for, as only the window knows it: a modal open, a test launch
// being prepared, the user at the keyboard or the mouse. The backend decides (updates.rs); the window tells it.

import { api } from './ipc';
import { app } from './state.svelte';
import { flows } from './test-launch.svelte';

const EVERY_MS = 5000;
const INPUTS = ['keydown', 'pointerdown', 'pointermove', 'wheel'] as const;

/** Tells the backend every few seconds, when it changed. Returns what stops it. */
export function watchPresence(): () => void {
  let activeAt = Date.now();
  let sent = '';
  /** The restart whose countdown was told of the user's return already. */
  let hurried: number | null = null;
  const onInput = () => {
    activeAt = Date.now();
    // During a restart's countdown, the user back at the window is told at once: up to 5 s later, the restart could
    // come right under their hands. Once per countdown: its first input calls it off.
    if (app.restartAt !== null && app.restartAt !== hurried) {
      hurried = app.restartAt;
      report();
    }
  };
  // Caught on the way down: a component that stops an event still had the user's hand on it.
  for (const e of INPUTS) window.addEventListener(e, onInput, { capture: true, passive: true });
  const report = () => {
    const presence = {
      modal: !!app.modal,
      testing: Object.values(flows.all).some((f) => f.phase === 'running'),
      activeAt,
    };
    const key = JSON.stringify(presence);
    if (key === sent) return;
    sent = key;
    // Not told: told again at the next report.
    api.updatePresence(presence).catch(() => (sent = ''));
  };
  report();
  const timer = setInterval(report, EVERY_MS);
  return () => {
    clearInterval(timer);
    for (const e of INPUTS) window.removeEventListener(e, onInput, { capture: true });
  };
}
