// Files open in the editor: their text, as last read or saved, and the state of the file on disk.
// A source is 'project' (the project's checkout) or the id of an agent (its worktree).

import { plural } from '../format';
import { api } from '../ipc';
import type { FileBase, FileText } from '../types';

export type BufferKind = 'text' | 'binary' | 'tooLarge' | 'missing' | 'error';

export interface Buffer {
  key: string;
  projectId: string;
  source: string;
  path: string;
  kind: BufferKind;
  /** Text in the editor (LF line endings). */
  text: string;
  /** Text as last read or saved. */
  saved: string;
  /** Of the file on disk as last read or saved. */
  hash: string;
  eol: 'lf' | 'crlf';
  bom: boolean;
  size: number;
  /** The version compared with: undefined while loading, null without one (not a repository). */
  base?: FileBase | null;
  /** The file on disk changed or vanished since it was read. */
  disk: 'ok' | 'changed' | 'deleted';
  /**
   * The file as it is on disk, read to compare what was typed with it (« Comparer »), until the user chooses: it
   * follows what the agent writes meanwhile. Null when not compared.
   */
  onDisk: { text: string; hash: string } | null;
  error: string | null;
  /** Bumped when the text is replaced from disk, for the editor to take it. */
  version: number;
}

export const sourceAgent = (source: string) => (source === 'project' ? null : source);

/** The sentence a confirmation adds when removing something loses `n` unsaved files ('' for none). */
export const lossNotice = (n: number) =>
  n ? ` ${plural(n, 'fichier non enregistré dans l’éditeur sera perdu', 'fichiers non enregistrés dans l’éditeur seront perdus')}.` : '';

const notFound = (e: unknown) => String(e).includes('introuvable');

class Buffers {
  all = $state<Record<string, Buffer>>({});
  private sent = 0;
  private pending = new Map<string, Promise<Buffer>>();
  private saving = new Set<string>();

  key(projectId: string, source: string, path: string) {
    return `${projectId}|${source}|${path}`;
  }

  isDirty(b: Buffer | undefined): boolean {
    return !!b && b.kind === 'text' && b.text !== b.saved;
  }

  get unsaved(): number {
    return Object.values(this.all).filter((b) => this.isDirty(b)).length;
  }

  open(projectId: string, source: string, path: string): Promise<Buffer> {
    const key = this.key(projectId, source, path);
    const open = this.all[key];
    // A file that was missing or unreadable is read again, it may be there now.
    if (open && open.kind !== 'missing' && open.kind !== 'error') return Promise.resolve(open);
    let p = this.pending.get(key);
    if (!p) {
      p = this.load(key, projectId, source, path).finally(() => this.pending.delete(key));
      this.pending.set(key, p);
    }
    return p;
  }

  private async load(key: string, projectId: string, source: string, path: string): Promise<Buffer> {
    const b: Buffer = {
      key,
      projectId,
      source,
      path,
      kind: 'text',
      text: '',
      saved: '',
      hash: '',
      eol: 'lf',
      bom: false,
      size: 0,
      base: undefined,
      disk: 'ok',
      onDisk: null,
      error: null,
      version: 0,
    };
    try {
      this.take(b, await api.fsRead(projectId, sourceAgent(source), path));
    } catch (e) {
      Object.assign(b, notFound(e) ? { kind: 'missing' } : { kind: 'error', error: String(e) });
    }
    // Forgotten while it was read (its source or its project is gone): not brought back.
    if (!this.pending.has(key)) return b;
    this.all[key] = b;
    if (b.kind === 'text') this.loadBase(key);
    return this.all[key];
  }

  private take(b: Buffer, f: FileText) {
    Object.assign(b, { kind: f.kind, text: f.text ?? '', saved: f.text ?? '', hash: f.hash, eol: f.eol, bom: f.bom, size: f.size });
  }

  private loadBase(key: string) {
    const b = this.all[key];
    if (!b) return;
    api
      .fsBase(b.projectId, sourceAgent(b.source), b.path)
      .then((base) => {
        const x = this.all[key];
        if (x) x.base = base;
      })
      .catch(() => {
        const x = this.all[key];
        if (x) x.base = null;
      });
  }

  edit(key: string, text: string) {
    const b = this.all[key];
    if (!b || b.kind !== 'text') return;
    b.text = text;
    this.sync();
  }

  /** Where the file on disk stands: a comparison with it lasts as long as it differs from the version read. */
  private mark(b: Buffer, disk: Buffer['disk']) {
    b.disk = disk;
    if (disk !== 'changed') b.onDisk = null;
  }

  /**
   * Writes the file; false when it changed or vanished on disk meanwhile (see `disk`). Forced, it writes over what
   * the disk has, or over the version compared with only (`onDisk`).
   */
  async save(key: string, force = false): Promise<boolean> {
    const b = this.all[key];
    if (!b || b.kind !== 'text') return false;
    if (!force && !this.isDirty(b) && b.disk === 'ok') return true;
    if (this.saving.has(key)) return false;
    this.saving.add(key);
    const text = b.text;
    try {
      const hash = await api.fsWrite({
        projectId: b.projectId,
        agentId: sourceAgent(b.source),
        path: b.path,
        text,
        eol: b.eol,
        bom: b.bom,
        expectedHash: force ? (b.onDisk?.hash ?? null) : b.hash,
      });
      Object.assign(b, { saved: text, hash });
      this.mark(b, 'ok');
      this.sync();
      return true;
    } catch (e) {
      const msg = String(e);
      if (msg.startsWith('changed')) this.mark(b, 'changed');
      else if (msg.startsWith('deleted')) this.mark(b, 'deleted');
      else throw e;
      return false;
    } finally {
      this.saving.delete(key);
    }
  }

  /**
   * Saves over what changed on disk (or creates the file again). Compared with the disk, over the version compared
   * only: one the agent wrote since is not lost unseen, it is read for the comparison instead (false then).
   */
  async keepMine(key: string): Promise<boolean> {
    if (await this.save(key, true)) return true;
    const b = this.all[key];
    if (b?.onDisk && b.disk === 'changed') await this.compare(key);
    return false;
  }

  /** Reads the file as it is on disk, to compare what was typed with it: what was typed is left alone. */
  async compare(key: string) {
    const b = this.all[key];
    if (!b || b.kind !== 'text') return;
    const f = await this.read(b);
    if (f === undefined) return;
    if (f === null) return this.mark(b, 'deleted');
    if (f.kind !== 'text') throw 'la version du disque n’est pas du texte';
    // Back to the version read: nothing to choose between.
    if (f.hash === b.hash) return this.mark(b, 'ok');
    b.disk = 'changed';
    b.onDisk = { text: f.text ?? '', hash: f.hash };
  }

  /**
   * The file as it is on disk now, null when it vanished, undefined when a save landed or is under way
   * meanwhile: the answer is then older than the buffer and must not be taken.
   */
  private async read(b: Buffer): Promise<FileText | null | undefined> {
    const hash = b.hash;
    let f: FileText | null;
    try {
      f = await api.fsRead(b.projectId, sourceAgent(b.source), b.path);
    } catch (e) {
      if (!notFound(e)) throw e;
      f = null;
    }
    return b.hash !== hash || this.saving.has(b.key) ? undefined : f;
  }

  /** Reads the file again: a clean one takes the new text, a modified one is flagged. */
  async refresh(key: string) {
    const b = this.all[key];
    if (!b || b.kind !== 'text') return;
    let f: FileText | null | undefined;
    try {
      f = await this.read(b);
    } catch {
      return;
    }
    if (f === undefined) return;
    if (f === null) {
      this.mark(b, 'deleted');
      return;
    }
    if (f.kind === 'text') {
      if (f.hash === b.hash) {
        this.mark(b, 'ok');
      } else if (this.isDirty(b)) {
        b.disk = 'changed';
        // Compared with the disk: the comparison shows what the agent wrote since.
        if (b.onDisk && b.onDisk.hash !== f.hash) b.onDisk = { text: f.text ?? '', hash: f.hash };
      } else {
        this.take(b, f);
        this.mark(b, 'ok');
        b.version++;
      }
    }
    this.loadBase(key);
  }

  async refreshAll(projectId: string, source: string) {
    const open = Object.values(this.all).filter((b) => b.projectId === projectId && b.source === source);
    await Promise.all(open.map((b) => this.refresh(b.key)));
  }

  /** Drops what was typed for the file as it is on disk. */
  async reload(key: string) {
    const b = this.all[key];
    if (!b) return;
    const f = await this.read(b);
    if (f === undefined) return;
    if (f === null) {
      this.mark(b, 'deleted');
      return;
    }
    this.take(b, f);
    this.mark(b, 'ok');
    b.version++;
    this.sync();
    this.loadBase(key);
  }

  close(key: string) {
    delete this.all[key];
    this.sync();
  }

  /** Unsaved files of a project, or of one of its sources. */
  unsavedIn(projectId: string, source?: string): number {
    return Object.values(this.all).filter(
      (b) => b.projectId === projectId && (source === undefined || b.source === source) && this.isDirty(b),
    ).length;
  }

  /** Forgets the files of a source that is gone (a deleted agent's worktree), unsaved changes included. */
  closeSource(projectId: string, source: string) {
    this.drop((b) => b.projectId === projectId && b.source === source);
  }

  /** Forgets the files of a closed project, unsaved changes included. */
  closeProject(projectId: string) {
    this.drop((b) => b.projectId === projectId);
  }

  private drop(gone: (b: Pick<Buffer, 'projectId' | 'source'>) => boolean) {
    for (const b of Object.values(this.all)) if (gone(b)) delete this.all[b.key];
    // A read under way is not taken when it arrives.
    for (const k of this.pending.keys()) {
      const [projectId, source] = k.split('|');
      if (gone({ projectId, source })) this.pending.delete(k);
    }
    this.sync();
  }

  reset() {
    this.all = {};
    this.sent = 0;
    this.pending.clear();
    this.saving.clear();
  }

  /** Tells the backend how many files are unsaved, for "Quitter". */
  private sync() {
    const n = this.unsaved;
    if (n === this.sent) return;
    this.sent = n;
    api.setUnsaved(n).catch(() => {
      // Not told: the next change tells it again, even if the count is the same.
      if (this.sent === n) this.sent = -1;
    });
  }
}

export const buffers = new Buffers();
