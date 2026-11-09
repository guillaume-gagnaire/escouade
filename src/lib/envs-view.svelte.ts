// The view « Environnements » of a project's board (ESC-67, ESC-390): what its header and its lanes share, the
// environments the user shows (kept per project on this machine) and the read under way.

import { NO_PREFS, parsePrefs, type EnvPrefs } from './environments';
import { api } from './ipc';
import { readPref, writePref } from './prefs';
import { app } from './state.svelte';

const key = (projectId: string) => `envs:${projectId}`;

class EnvsView {
  /** The choices of each project once changed here; the others are read from the preferences. */
  private prefs = $state<Record<string, EnvPrefs>>({});
  /** The projects whose deployments are being read now. */
  reading = $state<Record<string, boolean>>({});

  of(projectId: string): EnvPrefs {
    return this.prefs[projectId] ?? (readPref(key(projectId)) ? parsePrefs(readPref(key(projectId))) : NO_PREFS);
  }

  set(projectId: string, p: EnvPrefs) {
    this.prefs[projectId] = p;
    writePref(key(projectId), JSON.stringify(p));
  }

  /** Every choice forgotten here (the tests start from the preferences). */
  forget() {
    this.prefs = {};
  }

  /** « Rafraîchir »: the host read again now. */
  async refresh(projectId: string) {
    if (this.reading[projectId]) return;
    this.reading[projectId] = true;
    try {
      const p = await app.run(api.projectEnvironmentsRead(projectId));
      if (p) app.environments[projectId] = p;
    } finally {
      this.reading[projectId] = false;
    }
  }
}

export const envsView = new EnvsView();
