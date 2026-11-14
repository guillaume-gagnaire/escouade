// Le banc Android de l'étude ESC-445, le même pour les quatre maquettes, sur l'émulateur bridé (ou un téléphone) :
//   - le poids : l'APK, et ce qu'un téléphone arm64 téléchargerait (l'APK sans les bibliothèques des autres ABI) ;
//   - le démarrage à froid : `am start -W` (la première image) et le repère « ready » (la liste des agents affichée,
//     données du faux serveur comprises), mesuré sur l'horloge de l'hôte, après un vidage des caches ;
//   - l'ouverture de la conversation de 300 messages (du toucher au repère « conv-ready ») ;
//   - la mémoire, conversation ouverte : le PSS de l'app, plus celui du processus de rendu de la WebView s'il y en a un ;
//   - la fluidité : des glissés lents puis des lancers dans la conversation, les images présentées relevées par
//     SurfaceFlinger (`--latency` sur la couche BLAST de l'app : la même mesure pour une WebView, des vues natives ou
//     Flutter) : images par seconde pendant le geste, et saccades (un intervalle de plus d'1,5 période d'écran) ;
//   - le push app fermée (bench/android-push.mjs), en « notification » puis en « données ».
//
//   PORT=<port du faux serveur> node mobile-study/bench/android.mjs <nom> <paquet> <apk> [--runs 8]
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';

const here = dirname(fileURLToPath(import.meta.url));
const [name, pkg, apk, ...rest] = process.argv.slice(2);
const runs = Number(rest[rest.indexOf('--runs') + 1] || 8) || 8;
const port = process.env.PORT;
if (!name || !pkg || !apk || !port) throw new Error('usage : PORT=<port> android.mjs <nom> <paquet> <apk> [--runs N]');

const adb = (...a) => execFileSync('adb', a, { encoding: 'utf8', maxBuffer: 64 << 20 }).trim();
const sh = (cmd) => adb('shell', cmd);
const server = `http://127.0.0.1:${port}`;
const marks = async () => (await fetch(`${server}/api/marks`)).json();
const clearMarks = () => fetch(`${server}/api/marks`, { method: 'DELETE' });
const median = (a) => {
  const s = [...a].sort((x, y) => x - y);
  return s.length ? (s.length % 2 ? s[s.length >> 1] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2) : null;
};
const log = (...a) => console.log(`[${name}]`, ...a);

async function waitMark(ev, timeoutMs = 30000) {
  const end = Date.now() + timeoutMs;
  while (Date.now() < end) {
    const m = (await marks()).find((k) => k.ev === ev);
    if (m) return m;
    await sleep(50);
  }
  return null;
}

// ---------- le poids ----------

/** Les entrées d'un zip (répertoire central) : nom et taille compressée. */
function zipEntries(file) {
  const b = readFileSync(file);
  let eocd = b.length - 22;
  while (eocd >= 0 && b.readUInt32LE(eocd) !== 0x06054b50) eocd--;
  const count = b.readUInt16LE(eocd + 10);
  let p = b.readUInt32LE(eocd + 16);
  const out = [];
  for (let i = 0; i < count; i++) {
    const comp = b.readUInt32LE(p + 20);
    const nameLen = b.readUInt16LE(p + 28);
    const extra = b.readUInt16LE(p + 30);
    const comment = b.readUInt16LE(p + 32);
    out.push({ name: b.toString('utf8', p + 46, p + 46 + nameLen), comp });
    p += 46 + nameLen + extra + comment;
  }
  return out;
}

function weight() {
  const total = statSync(apk).size;
  const entries = zipEntries(apk);
  const byAbi = {};
  let dex = 0;
  let assets = 0;
  for (const e of entries) {
    const m = e.name.match(/^lib\/([^/]+)\//);
    if (m) byAbi[m[1]] = (byAbi[m[1]] ?? 0) + e.comp;
    else if (e.name.endsWith('.dex')) dex += e.comp;
    else if (e.name.startsWith('assets/')) assets += e.comp;
  }
  const others = Object.entries(byAbi).filter(([abi]) => abi !== 'arm64-v8a').reduce((s, [, v]) => s + v, 0);
  // Les plus grosses bibliothèques natives d'arm64 : ce qui pèse.
  const arm64Libs = entries
    .filter((e) => e.name.startsWith('lib/arm64-v8a/'))
    .sort((a, b) => b.comp - a.comp)
    .slice(0, 6)
    .map((e) => ({ lib: e.name.slice('lib/arm64-v8a/'.length), mo: +(e.comp / 1048576).toFixed(2) }));
  return {
    apkMo: +(total / 1048576).toFixed(2),
    arm64Mo: +((total - others) / 1048576).toFixed(2),
    abis: Object.fromEntries(Object.entries(byAbi).map(([k, v]) => [k, +(v / 1048576).toFixed(2)])),
    dexMo: +(dex / 1048576).toFixed(2),
    assetsMo: +(assets / 1048576).toFixed(2),
    arm64Libs,
  };
}

// ---------- le démarrage à froid ----------

function launcher() {
  const out = sh(`cmd package resolve-activity --brief -c android.intent.category.LAUNCHER ${pkg}`).split('\n');
  return out[out.length - 1].trim();
}

async function coldStart(component) {
  // Le coût d'un aller-retour adb, retranché du repère « ready » (mesuré sur l'hôte).
  const rt = [];
  for (let i = 0; i < 5; i++) {
    const t = Date.now();
    sh('true');
    rt.push(Date.now() - t);
  }
  const adbMs = median(rt);
  const first = [];
  const ready = [];
  for (let i = 0; i < runs; i++) {
    sh(`am force-stop ${pkg}`);
    sh('sync; echo 3 > /proc/sys/vm/drop_caches');
    await sleep(1500);
    await clearMarks();
    const t0 = Date.now();
    const out = sh(`am start -W -n ${component}`);
    const total = out.match(/TotalTime: (\d+)/);
    const m = await waitMark('ready');
    if (total) first.push(Number(total[1]));
    if (m) ready.push(m.t - t0 - adbMs / 2);
    log(`démarrage ${i + 1}/${runs} : première image ${total?.[1] ?? '?'} ms, liste affichée ${m ? Math.round(m.t - t0 - adbMs / 2) : '?'} ms`);
    await sleep(2500);
  }
  return {
    runs,
    adbRoundTripMs: adbMs,
    firstFrameMs: { median: median(first), min: Math.min(...first), max: Math.max(...first) },
    readyMs: { median: Math.round(median(ready)), min: Math.round(Math.min(...ready)), max: Math.round(Math.max(...ready)) },
  };
}

// ---------- l'écran ----------

// Le premier vidage de l'écran juste après un lancement échoue parfois (uiautomator, ou l'accessibilité de la
// WebView pas encore prête) : plusieurs essais.
async function find(label) {
  const re = new RegExp(`(?:text|content-desc)="[^"]*${label}[^"]*"[^>]*bounds="\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]"`);
  for (let i = 0; i < 10; i++) {
    try {
      sh('uiautomator dump /sdcard/ui.xml');
      const m = sh('cat /sdcard/ui.xml').match(re);
      if (m) return { x: (+m[1] + +m[3]) >> 1, y: (+m[2] + +m[4]) >> 1 };
    } catch {
      /* encore */
    }
    await sleep(1000);
  }
  return null;
}

// ---------- la mémoire ----------

function pssOf(target) {
  const out = sh(`dumpsys meminfo ${target}`);
  const m = out.match(/TOTAL PSS:\s+(\d+)/) ?? out.match(/^\s*TOTAL\s+(\d+)/m);
  return m ? Number(m[1]) : null;
}

function memory() {
  const app = pssOf(pkg);
  // Le processus de rendu de la WebView (isolé, au nom du fournisseur de WebView) : l'app est seule à tourner.
  const renderers = sh('ps -A -o PID,NAME')
    .split('\n')
    .filter((l) => /sandboxed_process|webview_zygote|:privileged_process/.test(l))
    .map((l) => l.trim().split(/\s+/)[0]);
  let renderer = 0;
  for (const pid of renderers) renderer += pssOf(pid) ?? 0;
  return { appMo: +(app / 1024).toFixed(1), webviewRendererMo: +(renderer / 1024).toFixed(1), totalMo: +((app + renderer) / 1024).toFixed(1) };
}

// ---------- la fluidité ----------

function layers() {
  // Toutes les couches de l'app (la fenêtre, et la SurfaceView de Flutter en « (BLAST) ») : celle qui a reçu le plus
  // d'images pendant le geste est celle qui dessine.
  return [...new Set(sh('dumpsys SurfaceFlinger --list').match(new RegExp(`[^\\s{}]*${pkg.replace(/\./g, '\\.')}[^\\s{}]*#\\d+`, 'g')) ?? [])];
}

function latency(layer) {
  const lines = sh(`dumpsys SurfaceFlinger --latency '${layer}'`).split('\n');
  const period = Number(lines[0]) / 1e6;
  const frames = lines
    .slice(1)
    .map((l) => l.trim().split(/\s+/).map(Number))
    .filter((f) => f.length === 3 && f[1] > 0 && f[1] < 9e18)
    .map((f) => f[1] / 1e6);
  return { period, frames };
}

async function gesture(kind) {
  const ls = layers();
  for (const l of ls) sh(`dumpsys SurfaceFlinger --latency-clear '${l}'`);
  // Le doigt descend : la conversation remonte vers les messages plus anciens, à rendre.
  if (kind === 'drag') sh('input swipe 360 450 360 1350 1500');
  else sh('input swipe 360 450 360 1350 90');
  await sleep(kind === 'drag' ? 300 : 1800);
  let best = { period: 16.67, frames: [] };
  for (const l of ls) {
    const r = latency(l);
    if (r.frames.length > best.frames.length) best = r;
  }
  const iv = [];
  for (let i = 1; i < best.frames.length; i++) iv.push(best.frames[i] - best.frames[i - 1]);
  // Les intervalles de plus de 250 ms : l'animation est finie, la liste au repos (pas une saccade).
  const moving = iv.filter((d) => d < 250);
  const janks = moving.filter((d) => d > best.period * 1.5).length;
  const span = moving.reduce((s, d) => s + d, 0);
  return { frames: moving.length + 1, spanMs: Math.round(span), janks, worstMs: Math.round(Math.max(0, ...moving)), period: best.period, iv: moving };
}

async function fluidity() {
  const out = {};
  for (const kind of ['drag', 'fling']) {
    const all = [];
    let frames = 0;
    let span = 0;
    let janks = 0;
    let worst = 0;
    for (let i = 0; i < 6; i++) {
      const g = await gesture(kind);
      frames += g.frames;
      span += g.spanMs;
      janks += g.janks;
      worst = Math.max(worst, g.worstMs);
      all.push(...g.iv);
      await sleep(600);
    }
    const sorted = all.sort((a, b) => a - b);
    out[kind] = {
      gestures: 6,
      frames,
      fps: span ? +((all.length / span) * 1000).toFixed(1) : null,
      jankPct: all.length ? +((janks / all.length) * 100).toFixed(1) : null,
      p95FrameMs: sorted.length ? +sorted[Math.floor(sorted.length * 0.95)].toFixed(1) : null,
      worstMs: worst,
    };
    log(`fluidité (${kind}) : ${out[kind].fps} i/s, ${out[kind].jankPct} % de saccades, p95 ${out[kind].p95FrameMs} ms`);
  }
  return out;
}

// ---------- le push ----------

function push(kind, from = 'session') {
  const env = { ...process.env, PUSH_FROM: from };
  const out = execFileSync('node', [join(here, 'android-push.mjs'), pkg, kind], { encoding: 'utf8', env });
  const last = out.trim().split('\n').pop();
  return JSON.parse(last);
}

// ---------- le banc ----------

adb('root');
await sleep(2000);
adb('wait-for-device');
for (const other of ['tauri', 'capacitor', 'flutter', 'rn']) sh(`am force-stop dev.escouade.study.${other}`);
sh('settings put global window_animation_scale 1; settings put global transition_animation_scale 1; settings put global animator_duration_scale 1');

const result = { name, pkg, date: new Date().toISOString(), device: sh('getprop ro.product.model') + ' · Android ' + sh('getprop ro.build.version.release') };
result.weight = weight();
log('poids', result.weight);
const component = launcher();
result.coldStart = await coldStart(component);

// La conversation : du toucher sur l'agent à son affichage.
await sleep(2000);
await clearMarks();
const agent = await find('hub-close');
if (!agent) log("l'agent hub-close est introuvable à l'écran");
const t = Date.now();
if (agent) sh(`input tap ${agent.x} ${agent.y}`);
const conv = await waitMark('conv-ready');
result.conversationOpenMs = conv ? conv.t - t : null;
log(`conversation : ${result.conversationOpenMs} ms`);
await sleep(5000);
result.memory = memory();
log('mémoire', result.memory);
result.fluidity = await fluidity();
// Le pont, mesuré par l'app au démarrage (1 000 ouvertures) : le dernier relevé.
await clearMarks();
sh(`am force-stop ${pkg}`);
sh(`am start -n ${component}`);
const bridge = await waitMark('bridge-us', 40000);
result.bridgeUsPerCall = bridge ? Number(bridge.v) : null;
sh(`am start -n ${component}`);
await sleep(8000);
result.push = { notification: push('notification'), notificationFromStopped: push('notification', 'stopped') };
sh(`am start -n ${component}`);
await sleep(8000);
result.push.data = push('data');
log('push', JSON.stringify(result.push));

mkdirSync(join(here, '../results'), { recursive: true });
writeFileSync(join(here, `../results/android-${name}.json`), JSON.stringify(result, null, 2) + '\n');
log('écrit', `results/android-${name}.json`);
