// Réglages › Téléphone (ESC-567, MO-2): the phone's access turned on in the real app, on a UDP port of the tests'; the
// network seen through a fake box (NAT-PMP, whose address on the Internet is the operators' shared one: a CGNAT, and
// which refuses to map) and a fake STUN server, both on the loopback: no test reaches the user's box. The tunnel answers
// nothing to a datagram that is not WireGuard, and the API is on no TCP port; turned off, the port is free again. The
// captures (light and dark) go to ESCOUADE_E2E_SHOTS, at 1440 × 900, or at ESCOUADE_E2E_VIEWPORT's size.

import dgram from 'node:dgram';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { expect, openSettingsPage, test } from './fixture';

/** The worktree's ports when given (ESCOUADE_PORT_BASE: + 7, + 8, + 9), else ports of this process's own. */
const BASE = Number(process.env.ESCOUADE_PORT_BASE) || 41000 + (process.pid % 2000) * 4;
const NATPMP = BASE + 7;
const STUN = BASE + 8;
const UDP = BASE + 9;
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZE = process.env.ESCOUADE_E2E_VIEWPORT ? `-${process.env.ESCOUADE_E2E_VIEWPORT}` : '-1440x900';

let box: dgram.Socket;
let stun: dgram.Socket;

test.beforeAll(async () => {
  // A box of NAT-PMP: its address on the Internet is 100.72.3.9 (a CGNAT); it refuses mappings (« refusé », NAT-PMP off in it).
  box = dgram.createSocket('udp4');
  box.on('message', (m, from) => {
    const epoch = [0, 0, 0, 1];
    if (m.length === 2 && m[1] === 0) box.send(Buffer.from([0, 128, 0, 0, ...epoch, 100, 72, 3, 9]), from.port, from.address);
    else if (m.length === 12 && m[1] === 1)
      box.send(Buffer.from([0, 129, 0, 2, ...epoch, m[4], m[5], 0, 0, 0, 0, 0, 0]), from.port, from.address);
  });
  // A STUN server: this machine seen from outside at 203.0.113.7.
  stun = dgram.createSocket('udp4');
  stun.on('message', (m, from) => {
    if (m.length < 20) return;
    const magic = 0x2112a442;
    const out = Buffer.alloc(32);
    out.writeUInt16BE(0x0101, 0);
    out.writeUInt16BE(12, 2);
    m.copy(out, 4, 4, 20);
    out.writeUInt16BE(0x0020, 20);
    out.writeUInt16BE(8, 22);
    out.writeUInt16BE(1, 24);
    out.writeUInt16BE(from.port ^ (magic >>> 16), 26);
    // JavaScript's ^ is signed: brought back to 32 bits without a sign.
    out.writeUInt32BE((((203 << 24) | (0 << 16) | (113 << 8) | 7) ^ magic) >>> 0, 28);
    stun.send(out, from.port, from.address);
  });
  await Promise.all([
    new Promise<void>((r) => box.bind(NATPMP, '127.0.0.1', () => r())),
    new Promise<void>((r) => stun.bind(STUN, '127.0.0.1', () => r())),
  ]);
});

test.afterAll(() => {
  box?.close();
  stun?.close();
});

test.use({
  appEnv: { ESCOUADE_PHONE_NET: `fake;natpmp=127.0.0.1:${NATPMP};stun=127.0.0.1:${STUN};local=192.168.1.20` },
});

/** The theme of the app, set as « Thème » would. */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, dialog: Locator, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['light', 'dark'] as const) {
    await theme(page, t);
    await page.waitForTimeout(150);
    await dialog.screenshot({ path: path.join(SHOTS, `phone-${name}${SIZE}-${t}.png`), animations: 'disabled' });
  }
}

/** Whether a datagram to the tunnel's port gets any answer in a second. */
function answers(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const s = dgram.createSocket('udp4');
    const timer = setTimeout(() => {
      s.close();
      resolve(false);
    }, 1000);
    s.on('message', () => {
      clearTimeout(timer);
      s.close();
      resolve(true);
    });
    s.on('error', () => {
      clearTimeout(timer);
      s.close();
      resolve(false);
    });
    s.send(Buffer.from('GET / HTTP/1.1\r\n\r\n'), port, '127.0.0.1');
  });
}

/** Whether a TCP connection opens on the port. */
function tcpOpens(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const c = net.connect({ port, host: '127.0.0.1' });
    c.once('connect', () => {
      c.destroy();
      resolve(true);
    });
    c.once('error', () => resolve(false));
  });
}

/** Whether the UDP port is free (nothing bound on it). */
function udpFree(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const s = dgram.createSocket('udp4');
    s.once('error', () => resolve(false));
    s.bind(port, '0.0.0.0', () => {
      s.close();
      resolve(true);
    });
  });
}

test('Réglages › Téléphone turns the tunnel on, says the box to open and the CGNAT, and turns it off', async ({ app }) => {
  const { page } = app;
  if (!process.env.ESCOUADE_E2E_VIEWPORT) await page.setViewportSize({ width: 1440, height: 900 });
  const dialog = await openSettingsPage(page, 'Téléphone');
  await expect(dialog.getByTestId('phone-state')).toContainText('Éteint');
  await expect(dialog.getByText(/joint ce poste par un tunnel WireGuard/)).toBeVisible();
  await shoot(page, dialog, 'off');

  // The port of the tests', then on.
  const port = dialog.getByRole('spinbutton', { name: 'Port UDP' });
  await port.fill(String(UDP));
  await dialog.getByRole('button', { name: 'Appliquer' }).click();
  await expect(dialog.getByRole('button', { name: 'Appliquer' })).toBeDisabled();
  await dialog.getByRole('switch', { name: 'Accès depuis le téléphone' }).click();
  await expect(dialog.getByTestId('phone-state')).toContainText(`Prêt sur le port UDP ${UDP}`);
  await expect(dialog.getByText('API dans le tunnel : ws://10.77.0.1/remote')).toBeVisible();

  // The box could not open it: the gesture said, with the machine's address; the CGNAT and its ways out.
  await expect(dialog.getByTestId('phone-box-todo')).toContainText(`redirige le port UDP ${UDP} vers 192.168.1.20`, {
    timeout: 20_000,
  });
  await expect(dialog.getByText(/NAT-PMP est coupé dans la box/)).toBeVisible();
  await expect(dialog.getByTestId('phone-cgnat')).toContainText('CGNAT');
  await expect(dialog.getByTestId('phone-public')).toHaveText('203.0.113.7');
  await expect(dialog.getByText(/Aucun téléphone appairé/)).toBeVisible();

  // Out of the tunnel, nothing: no answer to a datagram that is not WireGuard, no TCP port.
  expect(await answers(UDP)).toBe(false);
  expect(await tcpOpens(UDP)).toBe(false);
  await shoot(page, dialog, 'on');

  // « Avancé »: the tunnel's public key.
  await dialog.getByRole('button', { name: /^Avancé.*Changer la clé du tunnel/ }).click();
  await expect(dialog.getByText(/Clé publique du tunnel : [A-Za-z0-9+/]{43}=/)).toBeVisible();

  // Off: the port is free again.
  await dialog.getByRole('switch', { name: 'Accès depuis le téléphone' }).click();
  await expect(dialog.getByTestId('phone-state')).toContainText('Éteint');
  await expect.poll(() => udpFree(UDP)).toBe(true);
});
