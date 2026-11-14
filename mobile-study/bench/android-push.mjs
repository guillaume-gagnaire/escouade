// Le push app fermée, sous Android, sans projet Firebase : on ferme l'app (Accueil puis `am kill`, comme le système
// le fait d'une app en arrière-plan), puis on lui remet un message FCM comme les services Google Play le feraient :
// la diffusion `com.google.android.c2dm.intent.RECEIVE` aux récepteurs de l'app. Ils n'acceptent que
// les services Google Play (permission `c2dm.permission.SEND`) ; adb en root (une image google_apis de l'émulateur)
// passe outre. Ce qui est vérifié : le processus renaît, le SDK de FCM affiche la notification, et la toucher ouvre
// l'app qui déchiffre le contenu scellé par le Rust (repère `push-open` au faux serveur).
//
// Ce qui ne l'est pas (il faut un vrai projet) : l'inscription auprès de FCM (le jeton), l'envoi par l'API HTTP v1, la
// remise par les serveurs de Google, et la priorité haute qui réveille un téléphone en veille (Doze).
//
//   PORT=<port du faux serveur> node mobile-study/bench/android-push.mjs <paquet> [notification|data]
import { execFileSync } from 'node:child_process';
import { setTimeout as sleep } from 'node:timers/promises';

const [pkg, kind = 'notification'] = process.argv.slice(2);
const port = process.env.PORT;
const adb = (...a) => execFileSync('adb', a, { encoding: 'utf8' }).trim();
const sh = (cmd) => adb('shell', cmd);
const pid = () => {
  try {
    return sh(`pidof ${pkg}`);
  } catch {
    return '';
  }
};

const { sealed } = await (await fetch(`http://127.0.0.1:${port}/api/push-sample`)).json();
adb('root');
await sleep(2000);
adb('wait-for-device');
sh(`pm grant ${pkg} android.permission.POST_NOTIFICATIONS`);
sh('cmd notification set_dnd off');
sh('input keyevent KEYCODE_HOME');
await sleep(1500);
sh(`am kill ${pkg}`);
await sleep(1500);
const before = pid();
console.log(`app ${pkg} : processus ${before ? 'encore là (' + before + ')' : 'fermé'}`);

const extras = [
  '--es', 'google.message_id', `0:${Date.now()}%esc445`,
  '--es', 'from', '000000000000',
  '--es', 'google.delivered_priority', 'high',
  '--es', 'google.original_priority', 'high',
  '--es', 'sealed', sealed,
  '--es', 'agent', 'a1',
];
if (kind === 'notification') {
  // Un message « notification » : le SDK l'affiche lui-même, app fermée ; le texte en clair est un réveil générique,
  // le contenu reste scellé dans les données.
  extras.push('--es', 'gcm.n.e', '1', '--es', 'gcm.notification.title', 'Escouade', '--es', 'gcm.notification.body', 'Un agent attend ta réponse');
}
const t0 = Date.now();
// adb shell recolle les arguments par des espaces : chaque valeur est citée pour le shell du téléphone.
const q = (v) => (v.startsWith('--') ? v : `'${v.replace(/'/g, `'\''`)}'`);
// Au paquet entier (-p), comme les services Google Play : firebase_messaging (Flutter) a son propre récepteur.
console.log(sh(['am broadcast -p', pkg, '-a com.google.android.c2dm.intent.RECEIVE', ...extras.map(q)].join(' ')));
let after = '';
for (let i = 0; i < 20 && !after; i++) {
  await sleep(250);
  after = pid();
}
console.log(`processus relancé : ${after ? 'oui (' + after + ') en ' + (Date.now() - t0) + ' ms' : 'non'}`);
await sleep(2000);
const notif = sh('dumpsys notification --noredact');
const mine = notif.split('NotificationRecord').filter((r) => r.includes(`pkg=${pkg}`));
console.log(`notifications de l'app : ${mine.length}`);
for (const r of mine) {
  const title = r.match(/android\.title=String \(([^)]*)\)/);
  const text = r.match(/android\.text=String \(([^)]*)\)/);
  console.log(`  « ${title?.[1] ?? '?'} » : ${text?.[1] ?? '?'}`);
}
// Toucher la notification : le volet ouvert, son titre trouvé par uiautomator.
let opened = null;
let openMs = null;
if (kind === 'notification' && mine.length) {
  await fetch(`http://127.0.0.1:${port}/api/marks`, { method: 'DELETE' });
  sh('cmd statusbar collapse');
  await sleep(500);
  sh('cmd statusbar expand-notifications');
  await sleep(2500);
  sh('uiautomator dump /sdcard/ui.xml');
  const xml = sh('cat /sdcard/ui.xml');
  const m = xml.match(/text="Un agent attend ta réponse"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/);
  if (m) {
    // Le titre, juste au-dessus du texte : toucher le texte lui-même déplie parfois la notification au lieu de l'ouvrir.
    const [x, y] = [(+m[1] + +m[3]) >> 1, +m[2] - 30];
    console.log(`toucher en ${x},${y}`);
    sh(`input tap ${x} ${y}`);
    var tapAt = Date.now();
    for (let i = 0; i < 80 && opened === null; i++) {
      await sleep(250);
      const marks = await (await fetch(`http://127.0.0.1:${port}/api/marks`)).json();
      const hit = marks.find((k) => k.ev === 'push-open');
      if (hit) {
        opened = hit.v;
        openMs = hit.t - tapAt;
      }
    }
  }
  console.log(`notification touchée : ${opened ? 'app ouverte, contenu ' + opened + ' en ' + openMs + ' ms après le toucher' : 'pas de repère push-open en 20 s'}`);
}
console.log(JSON.stringify({ pkg, kind, closedBefore: !before, relaunched: Boolean(after), notifications: mine.length, opened, openMs }));
