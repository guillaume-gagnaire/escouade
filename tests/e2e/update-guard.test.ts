// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { onThisMachine, updateTrouble } from './update-guard';

describe('the guard against the real updates (ESC-543)', () => {
  it('takes for a test server only an address on this machine', () => {
    for (const local of ['http://127.0.0.1:4170/latest.json', 'http://127.9.9.9/x', 'http://localhost:9/x', 'http://[::1]:4170/x'])
      expect(onThisMachine(local), local).toBe(true);
    for (const remote of [
      'https://github.com/guillaume-gagnaire/escouade/releases/latest/download/latest.json',
      'https://escouade.dev/latest.json',
      'http://localhost.example.com/x',
      'http://10.0.0.2/x',
      'not an address',
    ])
      expect(onThisMachine(remote), remote).toBe(false);
  });

  it('finds in the log a check of the real server and an install started, not an installer the app refused', () => {
    const log = [
      '2026-11-19 10:00:00 INFO  update check off in a debug build: no ESCOUADE_UPDATE_ENDPOINT, the real server is never read',
      '2026-11-19 10:00:01 INFO  update check reads http://127.0.0.1:4170/beta.json, http://127.0.0.1:4170/latest.json',
      "2026-11-19 10:00:02 WARN  a debug build only downloads and installs an update from a test's server: https://github.com/x.exe not downloaded",
    ];
    expect(updateTrouble(log.join('\n'))).toEqual([]);
    const real = '2026-11-19 10:00:03 INFO  update check reads http://127.0.0.1:4170/beta.json, https://escouade.dev/latest.json';
    const install = '2026-11-19 10:00:04 INFO  installing Escouade 2.2.4 (relaunch: false)';
    expect(updateTrouble([...log, real, install].join('\r\n'))).toEqual([real, install]);
  });
});
