import { beforeEach, describe, expect, it } from 'vitest';
import { fakeBackend } from '../../test/ipc';
import { buffers, lossNotice } from './buffers.svelte';
import { trees } from './trees.svelte';

const text = (t: string, hash = 'h1', eol: 'lf' | 'crlf' = 'lf') => ({ kind: 'text', text: t, size: t.length, hash, eol, bom: false });

/** A value to hand out later, to make an answer come after what happens in between. */
const deferred = <T>() => {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
};

describe('buffers', () => {
  beforeEach(() => buffers.reset());

  it('opens a file once, then saves it with the hash it was read with', async () => {
    const backend = fakeBackend({
      fs_read: () => text('a\n', 'h1', 'crlf'),
      fs_base: () => ({ reference: 'main', text: 'a\n' }),
      fs_write: () => 'h2',
      set_unsaved: () => null,
    });
    const b = await buffers.open('p1', 'a2', 'src/x.ts');
    await buffers.open('p1', 'a2', 'src/x.ts');
    expect(backend.called('fs_read')).toHaveLength(1);
    expect(backend.called('fs_read')[0].args).toEqual({ projectId: 'p1', agentId: 'a2', path: 'src/x.ts' });
    await expect.poll(() => buffers.all[b.key].base).toEqual({ reference: 'main', text: 'a\n' });

    buffers.edit(b.key, 'b\n');
    expect(buffers.isDirty(buffers.all[b.key])).toBe(true);
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 1 });
    expect(await buffers.save(b.key)).toBe(true);
    expect(backend.called('fs_write')[0].args).toEqual({
      projectId: 'p1',
      agentId: 'a2',
      path: 'src/x.ts',
      text: 'b\n',
      eol: 'crlf',
      bom: false,
      expectedHash: 'h1',
    });
    expect(buffers.all[b.key]).toMatchObject({ saved: 'b\n', hash: 'h2', disk: 'ok' });
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 0 });
  });

  it('reads the project checkout without an agent', async () => {
    const backend = fakeBackend({ fs_read: () => text('x'), fs_base: () => null, set_unsaved: () => null });
    await buffers.open('p1', 'project', 'x.ts');
    expect(backend.called('fs_read')[0].args.agentId).toBeNull();
  });

  it('does not overwrite a file changed on disk until the user keeps their version', async () => {
    const backend = fakeBackend({
      fs_read: () => text('a\n'),
      fs_base: () => null,
      fs_write: (a: any) => {
        if (a.expectedHash) throw 'changed';
        return 'h3';
      },
      set_unsaved: () => null,
    });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(k, 'mine\n');
    expect(await buffers.save(k)).toBe(false);
    expect(buffers.all[k]).toMatchObject({ disk: 'changed', text: 'mine\n', saved: 'a\n' });
    expect(await buffers.keepMine(k)).toBe(true);
    expect(backend.called('fs_write').at(-1)?.args.expectedHash).toBeNull();
    expect(buffers.all[k]).toMatchObject({ disk: 'ok', saved: 'mine\n', hash: 'h3' });
  });

  it('reloads a clean file changed on disk, flags a modified one', async () => {
    let disk = text('a\n', 'h1');
    fakeBackend({ fs_read: () => disk, fs_base: () => null, set_unsaved: () => null });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    disk = text('agent\n', 'h2');
    await buffers.refresh(k);
    expect(buffers.all[k]).toMatchObject({ text: 'agent\n', saved: 'agent\n', hash: 'h2', disk: 'ok', version: 1 });

    buffers.edit(k, 'mine\n');
    disk = text('agent2\n', 'h3');
    await buffers.refresh(k);
    expect(buffers.all[k]).toMatchObject({ text: 'mine\n', disk: 'changed', version: 1 });
    await buffers.reload(k);
    expect(buffers.all[k]).toMatchObject({ text: 'agent2\n', saved: 'agent2\n', disk: 'ok', version: 2 });
  });

  it('flags a file deleted on disk and keeps what was typed', async () => {
    let gone = false;
    fakeBackend({
      fs_read: () => {
        if (gone) throw 'x.ts introuvable';
        return text('a\n');
      },
      fs_base: () => null,
      set_unsaved: () => null,
    });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(k, 'mine\n');
    gone = true;
    await buffers.refresh(k);
    expect(buffers.all[k]).toMatchObject({ disk: 'deleted', text: 'mine\n' });
  });

  it('opens binary, too large and missing files without text', async () => {
    fakeBackend({
      fs_read: (a: any) => {
        if (a.path === 'gone.ts') throw 'gone.ts introuvable';
        return { kind: a.path === 'a.png' ? 'binary' : 'tooLarge', text: null, size: 3_500_000, hash: '', eol: 'lf', bom: false };
      },
      set_unsaved: () => null,
    });
    expect((await buffers.open('p1', 'project', 'a.png')).kind).toBe('binary');
    expect((await buffers.open('p1', 'project', 'big.log')).kind).toBe('tooLarge');
    expect((await buffers.open('p1', 'project', 'gone.ts')).kind).toBe('missing');
  });

  it('forgets a closed file and counts it no more', async () => {
    const backend = fakeBackend({ fs_read: () => text('a'), fs_base: () => null, set_unsaved: () => null });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(k, 'b');
    buffers.close(k);
    expect(buffers.all[k]).toBeUndefined();
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 0 });
  });

  it('forgets the files of a source, or of a project, gone with their unsaved changes', async () => {
    const backend = fakeBackend({ fs_read: () => text('a'), fs_base: () => null, set_unsaved: () => null });
    const wt = (await buffers.open('p1', 'a2', 'x.ts')).key;
    const proj = (await buffers.open('p1', 'project', 'x.ts')).key;
    const other = (await buffers.open('p2', 'project', 'y.ts')).key;
    for (const k of [wt, proj, other]) buffers.edit(k, 'b');
    expect(buffers.unsavedIn('p1', 'a2')).toBe(1);
    expect(buffers.unsavedIn('p1')).toBe(2);
    buffers.closeSource('p1', 'a2');
    expect(Object.keys(buffers.all)).toEqual([proj, other]);
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 2 });
    buffers.closeProject('p2');
    expect(Object.keys(buffers.all)).toEqual([proj]);
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 1 });
  });

  it('does not bring back a file of a forgotten source whose read was under way', async () => {
    const gate = deferred<unknown>();
    fakeBackend({ fs_read: () => gate.promise, fs_base: () => null, set_unsaved: () => null });
    const opening = buffers.open('p1', 'a2', 'x.ts');
    buffers.closeSource('p1', 'a2');
    gate.resolve(text('a'));
    await opening;
    expect(buffers.all).toEqual({});
  });

  it('warns of the unsaved files a removal loses, in a sentence of their own', () => {
    expect(lossNotice(0)).toBe('');
    expect(lossNotice(1)).toBe(' 1 fichier non enregistré dans l’éditeur sera perdu.');
    expect(lossNotice(3)).toBe(' 3 fichiers non enregistrés dans l’éditeur seront perdus.');
  });

  it('reads a file once when it is opened twice at the same time', async () => {
    const gate = deferred<unknown>();
    const backend = fakeBackend({ fs_read: () => gate.promise, fs_base: () => null, set_unsaved: () => null });
    const first = buffers.open('p1', 'project', 'x.ts');
    const second = buffers.open('p1', 'project', 'x.ts');
    gate.resolve(text('a\n'));
    const [a, b] = await Promise.all([first, second]);
    expect(backend.called('fs_read')).toHaveLength(1);
    expect(a.key).toBe(b.key);
    expect(Object.keys(buffers.all)).toEqual([a.key]);
  });

  it('keeps what is typed while a save is in flight', async () => {
    const written = deferred<string>();
    const backend = fakeBackend({
      fs_read: () => text('a\n'),
      fs_base: () => null,
      fs_write: () => written.promise,
      set_unsaved: () => null,
    });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(k, 'b\n');
    const saving = buffers.save(k);
    buffers.edit(k, 'c\n');
    written.resolve('h2');
    expect(await saving).toBe(true);
    expect(backend.called('fs_write')[0].args.text).toBe('b\n');
    expect(buffers.all[k]).toMatchObject({ text: 'c\n', saved: 'b\n', hash: 'h2', disk: 'ok' });
    expect(buffers.isDirty(buffers.all[k])).toBe(true);
    expect(buffers.unsaved).toBe(1);
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 1 });
  });

  it('is not undone by an answer from the disk that comes after a save', async () => {
    const late = deferred<unknown>();
    let slow = false;
    fakeBackend({
      fs_read: () => (slow ? late.promise : text('a\n', 'h1')),
      fs_base: () => null,
      fs_write: () => 'h9',
      set_unsaved: () => null,
    });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(k, 'mine\n');
    slow = true;
    const refreshing = buffers.refresh(k);
    expect(await buffers.keepMine(k)).toBe(true);
    late.resolve(text('agent\n', 'hA'));
    await refreshing;
    expect(buffers.all[k]).toMatchObject({ text: 'mine\n', saved: 'mine\n', hash: 'h9', disk: 'ok', version: 0 });
  });

  it('does not take an answer from the disk that comes while the file is being written', async () => {
    const late = deferred<unknown>();
    const written = deferred<string>();
    let phase: 'open' | 'gone' | 'slow' = 'open';
    fakeBackend({
      fs_read: () => {
        if (phase === 'gone') throw 'x.ts introuvable';
        return phase === 'slow' ? late.promise : text('a\n', 'h1');
      },
      fs_base: () => null,
      fs_write: () => written.promise,
      set_unsaved: () => null,
    });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    phase = 'gone';
    await buffers.refresh(k);
    expect(buffers.all[k].disk).toBe('deleted');
    phase = 'slow';
    const keeping = buffers.keepMine(k);
    const refreshing = buffers.refresh(k);
    late.resolve(text('agent\n', 'hA'));
    await refreshing;
    expect(buffers.all[k]).toMatchObject({ text: 'a\n', hash: 'h1', version: 0 });
    written.resolve('h2');
    expect(await keeping).toBe(true);
    expect(buffers.all[k]).toMatchObject({ text: 'a\n', saved: 'a\n', hash: 'h2', disk: 'ok', version: 0 });
  });

  it('is not undone by a reload that was answered after a save', async () => {
    const late = deferred<unknown>();
    let slow = false;
    fakeBackend({
      fs_read: () => (slow ? late.promise : text('a\n', 'h1')),
      fs_base: () => null,
      fs_write: () => 'h9',
      set_unsaved: () => null,
    });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(k, 'mine\n');
    slow = true;
    const reloading = buffers.reload(k);
    expect(await buffers.save(k)).toBe(true);
    late.resolve(text('a\n', 'h1'));
    await reloading;
    expect(buffers.all[k]).toMatchObject({ text: 'mine\n', saved: 'mine\n', hash: 'h9', disk: 'ok', version: 0 });
  });

  it('flags a file that vanished when reloading it, and still fails on other errors', async () => {
    let failure: string | null = null;
    fakeBackend({
      fs_read: () => {
        if (failure) throw failure;
        return text('a\n');
      },
      fs_base: () => null,
      set_unsaved: () => null,
    });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(k, 'mine\n');
    failure = 'x.ts introuvable';
    await expect(buffers.reload(k)).resolves.toBeUndefined();
    expect(buffers.all[k]).toMatchObject({ disk: 'deleted', text: 'mine\n', version: 0 });
    failure = 'boom';
    await expect(buffers.reload(k)).rejects.toBe('boom');
  });

  it('reads again a file that was missing or failed to open when it is opened again', async () => {
    let state: 'missing' | 'broken' | 'there' = 'missing';
    const backend = fakeBackend({
      fs_read: () => {
        if (state === 'missing') throw 'x.ts introuvable';
        if (state === 'broken') throw 'boom';
        return text('a\n');
      },
      fs_base: () => null,
      set_unsaved: () => null,
    });
    expect((await buffers.open('p1', 'project', 'x.ts')).kind).toBe('missing');
    state = 'broken';
    expect(await buffers.open('p1', 'project', 'x.ts')).toMatchObject({ kind: 'error', error: 'boom' });
    state = 'there';
    const b = await buffers.open('p1', 'project', 'x.ts');
    expect(b).toMatchObject({ kind: 'text', text: 'a\n', error: null });
    expect(buffers.all[b.key]).toMatchObject({ kind: 'text', text: 'a\n' });
    expect(backend.called('fs_read')).toHaveLength(3);
  });

  it('tells the backend again about the unsaved files when a report failed', async () => {
    let fail = true;
    const backend = fakeBackend({
      fs_read: () => text('a'),
      fs_base: () => null,
      set_unsaved: () => {
        if (fail) {
          fail = false;
          throw 'backend gone';
        }
        return null;
      },
    });
    const k = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(k, 'b');
    await new Promise((r) => setTimeout(r, 0));
    buffers.edit(k, 'bb');
    await expect.poll(() => backend.called('set_unsaved')).toHaveLength(2);
    expect(backend.called('set_unsaved').map((c) => c.args)).toEqual([{ count: 1 }, { count: 1 }]);
  });

  describe('compared with the version on disk (« Comparer »)', () => {
    /** The file on disk, null once deleted. */
    let disk: ReturnType<typeof text> | null;

    /** A file typed in (`mine`) that the agent changed on disk meanwhile (`agent`, h2): its banner is up. */
    async function conflict() {
      disk = text('a\n', 'h1');
      const backend = fakeBackend({
        fs_read: () => {
          if (!disk) throw 'x.ts introuvable';
          return disk;
        },
        fs_base: () => null,
        // As the backend does: a write expecting another version than the one on disk is refused.
        fs_write: (a: any) => (a.expectedHash === null || a.expectedHash === disk?.hash ? 'h9' : Promise.reject('changed')),
        set_unsaved: () => null,
      });
      const k = (await buffers.open('p1', 'project', 'x.ts')).key;
      buffers.edit(k, 'mine\n');
      disk = text('agent\n', 'h2');
      await buffers.refresh(k);
      expect(buffers.all[k]).toMatchObject({ disk: 'changed', onDisk: null });
      return { k, backend };
    }

    it('reads the version on disk without touching what was typed', async () => {
      const { k } = await conflict();
      await buffers.compare(k);
      expect(buffers.all[k]).toMatchObject({
        text: 'mine\n',
        saved: 'a\n',
        hash: 'h1',
        disk: 'changed',
        version: 0,
        onDisk: { text: 'agent\n', hash: 'h2' },
      });
    });

    it('follows the disk while compared: a version written since replaces the one compared, what was typed stays', async () => {
      const { k } = await conflict();
      await buffers.compare(k);
      disk = text('agent2\n', 'h3');
      await buffers.refresh(k);
      expect(buffers.all[k]).toMatchObject({
        text: 'mine\n',
        hash: 'h1',
        disk: 'changed',
        version: 0,
        onDisk: { text: 'agent2\n', hash: 'h3' },
      });
    });

    it('keeps the text merged over the version compared', async () => {
      const { k, backend } = await conflict();
      await buffers.compare(k);
      buffers.edit(k, 'mine\nagent\n');
      expect(await buffers.keepMine(k)).toBe(true);
      expect(backend.called('fs_write')[0].args).toMatchObject({ text: 'mine\nagent\n', expectedHash: 'h2' });
      expect(buffers.all[k]).toMatchObject({ saved: 'mine\nagent\n', hash: 'h9', disk: 'ok', onDisk: null });
    });

    it('does not write over a version of the disk the comparison has not shown, and shows it instead', async () => {
      const { k, backend } = await conflict();
      await buffers.compare(k);
      // Written by the agent once the comparison was read, before anything read the file again.
      disk = text('agent2\n', 'h3');
      expect(await buffers.keepMine(k)).toBe(false);
      expect(backend.called('fs_write')[0].args.expectedHash).toBe('h2');
      expect(buffers.all[k]).toMatchObject({
        text: 'mine\n',
        saved: 'a\n',
        hash: 'h1',
        disk: 'changed',
        onDisk: { text: 'agent2\n', hash: 'h3' },
      });
    });

    it('ends the comparison with a reload', async () => {
      const { k } = await conflict();
      await buffers.compare(k);
      await buffers.reload(k);
      expect(buffers.all[k]).toMatchObject({ text: 'agent\n', disk: 'ok', onDisk: null });
    });

    it('ends the comparison once the disk is back to the version read, or once the file is gone', async () => {
      const { k } = await conflict();
      await buffers.compare(k);
      disk = text('a\n', 'h1');
      await buffers.refresh(k);
      expect(buffers.all[k]).toMatchObject({ text: 'mine\n', disk: 'ok', onDisk: null });
      disk = text('agent\n', 'h2');
      await buffers.refresh(k);
      await buffers.compare(k);
      disk = null;
      await buffers.refresh(k);
      expect(buffers.all[k]).toMatchObject({ text: 'mine\n', disk: 'deleted', onDisk: null });
    });

    it('refuses to compare with a version of the disk that is not text', async () => {
      const { k } = await conflict();
      disk = { kind: 'binary', text: null, size: 10, hash: 'h3', eol: 'lf', bom: false } as unknown as ReturnType<typeof text>;
      await expect(buffers.compare(k)).rejects.toBe('la version du disque n’est pas du texte');
      expect(buffers.all[k]).toMatchObject({ text: 'mine\n', disk: 'changed', onDisk: null });
    });

    it('does not take for a comparison an answer from the disk that comes after a save', async () => {
      const { k } = await conflict();
      const late = deferred<unknown>();
      fakeBackend({ fs_read: () => late.promise, fs_base: () => null, fs_write: () => 'h9', set_unsaved: () => null });
      const comparing = buffers.compare(k);
      expect(await buffers.keepMine(k)).toBe(true);
      late.resolve(text('agent\n', 'h2'));
      await comparing;
      expect(buffers.all[k]).toMatchObject({ text: 'mine\n', saved: 'mine\n', disk: 'ok', onDisk: null });
    });

    it('tells that a newer version took the place of the one compared, read again or found by a refused save, until the user types', async () => {
      const { k } = await conflict();
      await buffers.compare(k);
      expect(buffers.all[k].onDisk).toMatchObject({ hash: 'h2', replaced: null });
      disk = text('agent2\n', 'h3');
      await buffers.refresh(k);
      expect(buffers.all[k].onDisk).toMatchObject({ text: 'agent2\n', hash: 'h3', replaced: 'read' });
      // Read again as it is: nothing new to tell.
      await buffers.refresh(k);
      expect(buffers.all[k].onDisk).toMatchObject({ hash: 'h3', replaced: 'read' });
      buffers.edit(k, 'mine2\n');
      expect(buffers.all[k].onDisk).toMatchObject({ hash: 'h3', replaced: null });
      disk = text('agent3\n', 'h4');
      expect(await buffers.keepMine(k)).toBe(false);
      expect(buffers.all[k].onDisk).toMatchObject({ text: 'agent3\n', hash: 'h4', replaced: 'save' });
    });

    it('saves nothing, and has nothing left to choose, when a refused save finds the disk back at the version read', async () => {
      const { k, backend } = await conflict();
      await buffers.compare(k);
      disk = text('a\n', 'h1');
      expect(await buffers.keepMine(k)).toBe(false);
      expect(backend.called('fs_write')[0].args.expectedHash).toBe('h2');
      expect(buffers.all[k]).toMatchObject({ text: 'mine\n', saved: 'a\n', hash: 'h1', disk: 'ok', onDisk: null });
      expect(buffers.isDirty(buffers.all[k])).toBe(true);
    });
  });
});

describe('buffers of files renamed or deleted', () => {
  beforeEach(() => buffers.reset());

  /** A backend whose files read `<path>\n` (hash h1), and whose reference version is HEAD's for `src/a.ts` only. */
  function files() {
    return fakeBackend({
      fs_read: (a: any) => text(`${a.path}\n`),
      fs_base: (a: any) => ({ reference: 'HEAD', text: a.path === 'src/a.ts' ? 'head\n' : null }),
      fs_write: () => 'h2',
      set_unsaved: () => null,
    });
  }

  it('moves a file renamed to its new path: the same buffer, what was typed and the comparison with the disk kept', async () => {
    const backend = files();
    const from = (await buffers.open('p1', 'project', 'src/a.ts')).key;
    await expect.poll(() => buffers.all[from].base?.text).toBe('head\n');
    buffers.edit(from, 'mine\n');
    buffers.all[from].disk = 'changed';
    buffers.all[from].onDisk = { text: 'agent\n', hash: 'h3', replaced: null };
    const before = buffers.all[from];
    buffers.move('p1', 'project', 'src/a.ts', 'lib/b.ts');
    const to = buffers.key('p1', 'project', 'lib/b.ts');
    expect(buffers.all[from]).toBeUndefined();
    expect(buffers.all[to]).toBe(before);
    expect(buffers.all[to]).toMatchObject({
      key: to,
      path: 'lib/b.ts',
      text: 'mine\n',
      saved: 'src/a.ts\n',
      hash: 'h1',
      disk: 'changed',
      onDisk: { text: 'agent\n', hash: 'h3' },
    });
    expect(buffers.isDirty(buffers.all[to])).toBe(true);
    // The reference version is the new path's.
    await expect.poll(() => buffers.all[to].base).toEqual({ reference: 'HEAD', text: null });
    expect(backend.called('fs_base').at(-1)?.args.path).toBe('lib/b.ts');
    // Saved where it is now, over the version read.
    expect(await buffers.save(to, true)).toBe(true);
    expect(backend.called('fs_write')[0].args).toMatchObject({ path: 'lib/b.ts', text: 'mine\n', expectedHash: 'h3' });
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 0 });
  });

  it('moves the files of a folder renamed, of its source only, and none of a folder whose name starts the same', async () => {
    files();
    for (const p of ['src/a.ts', 'src/lib/x.ts', 'src2/y.ts']) await buffers.open('p1', 'project', p);
    await buffers.open('p1', 'a2', 'src/a.ts');
    buffers.move('p1', 'project', 'src', 'source');
    expect(Object.keys(buffers.all).sort()).toEqual([
      'p1|a2|src/a.ts',
      'p1|project|source/a.ts',
      'p1|project|source/lib/x.ts',
      'p1|project|src2/y.ts',
    ]);
    expect(buffers.all['p1|project|source/lib/x.ts']).toMatchObject({ path: 'source/lib/x.ts', text: 'src/lib/x.ts\n' });
  });

  it('keeps the same id through a rename, and gives each file opened its own', async () => {
    files();
    const a = await buffers.open('p1', 'project', 'a.ts');
    const b = await buffers.open('p1', 'project', 'b.ts');
    expect(a.id).not.toBe(b.id);
    const id = a.id;
    buffers.move('p1', 'project', 'a.ts', 'c.ts');
    expect(buffers.all['p1|project|c.ts'].id).toBe(id);
  });

  it('does not take a read that was under way for the old path', async () => {
    const late = deferred<unknown>();
    fakeBackend({ fs_read: () => late.promise, fs_base: () => null, set_unsaved: () => null });
    const reading = buffers.open('p1', 'project', 'a.ts');
    buffers.move('p1', 'project', 'a.ts', 'b.ts');
    late.resolve(text('a\n'));
    await reading;
    expect(buffers.all).toEqual({});
  });

  it('refuses to move a file over an unsaved one left open at its new path, and moves nothing then', async () => {
    files();
    await buffers.open('p1', 'project', 'src/a.ts');
    await buffers.open('p1', 'project', 'src/b.ts');
    // Deleted on disk by the agent, their tabs still hold what was typed.
    await buffers.open('p1', 'project', 'gone.ts');
    buffers.edit('p1|project|gone.ts', 'mine\n');
    await buffers.open('p1', 'project', 'lib/b.ts');
    buffers.edit('p1|project|lib/b.ts', 'mine\n');
    expect(buffers.inTheWay('p1', 'project', 'src/a.ts', 'gone.ts')?.path).toBe('gone.ts');
    expect(() => buffers.move('p1', 'project', 'src/a.ts', 'gone.ts')).toThrow(
      '« gone.ts » est ouvert avec des modifications non enregistrées.',
    );
    // A folder whose files would take the place of one.
    expect(() => buffers.move('p1', 'project', 'src', 'lib')).toThrow('« b.ts » est ouvert avec des modifications non enregistrées.');
    expect(Object.keys(buffers.all).sort()).toEqual([
      'p1|project|gone.ts',
      'p1|project|lib/b.ts',
      'p1|project|src/a.ts',
      'p1|project|src/b.ts',
    ]);
    expect(buffers.all['p1|project|gone.ts'].text).toBe('mine\n');
    // Saved, it gives its place.
    buffers.edit('p1|project|gone.ts', 'gone.ts\n');
    expect(buffers.inTheWay('p1', 'project', 'src/a.ts', 'gone.ts')).toBeUndefined();
    buffers.move('p1', 'project', 'src/a.ts', 'gone.ts');
    expect(buffers.all['p1|project|gone.ts'].text).toBe('src/a.ts\n');
  });

  it('forgets the clean files of a file or a folder deleted, and the unsaved ones given up, as they were then', async () => {
    const backend = files();
    for (const p of ['src/a.ts', 'src/lib/x.ts', 'src/lib/y.ts', 'src2/y.ts', 'README.md']) await buffers.open('p1', 'project', p);
    await buffers.open('p1', 'a2', 'src/a.ts');
    buffers.edit('p1|project|src/lib/x.ts', 'mine\n');
    buffers.edit('p1|project|src/lib/y.ts', 'given up\n');
    // « Ne pas enregistrer » answered for both, then typed in again in one of them.
    const discarded = new Map([
      ['p1|project|src/lib/x.ts', 'mine\n'],
      ['p1|project|src/lib/y.ts', 'given up\n'],
    ]);
    buffers.edit('p1|project|src/lib/x.ts', 'mine, typed since\n');
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 2 });
    expect(buffers.closePath('p1', 'project', 'src', discarded)).toEqual(['src/lib/x.ts']);
    expect(buffers.closePath('p1', 'project', 'README.md')).toEqual([]);
    expect(Object.keys(buffers.all).sort()).toEqual(['p1|a2|src/a.ts', 'p1|project|src/lib/x.ts', 'p1|project|src2/y.ts']);
    expect(buffers.all['p1|project|src/lib/x.ts'].text).toBe('mine, typed since\n');
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 1 });
  });
});

describe('trees', () => {
  beforeEach(() => trees.reset());

  it('keeps the tree of each source', async () => {
    const backend = fakeBackend({ fs_tree: (a: any) => ({ root: a.agentId ? 'C:/wt' : 'C:/p', files: ['a.ts'], truncated: false }) });
    await trees.load('p1', 'a2');
    await trees.load('p1', 'project');
    expect(backend.called('fs_tree').map((c) => c.args.agentId)).toEqual(['a2', null]);
    expect(trees.get('p1', 'a2')?.root).toBe('C:/wt');
    expect(trees.get('p1', 'project')?.root).toBe('C:/p');
  });

  it('forgets the tree of a source, or the trees of a project', async () => {
    fakeBackend({ fs_tree: () => ({ root: 'C:/p', files: [], truncated: false }) });
    await trees.load('p1', 'a2');
    await trees.load('p1', 'project');
    await trees.load('p2', 'project');
    trees.closeSource('p1', 'a2');
    expect(Object.keys(trees.all)).toEqual(['p1|project', 'p2|project']);
    trees.closeProject('p1');
    expect(Object.keys(trees.all)).toEqual(['p2|project']);
  });
});
