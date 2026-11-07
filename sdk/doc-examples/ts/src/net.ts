// `net/fetch` and `net/stream`.
import { openStream, type Plugin } from '@escouade-dev/plugin';

// The standard fetch, which goes through Escouade in WASM.
export async function issue(p: Plugin, id: string) {
  const token = await p.secret('token');
  const res = await fetch(`https://api.example.dev/issues/${id}`, {
    headers: { authorization: `Bearer ${token ?? ''}` },
  });
  return { status: res.status, body: await res.text() };
}

// Each piece taken gives its bytes back to Escouade, which reads no further than 256 KiB ahead of the plugin.
export async function follow(p: Plugin, url: string) {
  const stream = await openStream(p, url, { sse: true });
  for await (const piece of stream) p.log.debug(`${piece.data?.length ?? 0} characters`);
}
