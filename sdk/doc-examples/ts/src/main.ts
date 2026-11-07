// The examples of the documentation, in TypeScript. Each `example/*` method runs one of them, for `escouade-plugin dev`.
import { createPlugin } from '@escouade-dev/plugin';
import { keep, notes, words } from './files.ts';
import { follow, issue } from './net.ts';
import { input, status, stop } from './procs.ts';
import { channel, journal, token } from './settings.ts';
import { live } from './socket.ts';
import { later, patient } from './timers.ts';

const plugin = createPlugin();

plugin.onEvent('ticket.column', journal);
plugin.handle('example/fetch', (id, p) => issue(p, id as string));
plugin.handle('example/stream', (url, p) => follow(p, url as string));
plugin.handle('example/socket', (_, p) => live(p));
plugin.handle('example/later', (_, p) => later(p));
plugin.handle('example/patient', (_, p) => patient(p));
plugin.handle('example/keep', (text, p) => keep(p, text as string));
plugin.handle('example/notes', (_, p) => notes(p));
plugin.handle('example/words', (_, p) => words(p));
plugin.handle('example/status', async (project, p) => (await status(p, project as string)).exited);
plugin.handle('example/input', (_, p) => input(p));
plugin.handle('example/kill', (_, p) => stop(p));
plugin.handle('example/settings', (project, p) => channel(p, project as string));
plugin.handle('example/secret', (_, p) => token(p));

void plugin.start();
