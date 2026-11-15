// Prepares a data folder as a power cut leaves it (ESC-469), to see the start take back the copy by hand:
//   node scripts/recovery-demo.mjs <folder>
// then start the app with ESCOUADE_DATA_DIR=<folder>. Its `state.json` holds nothing but zeros; `backups/state/` holds one
// copy with a project (this repository, its autopilot off) and two tickets. The folder is emptied first: never give it your own data folder.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';

const dir = path.resolve(process.argv[2] ?? '');
if (!process.argv[2] || dir === path.join(os.homedir(), '.escouade')) {
  console.error('usage: node scripts/recovery-demo.mjs <a folder of its own, not ~/.escouade>');
  process.exit(1);
}
fs.rmSync(dir, { recursive: true, force: true });
fs.mkdirSync(path.join(dir, 'backups', 'state'), { recursive: true });
const repo = path.resolve(import.meta.dirname, '..');
const state = {
  // The autopilot off: its tickets wait, no agent starts on them.
  projects: [{ id: 'demo469', name: 'escouade', path: repo, color: 'oklch(0.72 0.12 48)', board: { autopilot: false } }],
  tickets: [
    { id: 'd1', projectId: 'demo469', key: 'DEM-1', title: 'Relances par e-mail', column: 'todo', rank: 1 },
    { id: 'd2', projectId: 'demo469', key: 'DEM-2', title: 'Facture en PDF', column: 'todo', rank: 2 },
  ],
  ui: { activeProject: 'demo469', view: 'project', selectedAgent: {} },
};
const stamp = new Date(Date.now() - 20 * 60_000)
  .toISOString()
  .replace(/[-:]/g, '')
  .replace(/\.\d+Z$/, 'Z');
fs.writeFileSync(path.join(dir, 'backups', 'state', `state-${stamp}.json.gz`), zlib.gzipSync(JSON.stringify(state)));
fs.writeFileSync(path.join(dir, 'state.json'), Buffer.alloc(64 * 1024));
fs.writeFileSync(path.join(dir, 'settings.json'), JSON.stringify({ telemetry: false, nameAsked: true, agentMode: 'auto' }));
console.log(`${dir}: state.json in zeros, a copy of ${stamp} in backups/state/`);
