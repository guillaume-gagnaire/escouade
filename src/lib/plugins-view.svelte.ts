// The view Plugins (PLG-4, ESC-176; choices 213 to 218): the installed plugins as the core last gave them, the page shown,
// and the install window — a plugin is read and set aside first (`inspect*`), installed only on the user's consent.
import { open } from '@tauri-apps/plugin-dialog';
import { t } from './i18n';
import { pluginsApi, type Category, type PluginPreview, type PluginRefusal, type PluginView } from './plugins';

export type PluginTab = 'overview' | 'permissions' | 'versions' | 'projects' | 'settings' | 'journal';

export type PluginsPage =
  | { kind: 'store' }
  | { kind: 'installed' }
  | { kind: 'updates' }
  | { kind: 'dev' }
  | { kind: 'category'; category: Category }
  | { kind: 'plugin'; id: string; tab: PluginTab };

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

  get updates() {
    return this.list.filter((p) => p.update);
  }

  get dev() {
    return this.list.filter((p) => p.origin.kind === 'dev');
  }

  byId(id: string) {
    return this.list.find((p) => p.id === id) ?? null;
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
    if (o.kind === 'git' && p.update.tag) await this.inspect(() => pluginsApi.inspectGit(o.url, p.update!.tag!));
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

export const plugins = new PluginsStore();
