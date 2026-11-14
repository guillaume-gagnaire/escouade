// Le banc iOS de l'étude ESC-445, sur un simulateur (macOS, la CI de la branche study/esc-445-ios) :
//   - le poids : le .app du simulateur, et celui de l'appareil (arm64, non signé) quand il a été construit ;
//   - le démarrage à froid : `simctl launch` jusqu'au repère « ready » du faux serveur (même horloge, celle du Mac) ;
//   - la mémoire, la liste affichée : l'empreinte du processus de l'app (et des processus WebContent de WebKit) ;
//   - le pont : le repère « bridge-us » (1 000 ouvertures appelées par l'app) ;
//   - le push app fermée par `simctl push` : une alerte (une capture d'écran montre la notification), puis un push
//     « données » (content-available) pour voir si l'app se réveille et déchiffre (repère « push-bg »).
// Ce qui ne se mesure pas ici : la fluidité (le simulateur rend par le GPU du Mac), la biométrie et le QR code (il
// faudrait toucher l'écran ; le simulateur n'a pas d'appareil photo).
//
//   PORT=<port du faux serveur> node mobile-study/bench/ios.mjs <nom> <bundle id> <app du simulateur> [<app de l'appareil>]
import { execFileSync, execSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';

const here = dirname(fileURLToPath(import.meta.url));
const [name, bundle, simApp, deviceApp] = process.argv.slice(2);
const port = process.env.PORT;
const runs = Number(process.env.RUNS ?? 6);
const out = join(here, '../results');
mkdirSync(out, { recursive: true });
const simctl = (...a) => execFileSync('xcrun', ['simctl', ...a], { encoding: 'utf8' }).trim();
const server = `http://127.0.0.1:${port}`;
const marks = async () => (await fetch(`${server}/api/marks`)).json();
const clearMarks = () => fetch(`${server}/api/marks`, { method: 'DELETE' });
const median = (a) => {
  const s = [...a].sort((x, y) => x - y);
  return s.length ? s[s.length >> 1] : null;
};
const log = (...a) => console.log(`[${name}]`, ...a);
const du = (p) => (p ? Number(execSync(`du -sk "${p}"`, { encoding: 'utf8' }).split(/\s+/)[0]) / 1024 : null);

async function waitMark(ev, timeoutMs = 30000) {
  const end = Date.now() + timeoutMs;
  while (Date.now() < end) {
    const m = (await marks()).find((k) => k.ev === ev);
    if (m) return m;
    await sleep(50);
  }
  return null;
}

const result = { name, bundle, date: new Date().toISOString(), device: process.env.SIM_NAME ?? 'simulateur' };
result.weight = { simulatorAppMo: +du(simApp).toFixed(1), deviceAppMo: deviceApp ? +du(deviceApp).toFixed(1) : null };
log('poids', result.weight);

simctl('install', 'booted', simApp);
// La permission des notifications, d'avance (applesimutils) : sans elle, simctl push n'affiche rien.
try {
  execSync(`applesimutils --booted --bundle ${bundle} --setPermissions notifications=YES`, { stdio: 'inherit' });
  result.notificationsGranted = true;
} catch {
  result.notificationsGranted = false;
}

const ready = [];
for (let i = 0; i < runs; i++) {
  try {
    simctl('terminate', 'booted', bundle);
  } catch {
    /* pas lancée */
  }
  await sleep(1500);
  await clearMarks();
  const t0 = Date.now();
  simctl('launch', 'booted', bundle);
  const m = await waitMark('ready');
  if (m) ready.push(m.t - t0);
  log(`démarrage ${i + 1}/${runs} : ${m ? m.t - t0 : '?'} ms`);
  await sleep(2000);
}
result.coldStart = { runs, readyMs: { median: median(ready), min: Math.min(...ready), max: Math.max(...ready) } };

// La mémoire, la liste affichée et le pont mesuré : le processus de l'app, et ceux de WebKit nés avec elle.
const bridge = await waitMark('bridge-us', 20000);
result.bridgeUsPerCall = bridge ? Number(bridge.v) : null;
await sleep(3000);
const pid = simctl('spawn', 'booted', 'launchctl', 'list').split('\n').find((l) => l.includes(bundle))?.split(/\s+/)[0];
const rss = (p) => Number(execSync(`ps -o rss= -p ${p}`, { encoding: 'utf8' }).trim() || 0) / 1024;
let webkit = 0;
try {
  const pids = execSync(`pgrep -f "com.apple.WebKit.WebContent" || true`, { encoding: 'utf8' }).split('\n').filter(Boolean);
  // Seuls ceux nés après le dernier lancement (les WebContent d'autres apps du simulateur sont plus vieux).
  for (const p of pids) {
    const age = Number(execSync(`ps -o etimes= -p ${p}`, { encoding: 'utf8' }).trim() || 1e9);
    if (age < 20) webkit += rss(p);
  }
} catch {
  /* pas de WebKit */
}
result.memory = { appRssMo: pid && pid !== '-' ? +rss(pid).toFixed(1) : null, webkitRssMo: +webkit.toFixed(1) };
log('mémoire', result.memory);

// Le push, app fermée : une alerte, puis un push « données ».
const { sealed } = await (await fetch(`${server}/api/push-sample`)).json();
const payload = (aps) => {
  const f = join(out, `push-${name}.json`);
  writeFileSync(f, JSON.stringify({ aps, sealed, agent: 'a1' }));
  return f;
};
simctl('terminate', 'booted', bundle);
await sleep(1500);
await clearMarks();
simctl('push', 'booted', bundle, payload({ alert: { title: 'Escouade', body: 'Un agent attend ta réponse' }, sound: 'default' }));
await sleep(3000);
simctl('io', 'booted', 'screenshot', join(out, `ios-push-${name}.png`));
await clearMarks();
simctl('push', 'booted', bundle, payload({ 'content-available': 1 }));
const bg = await waitMark('push-bg', 15000);
result.push = {
  alertShot: `results/ios-push-${name}.png`,
  backgroundWake: bg ? bg.v : null,
};
log('push', result.push);
writeFileSync(join(out, `ios-${name}.json`), JSON.stringify(result, null, 2) + '\n');
log('écrit', `results/ios-${name}.json`);
