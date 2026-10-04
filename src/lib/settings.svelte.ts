// The settings modal: one draft of all it sets (the app's settings, each project's own and its
// board's), saved at once by « Enregistrer » and dropped by « Annuler ».

import { api } from './ipc';
import { forgetLaunches } from './launch-actions';
import { app } from './state.svelte';
import type { BoardSettings, Project, ProjectIntegrations, RunCommand, Settings } from './types';

export type SettingsTab = 'claude' | 'notifications' | 'projects' | 'board' | 'integrations' | 'terminals' | 'network' | 'about';

export const SETTINGS_TABS: { id: SettingsTab; label: string; icon: string; desc: string; scoped?: boolean }[] = [
  { id: 'claude', label: 'Claude Code', icon: '✳', desc: 'Exécutable, modèle et permissions par défaut' },
  { id: 'notifications', label: 'Notifications', icon: '♪', desc: 'Alertes visuelles et sonores' },
  { id: 'projects', label: 'Projets', icon: '▤', desc: 'Réglages propres à chaque projet', scoped: true },
  { id: 'board', label: 'Kanban', icon: '▦', desc: 'Pilote auto et tickets validés', scoped: true },
  { id: 'integrations', label: 'Intégrations', icon: '⧉', desc: 'Jira, Trello et GitHub Issues', scoped: true },
  { id: 'terminals', label: 'Terminaux', icon: '$_', desc: 'Shells disponibles dans les terminaux intégrés' },
  { id: 'network', label: 'Réseau', icon: '⇄', desc: 'Proxy HTTP(S) pour Claude Code et les mises à jour' },
  { id: 'about', label: 'À propos', icon: 'ⓘ', desc: 'Version et données locales' },
];

/** The app's settings each tab sets. */
const SETTINGS_OF: Partial<Record<SettingsTab, (keyof Settings)[]>> = {
  claude: ['claudePath', 'defaultModel', 'defaultEffort', 'defaultMode', 'autoResume', 'idleStopMinutes'],
  notifications: ['sound', 'osNotifications'],
  terminals: ['pwshPath', 'bashPath', 'wslDistro'],
  network: ['proxyUrl', 'noProxy', 'proxyTerminals'],
  integrations: ['integrations'],
};

/** What the modal sets of a project, as the backend's `update_project` takes it, its board apart. */
type ProjectFields = Pick<Project, 'name' | 'color' | 'worktreePerAgent' | 'runCommands' | 'worktreeCopy'>;

export interface ProjectDraft {
  name: string;
  color: string;
  worktreePerAgent: boolean;
  runCommands: RunCommand[];
  /** The patterns of the files copied into new worktrees, one per line. */
  copy: string;
  board: BoardSettings;
  /** Its sources in external ticket systems (the « Intégrations » tab). */
  integrations: ProjectIntegrations;
}

function draftOf(p: Project): ProjectDraft {
  const { name, color, worktreePerAgent, runCommands, worktreeCopy, board, integrations } = $state.snapshot(p);
  return { name, color, worktreePerAgent, runCommands, copy: worktreeCopy.join('\n'), board, integrations };
}

/** The links as saved: one without a container is none, a column is commented once. */
function linksOf(d: ProjectDraft): ProjectIntegrations {
  return { links: d.integrations.links.filter((l) => l.container.trim()), comments: [...new Set(d.integrations.comments)] };
}

/** The draft as it would be saved: trimmed, the patterns split into lines. */
function fieldsOf(d: ProjectDraft): ProjectFields {
  return {
    name: d.name.trim(),
    color: d.color,
    worktreePerAgent: d.worktreePerAgent,
    runCommands: d.runCommands.map((c) => ({ ...c, name: c.name.trim(), command: c.command.trim(), cwd: c.cwd.trim() })),
    worktreeCopy: d.copy
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean),
  };
}

function boardOf(d: ProjectDraft): BoardSettings {
  const testCommand = d.board.testCommand.trim();
  return { ...d.board, testCommand, testsFirst: d.board.testsFirst && !!testCommand };
}

function settingsOf(s: Settings): Settings {
  // An emptied number field is null: the backend expects a number.
  return { ...s, idleStopMinutes: Math.max(0, Math.floor(Number(s.idleStopMinutes) || 0)) };
}

/** The fields of `next` that differ from `base`. */
function changes<T extends object>(base: T, next: T): Partial<T> {
  const out: Partial<T> = {};
  for (const k of Object.keys(next) as (keyof T)[]) {
    if (JSON.stringify(base[k]) !== JSON.stringify(next[k])) out[k] = next[k];
  }
  return out;
}

const empty = (o: object) => Object.keys(o).length === 0;

class SettingsForm {
  tab = $state<SettingsTab>('claude');
  /** The project the "Projets" and "Kanban" tabs set. */
  projectId = $state<string | null>(null);
  settings = $state<Settings>({} as Settings);
  /** By project. */
  projects = $state<Record<string, ProjectDraft>>({});
  busy = $state(false);
  /** As last opened or saved: what a change is measured against. */
  #base = $state.raw<{ settings: Settings; projects: Record<string, ProjectDraft> }>({ settings: {} as Settings, projects: {} });

  project = $derived(this.projectId ? this.projects[this.projectId] : undefined);

  /** What keeps the draft from being saved. */
  problem = $derived.by(() => {
    for (const p of app.projects) {
      const d = this.projects[p.id];
      if (!d) continue;
      if (!d.name.trim()) return `Le projet « ${p.name} » doit garder un nom.`;
      if (d.runCommands.some((c) => !c.name.trim() || !c.command.trim())) {
        return `Chaque commande de lancement de « ${d.name.trim()} » demande un nom et une ligne de commande.`;
      }
    }
    return null;
  });

  /** A fresh draft of everything, on `tab` and the project given (else the one on screen). */
  open({ tab = 'claude', projectId }: { tab?: SettingsTab; projectId?: string } = {}) {
    this.resume({ tab, projectId });
    const settings = $state.snapshot(app.settings);
    const projects = Object.fromEntries(app.projects.map((p) => [p.id, draftOf(p)]));
    this.#base = { settings, projects };
    this.settings = structuredClone(settings);
    this.projects = structuredClone(projects);
  }

  /** The draft as it is, on `tab` and the project given (else the one on screen). */
  resume({ tab = 'claude', projectId }: { tab?: SettingsTab; projectId?: string } = {}) {
    const has = (id: string | null | undefined) => !!id && app.projects.some((p) => p.id === id);
    this.tab = tab;
    this.projectId = has(projectId) ? projectId! : has(app.project?.id) ? app.project!.id : (app.projects[0]?.id ?? null);
  }

  /** A tab has something to save. */
  changed(tab: SettingsTab): boolean {
    const keys = SETTINGS_OF[tab];
    const own = !!keys && keys.some((k) => k in changes(settingsOf(this.#base.settings), settingsOf(this.settings)));
    if (tab === 'integrations') return own || Object.keys(this.projects).some((id) => this.#linksChanged(id));
    if (keys) return own;
    if (tab === 'projects') return Object.keys(this.projects).some((id) => !empty(this.#projectChanges(id)));
    if (tab === 'board') return Object.keys(this.projects).some((id) => !empty(this.#boardChanges(id)));
    return false;
  }

  /** A launch command for the project on screen in the modal, run by the first shell found. */
  addCommand() {
    this.project?.runCommands.push({ id: crypto.randomUUID(), name: '', command: '', shell: app.shells[0]?.id ?? 'pwsh', cwd: '' });
  }

  #projectChanges(id: string): Partial<ProjectFields> {
    const base = this.#base.projects[id];
    const d = this.projects[id];
    return base && d ? changes(fieldsOf(base), fieldsOf(d)) : {};
  }

  #linksChanged(id: string): boolean {
    const base = this.#base.projects[id];
    const d = this.projects[id];
    return !!base && !!d && JSON.stringify(linksOf(base)) !== JSON.stringify(linksOf(d));
  }

  #boardChanges(id: string): Partial<BoardSettings> {
    const base = this.#base.projects[id];
    const d = this.projects[id];
    return base && d ? changes(boardOf(base), boardOf(d)) : {};
  }

  /**
   * Saves what changed, each part over what the app has now (the backend moves a board on by
   * itself). Returns whether all of it was saved; what was is no longer a change.
   */
  async save(): Promise<boolean> {
    if (this.problem || this.busy) return false;
    this.busy = true;
    let ok = true;
    try {
      const diff = changes(settingsOf(this.#base.settings), settingsOf(this.settings));
      if (!empty(diff)) {
        const next = { ...$state.snapshot(app.settings), ...diff };
        const shells = await app.run(api.saveSettings(next));
        if (shells) {
          app.settings = next;
          app.shells = shells;
          this.#base = { ...this.#base, settings: { ...this.#base.settings, ...diff } };
        } else ok = false;
      }
      for (const id of Object.keys(this.projects)) {
        if (!(await this.#saveProject(id))) ok = false;
      }
    } finally {
      this.busy = false;
    }
    return ok;
  }

  async #saveProject(id: string): Promise<boolean> {
    let ok = true;
    const fields = this.#projectChanges(id);
    const links = this.#linksChanged(id) ? linksOf($state.snapshot(this.projects[id])) : null;
    const current = () => app.projects.find((p) => p.id === id);
    let p = current();
    if (!p) return true;
    if (!empty(fields) || links) {
      // As sent: what is typed meanwhile stays a change.
      const sent: Partial<ProjectDraft> = { ...this.#draftFields(id, fields), ...(links ? { integrations: links } : {}) };
      const next = { ...$state.snapshot(p), ...fields, ...(links ? { integrations: links } : {}) };
      const saved = await app.run(api.updateProject(next).then(() => true));
      p = current();
      if (saved && p) {
        if (fields.runCommands)
          forgetLaunches(p.runCommands.filter((c) => !fields.runCommands!.some((x) => x.id === c.id)).map((c) => c.id));
        // What was saved, and nothing else of the project.
        Object.assign(p, fields, links ? { integrations: links } : {});
        this.#rebase(id, (b) => ({ ...b, ...sent }));
      } else if (!saved) ok = false;
    }
    const board = this.#boardChanges(id);
    p = current();
    if (p && !empty(board)) {
      const saved = await app.run(api.boardSet(id, { ...$state.snapshot(p.board), ...board }));
      if (saved) {
        app.replaceProject(saved);
        this.#rebase(id, (b) => ({ ...b, board: { ...b.board, ...board } }));
      } else ok = false;
    }
    return ok;
  }

  /** The draft's own values for the fields saved: the patterns as typed, the commands as edited. */
  #draftFields(id: string, fields: Partial<ProjectFields>): Partial<ProjectDraft> {
    const d = $state.snapshot(this.projects[id]);
    const out: Partial<ProjectDraft> = {};
    for (const k of Object.keys(fields) as (keyof ProjectFields)[]) {
      if (k === 'worktreeCopy') out.copy = d.copy;
      else (out as Record<string, unknown>)[k] = d[k];
    }
    return out;
  }

  #rebase(id: string, f: (b: ProjectDraft) => ProjectDraft) {
    const b = this.#base.projects[id];
    if (b) this.#base = { ...this.#base, projects: { ...this.#base.projects, [id]: f(b) } };
  }
}

export const settingsForm = new SettingsForm();
