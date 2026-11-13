// The view Plugins (PLG-4, ESC-176; choices 213 to 218): the installed plugins as the core last gave them, the page shown,
// and the install window — a plugin is read and set aside first (`inspect*`), installed only on the user's consent.
import { open } from '@tauri-apps/plugin-dialog';
import { untrack } from 'svelte';
import { fDayMonth, fTime } from './format';
import { t } from './i18n';
import {
  contributionsOf,
  pluginsApi,
  type Category,
  type Manifest,
  type PluginPreview,
  type PluginRefusal,
  type PluginView,
  type SeedOffer,
} from './plugins';
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
  | { kind: 'preview'; preview: PluginPreview; busy: boolean; error: string | null }
  /** « Tout mettre à jour »: the updates that ask for nothing more, agreed to together (ESC-436). */
  | { kind: 'batch'; previews: PluginPreview[]; busy: boolean; error: string | null };

/** An update's line: read, waiting for the user's word, installing, done, or what went wrong. */
export type UpdateRow =
  | { kind: 'reading' }
  | { kind: 'waiting' }
  | { kind: 'installing' }
  | { kind: 'done'; version: string }
  | { kind: 'error'; message: string };

/** The text of a refusal of the core: its message (a plugin's Refusal, an Error), or what came. */
function refusalText(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e && typeof (e as PluginRefusal).message === 'string') return (e as PluginRefusal).message;
  return String(e);
}
const errorText = refusalText;

class PluginsStore {
  list = $state<PluginView[]>([]);
  /** What the app's seed holds that is not installed: proposed, never put back by itself (ESC-338). */
  offers = $state<SeedOffer[]>([]);
  loaded = $state(false);
  page = $state<PluginsPage>({ kind: 'installed' });
  install = $state<InstallStep | null>(null);
  /** A refusal of the last gesture, shown in the view's head until the next one. */
  error = $state<string | null>(null);
  checking = $state(false);
  /** The installed plugins' icons, by id, once read (null: none, its letters). */
  icons = $state<Record<string, string | null>>({});
  /** « Tout mettre à jour »: the updates read that ask for more, still to show, one window each (ESC-436). */
  ahead = $state<PluginPreview[]>([]);
  /** How each update under way goes, by id, on its line of the updates' page (ESC-436); the finished ones until it is left. */
  rows = $state<Record<string, UpdateRow>>({});
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

  /** The lines of the updates' page: the updates offered, and those done or refused since it was opened (ESC-436). */
  get updateLines() {
    return this.list.filter((p) => p.update || this.rows[p.id]);
  }

  /** The lines finished (done, an error) forgotten: the page shows what is offered now. */
  private forgetFinished() {
    for (const id of Object.keys(this.rows)) if (!this.busy(id)) delete this.rows[id];
  }

  get dev() {
    return this.list.filter((p) => p.origin.kind === 'dev');
  }

  byId(id: string) {
    return this.list.find((p) => p.id === id) ?? null;
  }

  /** When the store was read, beside « Rechercher des mises à jour » (ESC-405); that it did not answer, and of when the offers looked at are. */
  get storeRead(): string {
    const s = this.store;
    if (!s) return '';
    const at = s.readAt ? Date.parse(s.readAt) : null;
    const when = at === null ? '' : t('plugins.updates.when', { date: fDayMonth(at), time: fTime(at) });
    if (s.error) return at === null ? t('plugins.updates.unreachableNone') : t('plugins.updates.unreachable', { when });
    return at === null ? '' : t('plugins.updates.read', { when });
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
      await this.loadOffers();
    } catch (e) {
      this.error = errorText(e);
    }
    this.loaded = true;
  }

  private async loadOffers() {
    this.offers = (await pluginsApi.seedOffers()) ?? [];
  }

  /**
   * A plugin the seed proposes: from the store when it has a newer version, else from the seed, with no network, in one
   * click (ESC-373): a plugin of the first hand, signed, as a gesture that needs it installs it.
   */
  installOffer(o: SeedOffer) {
    const best = this.store?.best?.[o.id];
    if (best && best.localeCompare(o.version, undefined, { numeric: true }) > 0) return this.fromStore(o.id);
    return this.installSeed(o.id);
  }

  async installSeed(id: string) {
    this.error = null;
    try {
      const view = await pluginsApi.installSeed(id);
      if (view) this.put(view);
      void app.refreshEngines();
      await this.loadOffers();
    } catch (e) {
      this.error = errorText(e);
    }
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
    if (page.kind === 'updates' && this.page.kind !== 'updates') this.forgetFinished();
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
      // A harness the plugin describes comes or goes with it (PLG-6).
      void app.refreshEngines();
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
      void app.refreshEngines();
      this.list = this.list.filter((p) => p.id !== id);
      delete this.icons[id];
      delete this.rows[id];
      await this.loadOffers();
      if (this.page.kind === 'plugin' && this.page.id === id) this.show({ kind: 'installed' });
    } catch (e) {
      this.error = errorText(e);
    }
  }

  /** « Rechercher des mises à jour »: the store read again by the core first (ESC-405); `store` says when, or that it did not answer. */
  async checkUpdates() {
    this.checking = true;
    this.forgetFinished();
    try {
      const got = await pluginsApi.checkUpdates();
      if (got) {
        this.list = got.plugins;
        this.store = got.store;
      }
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

  /** How an update offered reads its new version (from the seed, the store, its repository or its folder); null: it cannot. */
  private readUpdate(p: PluginView): (() => Promise<PluginPreview>) | null {
    const u = p.update;
    if (!u) return null;
    const o = p.origin;
    if (u.seed) return () => pluginsApi.inspectSeed(p.id);
    if (o.kind === 'store') return () => pluginsApi.inspectStore(p.id, u.version);
    if (o.kind === 'git' && u.tag) {
      const tag = u.tag;
      return () => pluginsApi.inspectGit(o.url, tag);
    }
    if (o.kind === 'folder') return () => pluginsApi.inspectFolder(o.path);
    return null;
  }

  /** A refusal of an update, said on its line; on a page without its line (the installed ones, a plugin's), in the head too. */
  private failed(id: string, message: string) {
    this.rows[id] = { kind: 'error', message };
    if (this.page.kind !== 'updates') {
      const name = this.byId(id)?.name ?? id;
      const line = t('plugins.updates.failedOf', { name, message });
      this.error = this.error ? `${this.error}\n${line}` : line;
    }
  }

  /** Whether an update's line is under way (read, waiting for the user's word, installing): its button cannot be clicked again. */
  busy(id: string) {
    const r = this.rows[id];
    return !!r && (r.kind === 'reading' || r.kind === 'waiting' || r.kind === 'installing');
  }

  /** Whether an update is under way, one or « Tout mettre à jour ». */
  get updating() {
    return Object.keys(this.rows).some((id) => this.busy(id));
  }

  /**
   * An update offered: the new version read and shown, the permissions it adds marked, before anything is installed. The
   * page stays the one it was clicked on (ESC-436); its line says how it goes.
   */
  async update(p: PluginView) {
    if (this.busy(p.id)) return;
    const read = this.readUpdate(p);
    if (!read) return;
    this.error = null;
    this.rows[p.id] = { kind: 'reading' };
    try {
      const preview = await read();
      this.rows[p.id] = { kind: 'waiting' };
      this.install = { kind: 'preview', preview, busy: false, error: null };
    } catch (e) {
      this.failed(p.id, refusalText(e));
    }
  }

  /**
   * « Tout mettre à jour » (ESC-436): every update read first. Those that ask for nothing more than the version installed are
   * agreed to together, in one window; each of the others after, in its own, what it adds marked. Nothing is installed
   * without a gesture; a refusal or an error on one is said on its line and stops none of the others.
   */
  async updateAll() {
    if (this.updating) return;
    const list = this.updates.filter((p) => this.readUpdate(p));
    if (!list.length) return;
    this.error = null;
    for (const p of list) this.rows[p.id] = { kind: 'reading' };
    const quiet: PluginPreview[] = [];
    const loud: PluginPreview[] = [];
    for (const p of list) {
      try {
        const preview = await this.readUpdate(p)!();
        this.rows[p.id] = { kind: 'waiting' };
        // A version that will be loaded switched off (its refusal) is said in its own window, as one that asks for more.
        (preview.refusal || asksMore(preview, this.byId(preview.id)) ? loud : quiet).push(preview);
      } catch (e) {
        this.failed(p.id, refusalText(e));
      }
    }
    this.ahead = loud;
    if (quiet.length) this.install = { kind: 'batch', previews: quiet, busy: false, error: null };
    else this.next();
  }

  /** The next update of « Tout mettre à jour » that asks for more, in its own window. */
  private next() {
    const preview = this.ahead.shift();
    if (preview) this.install = { kind: 'preview', preview, busy: false, error: null };
  }

  /** The updates that ask for nothing more, agreed to together: installed one after the other, an error on one said on its line. */
  async confirmBatch() {
    const step = this.install;
    if (step?.kind !== 'batch' || step.busy) return;
    step.busy = true;
    for (const preview of step.previews) {
      this.rows[preview.id] = { kind: 'installing' };
      try {
        const view = await pluginsApi.install(preview.token);
        this.installed(view);
        this.rows[view.id] = { kind: 'done', version: view.version };
      } catch (e) {
        this.failed(preview.id, refusalText(e));
      }
    }
    void app.refreshEngines();
    void this.loadOffers().catch(() => {});
    this.install = null;
    this.next();
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

  /** A plugin installed anew: its view in the list, its icon read again (it may have changed). */
  private installed(view: PluginView) {
    delete this.icons[view.id];
    this.put(view);
  }

  /** « Installer »: what was read is installed; a new plugin's page shown, an update leaving the page where it was (ESC-436). */
  async confirm() {
    const step = this.install;
    if (step?.kind !== 'preview' || step.busy) return;
    const update = !!step.preview.previous;
    step.busy = true;
    step.error = null;
    if (update) this.rows[step.preview.id] = { kind: 'installing' };
    try {
      const view = await pluginsApi.install(step.preview.token);
      this.installed(view);
      void app.refreshEngines();
      this.install = null;
      void this.loadOffers().catch(() => {});
      if (update) this.rows[view.id] = { kind: 'done', version: view.version };
      else this.openPlugin(view.id);
      this.next();
    } catch (e) {
      if (update) this.rows[step.preview.id] = { kind: 'waiting' };
      if (this.install === step) {
        step.busy = false;
        step.error = refusalText(e);
      }
    }
  }

  /**
   * The window closed without installing: what was set aside goes. An update refused stays offered; the next one of « Tout
   * mettre à jour » is shown all the same.
   */
  cancel() {
    const step = this.install;
    if (step?.kind === 'batch' && step.busy) return;
    this.install = null;
    const previews = step?.kind === 'preview' ? [step.preview] : step?.kind === 'batch' ? step.previews : [];
    for (const preview of previews) {
      void pluginsApi.discard(preview.token).catch(() => {});
      if (this.busy(preview.id)) delete this.rows[preview.id];
    }
    this.next();
  }
}

/**
 * Whether an update asks for more than the version installed: a permission (`grown`, by the core), or a contribution its
 * manifest did not have. Unknown (no manifest installed to compare with): it does, its own window shown.
 */
export function asksMore(preview: PluginPreview, installed: PluginView | null): boolean {
  if (preview.grown.length) return true;
  if (!installed?.manifest) return true;
  return newContributions(preview.manifest, installed.manifest).length > 0;
}

/** The lines of what `next` adds that `before` did not (`contributionsOf`, in one language: they are compared, not shown). */
export function newContributions(next: Manifest | null, before: Manifest | null, lang = 'en'): string[] {
  const had = new Set(contributionsOf(before, lang));
  return contributionsOf(next, lang).filter((line) => !had.has(line));
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
