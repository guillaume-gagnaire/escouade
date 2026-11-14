// Le faux serveur d'Escouade de l'étude ESC-445 : il rejoue aux quatre maquettes la conversation enregistrée
// (`../fixture/conversation.json`), reçoit les réponses aux autorisations, sert le QR code de l'appairage et note les
// repères que les maquettes lui envoient (le démarrage à froid se mesure ainsi sur l'horloge de la machine hôte).
//
//   PORT=<port> node mobile-study/shared/fake-server/server.mjs
//
// Le port vient de l'environnement (aucun n'est écrit ici). L'émulateur Android joint la machine par 10.0.2.2, le
// simulateur iOS par 127.0.0.1.
import { createServer } from 'node:http';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import QRCode from 'qrcode';
import { seal } from './sealed.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const { agents, items } = JSON.parse(readFileSync(join(here, '../fixture/conversation.json'), 'utf8'));
const vectors = JSON.parse(readFileSync(join(here, '../fixture/vectors.json'), 'utf8'));
const port = Number(process.env.PORT ?? 0);
const key = process.env.MS_KEY ?? vectors.key;
// L'adresse que le QR code donne au téléphone : celle de la machine vue de l'émulateur, par défaut.
const publicUrl = process.env.MS_PUBLIC_URL ?? `http://10.0.2.2:${port}`;

const marks = [];
const answers = [];

const pairUri = () => `escouade://pair?u=${encodeURIComponent(publicUrl)}&k=${key}&n=${encodeURIComponent('Poste de test')}`;

function send(res, status, body, type = 'application/json') {
  const data = type === 'application/json' ? JSON.stringify(body) : body;
  res.writeHead(status, {
    'content-type': type,
    'access-control-allow-origin': '*',
    'access-control-allow-headers': 'content-type',
    'access-control-allow-methods': 'GET, POST, DELETE, OPTIONS',
    'cache-control': 'no-store',
  });
  res.end(data);
}

const readBody = (req) =>
  new Promise((resolve) => {
    let s = '';
    req.on('data', (d) => (s += d));
    req.on('end', () => {
      try {
        resolve(s ? JSON.parse(s) : {});
      } catch {
        resolve({});
      }
    });
  });

const server = createServer(async (req, res) => {
  const url = new URL(req.url, 'http://x');
  const p = url.pathname;
  if (req.method === 'OPTIONS') return send(res, 204, '');
  if (p === '/api/agents') return send(res, 200, agents);
  let m = p.match(/^\/api\/agents\/([^/]+)\/conversation$/);
  if (m) return send(res, 200, m[1] === 'a1' ? items : items.slice(0, 40));
  m = p.match(/^\/api\/agents\/([^/]+)\/permission$/);
  if (m && req.method === 'POST') {
    const body = await readBody(req);
    answers.push({ agent: m[1], ...body, t: Date.now() });
    console.log(`réponse ${m[1]} : ${body.decision} (${body.app ?? '?'}, biométrie ${body.biometric ?? '?'})`);
    return send(res, 200, { ok: true });
  }
  if (m) return send(res, 200, items[items.length - 1]);
  // Après l'appairage : un texte scellé avec la clé du QR code, que la maquette ouvre par le Rust.
  if (p === '/api/challenge') return send(res, 200, { sealed: seal(key, `Appairé avec « Poste de test » à ${new Date().toISOString()}`) });
  if (p === '/api/push-sample') {
    const text = JSON.stringify({ title: 'hub-close attend', body: 'git push --force-with-lease origin ticket/esc-445', agent: 'a1' });
    return send(res, 200, { sealed: seal(key, text) });
  }
  if (p === '/api/mark') {
    const mark = { app: url.searchParams.get('app'), ev: url.searchParams.get('ev'), v: url.searchParams.get('v'), t: Date.now() };
    marks.push(mark);
    console.log(`repère ${mark.app} ${mark.ev}${mark.v ? ' ' + mark.v : ''}`);
    return send(res, 200, { ok: true });
  }
  if (p === '/api/marks' && req.method === 'DELETE') {
    marks.length = 0;
    return send(res, 200, { ok: true });
  }
  if (p === '/api/marks') return send(res, 200, marks);
  if (p === '/api/answers') return send(res, 200, answers);
  // ?margin=<modules> : l affiche de l émulateur (2 m de côté dans sa scène) veut une large marge blanche autour du code.
  if (p === '/pair.png') {
    const margin = Number(url.searchParams.get('margin') ?? 4);
    return send(res, 200, await QRCode.toBuffer(pairUri(), { width: 1200, margin }), 'image/png');
  }
  if (p === '/pair') return send(res, 200, { uri: pairUri() });
  send(res, 404, { error: 'inconnu' });
});

server.listen(port, '0.0.0.0', () => {
  const actual = server.address().port;
  console.log(`faux serveur sur http://127.0.0.1:${actual} (${items.length} messages, ${agents.length} agents)`);
  if (process.env.MS_QR_OUT) {
    QRCode.toBuffer(pairUri(), { width: 900, margin: 4 }).then((b) => writeFileSync(process.env.MS_QR_OUT, b));
  }
});
