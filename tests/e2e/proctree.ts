// The end of an app under test (fixture.ts): the process the test launched and everything it started, by their pids,
// never by a program's name (`escouade.exe` is also the user's own app, running on the same machine, ESC-522).

import { execFile } from 'node:child_process';

/** A process of the machine, as Windows lists it. */
export interface Proc {
  pid: number;
  /** The pid of the process that started it, which may be gone (an orphan) or since given to another. */
  ppid: number;
  /** When it started, in ms since 1970. */
  created: number;
  cmd: string;
}

/** A process the test started: its pid, when it was launched, and when it ended (null while it runs, as Node knows it). */
export interface Launched {
  pid: number;
  at: number;
  exitedAt: number | null;
}

/** How far the clock of Windows' start times and Node's `Date.now()` may differ. */
const SKEW_MS = 2000;

/**
 * The processes of `root` in `procs`: itself while it runs, every process it started and theirs. A process is a child
 * of a process of the tree that is still there and started no earlier than it: a pid Windows gave again to a later process
 * is never taken for its parent. The root gone, its orphans are the processes that name it and started between its launch
 * and its end (one started after is another's, its pid given again). `ours` adds the processes known to be the test's by
 * something else (its own folder in their command line: a WebView2 whose parent died before it), and theirs.
 */
export function treeOf(procs: Proc[], root: Launched, ours: (p: Proc) => boolean = () => false): Proc[] {
  const found = new Map<number, Proc>();
  const since = root.at - SKEW_MS;
  for (const p of procs) {
    if (p.created < since) continue;
    if ((p.pid === root.pid && root.exitedAt === null) || ours(p)) found.set(p.pid, p);
  }
  const orphanOfRoot = (p: Proc) =>
    root.exitedAt !== null && p.ppid === root.pid && p.created >= since && p.created <= root.exitedAt + SKEW_MS;
  for (let grew = true; grew;) {
    grew = false;
    for (const p of procs) {
      if (found.has(p.pid) || p.pid === p.ppid) continue;
      const parent = found.get(p.ppid);
      if ((parent && p.created >= parent.created) || orphanOfRoot(p)) {
        found.set(p.pid, p);
        grew = true;
      }
    }
  }
  return [...found.values()];
}

/**
 * Every process of the machine (pid, parent, start, command line), by one query; null when it could not be read (the query
 * failed or took too long, on a loaded machine): an unread list is never an empty one (ESC-522: the end of an app took it for
 * « all gone », a WebView2 left on the DevTools port). A list without this very process is not read either.
 */
export function processes(): Promise<Proc[] | null> {
  const script =
    '[Console]::OutputEncoding = [Text.Encoding]::UTF8; ' +
    'Get-CimInstance Win32_Process | ForEach-Object { ' +
    "$c = if ($_.CreationDate) { [long](($_.CreationDate.ToUniversalTime() - [datetime]'1970-01-01').TotalMilliseconds) } else { 0 }; " +
    '"$($_.ProcessId)`t$($_.ParentProcessId)`t$c`t$($_.CommandLine)" }';
  return new Promise((resolve) =>
    execFile(
      'powershell',
      ['-NoProfile', '-NonInteractive', '-Command', script],
      { encoding: 'utf8', timeout: 60_000, maxBuffer: 64 * 1024 * 1024, windowsHide: true },
      (err, out) => {
        if (err) return resolve(null);
        const list: Proc[] = [];
        for (const line of out.split(/\r?\n/)) {
          const [pid, ppid, created, ...cmd] = line.split('\t');
          if (!pid || Number.isNaN(Number(pid))) continue;
          list.push({ pid: Number(pid), ppid: Number(ppid), created: Number(created), cmd: cmd.join('\t') });
        }
        resolve(list.some((p) => p.pid === process.pid) ? list : null);
      },
    ),
  );
}

/** Ends the process `pid` and the tree Windows still links to it (`taskkill /T`), by its pid: when the list cannot be read. */
function killTree(pid: number): Promise<string> {
  return new Promise((resolve) =>
    execFile('taskkill', ['/F', '/T', '/PID', String(pid)], { windowsHide: true, timeout: 30_000, encoding: 'latin1' }, (_err, out, err) =>
      resolve(`${out}${err}`.trim()),
    ),
  );
}

/** Ends the processes `pids` at once, each by its pid alone (`taskkill /F /PID`, never `/IM` nor `/T`). */
function kill(pids: number[]): Promise<string> {
  if (!pids.length) return Promise.resolve('');
  return new Promise((resolve) =>
    execFile(
      'taskkill',
      ['/F', ...pids.flatMap((p) => ['/PID', String(p)])],
      { windowsHide: true, timeout: 30_000, encoding: 'latin1' },
      (_err, out, err) => resolve(`${out}${err}`.trim()),
    ),
  );
}

/**
 * Ends `root` (as it is now: `root()`) and all its processes, and waits until none of them is left (`timeoutMs`): the ones still there then, by
 * their pids and command lines (empty when all are gone), and what taskkill said of them the last time. `ours` adds the processes known to be the test's by their command
 * line (its own folder: a WebView2 or a fake CLI whose parent died before it). A process that started after the list was
 * read is caught by the next reading: the tree is read again until it is empty.
 */
export async function endTree(
  root: () => Launched,
  ours: (p: Proc) => boolean,
  timeoutMs = 30_000,
): Promise<{ left: Proc[]; said: string }> {
  const deadline = Date.now() + timeoutMs;
  let said = '';
  for (;;) {
    // Read again each time: the root's end, once Node has seen it, makes its children orphans.
    const procs = await processes();
    if (!procs) {
      // Not read: nothing is known gone. The root at least, by its pid while Node holds it, with what it started.
      const r = root();
      if (r.exitedAt === null) said = await killTree(r.pid);
      if (Date.now() >= deadline) {
        return { left: [{ pid: r.pid, ppid: 0, created: r.at, cmd: '(the list of processes could not be read)' }], said };
      }
      await new Promise((res) => setTimeout(res, 1000));
      continue;
    }
    const tree = treeOf(procs, root(), ours);
    if (!tree.length || Date.now() >= deadline) return { left: tree, said };
    // What taskkill answered the last time (a refusal says why a process outlives it).
    said = await kill(tree.map((p) => p.pid));
    await new Promise((r) => setTimeout(r, 250));
  }
}

/**
 * Who listens on the local `port`: each process by its pid, its parent and its command line (for the message of a port
 * still taken: what the end of an app did not reach, ESC-522); « nobody » when none does.
 */
export function listenersOn(port: number): Promise<string> {
  const script =
    '[Console]::OutputEncoding = [Text.Encoding]::UTF8; ' +
    `Get-NetTCPConnection -LocalPort ${port} -State Listen -ErrorAction SilentlyContinue | ForEach-Object { ` +
    '$p = Get-CimInstance Win32_Process -Filter "ProcessId=$($_.OwningProcess)"; ' +
    '"pid $($_.OwningProcess) (parent $($p.ParentProcessId), started $($p.CreationDate)): $($p.CommandLine)" }';
  return new Promise((resolve) =>
    execFile(
      'powershell',
      ['-NoProfile', '-NonInteractive', '-Command', script],
      { encoding: 'utf8', timeout: 30_000, windowsHide: true },
      (err, out) => resolve(err ? `unknown (${err.message.split('\n')[0]})` : out.trim() || 'nobody'),
    ),
  );
}
