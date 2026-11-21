import { describe, expect, it } from 'vitest';
import { treeOf, type Proc } from './proctree';

const AT = 1_000_000;
const p = (pid: number, ppid: number, created: number, cmd = ''): Proc => ({ pid, ppid, created, cmd });
const pids = (list: Proc[]) => list.map((x) => x.pid).sort((a, b) => a - b);

describe('the processes of an app under test (ESC-522)', () => {
  it('are the app while it runs, its children and theirs', () => {
    const procs = [p(10, 1, AT + 5), p(11, 10, AT + 100), p(12, 11, AT + 200), p(13, 12, AT + 300), p(99, 1, AT + 50)];
    expect(pids(treeOf(procs, { pid: 10, at: AT, exitedAt: null }))).toEqual([10, 11, 12, 13]);
  });

  it('never take a later process given the pid of one gone for its child', () => {
    // 12's parent, gone, had the pid 11, which a later process got: 12 is older than it, so not its child.
    const procs = [p(10, 1, AT + 5), p(12, 11, AT + 200), p(11, 1, AT + 900), p(14, 10, AT + 50)];
    expect(pids(treeOf(procs, { pid: 10, at: AT, exitedAt: null }))).toEqual([10, 14]);
  });

  it('nor a process started before the app, whatever parent it names', () => {
    const procs = [p(10, 1, AT + 5), p(20, 10, AT - 60_000)];
    expect(pids(treeOf(procs, { pid: 10, at: AT, exitedAt: null }))).toEqual([10]);
  });

  it('the app gone, are its orphans started before its end, never a process that got its pid since, nor its children', () => {
    const exitedAt = AT + 5_000;
    const procs = [
      // Its WebView2 and a fake CLI, alive after it.
      p(11, 10, AT + 100),
      p(12, 11, AT + 150),
      p(15, 10, AT + 400),
      // The pid 10 given to another process after its end, and that one's child.
      p(10, 1, exitedAt + 60_000),
      p(30, 10, exitedAt + 61_000),
    ];
    expect(pids(treeOf(procs, { pid: 10, at: AT, exitedAt }))).toEqual([11, 12, 15]);
  });

  it('take the processes the test knows by their command line, and their children, never ones started before', () => {
    const folder = 'C:\Temp\escouade-e2e-abc';
    const procs = [
      p(40, 7, AT + 100, `msedgewebview2.exe --user-data-dir="${folder}\webview\EBWebView"`),
      p(41, 40, AT + 120, 'msedgewebview2.exe --type=renderer'),
      p(50, 7, AT - 10_000, `msedgewebview2.exe --user-data-dir="${folder}\webview\EBWebView"`),
      p(60, 7, AT + 100, 'C:\Program Files\Escouade\escouade.exe'),
    ];
    const ours = (x: Proc) => x.cmd.includes(folder);
    expect(pids(treeOf(procs, { pid: 10, at: AT, exitedAt: AT + 1000 }, ours))).toEqual([40, 41]);
  });
});
