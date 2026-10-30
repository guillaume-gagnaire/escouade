// The view Plugins (PLG-4, ESC-176; choices 213 to 218): the installed plugins as the core last gave them, the page shown,
// and the install window — a plugin is read and set aside first (`inspect*`), installed only on the user's consent.
import { open } from '@tauri-apps/plugin-dialog';
import { untrack } from 'svelte';
import { t } from './i18n';
import { pluginsApi, type Category, type PluginPreview, type PluginRefusal, type PluginView } from './plugins';
import { storeApi, type CatalogPlugin, type DeepLink, type StoreView, type Suggestion } from './store';
import { readSetAside, setAside } from './proposed-plugins';
import { app } from './state.svelte';
import { api } from './ipc';

export type PluginTab = 'overview' | 'permissions' | 'versions' | 'projects' | 'settings' | 'journal';

export type PluginsPage =
  | { kind: 'store' }
  | { kind: 'installed' }
  | { kind: 'updates' }
  | { kind: 'dev' }
  | { kind: 'category'; category: Category }
  | { kind: 'plugin'; id: string; tab: PluginTab }
  /** A plugin of the store's catalogue, not installed (STORE-9). */
  | { kind: 'storePlugin'; id: string; tab: StoreTab };

export type StoreTab = 'overview' | 'permissions' | 'versions';

/** « Pas maintenant » keeps a suggestion away for a week (docs/FEATURES.md §2); « Ne plus proposer », for good. */
const SUGGESTIONS_KEY = 'escouade.store.suggestions';
const WEEK = 7 * 24 * 3600 * 1000;

/** What the install window shows: a plugin read, or the fields of a git repository before it is read. */
export type InstallStep =
  | { kind: 'git'; url: string; tag: string; busy: boolean; error: string | null }
  | { kind: 'preview'; preview: PluginPreview; busy: boolean; error: string | null };

/** The text of a refusal of the core: its message (a plugin's Refusal, an Error), or what came. */
function refusalText(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e && typeof (e as PluginRefusal).message === 'string') return (e as PluginRefusal).message;
  return String(e);
}
const errorText = refusalText;

class PluginsStore {
  list = $state<PluginView[]>([]);
  loaded = $state(false);
  page = $state<PluginsPage>({ kind: 'installed' });
  install = $state<InstallStep | null>(null);
  /** A refusal of the last gesture, shown in the view's head until the next one. */
  error = $state<string | null>(null);
  checking = $state(false);
  /** The installed plugins' icons, by id, once read (null: none, its letters). */
  icons = $state<Record<string, string | null>>({});
  /** « Tout mettre à jour »: the updates still to show, one window each, the user's consent asked for every one. */
  queue: string[] = [];
  /** The store as the core keeps it (STORE-9): its catalogue cached, when it was read. */
  store = $state<StoreView | null>(null);
  storeBusy = $state(false);
  /** The search in the catalogue: on this machine, nothing sent. */
  query = $state('');
  /** « Vérifiés seulement »: the store's lines with a seal alone. */
  verifiedFilter = $state(false);
  /** « Pour <projet> »: what Escouade saw, for the project it was read for. */
  suggestions = $state<Suggestion[]>([]);
  suggestionsFor = $state<string | null>(null);
  dismissed = $state<Record<string, number>>(readDismissed());
  /** « Pas pour ce projet » (PLG-14): by project, the plugins its `.escouade.yml` proposes that the band no longer names. */
  notForProject = $state<Record<string, string[]>>(readSetAside());

  /** « Pas pour ce projet »: the band says nothing more of `ids` in this project. */
  setAside(project: string, ids: string[]) {
    this.notForProject = setAside(this.notForProject, project, ids);
  }

  /** « Voir dans le store », from Settings: the window closed, the plugin's page of the store shown (never an install). */
  showFromSettings(id: string) {
    app.modal = null;
    app.openPlugins();
    this.openStorePlugin(id);
  }

  get updates() {
    return this.list.filter((p) => p.update);
  }

  get dev() {
    return this.list.filter((p) => p.origin.kind === 'dev');
  }

  byId(id: string) {
    return this.list.find((p) => p.id === id) ?? null;
  }

  get catalog(): CatalogPlugin[] {
    return this.store?.catalog?.plugins ?? [];
  }

  catalogPlugin(id: string) {
    return this.catalog.find((p) => p.id === id) ?? null;
  }

  /** The store as kept, then read again when it never was (the core reads it at the start and every 6 hours). */
  async loadStore() {
    try {
      const view = await storeApi.view();
      this.store = view ?? null;
      if (view && !view.readAt && !this.storeBusy) await this.refreshStore();
    } catch (e) {
      this.error = errorText(e);
    }
  }

  /** Reads the catalogue and the revocation list again, now; the plugins follow (a version withdrawn, an update). */
  async refreshStore() {
    this.storeBusy = true;
    try {
      this.store = (await storeApi.refresh()) ?? this.store;
      this.list = (await pluginsApi.list()) ?? this.list;
    } catch (e) {
      this.error = errorText(e);
    }
    this.storeBusy = false;
  }

  /** « Pour <projet> »: read for the project open, once per project. */
  async loadSuggestions(project: string | null) {
    // Read without being tracked: the effect that calls it runs again for another project alone.
    if (untrack(() => this.suggestionsFor) === (project ?? '')) return;
    this.suggestionsFor = project ?? '';
    try {
      this.suggestions = (await storeApi.suggestions(project)) ?? [];
    } catch {
      this.suggestions = [];
    }
  }

  /** The suggestions to show: not dismissed, not installed meanwhile, three at most. */
  get shownSuggestions() {
    const now = Date.now();
    return this.suggestions
      .filter((s) => !this.byId(s.id) && this.catalogPlugin(s.id))
      .filter((s) => {
        const until = this.dismissed[s.id];
        return until === undefined || (until !== 0 && until < now);
      })
      .slice(0, 3);
  }

  /** « Pas maintenant » (a week), or « Ne plus proposer » (`forever`). */
  dismiss(id: string, forever: boolean) {
    this.dismissed = { ...this.dismissed, [id]: forever ? 0 : Date.now() + WEEK };
    try {
      localStorage.setItem(SUGGESTIONS_KEY, JSON.stringify(this.dismissed));
    } catch {
      // Not kept: shown again at the next start.
    }
  }

  /** « N'installer que des versions vérifiées »: saved with the app's settings. */
  setVerifiedOnly(on: boolean) {
    app.settings.pluginsVerifiedOnly = on;
    app.run(api.saveSettings($state.snapshot(app.settings)));
  }

  /** A plugin of the store: its page in the view, the installed one's when it is installed. */
  openStorePlugin(id: string, tab: StoreTab = 'overview') {
    if (this.byId(id)) this.openPlugin(id);
    else this.show({ kind: 'storePlugin', id, tab });
  }

  /**
   * The links `escouade://` not yet taken (STORE-11): a plugin's page opened in the view, a link it cannot read said. Never
   * an install: that stays the user's gesture, from the page, with its permissions shown.
   */
  async takeLinks() {
    let links: DeepLink[] = [];
    try {
      links = (await storeApi.deepLinks()) ?? [];
    } catch {
      return;
    }
    for (const link of links) {
      if (link.kind === 'plugin') {
        app.openPlugins();
        this.openStorePlugin(link.id);
      } else app.toast(t('plugins.deepLink.bad', { url: link.url }), 'error');
    }
  }

  /** « Installer » from the store: downloaded and checked, then shown before anything is installed. */
  fromStore(id: string, version: string | null = null) {
    return this.inspect(() => pluginsApi.inspectStore(id, version));
  }

  async load() {
    try {
      this.list = (await pluginsApi.list()) ?? [];
    } catch (e) {
      this.error = errorText(e);
    }
    this.loaded = true;
  }

  /** Reads the icon of the installed plugin `id` once; read again after it is installed anew. */
  async loadIcon(id: string) {
    if (id in this.icons) return;
    this.icons[id] = null;
    try {
      this.icons[id] = (await pluginsApi.icon(id)) ?? null;
    } catch {
      // Not installed (any more): its letters.
    }
  }

  /** Puts a plugin's new view in the list, in place of the old one. */
  put(view: PluginView) {
    const at = this.list.findIndex((p) => p.id === view.id);
    if (at >= 0) this.list[at] = view;
    else this.list = [...this.list, view];
  }

  show(page: PluginsPage) {
    this.page = page;
    this.error = null;
  }

  openPlugin(id: string, tab: PluginTab = 'overview') {
    this.show({ kind: 'plugin', id, tab });
  }

  /** Runs a gesture on a plugin, its new view put in the list; a refusal is said in the view. */
  async act(run: () => Promise<PluginView>) {
    this.error = null;
    try {
      this.put(await run());
    } catch (e) {
      this.error = errorText(e);
    }
  }

  setEnabled = (id: string, on: boolean) => this.act(() => pluginsApi.setEnabled(id, on));
  setProject = (id: string, project: string, on: boolean) => this.act(() => pluginsApi.setProject(id, project, on));
  setPermission = (id: string, word: string, granted: boolean) => this.act(() => pluginsApi.setPermission(id, word, granted));
  setSetting = (id: string, key: string, value: unknown) => this.act(() => pluginsApi.setSetting(id, key, value));
  approveCommands = (id: string, on: boolean) => this.act(() => pluginsApi.approveCommands(id, on));

  async remove(id: string, keep: boolean) {
    this.error = null;
    try {
      await pluginsApi.remove(id, keep);
      this.list = this.list.filter((p) => p.id !== id);
      delete this.icons[id];
      if (this.page.kind === 'plugin' && this.page.id === id) this.show({ kind: 'installed' });
    } catch (e) {
      this.error = errorText(e);
    }
  }

  async checkUpdates() {
    this.checking = true;
    try {
      this.list = (await pluginsApi.checkUpdates()) ?? this.list;
    } catch (e) {
      this.error = errorText(e);
    }
    this.checking = false;
  }

  /** « Depuis un dossier… » (`dev`: « Charger un dossier… »): the folder chosen, then read and shown before anything. */
  async fromFolder(dev = false) {
    const dir = await open({ directory: true, multiple: false, title: t('plugins.pickFolder') });
    if (typeof dir !== 'string' || !dir) return;
    await this.inspect(() => (dev ? pluginsApi.inspectDev(dir) : pluginsApi.inspectFolder(dir)));
  }

  /** « Depuis un dépôt git… »: its address and its tag asked first. */
  fromGit(url = '', tag = '') {
    this.install = { kind: 'git', url, tag, busy: false, error: null };
  }

  /** Reads the repository the window was given. */
  async readGit() {
    const step = this.install;
    if (step?.kind !== 'git') return;
    step.busy = true;
    step.error = null;
    try {
      const preview = await pluginsApi.inspectGit(step.url, step.tag);
      this.install = { kind: 'preview', preview, busy: false, error: null };
    } catch (e) {
      if (this.install === step) {
        step.busy = false;
        step.error = refusalText(e);
      }
    }
  }

  /** An update offered: the new version read and shown, the permissions it adds marked, before anything is installed. */
  async update(p: PluginView) {
    if (!p.update) return;
    const o = p.origin;
    if (o.kind === 'store') await this.fromStore(p.id, p.update.version);
    else if (o.kind === 'git' && p.update.tag) await this.inspect(() => pluginsApi.inspectGit(o.url, p.update!.tag!));
    else if (o.kind === 'folder') await this.inspect(() => pluginsApi.inspectFolder(o.path));
  }

  /** « Tout mettre à jour »: each update's window in turn; cancelling one stops there. */
  async updateAll() {
    this.queue = this.updates.map((p) => p.id);
    await this.next();
  }

  private async next() {
    const id = this.queue.shift();
    const p = id ? this.byId(id) : null;
    if (p) await this.update(p);
  }

  async inspect(read: () => Promise<PluginPreview>) {
    this.error = null;
    try {
      const preview = await read();
      this.install = { kind: 'preview', preview, busy: false, error: null };
    } catch (e) {
      this.error = refusalText(e);
    }
  }

  /** « Installer »: what was read is installed, and its page shown. */
  async confirm() {
    const step = this.install;
    if (step?.kind !== 'preview') return;
    step.busy = true;
    step.error = null;
    try {
      const view = await pluginsApi.install(step.preview.token);
      // Installed anew (an update): its icon may have changed.
      delete this.icons[view.id];
      this.put(view);
      this.install = null;
      if (this.queue.length) await this.next();
      else this.openPlugin(view.id);
    } catch (e) {
      if (this.install === step) {
        step.busy = false;
        step.error = refusalText(e);
      }
    }
  }

  /** The window closed without installing: what was set aside goes. */
  cancel() {
    const step = this.install;
    this.install = null;
    this.queue = [];
    if (step?.kind === 'preview') void pluginsApi.discard(step.preview.token).catch(() => {});
  }
}

function readDismissed(): Record<string, number> {
  try {
    const v = JSON.parse(localStorage.getItem(SUGGESTIONS_KEY) ?? '{}');
    return v && typeof v === 'object' ? v : {};
  } catch {
    return {};
  }
}

export const plugins = new PluginsStore();
