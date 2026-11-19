// A small client of the remote API of escouade-server, run in its image by `node` (ESC-515, scripts/server-image-test.sh):
// what the desktop client does, at the protocol's level (src-tauri/src/remoteapi/proto.rs), on 127.0.0.1 of the
// container. Node 22's WebSocket; nothing else.
//
//   node server-image-client.mjs pair <code>                       → the device's token, on its own line
//   node server-image-client.mjs hello <version> <token>           → the first answer of the server (JSON)
//   node server-image-client.mjs agent <version> <token> <repo> <text>
//        → a project on <repo>, an agent of it, <text> sent to it; its id
//   node server-image-client.mjs call <version> <token> <cmd> <args as JSON>  → the reply (JSON)
//
// ESCOUADE_SERVER_PORT: the port (46480 by default).

const port = process.env.ESCOUADE_SERVER_PORT || '46480';
const url = `ws://127.0.0.1:${port}/remote`;
const [command, ...rest] = process.argv.slice(2);

function open() {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const queue = [];
    const waiting = [];
    ws.onmessage = (e) => {
      const m = JSON.parse(String(e.data));
      const w = waiting.shift();
      if (w) w(m);
      else queue.push(m);
    };
    ws.onopen = () =>
      resolve({
        send: (m) => ws.send(JSON.stringify(m)),
        next: () =>
          new Promise((r, fail) => {
            if (queue.length) return r(queue.shift());
            const timer = setTimeout(() => fail(new Error('no answer in 60 s')), 60_000);
            waiting.push((m) => {
              clearTimeout(timer);
              r(m);
            });
          }),
        close: () => ws.close(),
      });
    ws.onerror = () => reject(new Error(`the WebSocket to ${url} failed`));
  });
}

async function connected(version, token) {
  const c = await open();
  c.send({ t: 'hello', version, token });
  const first = await c.next();
  if (first.t !== 'welcome') throw new Error(`the handshake was refused: ${JSON.stringify(first)}`);
  let id = 0;
  c.call = async (cmd, args) => {
    const me = ++id;
    c.send({ t: 'call', id: me, cmd, args });
    for (;;) {
      const m = await c.next();
      if (m.t === 'reply' && m.id === me) {
        if (m.error !== undefined) throw new Error(`${cmd}: ${JSON.stringify(m.error)}`);
        return m.ok;
      }
    }
  };
  return c;
}

try {
  if (command === 'pair') {
    const c = await open();
    c.send({ t: 'pair', code: rest[0], name: 'Image test' });
    const m = await c.next();
    c.close();
    if (m.t !== 'paired') throw new Error(`the pairing was refused: ${JSON.stringify(m)}`);
    console.log(m.token);
  } else if (command === 'hello') {
    const c = await open();
    c.send({ t: 'hello', version: rest[0], token: rest[1] });
    console.log(JSON.stringify(await c.next()));
    c.close();
  } else if (command === 'agent') {
    const [version, token, repo, text] = rest;
    const c = await connected(version, token);
    const project = await c.call('create_project', {
      path: repo,
      name: 'image-test',
      color: 'oklch(0.72 0.12 48)',
      worktreePerAgent: false,
    });
    const agent = await c.call('create_agent', { projectId: project.id });
    await c.call('send_message', { id: agent.id, text, attachments: [] });
    console.log(agent.id);
    c.close();
  } else if (command === 'call') {
    const [version, token, cmd, args] = rest;
    const c = await connected(version, token);
    console.log(JSON.stringify(await c.call(cmd, JSON.parse(args || '{}'))));
    c.close();
  } else {
    throw new Error('usage: pair | hello | agent | call');
  }
  process.exit(0);
} catch (e) {
  console.error(String(e?.message ?? e));
  process.exit(1);
}
