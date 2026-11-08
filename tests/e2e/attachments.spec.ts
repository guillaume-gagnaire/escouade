// ESC-95, « Joindre n'importe quel fichier »: a file the API cannot read (a .zip, a .xlsx), or one too big for the message, is
// joined by its path. The test files are written in the test's own folder (`app.root`), never in the repository.

import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, send, test, type App, openSettingsPage, closeSettings } from './fixture';

const KB = 1024;
const MB = 1024 * KB;

/** `size` bytes that start as a zip does and are all `fill` after (two files of one test are told apart by it). */
function zip(size: number, fill = 0x61): Buffer {
  const bytes = Buffer.alloc(size, fill);
  Buffer.from([0x50, 0x4b, 0x03, 0x04]).copy(bytes);
  return bytes;
}

/** A file of the disk, as the user's own documents are: out of the app's data folder and of the repository. */
function diskFile(app: App, name: string, bytes: Buffer): string {
  const folder = path.join(app.root, 'depot');
  fs.mkdirSync(folder, { recursive: true });
  const file = path.join(folder, name);
  fs.writeFileSync(file, bytes);
  return file;
}

/**
 * Hands real files of the disk to the file input of the page, as the system's file dialog does. `setInputFiles` of Playwright,
 * through a remote connection, sends the bytes: the page then holds a file in memory, whose place nobody knows (the app copies
 * it). The DevTools' own command gives it files the WebView knows by their path (`ICoreWebView2File`), as a dialog or a drop does.
 */
async function diskFiles(page: Page, paths: string[], selector = 'input[type=file]') {
  const cdp = await page.context().newCDPSession(page);
  try {
    const { root } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector });
    expect(nodeId, `no ${selector} in the page`).toBeGreaterThan(0);
    await cdp.send('DOM.setFileInputFiles', { files: paths, nodeId });
  } finally {
    await cdp.detach();
  }
}

/** Both paths are one place of the disk (CI's temporary folder is a short 8.3 name, and Windows does not tell cases apart). */
function samePath(a: string, b: string): boolean {
  const real = (p: string) => {
    try {
      return fs.realpathSync.native(p);
    } catch {
      return p;
    }
  };
  return real(a).toLowerCase() === real(b).toLowerCase();
}

/** What the fake CLI read on its stdin (`<log>.stdin.jsonl`, beside the data folder): one message of the user per line. */
function stdinMessages(app: App): { message: { content: string | { type: string; text?: string }[] } }[] {
  const file = path.join(app.root, 'fake-claude.stdin.jsonl');
  if (!fs.existsSync(file)) return [];
  return fs
    .readFileSync(file, 'utf8')
    .trim()
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l));
}

/** The text of a message of the user, whether it went as a string or as blocks. */
function textOf(m: ReturnType<typeof stdinMessages>[number]): string {
  const { content } = m.message;
  return typeof content === 'string' ? content : content.map((b) => b.text ?? '').join('\n');
}

/** The files a message tells Claude of, with the lines of Escouade in either language: « Fichier joint : <nom>, <chemin> ». */
function told(text: string): { name: string; file: string }[] {
  return [...text.matchAll(/^(?:Fichier joint : |Attached file: )(.+?), ([A-Za-z]:\\.+|\/.+)$/gm)].map((m) => ({
    name: m[1],
    file: m[2].trim(),
  }));
}

/** The message of the user's the fake CLI got that holds a line of a file joined (its text), once it came. */
function joinedMessage(app: App, nth = 0): string | undefined {
  return stdinMessages(app)
    .filter((m) => /(?:Fichier joint : |Attached file: )/.test(textOf(m)))
    .map(textOf)[nth];
}

/** The folders of the files joined to the agents (`attachments/<agent id>`), the drafts' and the tickets' left out. */
function agentFolders(app: App): string[] {
  const root = path.join(app.data, 'attachments');
  return fs.existsSync(root) ? fs.readdirSync(root).filter((n) => !n.startsWith('_')) : [];
}

/** The copies kept for an agent: `attachments/<agent id>/<uniq>/<name>`. */
function copiesOf(app: App, id: string): { uniq: string; name: string; file: string }[] {
  const folder = path.join(app.data, 'attachments', id);
  if (!fs.existsSync(folder)) return [];
  return fs
    .readdirSync(folder, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .flatMap((d) =>
      fs.readdirSync(path.join(folder, d.name)).map((name) => ({ uniq: d.name, name, file: path.join(folder, d.name, name) })),
    );
}

/** Everything in the drafts' folder (the copies of the files of a ticket's form, before the ticket is saved), as paths under it. */
function drafts(app: App): string[] {
  const folder = path.join(app.data, 'attachments', '_drafts');
  return fs.existsSync(folder) ? fs.readdirSync(folder, { recursive: true, encoding: 'utf8' }) : [];
}

/** The `--add-dir=` of every launch of the fake CLI: the folders Claude Code reads without asking. */
function addDirs(app: App): string[] {
  return app.launches().flatMap((l) => l.argv.filter((a) => a.startsWith('--add-dir=')).map((a) => a.slice('--add-dir='.length)));
}

/** The id of the agent the project started with: its folder is made when its Claude Code starts (`--add-dir`). */
async function agentId(app: App): Promise<string> {
  await expect.poll(() => agentFolders(app).length, { timeout: 20_000 }).toBe(1);
  return agentFolders(app)[0];
}

/** The chips of the files joined, over the field of the composer. */
const chipsOf = (page: Page) => page.locator('.composer .atts .file');

/** The mark of a chip whose file is the original one, attached where it lies (a copy has none). It is named by its tooltip. */
const originMark = (chip: Locator) => chip.getByRole('img', { name: /Fichier d.origine, lu là où il est/ });

/** Every copy, or check of a path, is over: the message can be sent (the key of the field does nothing before). */
const settled = (page: Page) => expect(page.locator('.composer .atts .file.copying')).toHaveCount(0);

/** The message of the user shown in the conversation, whole (a file joined by its path is told in its last lines). */
const shownMessage = (page: Page) => page.locator('.bubble .text').filter({ hasText: /(Fichier joint|Attached file)/ });

const toast = (page: Page) => page.locator('.toasts .toast');

test('files joined from the disk are read where they lie: no copy, their own paths in the message, no permission asked', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const rapport = diskFile(app, 'rapport.zip', zip(3 * KB, 0x61));
  const tableau = diskFile(app, 'tableau.xlsx', zip(5 * KB, 0x62));
  const id = await agentId(app);

  // The file dialog offers every file: its input carries no filter.
  await expect(page.locator('input[type=file]').first()).not.toHaveAttribute('accept', /.+/);
  // The file dialog's answer: two files, each a chip with its name and its size.
  await diskFiles(page, [rapport, tableau]);
  const chips = chipsOf(page);
  await expect(chips).toHaveCount(2);
  await expect(chips.nth(0)).toContainText('rapport.zip');
  await expect(chips.nth(0)).toContainText(/3\s+Ko/);
  await expect(chips.nth(1)).toContainText('tableau.xlsx');
  await expect(chips.nth(1)).toContainText(/5\s+Ko/);
  await settled(page);
  // Nothing was copied: the files stay where they are, and each chip says it is the original file, read where it is.
  expect(copiesOf(app, id)).toEqual([]);
  for (const chip of [chips.nth(0), chips.nth(1)]) {
    await expect(originMark(chip)).toHaveCount(1);
    await expect(originMark(chip)).toHaveAttribute('data-tip', /Fichier d.origine, lu là où il est/);
  }

  await send(page, 'Lis ces fichiers');
  await expect(shownMessage(page)).toBeVisible();
  // The conversation shows the message whole, each file with its own path…
  const shown = told(await shownMessage(page).innerText());
  expect(shown.map((f) => f.name)).toEqual(['rapport.zip', 'tableau.xlsx']);
  expect(samePath(shown[0].file, rapport), `${shown[0].file} is not ${rapport}`).toBe(true);
  expect(samePath(shown[1].file, tableau), `${shown[1].file} is not ${tableau}`).toBe(true);
  // …and so does what Claude Code received: a text, not blocks, with the same lines.
  await expect.poll(() => joinedMessage(app)).toBeTruthy();
  const sent = stdinMessages(app).find((m) => textOf(m).includes('Fichier joint'))!;
  expect(typeof sent.message.content).toBe('string');
  const received = told(sent.message.content as string);
  expect(received).toEqual(shown);
  expect(sent.message.content as string).toMatch(/^Lis ces fichiers\n\nFichier joint : rapport\.zip, /);

  // Read without a permission card: the agent reads them where they lie, Escouade having allowed that reading (and no other).
  await expect(page.getByText(`Lu : rapport.zip=${3 * KB}, tableau.xlsx=${5 * KB}`)).toBeVisible();
  await expect(page.getByTestId('permission-pending')).toHaveCount(0);
  await expect(page.getByText('Claude demande une autorisation')).toHaveCount(0);

  // The originals are as they were, and no copy was made at any moment.
  expect(fs.readFileSync(rapport).equals(zip(3 * KB, 0x61))).toBe(true);
  expect(fs.readFileSync(tableau).equals(zip(5 * KB, 0x62))).toBe(true);
  expect(copiesOf(app, id)).toEqual([]);
  // The agent's folder is there (made for `--add-dir`) and empty: not a copy, not a mark of one.
  expect(fs.readdirSync(path.join(app.data, 'attachments', id))).toEqual([]);
});

test.describe('with « Joindre le fichier d’origine » off', () => {
  test.use({ preStart: { run: () => ({ attachOrigin: false }) } });

  test('every file is copied under the agent’s folder, which Claude Code reads without asking, and a chip removed takes its copy away', async ({
    app,
  }) => {
    const { page } = app;
    await addProject(page, app.repo);
    const rapport = diskFile(app, 'rapport.zip', zip(3 * KB, 0x61));
    const tableau = diskFile(app, 'tableau.xlsx', zip(5 * KB, 0x62));
    const id = await agentId(app);

    await diskFiles(page, [rapport, tableau]);
    // A file of the page's memory has no place to be joined at: copied too.
    await page
      .locator('input[type=file]')
      .setInputFiles({ name: 'journal.log.zip', mimeType: 'application/zip', buffer: zip(2 * KB, 0x63) });
    await expect(chipsOf(page)).toHaveCount(3);
    await settled(page);
    // Copies, all of them: no chip is marked as an original file.
    await expect(originMark(chipsOf(page))).toHaveCount(0);
    const names = () =>
      copiesOf(app, id)
        .map((c) => c.name)
        .sort();
    await expect.poll(names).toEqual(['journal.log.zip', 'rapport.zip', 'tableau.xlsx']);
    for (const copy of copiesOf(app, id)) expect(copy.uniq).toMatch(/^[0-9a-f]{8}$/);

    // A chip taken away takes its copy away, with the folder that held it; the others stay.
    await page.getByRole('button', { name: 'Retirer journal.log.zip' }).click();
    await expect(chipsOf(page)).toHaveCount(2);
    await expect.poll(names).toEqual(['rapport.zip', 'tableau.xlsx']);
    expect(fs.readdirSync(path.join(app.data, 'attachments', id))).toHaveLength(2);

    await send(page, 'Lis ces fichiers');
    await expect(shownMessage(page)).toBeVisible();
    const shown = told(await shownMessage(page).innerText());
    expect(shown.map((f) => f.name)).toEqual(['rapport.zip', 'tableau.xlsx']);
    // Each path is the copy under the agent's folder: `attachments/<agent id>/<8 hex>/<name>`, not the file of the disk.
    for (const [i, name] of ['rapport.zip', 'tableau.xlsx'].entries()) {
      const copy = copiesOf(app, id).find((c) => c.name === name)!;
      expect(samePath(shown[i].file, copy.file), `${shown[i].file} is not ${copy.file}`).toBe(true);
      expect(shown[i].file.replaceAll('\\', '/')).toMatch(new RegExp(`/attachments/${id}/[0-9a-f]{8}/${name.replace('.', '\\.')}$`));
      expect(samePath(shown[i].file, name === 'rapport.zip' ? rapport : tableau)).toBe(false);
    }
    // What Claude Code got is the same text.
    await expect.poll(() => joinedMessage(app)).toBeTruthy();
    expect(told(joinedMessage(app)!)).toEqual(shown);

    // Read without asking: the agent's folder is one of its `--add-dir`.
    await expect(page.getByText(`Lu : rapport.zip=${3 * KB}, tableau.xlsx=${5 * KB}`)).toBeVisible();
    await expect(page.getByTestId('permission-pending')).toHaveCount(0);
    expect(addDirs(app).some((d) => samePath(d, path.join(app.data, 'attachments', id)))).toBe(true);

    // The copies are the files' bytes, and the originals are as they were, apart.
    expect(fs.readFileSync(copiesOf(app, id).find((c) => c.name === 'rapport.zip')!.file).equals(zip(3 * KB, 0x61))).toBe(true);
    expect(fs.readFileSync(copiesOf(app, id).find((c) => c.name === 'tableau.xlsx')!.file).equals(zip(5 * KB, 0x62))).toBe(true);
    expect(fs.readFileSync(rapport).equals(zip(3 * KB, 0x61))).toBe(true);
    expect(fs.readFileSync(tableau).equals(zip(5 * KB, 0x62))).toBe(true);
  });

  test('a name that no folder could hold is joined under a cleaned name, which Claude reads', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    const id = await agentId(app);
    // « ../x.zip » cannot be the name of a file of a page; a colon and a question mark can, and Windows cannot keep them.
    await page.locator('input[type=file]').setInputFiles({ name: 'a:b?.zip', mimeType: 'application/zip', buffer: zip(1 * KB, 0x64) });
    await expect(chipsOf(page)).toHaveCount(1);
    await settled(page);
    await expect.poll(() => copiesOf(app, id).map((c) => c.name)).toEqual(['a_b_.zip']);
    await send(page, 'Lis ce fichier');
    await expect(shownMessage(page)).toBeVisible();
    // The message may show before the line of its file: read once it is there.
    await expect.poll(async () => told(await shownMessage(page).innerText()).length).toBe(1);
    const [file] = told(await shownMessage(page).innerText());
    expect(file.name).toBe('a_b_.zip');
    const copy = copiesOf(app, id)[0];
    expect(samePath(file.file, copy.file)).toBe(true);
    expect(path.basename(file.file)).toBe('a_b_.zip');
    expect(path.dirname(path.dirname(file.file)).toLowerCase()).toBe(path.join(app.data, 'attachments', id).toLowerCase());
    await expect(page.getByText(`Lu : a_b_.zip=${KB}`)).toBeVisible();
  });
});

test('what the API reads stays in the message: an image and a text file go as blocks, nothing is copied', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const id = await agentId(app);
  const png = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13]);
  const notes = 'bonjour le monde\n';
  await page.locator('input[type=file]').setInputFiles([
    { name: 'capture.png', mimeType: 'image/png', buffer: png },
    { name: 'notes.txt', mimeType: 'text/plain', buffer: Buffer.from(notes) },
  ]);
  // The image keeps its preview, the text is a chip: neither is a file by its path.
  await expect(page.locator('.composer .atts .img img[alt="capture.png"]')).toBeVisible();
  await expect(chipsOf(page)).toHaveCount(1);
  await expect(chipsOf(page).first()).toContainText('notes.txt');
  await send(page, 'Regarde ça');
  await expect(page.getByText('Bonjour, tu as dit : Regarde ça')).toBeVisible();
  const sent = stdinMessages(app).find((m) => Array.isArray(m.message.content));
  expect(sent!.message.content).toEqual([
    { type: 'image', source: { type: 'base64', media_type: 'image/png', data: png.toString('base64') } },
    { type: 'document', title: 'notes.txt', source: { type: 'text', media_type: 'text/plain', data: notes } },
    { type: 'text', text: 'Regarde ça' },
  ]);
  expect(copiesOf(app, id)).toEqual([]);
});

test('a text over 256 Ko and an image over 5 Mo are joined by their path, the first where it lies, the second copied in chunks', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const id = await agentId(app);
  // A text of 300 Ko is no part of the message (the API reads 256 Ko of it), and `attachBigText` is « path » by default.
  const text = Buffer.from('ligne de texte\n'.repeat(Math.ceil((300 * KB) / 15))).subarray(0, 300 * KB);
  const gros = diskFile(app, 'gros.txt', text);
  await diskFiles(page, [gros]);
  // A PNG of 6 Mo, over the 5 Mo of an image of a message; more than a chunk of 4 Mo, so it crosses in two.
  const png = Buffer.alloc(6 * MB, 0x7a);
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]).copy(png);
  await page.locator('input[type=file]').setInputFiles({ name: 'grande.png', mimeType: 'image/png', buffer: png });
  // Chips, no preview: neither goes in the message.
  await expect(chipsOf(page)).toHaveCount(2);
  await expect(page.locator('.composer .atts .img')).toHaveCount(0);
  await expect(chipsOf(page).nth(0)).toContainText('gros.txt');
  await expect(chipsOf(page).nth(0)).toContainText(/300\s+Ko/);
  await expect(chipsOf(page).nth(1)).toContainText('grande.png');
  await expect(chipsOf(page).nth(1)).toContainText(/6\s+Mo/);
  await settled(page);

  await send(page, 'Lis les gros fichiers');
  await expect(shownMessage(page)).toBeVisible();
  const shown = told(await shownMessage(page).innerText());
  expect(shown.map((f) => f.name)).toEqual(['gros.txt', 'grande.png']);
  expect(samePath(shown[0].file, gros)).toBe(true);
  const [copy] = copiesOf(app, id);
  expect(copy.name).toBe('grande.png');
  expect(samePath(shown[1].file, copy.file)).toBe(true);
  // A text, with the two lines: no block of the message carries them.
  await expect.poll(() => joinedMessage(app)).toBeTruthy();
  const sent = stdinMessages(app).find((m) => textOf(m).includes('Fichier joint'))!;
  expect(typeof sent.message.content).toBe('string');
  expect(told(sent.message.content as string)).toEqual(shown);
  await expect(page.getByText(`Lu : gros.txt=${300 * KB}, grande.png=${6 * MB}`)).toBeVisible();
  expect(fs.readFileSync(copy.file).equals(png)).toBe(true);
  expect(fs.readFileSync(gros).equals(text)).toBe(true);
});

test.describe('with « Texte de plus de 256 Ko » refused', () => {
  test.use({ preStart: { run: () => ({ attachBigText: 'refuse' }) } });

  test('a text over 256 Ko is refused with a toast, and no chip; any other file is still joined', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await page
      .locator('input[type=file]')
      .setInputFiles({ name: 'gros.txt', mimeType: 'text/plain', buffer: Buffer.alloc(300 * KB, 0x61) });
    await expect(toast(page).filter({ hasText: /gros\.txt dépasse 256\s+Ko/ })).toBeVisible();
    await expect(chipsOf(page)).toHaveCount(0);
    // The refusal is for the texts: a zip of the same size goes by its path.
    await page.locator('input[type=file]').setInputFiles({ name: 'gros.zip', mimeType: 'application/zip', buffer: zip(300 * KB) });
    await expect(chipsOf(page)).toHaveCount(1);
    await expect(chipsOf(page).first()).toContainText('gros.zip');
    await settled(page);
  });
});

test('the settings of the attachments are saved, and apply to the next file', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const id = await agentId(app);
  const saved = () => JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
  const open = async () => {
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(dialog, 'Avancé');
    await expect(dialog.getByRole('heading', { name: 'Pièces jointes' })).toBeVisible();
    return dialog;
  };
  const save = async (dialog: Locator) => {
    await closeSettings(dialog);
  };
  const max = (dialog: Locator) => dialog.getByRole('spinbutton', { name: /Taille maximale d.un fichier joint/ });
  const warn = (dialog: Locator) => dialog.getByRole('spinbutton', { name: /Avertir pour un fichier joint/ });
  const memory = (name: string, bytes: Buffer) =>
    page.locator('input[type=file]').setInputFiles({ name, mimeType: 'application/zip', buffer: bytes });

  // The defaults: no maximum, a warning over 200 Mo, the original file, a big text by its path, no purge.
  let dialog = await open();
  await expect(max(dialog)).toHaveValue('0');
  await expect(warn(dialog)).toHaveValue('200');
  await expect(dialog.getByRole('switch', { name: /Joindre le fichier d.origine/ })).toHaveAttribute('aria-checked', 'true');
  await expect(
    dialog.getByRole('group', { name: 'Texte de plus de 256 Ko' }).getByRole('button', { name: 'Joint par son chemin' }),
  ).toHaveAttribute('aria-pressed', 'true');
  await expect(dialog.getByRole('spinbutton', { name: /Purger les copies après/ })).toHaveValue('0');

  // A maximum of 1 Mo: kept in the settings, and the next file over it is refused, one under it joined.
  await max(dialog).fill('1');
  await save(dialog);
  await expect.poll(() => saved().attachMaxMb).toBe(1);
  await memory('grosse.zip', zip(2 * MB));
  await expect(toast(page).filter({ hasText: /grosse\.zip dépasse 1\s+Mo/ })).toBeVisible();
  await expect(chipsOf(page)).toHaveCount(0);
  await memory('moyenne.zip', zip(500 * KB));
  await expect(chipsOf(page)).toHaveCount(1);
  await expect(chipsOf(page).first()).toContainText('moyenne.zip');
  await settled(page);
  // Under the threshold of the warning: no mark on it.
  await expect(page.locator('.composer .atts [role=img]')).toHaveCount(0);

  // No maximum and a warning over 1 Mo: the file of 2 Mo is joined, with its warning; the one of 500 Ko was not warned of.
  dialog = await open();
  await max(dialog).fill('0');
  await warn(dialog).fill('1');
  await save(dialog);
  await expect.poll(() => [saved().attachMaxMb, saved().attachWarnMb]).toEqual([0, 1]);
  await memory('grosse.zip', zip(2 * MB));
  await expect(chipsOf(page)).toHaveCount(2);
  await settled(page);
  const badge = page.locator('.composer .atts [role=img]');
  await expect(badge).toHaveCount(1);
  await expect(badge).toHaveAccessibleName(/Gros fichier : 2\s+Mo, copié sur le disque/);
  await expect(chipsOf(page).nth(1)).toContainText('grosse.zip');
  await expect(chipsOf(page).nth(1).getByRole('img')).toHaveCount(1);

  // The original file off: the next file of the disk is copied, where it was joined as it is before (the first test).
  dialog = await open();
  await dialog.getByRole('switch', { name: /Joindre le fichier d.origine/ }).click();
  await expect(dialog.getByRole('switch', { name: /Joindre le fichier d.origine/ })).toHaveAttribute('aria-checked', 'false');
  // The others, changed in the same visit: all kept.
  await dialog.getByRole('group', { name: 'Texte de plus de 256 Ko' }).getByRole('button', { name: 'Refusé' }).click();
  await dialog.getByRole('spinbutton', { name: /Purger les copies après/ }).fill('7');
  await save(dialog);
  await expect.poll(() => [saved().attachOrigin, saved().attachBigText, saved().attachKeepDays]).toEqual([false, 'refuse', 7]);
  const tableau = diskFile(app, 'tableau.xlsx', zip(5 * KB, 0x62));
  await diskFiles(page, [tableau]);
  await expect(chipsOf(page)).toHaveCount(3);
  await settled(page);
  await expect.poll(() => copiesOf(app, id).map((c) => c.name)).toContain('tableau.xlsx');
  const copy = copiesOf(app, id).find((c) => c.name === 'tableau.xlsx')!;
  expect(samePath(copy.file, tableau)).toBe(false);
  expect(fs.readFileSync(copy.file).equals(zip(5 * KB, 0x62))).toBe(true);
  // And the big text is refused now, as the setting says.
  await page.locator('input[type=file]').setInputFiles({ name: 'gros.txt', mimeType: 'text/plain', buffer: Buffer.alloc(300 * KB, 0x61) });
  await expect(toast(page).filter({ hasText: /gros\.txt dépasse 256\s+Ko/ })).toBeVisible();
  await expect(chipsOf(page)).toHaveCount(3);
});

test('the page cannot make a file of its own up: a path or an id the host never read is refused, a message with one is too', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const id = await agentId(app);
  const secret = diskFile(app, 'secret.txt', Buffer.from('ne sort pas'));
  // What the page can ask of the host with the app's own commands, as a script of it would.
  const invoke = (cmd: string, args: Record<string, unknown>) =>
    page.evaluate(
      async ([cmd, args]) => {
        try {
          await (
            window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }
          ).__TAURI_INTERNALS__.invoke(cmd as string, args);
          return 'accepted';
        } catch (e) {
          return String(e);
        }
      },
      [cmd, args] as const,
    );
  // A path in place of an id, an id nobody was answered a path for: nothing is joined.
  expect(await invoke('attach_origin', { owner: id, path: secret })).toMatch(/id|missing/i);
  expect(await invoke('attach_origin', { owner: id, id: secret })).toMatch(/connaît pas ce fichier/);
  expect(await invoke('attach_origin', { owner: id, id: 'jamais-rendu-par-le-pont' })).toMatch(/connaît pas ce fichier/);
  // A message that gives that file as joined by its path, which nobody did: refused whole, nothing sent, nothing granted.
  const forged = { name: 'secret.txt', path: secret, claudePath: secret, size: 11, origin: true };
  expect(await invoke('send_message', { id, text: 'Lis ça', attachments: [], files: [forged] })).toMatch(
    /n’a pas été joint par son chemin d’origine/,
  );
  await page.waitForTimeout(500);
  expect(stdinMessages(app).filter((m) => textOf(m).includes('secret.txt'))).toEqual([]);
  // A file of the app's own data folder is never joined by its path (not a text, which a message carries: its path is what
  // would be granted).
  const inData = path.join(app.data, 'copie.zip');
  fs.writeFileSync(inData, zip(KB));
  await diskFiles(page, [inData]);
  await expect(toast(page).filter({ hasText: /dossier de données/ })).toBeVisible();
  await expect(chipsOf(page)).toHaveCount(0);
});

test('a deleted agent takes the files joined to it away, and the originals stay', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const id = await agentId(app);
  const rapport = diskFile(app, 'rapport.zip', zip(3 * KB));
  // A file of the page's memory is copied for the agent; one of the disk is joined where it lies.
  await page.locator('input[type=file]').setInputFiles({ name: 'journal.zip', mimeType: 'application/zip', buffer: zip(2 * KB, 0x63) });
  await diskFiles(page, [rapport]);
  await expect(chipsOf(page)).toHaveCount(2);
  await settled(page);
  await send(page, 'Lis ces fichiers');
  await expect(page.getByText(`Lu : journal.zip=${2 * KB}, rapport.zip=${3 * KB}`)).toBeVisible();
  expect(copiesOf(app, id).map((c) => c.name)).toEqual(['journal.zip']);

  // Another agent first: the project keeps one when this one is deleted.
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).click();
  await expect(page.locator('.card')).toHaveCount(2);
  await page.locator('.card').first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Supprimer…' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Supprimer' }).click();
  await expect(page.locator('.card')).toHaveCount(1);
  await expect.poll(() => fs.existsSync(path.join(app.data, 'attachments', id))).toBe(false);
  // The folder of the agent that is left is its own.
  expect(agentFolders(app)).not.toContain(id);
  // Never the file of the user's.
  expect(fs.readFileSync(rapport).equals(zip(3 * KB))).toBe(true);
});

test('in the Claude Code view, a file pasted and a file dropped are copied for the agent, their paths typed in its interface', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const id = await agentId(app);
  await page.getByRole('button', { name: 'Vue Claude Code' }).click();
  await expect.poll(() => app.launches().some((l) => l.argv.includes('--settings')), { timeout: 20_000 }).toBe(true);
  // Its pastes and drops go through Escouade once its interface started.
  const field = page.locator('main [data-pastes="images"] .xterm textarea');
  await expect(field).toBeAttached({ timeout: 20_000 });
  const give = (kind: 'paste' | 'drop', file: { name: string; type: string; bytes: number[] }) =>
    field.evaluate(
      (el, { kind, name, type, bytes }) => {
        const data = new DataTransfer();
        data.items.add(new File([new Uint8Array(bytes)], name, { type }));
        const init = { bubbles: true, cancelable: true };
        el.dispatchEvent(
          kind === 'paste'
            ? new ClipboardEvent('paste', { ...init, clipboardData: data })
            : new DragEvent('drop', { ...init, dataTransfer: data }),
        );
      },
      { kind, ...file },
    );
  const [rapport, tableau] = [zip(3 * KB, 0x61), zip(5 * KB, 0x62)];
  await give('paste', { name: 'rapport.zip', type: 'application/zip', bytes: [...rapport] });
  await give('drop', {
    name: 'tableau.xlsx',
    type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    bytes: [...tableau],
  });

  // Copied under the agent's folder, by their names, whatever the settings say of the original file. A copy is there (its
  // file is made) before it is written: waited for whole, not for its name.
  const copyOf = (name: string) => copiesOf(app, id).find((c) => c.name === name)!;
  const whole = (name: string, bytes: Buffer) => {
    const copy = copyOf(name);
    return !!copy && fs.readFileSync(copy.file).equals(bytes);
  };
  await expect.poll(() => [whole('rapport.zip', rapport), whole('tableau.xlsx', tableau)], { timeout: 15_000 }).toEqual([true, true]);
  expect(
    copiesOf(app, id)
      .map((c) => c.name)
      .sort(),
  ).toEqual(['rapport.zip', 'tableau.xlsx']);
  for (const c of copiesOf(app, id)) expect(c.uniq).toMatch(/^[0-9a-f]{8}$/);

  // Their paths are typed in the interface: they come to it as pastes, read as the prompt it is given. Enter does nothing while
  // the line is empty, so it is pressed until the prompts hold both (a path is pasted once its copy is made).
  const prompts = () =>
    fs.existsSync(path.join(app.root, 'claude', 'projects'))
      ? fs
          .readdirSync(path.join(app.root, 'claude', 'projects'), { recursive: true, encoding: 'utf8' })
          .filter((f) => f.endsWith('.jsonl'))
          .flatMap((f) =>
            fs
              .readFileSync(path.join(app.root, 'claude', 'projects', f), 'utf8')
              .trim()
              .split('\n'),
          )
          .map((l) => JSON.parse(l))
          .filter((e) => e.type === 'user' && typeof e.message?.content === 'string')
          .map((e) => e.message.content as string)
      : [];
  const typed = () => prompts().join('\n').toLowerCase();
  await expect
    .poll(
      async () => {
        await field.press('Enter');
        return [copyOf('rapport.zip').file, copyOf('tableau.xlsx').file].map((f) => typed().includes(f.toLowerCase()));
      },
      { timeout: 20_000 },
    )
    .toEqual([true, true]);
  // The folder is one Claude Code reads without asking: `--add-dir` of the launch of its interface, the folder of the copies.
  const tui = app.launches().find((l) => l.argv.includes('--settings'))!;
  const dirs = tui.argv.filter((a) => a.startsWith('--add-dir=')).map((a) => a.slice('--add-dir='.length));
  expect(dirs.some((d) => samePath(d, path.join(app.data, 'attachments', id)))).toBe(true);
});

test('a file joined to a ticket is kept in its folder, said on its card, read by its agent from the first message, and gone with the ticket', async ({
  app,
}) => {
  const { page } = app;
  // The window of the CI runners.
  await page.setViewportSize({ width: 1028, height: 779 });
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  // Nothing starts before « Lancer ».
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  const tickets = path.join(app.data, 'tickets');
  const folders = () => (fs.existsSync(tickets) ? fs.readdirSync(tickets) : []);

  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Lire le rapport');
  await dialog.locator('input[type=file]').setInputFiles({ name: 'rapport.zip', mimeType: 'application/zip', buffer: zip(3 * KB, 0x61) });
  // A chip under the description: its name and its size, copied at once into the drafts (never joined where it lies).
  const files = dialog.getByRole('list', { name: 'Fichiers du ticket' });
  await expect(files.getByRole('listitem')).toHaveCount(1);
  await expect(files).toContainText('rapport.zip');
  await expect(files).toContainText(/3\s+Ko/);
  await expect(dialog.getByRole('progressbar')).toHaveCount(0);
  await expect.poll(() => drafts(app).filter((f) => f.endsWith('rapport.zip'))).toHaveLength(1);
  await page.keyboard.press('Control+Enter');
  await expect(dialog).toBeHidden();

  // The file is the ticket's, in its own folder; the draft that held it is empty.
  await expect.poll(() => folders().length).toBe(1);
  const folder = path.join(tickets, folders()[0], 'files');
  await expect.poll(() => fs.existsSync(folder) && fs.readdirSync(folder, { recursive: true }).length).toBeGreaterThan(0);
  const [uniq] = fs.readdirSync(folder);
  expect(uniq).toMatch(/^[0-9a-f]{8}$/);
  expect(fs.readdirSync(path.join(folder, uniq))).toEqual(['rapport.zip']);
  const kept = path.join(folder, uniq, 'rapport.zip');
  expect(fs.readFileSync(kept).equals(zip(3 * KB, 0x61))).toBe(true);
  expect(drafts(app).filter((f) => f.endsWith('rapport.zip'))).toEqual([]);
  // The card says it has one, in the words of a file (images are said apart).
  const card = page.getByRole('button', { name: 'DEM-1 Lire le rapport', exact: true });
  await expect(card).toHaveAccessibleDescription('1 fichier joint');
  await expect(card.locator('.clip')).toBeVisible();

  // Its agent is told of it with its first message, and reads it with no permission asked: the folder of the ticket's files is
  // one of its `--add-dir`.
  await card.getByRole('button', { name: 'Lancer' }).click();
  await expect.poll(() => joinedMessage(app), { timeout: 30_000 }).toBeTruthy();
  const [file] = told(joinedMessage(app)!);
  expect(file.name).toBe('rapport.zip');
  expect(samePath(file.file, kept), `${file.file} is not ${kept}`).toBe(true);
  expect(joinedMessage(app)).toMatch(/^Ticket DEM-1 : Lire le rapport/);
  expect(addDirs(app).some((d) => samePath(d, folder))).toBe(true);
  await card.click();
  await expect(page.getByText(`Lu : rapport.zip=${3 * KB}`)).toBeVisible();
  await expect(page.getByTestId('permission-pending')).toHaveCount(0);

  // Another ticket, not started: deleted with its file, which the confirmation says.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const first = folders();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const second = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await second.getByRole('textbox', { name: 'Titre du ticket' }).fill('À jeter');
  await second.locator('input[type=file]').setInputFiles({ name: 'brouillon.zip', mimeType: 'application/zip', buffer: zip(1 * KB, 0x63) });
  await expect(second.getByRole('list', { name: 'Fichiers du ticket' })).toContainText('brouillon.zip');
  await expect(second.getByRole('progressbar')).toHaveCount(0);
  await page.keyboard.press('Control+Enter');
  await expect(second).toBeHidden();
  await expect.poll(() => folders().length).toBe(first.length + 1);
  const other = path.join(
    tickets,
    folders().find((f) => !first.includes(f))!,
  );
  await expect
    .poll(() => fs.existsSync(path.join(other, 'files')) && fs.readdirSync(path.join(other, 'files'), { recursive: true }).length)
    .toBeGreaterThan(0);
  await page.getByRole('button', { name: 'DEM-2 À jeter', exact: true }).click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Supprimer' }).click();
  const confirm = page.getByRole('dialog', { name: 'Supprimer DEM-2 ?' });
  await expect(confirm).toContainText('sa pièce jointe');
  await confirm.getByRole('button', { name: 'Supprimer' }).click();
  await expect(page.getByRole('button', { name: 'DEM-2 À jeter', exact: true })).toHaveCount(0);
  await expect.poll(() => fs.existsSync(other)).toBe(false);
  // The first ticket's file is where it was.
  expect(fs.existsSync(kept)).toBe(true);
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the chips, the lines of the message and the settings of the attachments are in English', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    const id = await agentId(app);
    const rapport = diskFile(app, 'rapport.zip', zip(3 * KB, 0x61));
    await diskFiles(page, [rapport]);
    await expect(chipsOf(page)).toHaveCount(1);
    await expect(chipsOf(page).first()).toContainText(/3\s+KB/);
    await settled(page);
    await expect(page.getByRole('button', { name: 'Remove rapport.zip' })).toBeVisible();
    await send(page, 'Read this file');
    await expect(shownMessage(page)).toBeVisible();
    const text = await shownMessage(page).innerText();
    const [file] = told(text);
    expect(file.name).toBe('rapport.zip');
    expect(samePath(file.file, rapport)).toBe(true);
    expect(text).toContain(`Attached file: rapport.zip, ${file.file}`);
    expect(text).not.toContain('Fichier joint');
    // Claude Code was told in English as well.
    await expect.poll(() => joinedMessage(app)).toBeTruthy();
    expect(joinedMessage(app)).toContain(`Attached file: rapport.zip, ${file.file}`);
    await expect(page.getByText(`Lu : rapport.zip=${3 * KB}`)).toBeVisible();
    expect(copiesOf(app, id)).toEqual([]);

    // The settings: a maximum of 1 MB, a warning over 1 MB, and the words of both in English.
    await page.getByRole('button', { name: 'Settings (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Settings' });
    await openSettingsPage(dialog, 'Advanced');
    await expect(dialog.getByRole('heading', { name: 'Attachments' })).toBeVisible();
    await expect(dialog.getByRole('switch', { name: 'Attach the original file' })).toHaveAttribute('aria-checked', 'true');
    await expect(
      dialog.getByRole('group', { name: 'Text over 256 KB' }).getByRole('button', { name: 'Attached by its path' }),
    ).toBeVisible();
    await expect(dialog.getByRole('group', { name: 'Text over 256 KB' }).getByRole('button', { name: 'Refused' })).toBeVisible();
    await expect(dialog.getByRole('spinbutton', { name: 'Purge copies after (days)' })).toBeVisible();
    await dialog.getByRole('spinbutton', { name: 'Largest attached file (MB)' }).fill('2');
    await dialog.getByRole('spinbutton', { name: 'Warn for an attached file over (MB)' }).fill('1');
    await closeSettings(dialog);
    const saved = () => JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
    await expect.poll(() => [saved().attachMaxMb, saved().attachWarnMb]).toEqual([2, 1]);

    const memory = (name: string, bytes: Buffer) =>
      page.locator('input[type=file]').setInputFiles({ name, mimeType: 'application/zip', buffer: bytes });
    await memory('huge.zip', zip(3 * MB));
    await expect(toast(page).filter({ hasText: /huge\.zip is over 2\s+MB/ })).toBeVisible();
    await memory('big.zip', zip(1536 * KB));
    await expect(chipsOf(page)).toHaveCount(1);
    await settled(page);
    await expect(chipsOf(page).first().getByRole('img')).toHaveAccessibleName(/Big file: 1\.5\s+MB, copied to the disk/);
  });
});
