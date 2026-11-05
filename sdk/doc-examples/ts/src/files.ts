// `fs/*`, and `files/read` for the plugin's own folder.
import type { Plugin } from '@escouade-dev/plugin';

// Writes in its own temporary folder (`permissions.files`, root `temp`).
export function keep(p: Plugin, text: string) {
  return p.fs.write({ root: 'temp', path: 'notes.txt', text, append: true });
}

// Reads it back, what is there and what the host calls it.
export async function notes(p: Plugin) {
  const file = { root: 'temp', path: 'notes.txt' } as const;
  return {
    text: await p.fs.text(file),
    stat: await p.fs.stat(file),
    canon: await p.fs.canon(file),
    list: await p.fs.list({ root: 'temp', path: '.' }),
  };
}

// A file of the plugin's own folder.
export function words(p: Plugin) {
  return p.file('data/words.txt');
}
