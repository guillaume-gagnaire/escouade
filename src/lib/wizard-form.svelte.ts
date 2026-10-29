// The state of the new project's assistant: where it starts from, the step on screen, the settings as they are set
// so far, what the folder was found to hold, and what « Créer » (or « Enregistrer », for an existing project) does.

import { accountName, CLAUDE_DEFAULT_MODEL, dedicated, newAgentDefaults } from './accounts';
import { t } from './i18n';
import { api } from './ipc';
import { app } from './state.svelte';
import type { FolderInfo, Project, ProjectIntegrations, SetupInfo } from './types';
import {
  applyCopy,
  blankConfig,
  configOf,
  controls,
  copyDirs,
  copyItems,
  recommendedConfig,
  stepsFor,
  withConfig,
  type ControlId,
  type CopyItem,
  type NewProjectDefaults,
  type Start,
  type StepId,
  type WizardConfig,
} from './wizard';

export class WizardForm {
  path = $state('');
  /** What the window knows of the folder (the new project's: read as typed). */
  info = $state<FolderInfo | null>(null);
  /** What the assistant checked in it: a commit, its branches, the folders of the commands, the worktrees folder. */
  setup = $state<SetupInfo | null>(null);
  start = $state<Start>('recommended');
  /** The project copied, when `start` is `copy`. */
  sourceId = $state('');
  /** The ids of the copied settings taken (`CopyItem.id`). */
  checked = $state<string[]>([]);
  config = $state<WizardConfig>(recommendedConfig(this.#defaults()));
  integrations = $state<ProjectIntegrations>({ links: [], comments: [] });
  step = $state<StepId>('start');
  /** The steps left with « Passer ». */
  skipped = $state<StepId[]>([]);
  /** The steps gone through with « Suivant ». */
  done = $state<StepId[]>([]);
  firstAgent = $state(true);
  model = $state('');
  /** « Indexer le code » (ESC-254): offered, as « Index du code sur tous les projets » says (ESC-293); a choice of the project's once it is made (a new one only) when it was changed here. */
  codeIndex = $state(!!app.settings?.codeIndexAll);
  /** The project to copy is being checked in the new folder: what is taken by default is not known yet. */
  copying = $state(false);
  /** « Remplir automatiquement » running, for the worktrees or the launch. */
  filling = $state<'worktrees' | 'launch' | null>(null);
  /** What `config` was last made from: made again when the start or what is copied changes, not otherwise. */
  #madeFrom = '';
  /**
   * The settings the folder's .escouade.yml gives (ESC-146), its commands left out (they wait for the user once the
   * project is made): the start « Réglages de .escouade.yml ». Null without one.
   */
  shared = $state<Project | null>(null);
  /** A start chosen by the user: the folder's file no longer picks it. */
  #chosen = false;

  /** `project`: an existing project set up anew (Réglages › Projets), else a new one. */
  constructor(readonly project: Project | null = null) {
    this.model = newAgentDefaults(undefined).model || CLAUDE_DEFAULT_MODEL;
    if (project) {
      this.path = project.path;
      this.start = 'current';
      this.config = configOf(project, app.settings.accounts ?? []);
      this.integrations = $state.snapshot(project.integrations);
      this.firstAgent = false;
      this.#madeFrom = this.#signature();
    }
  }

  steps = $derived(stepsFor(this.start));
  source = $derived(app.projects.find((p) => p.id === this.sourceId) ?? null);
  /** The other projects open, those the assistant may copy: never a private one (« Projet privé », nothing of it goes to another project). */
  sources = $derived.by(() => app.projects.filter((p) => p.id !== this.project?.id && !p.private));
  /** The other projects left out of `sources` for being private, which the start says. */
  privateLeftOut = $derived(app.projects.filter((p) => p.id !== this.project?.id && p.private).length);
  items = $derived<CopyItem[]>(this.source ? copyItems(this.source, this.setup, app.settings.accounts, accountName) : []);
  /** What would keep the way of working from working, as set so far. */
  problems = $derived.by<ControlId[]>(() =>
    controls(this.setup, this.config, {
      accounts: app.settings.accounts,
      claudeFound: app.enginesFound.length > 0,
      projectId: this.project?.id,
    }),
  );

  /** Starts from another project: the one on screen by default, its settings checked as `copyItems` says. */
  chooseStart(start: Start) {
    this.#chosen = true;
    this.start = start;
    if (start === 'copy' && !this.source)
      this.chooseSource(this.sources.find((p) => p.id === app.ui.activeProject)?.id ?? this.sources[0]?.id ?? '');
  }

  chooseSource(id: string) {
    this.sourceId = id;
    this.checked = [];
    // Until the new folder is checked for it, what to take is not known: the steps wait.
    this.copying = true;
    void this.check().then(() => {
      // Another project chosen meanwhile: its own check decides.
      if (this.sourceId !== id) return;
      this.#checkDefaults();
      this.copying = false;
    });
  }

  /** The copied settings taken by default: all but those that cannot work in the new folder. */
  #checkDefaults() {
    this.checked = this.items.filter((i) => i.checked).map((i) => i.id);
  }

  toggle(id: string, on: boolean) {
    this.checked = on ? [...new Set([...this.checked, id])] : this.checked.filter((x) => x !== id);
  }

  /**
   * Reads the folder's .escouade.yml: its settings are where a new project starts from, above the recommended ones, unless
   * the user chose another start; without one, that start goes.
   */
  async readShared() {
    const path = this.path.trim();
    const shared = this.project || !path ? null : await api.sharedPreview(path).catch(() => null);
    if (this.path.trim() !== path) return;
    this.shared = shared;
    if (shared && !this.#chosen) this.start = 'file';
    else if (!shared && this.start === 'file') this.start = 'recommended';
  }

  /** Checks the folder again: its commit and branches, the folders of the commands copied or set, the worktrees folder. */
  async check() {
    const path = this.path.trim();
    if (!path || (this.info && !this.info.exists)) {
      this.setup = null;
      return;
    }
    const dirs = [...new Set([...(this.source ? copyDirs(this.source) : []), ...this.#ownDirs()])];
    const s = await api.inspectSetup(path, this.config.worktreeDir, dirs).catch(() => null);
    if (this.path.trim() === path) this.setup = s;
  }

  #ownDirs(): string[] {
    const c = this.config;
    return [...c.runCommands, ...c.worktreeSetup, ...c.worktreeTeardown].map((x) => x.cwd.trim()).filter((d) => d && d !== '.');
  }

  /** What a new project starts with: private when « Nouveaux projets privés » says so. */
  #defaults(): NewProjectDefaults {
    return { private: !!app.settings?.privateProjects };
  }

  #signature() {
    return JSON.stringify([this.start, this.sourceId, [...this.checked].sort(), this.start === 'file' ? this.path.trim() : '']);
  }

  /** The settings the start gives, made again only when the start or what is copied changed since. */
  #make() {
    const sig = this.#signature();
    if (sig === this.#madeFrom) return;
    this.#madeFrom = sig;
    const defaults = this.#defaults();
    if (this.start === 'current' && this.project) this.config = configOf(this.project, app.settings.accounts ?? []);
    else if (this.start === 'blank') this.config = blankConfig(defaults);
    else if (this.start === 'recommended') this.config = recommendedConfig(defaults);
    else if (this.start === 'file' && this.shared) {
      // The file may make it private, never public (as when it is read into a project).
      const c = configOf($state.snapshot(this.shared));
      this.config = { ...c, private: c.private || !!defaults.private };
    } else if (this.source) this.config = applyCopy(blankConfig(defaults), $state.snapshot(this.source), new Set(this.checked), this.setup);
  }

  /** Goes to a step: the settings made first when leaving the start or the copy. */
  go(step: StepId) {
    if (this.step === 'start' || this.step === 'copied') this.#make();
    this.step = step;
    void this.check();
  }

  /** « Suivant » (`skip`: « Passer », the values as they are). */
  next(skip = false) {
    const at = this.steps.indexOf(this.step);
    const list = skip ? 'skipped' : 'done';
    const other = skip ? 'done' : 'skipped';
    this[other] = this[other].filter((s) => s !== this.step);
    if (!this[list].includes(this.step)) this[list] = [...this[list], this.step];
    const to = this.steps[at + 1];
    if (to) this.go(to);
  }

  back() {
    const at = this.steps.indexOf(this.step);
    if (at > 0) this.go(this.steps[at - 1]);
  }

  /** « Remplir automatiquement »: Claude reads the folder, and its commands replace those of the step. */
  async fill(what: 'worktrees' | 'launch') {
    if (this.filling) return;
    this.filling = what;
    const id = this.project?.id;
    const path = this.path.trim();
    try {
      if (what === 'worktrees') {
        const s = await (id ? api.suggestWorktreeSteps(id) : api.suggestWorktreeStepsAt(path));
        if (!s.setup.length && !s.teardown.length) return app.toast(t('settings.suggest.nothing'));
        this.config.worktreeSetup = s.setup;
        this.config.worktreeTeardown = s.teardown;
        app.toast(t('wizard.filled', { count: s.setup.length + s.teardown.length }));
      } else {
        const s = await (id ? api.suggestRunCommands(id) : api.suggestRunCommandsAt(path));
        if (!s.commands.length) return app.toast(t('settings.suggest.nothing'));
        this.config.runCommands = s.commands;
        app.toast(t('wizard.filled', { count: s.commands.length }));
      }
      await this.check();
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      this.filling = null;
    }
  }

  /** « Faire un premier commit »: an empty one, then the folder checked again. */
  async firstCommit() {
    const ok = await app.run(api.firstCommit(this.path.trim()).then(() => true));
    if (ok) {
      this.info = (await api.inspectFolder(this.path.trim()).catch(() => null)) ?? this.info;
      await this.check();
    }
  }

  /** The corrections the assistant makes itself: the current branch as target, the default worktrees folder. */
  fix(id: ControlId) {
    if (id === 'noCommit') return this.firstCommit();
    if (id === 'noTarget') this.config.board.target = '';
    if (id === 'tracked') this.config.worktreeDir = '';
    return this.check();
  }

  /**
   * Saves the settings into the project made (or the existing one), its Kanban apart, then its first agent if asked
   * for. Returns the project, or null when something was refused (said by a toast).
   */
  async save(made: Project): Promise<Project | null> {
    this.#make();
    const base = $state.snapshot(made);
    // Its sources are the step's; the rest of its integrations (the columns commented…) stays the project's own.
    const integrations = { ...base.integrations, links: $state.snapshot(this.integrations.links) };
    const next = { ...withConfig(base, $state.snapshot(this.config)), integrations };
    const saved = await app.run(api.updateProject(next).then(() => true));
    if (!saved) return null;
    await this.#reserve(made.id);
    const p = await app.run(api.boardSet(made.id, next.board));
    return p ? { ...next, board: p.board } : next;
  }

  /**
   * The accounts chosen to be reserved to the project alone, as « Réserver <compte> à ce projet » does (ESC-163): a private
   * project's only, where the step offers them.
   */
  async #reserve(projectId: string) {
    if (!this.config.private) return;
    for (const id of this.config.reserved) {
      const a = app.settings.accounts.find((x) => x.id === id);
      if (!a || dedicated(a, projectId)) continue;
      const list = await app.run(api.accountUpdate({ ...$state.snapshot(a), projects: [projectId] }));
      if (list) app.settings.accounts = list;
    }
  }
}
