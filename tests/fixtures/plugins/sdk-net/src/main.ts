// A plugin written for @escouade-dev/plugin that uses what a plugin in WASM gets of the host's network (ESC-332): the
// timers of the prelude, a binary fetch, a stream and a socket. src-tauri/src/plugins_wasm_tests.rs plays its committed
// bundle; sdk/plugin/src/both.test.ts checks dist/plugin.mjs is the build of this file.
//
//   npx esbuild tests/fixtures/plugins/sdk-net/src/main.ts --bundle --platform=node --format=esm --target=node20 \
//     --outfile=tests/fixtures/plugins/sdk-net/dist/plugin.mjs
import { createPlugin, openSocket, openStream } from '../../../../../sdk/plugin/src/index';

const plugin = createPlugin();

plugin.handle(
  'later',
  (params) =>
    new Promise((resolve) => {
      const { ms } = params as { ms: number };
      clearTimeout(setTimeout(() => resolve('taken back'), 10));
      setTimeout(resolve, ms, 'fired');
    }),
);

plugin.handle('bytes', async (params) => {
  const { url, bytes } = params as { url: string; bytes: number[] };
  const r = await fetch(url, { method: 'POST', body: new Uint8Array(bytes) });
  return [...new Uint8Array(await r.arrayBuffer())];
});

plugin.handle('stream', async (params) => {
  const stream = await openStream(plugin, (params as { url: string }).url, { sse: true });
  let text = '';
  for await (const piece of stream) text += piece.data ?? '';
  return { status: stream.status, text };
});

plugin.handle('socket', async (params) => {
  const { url, text } = params as { url: string; text: string };
  const socket = await openSocket(plugin, url);
  await socket.send({ text });
  for await (const m of socket) {
    await socket.close();
    return { got: m.text, closed: await socket.closed };
  }
  return null;
});

void plugin.start();
