// Écrit conversation.json : une conversation enregistrée de 300 messages (Markdown, blocs de code, diffs, sorties de
// commandes), toujours la même (tirage à graine fixe), que le faux serveur rejoue aux quatre maquettes. Et vectors.json :
// une clé, un texte et ce texte scellé en JavaScript, que le Rust ouvre dans ses tests.
//
//   node mobile-study/shared/fixture/make-conversation.mjs
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { seal } from '../fake-server/sealed.mjs';

const here = dirname(fileURLToPath(import.meta.url));
let seed = 445;
const rnd = () => (seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648;
const pick = (a) => a[Math.floor(rnd() * a.length)];
const tick = '`';
const fence = tick.repeat(3);

const files = [
  'src/lib/remote.ts',
  'src-tauri/src/remote/tunnel.rs',
  'src/components/Conversation.svelte',
  'crates/escouade-base/src/hub.rs',
  'src/lib/ipc.ts',
];
const asks = [
  'Peux-tu brancher la reconnexion du client distant quand le tunnel tombe ?',
  'Ajoute un test pour la poignée de main des versions.',
  'Le hub perd un abonné quand deux clients ferment en même temps, regarde.',
  'Relis le diff avant de committer, et lance clippy.',
  "Pourquoi l'e2e de la conversation est-il rouge sous macOS ?",
];
const rustBlock = (n) =>
  [
    fence + 'rust',
    '/// Reprend un client après une coupure : rejoue ce qu\'il n\'a pas vu depuis `since`.',
    'pub fn resume(&mut self, client: ClientId, since: u64) -> Result<Vec<Frame>, HubError> {',
    '    let sub = self.subscribers.get_mut(&client).ok_or(HubError::Unknown(client))?;',
    '    let frames: Vec<Frame> = self.log.iter().filter(|f| f.seq > since).cloned().collect();',
    '    sub.last_seen = frames.last().map(|f| f.seq).unwrap_or(since);',
    `    tracing::debug!(client = %client, count = frames.len(), "reprise n°${n}");`,
    '    Ok(frames)',
    '}',
    fence,
  ].join('\n');
const tsBlock = (n) =>
  [
    fence + 'ts',
    `export async function reconnect(url: string, attempt = ${n % 5}): Promise<WebSocket> {`,
    '  const delay = Math.min(30_000, 500 * 2 ** attempt);',
    '  await new Promise((r) => setTimeout(r, delay));',
    '  const ws = new WebSocket(url);',
    "  ws.binaryType = 'arraybuffer';",
    '  return ws;',
    '}',
    fence,
  ].join('\n');
const prose = (n) =>
  pick([
    `J'ai regardé **${tick}hub.rs${tick}** : l'abonné est retiré deux fois quand deux clients ferment dans le même tour de boucle.\n\n- le premier ${tick}close${tick} retire l'entrée ;\n- le second trouve une place vide et **panique** sur ${tick}unwrap()${tick} ;\n- la correction : ${tick}remove${tick} rend une ${tick}Option${tick}, qu'on ignore.\n\nJe corrige et j'ajoute le test ${tick}two_clients_close_together${tick} (n° ${n}).`,
    "## Ce que j'ai changé\n\n1. La reconnexion attend de plus en plus (500 ms, 1 s, 2 s… jusqu'à 30 s).\n2. Le client rejoue ce qu'il a manqué depuis son dernier numéro vu.\n3. Les terminaux gardent un tampon de 256 Kio.\n\n| Cas | Avant | Après |\n|---|---|---|\n| Coupure de 2 s | conversation vide | reprise sans trou |\n| Coupure de 5 min | erreur | reprise, tampon rejoué |\n\n> La poignée de main refuse un client d'une autre version majeure.",
    `Le test passe maintenant. Voici le cœur de la correction :\n\n${rustBlock(n)}\n\nEt côté interface :\n\n${tsBlock(n)}`,
    `C'est la frame lue dans le même lot que le plantage : sous macOS, ${tick}read()${tick} rend les deux d'un coup. Je découpe par longueur annoncée plutôt que par appel, voir ${tick}Frame::split${tick} dans ${tick}crates/escouade-base/src/frame.rs${tick}.`,
    `Clippy dit deux choses :\n\n${fence}text\nwarning: this ${tick}if let${tick} can be collapsed into the outer ${tick}match${tick}\n  --> src-tauri/src/remote/tunnel.rs:${40 + (n % 60)}:13\nwarning: redundant clone\n  --> crates/escouade-base/src/hub.rs:${100 + (n % 80)}:41\n${fence}\n\nJe les corrige toutes les deux.`,
  ]);
const diffOf = (file, n) =>
  [
    `diff --git a/${file} b/${file}`,
    `--- a/${file}`,
    `+++ b/${file}`,
    `@@ -${10 + n},9 +${10 + n},12 @@ impl Hub {`,
    '     pub fn close(&mut self, client: ClientId) {',
    '-        let sub = self.subscribers.remove(&client).unwrap();',
    '-        sub.sender.close();',
    '+        // Deux fermetures dans le même tour : la seconde ne trouve plus rien.',
    '+        if let Some(sub) = self.subscribers.remove(&client) {',
    '+            sub.sender.close();',
    '+        }',
    '         self.metrics.clients.set(self.subscribers.len() as i64);',
    '     }',
    ' ',
    '     pub fn publish(&mut self, frame: Frame) {',
    '-        for sub in self.subscribers.values() {',
    '+        self.log.push(frame.clone());',
    '+        for sub in self.subscribers.values_mut() {',
    '             let _ = sub.sender.try_send(frame.clone());',
    '         }',
    '     }',
  ].join('\n');
const bashOut = (n) =>
  pick([
    `running 48 tests\ntest hub::tests::two_clients_close_together ... ok\ntest hub::tests::resume_replays_since ... ok\ntest remote::handshake::refuses_other_major ... ok\n\ntest result: ok. 48 passed; 0 failed; 0 ignored; finished in ${(1 + n / 100).toFixed(2)}s`,
    `    Checking escouade-base v2.2.0\n    Checking escouade v2.2.0 (src-tauri)\n    Finished ${tick}dev${tick} profile [unoptimized + debuginfo] target(s) in ${(8 + n / 30).toFixed(1)}s`,
    ' src/lib/remote.ts | 34 ++++++++++++++++++++++++----------\n crates/escouade-base/src/hub.rs | 12 ++++++++----\n 2 files changed, 32 insertions(+), 14 deletions(-)',
  ]);

const items = [];
let ts = Date.UTC(2026, 10, 14, 9, 0, 0);
let id = 0;
const push = (it) => items.push({ id: `m${++id}`, ts: (ts += 7000 + Math.floor(rnd() * 20000)), ...it });
while (items.length < 299) {
  push({ kind: 'user', text: pick(asks) });
  const steps = 2 + Math.floor(rnd() * 4);
  for (let s = 0; s < steps && items.length < 299; s++) {
    const r = rnd();
    if (r < 0.4) push({ kind: 'text', text: prose(items.length) });
    else if (r < 0.7) {
      const command = pick(['cargo nextest run -p escouade-base hub', 'cargo clippy --all-targets -- -D warnings', 'git diff --stat', 'npm run check']);
      push({ kind: 'tool', name: 'Bash', status: 'ok', input: { command }, output: bashOut(items.length) });
    } else {
      const f = pick(files);
      push({ kind: 'tool', name: 'Edit', status: 'ok', input: { file_path: f }, diff: diffOf(f, items.length) });
    }
  }
}
// Le dernier : l'autorisation qui attend, la commande entière (longue, sur plusieurs lignes) à montrer telle quelle.
push({
  kind: 'permission',
  toolUseId: 'toolu_esc445',
  toolName: 'Bash',
  input: {
    description: 'Pousser la branche et ouvrir la PR',
    command: [
      'git push --force-with-lease origin ticket/esc-445 && gh pr create --base main --head ticket/esc-445 \\',
      '  --title "fix(hub): deux clients qui ferment ensemble" \\',
      '  --body "Le second close ne panique plus. Tests : cargo nextest run -p escouade-base hub (48 ok)" \\',
      '  && rm -rf target/debug/incremental',
    ].join('\n'),
  },
});

const agents = [
  { id: 'a1', name: 'hub-close', project: 'Escouade', waiting: 'permission', preview: 'Pousser la branche et ouvrir la PR' },
  { id: 'a2', name: 'tunnel-wg', project: 'Escouade', waiting: 'question', preview: 'SSH ou WireGuard par défaut ?' },
  { id: 'a3', name: 'site-docs', project: 'Site', waiting: 'permission', preview: 'npm run build' },
  { id: 'a4', name: 'store-checks', project: 'Store', waiting: null, preview: 'Lance les onze vérifications…' },
];

writeFileSync(join(here, 'conversation.json'), JSON.stringify({ agents, items }, null, 1) + '\n');
const key = Buffer.from([...Array(32).keys()]).toString('base64url');
const plain = 'Escouade · hub-close attend une autorisation : git push --force-with-lease';
const vectors = { key, plain, sealed: seal(key, plain, Buffer.alloc(24, 9)) };
writeFileSync(join(here, 'vectors.json'), JSON.stringify(vectors, null, 2) + '\n');
const kinds = {};
for (const i of items) {
  const k = i.kind + (i.name ? ':' + i.name : '');
  kinds[k] = (kinds[k] ?? 0) + 1;
}
console.log(items.length, 'messages', kinds);
