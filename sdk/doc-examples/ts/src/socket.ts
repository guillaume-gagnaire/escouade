// `ws/*`.
import { openSocket, type Plugin } from '@escouade-dev/plugin';

// Its messages as they come, until it closes.
export async function live(p: Plugin) {
  const token = await p.secret('token');
  const socket = await openSocket(p, 'wss://api.example.dev/live', { authorization: `Bearer ${token ?? ''}` });
  await socket.send({ text: 'hello' });
  for await (const message of socket) {
    p.log.info(`the socket says ${message.text ?? ''}`);
    await socket.close();
  }
  return (await socket.closed).code;
}
