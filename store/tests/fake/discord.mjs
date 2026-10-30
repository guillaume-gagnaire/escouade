import { createServer } from 'node:http';

// A fake Discord for the tests of the announcements (server/utils/discord.ts): it takes the webhooks' POSTs as
// Discord does (`/api/webhooks/<id>/<token>`, 204), keeps each message with its webhook and when it came, and can
// answer the next ones with a 429 and its `Retry-After` (in the header and in the body, as Discord), a 5xx, a 404,
// or wait before it answers. Nothing leaves this machine; no address of a real webhook is ever needed.

/**
 * Starts the fake on a free port of 127.0.0.1 (or `port`). Gives its address, the messages received, the webhook
 * address of a channel (`hook('admin')`), the next answers to give (`plan([{ status: 429, retryAfter: 1 }])`), and
 * `close()`.
 */
export async function fakeDiscord(port = 0) {
  /** @type {{ hook: string, at: number, body: any }[]} */
  const received = [];
  /** @type {{ status: number, retryAfter?: number, delay?: number }[]} */
  let planned = [];
  const server = createServer((req, res) => {
    const m = /^\/api\/webhooks\/(\d+)\/([\w-]+)\/?$/.exec(req.url?.split('?')[0] ?? '');
    if (req.method !== 'POST' || !m) {
      res.writeHead(404).end();
      return;
    }
    let raw = '';
    req.on('data', (c) => (raw += c));
    req.on('end', () => {
      const next = planned.shift() ?? { status: 204 };
      const answer = () => {
        if (next.status === 204 || next.status === 200) received.push({ hook: m[2], at: Date.now(), body: JSON.parse(raw) });
        if (next.status === 429) {
          res.writeHead(429, { 'content-type': 'application/json', 'retry-after': String(next.retryAfter ?? 1) });
          res.end(JSON.stringify({ message: 'You are being rate limited.', retry_after: next.retryAfter ?? 1, global: false }));
          return;
        }
        res.writeHead(next.status).end(next.status >= 400 ? JSON.stringify({ message: 'nope', code: 0 }) : undefined);
      };
      if (next.delay) setTimeout(answer, next.delay);
      else answer();
    });
  });
  await new Promise((resolve) => server.listen(port, '127.0.0.1', () => resolve(undefined)));
  const address = server.address();
  const base = `http://127.0.0.1:${typeof address === 'object' && address ? address.port : port}`;
  return {
    base,
    received,
    /** The webhook of a channel: its token is the channel's name, followed by a secret-looking tail. */
    hook: (channel) => `${base}/api/webhooks/1234567890/${channel}-tok3n-SECRET`,
    /** The messages of a channel. */
    of: (channel) => received.filter((r) => r.hook === `${channel}-tok3n-SECRET`).map((r) => r.body),
    plan: (answers) => {
      planned = [...answers];
    },
    reset: () => {
      received.length = 0;
      planned = [];
    },
    close: () => new Promise((resolve) => server.close(() => resolve(undefined))),
  };
}
