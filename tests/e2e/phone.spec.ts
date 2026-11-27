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
/** The requests the fake STUN server received. */
let stunAsked = 0;

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
    stunAsked += 1;
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

test('Réglages › Téléphone says who STUN asks and when, and STUN turned off asks it nothing more (SR1-11)', async ({ app }) => {
  const { page } = app;
  const dialog = await openSettingsPage(page, 'Téléphone');
  const sw = dialog.getByRole('switch', { name: 'Demander l’adresse publique à STUN' });
  await expect(sw).toHaveAttribute('aria-checked', 'true');
  await expect(dialog.getByText(/toutes les 5 minutes, ce poste demande son adresse publique à stun\.cloudflare\.com/)).toBeVisible();
  await sw.click();
  await expect(sw).toHaveAttribute('aria-checked', 'false');
  const port = dialog.getByRole('spinbutton', { name: 'Port UDP' });
  await port.fill(String(UDP));
  // The app of the worker may hold the port of the tests' already (the test before): « Appliquer » is then greyed.
  const apply = dialog.getByRole('button', { name: 'Appliquer' });
  if (await apply.isEnabled()) await apply.click();
  await expect(apply).toBeDisabled();
  stunAsked = 0;
  await dialog.getByRole('switch', { name: 'Accès depuis le téléphone' }).click();
  await expect(dialog.getByTestId('phone-state')).toContainText(`Prêt sur le port UDP ${UDP}`);
  // The box is still asked (it says the machine's address to redirect), STUN never.
  await expect(dialog.getByTestId('phone-box-todo')).toContainText(`vers 192.168.1.20`, { timeout: 20_000 });
  await dialog.getByRole('button', { name: 'Revoir le réseau' }).click();
  await expect(dialog.getByTestId('phone-public')).not.toHaveText('203.0.113.7');
  expect(stunAsked).toBe(0);
  // Kept in phone.json.
  const saved = JSON.parse(fs.readFileSync(path.join(app.data, 'phone.json'), 'utf8')) as { stun: boolean; tailscale: boolean };
  expect(saved).toMatchObject({ stun: false, tailscale: false });
  await dialog.getByRole('switch', { name: 'Accès depuis le téléphone' }).click();
  await expect(dialog.getByTestId('phone-state')).toContainText('Éteint');
});

test('« Retirer » a phone revokes the device it was paired as, and says so', async ({ app }) => {
  // A phone paired through the tunnel (its peer tied to its device), and another device, written before the start.
  const now = Date.now();
  fs.writeFileSync(
    path.join(app.data, 'remote-devices.json'),
    JSON.stringify([
      { id: 'dev-pixel', name: 'Pixel 8', createdAt: now, token: 'e2e-token-of-the-pixel-0123456789abcdef' },
      { id: 'dev-pc', name: 'Portable', createdAt: now, token: 'e2e-token-of-the-laptop-0123456789abcde' },
    ]),
  );
  fs.writeFileSync(
    path.join(app.data, 'phone.json'),
    JSON.stringify({
      enabled: false,
      port: UDP,
      tailscale: false,
      peers: [
        {
          id: 'peer-pixel',
          name: 'Pixel 8',
          publicKey: 'cHVibGljLWtleS1vZi10aGUtcGl4ZWwtZm9yLXRlc3Q=',
          address: '10.77.0.2',
          device: 'dev-pixel',
          addedAt: now,
        },
      ],
    }),
  );
  const page = await app.restart();
  const dialog = await openSettingsPage(page, 'Téléphone');
  const row = dialog.getByTestId('phone-row').filter({ hasText: 'Pixel 8' });
  await row.getByRole('button', { name: 'Retirer' }).click();
  await expect(page.getByText('Pixel 8 retiré et son accès révoqué')).toBeVisible();
  await expect(dialog.getByText(/Aucun téléphone appairé/)).toBeVisible();
  // Its device revoked, signed by the window; the other one kept.
  const devices = JSON.parse(fs.readFileSync(path.join(app.data, 'remote-devices.json'), 'utf8')) as { id: string }[];
  expect(devices.map((d) => d.id)).toEqual(['dev-pc']);
  const traces = JSON.parse(fs.readFileSync(path.join(app.data, 'remote-revoked.json'), 'utf8')) as { name: string; by: string }[];
  expect(traces).toEqual([expect.objectContaining({ name: 'Pixel 8', by: 'la fenêtre d’Escouade' })]);
});

/**
 * The QR code shown as a capture may show it (`docs/MOBILE.md` § 12): a pattern drawn at random that holds nothing, and
 * a code of fiction. The real one holds the phone's key of the test's core: it never goes in a capture.
 */
async function fictionalQr(dialog: Locator) {
  await dialog.evaluate((root) => {
    const img = root.querySelector('[data-testid="pair-qr"] .code-img');
    if (img) {
      const n = 29;
      let cells = '';
      for (let y = 0; y < n; y++)
        for (let x = 0; x < n; x++) {
          const finder = (x < 7 || x >= n - 7) && y < 7 ? true : x < 7 && y >= n - 7;
          const on = finder
            ? (() => {
                const fx = x < 7 ? x : x - (n - 7);
                const fy = y < 7 ? y : y - (n - 7);
                return fx === 0 || fx === 6 || fy === 0 || fy === 6 || (fx >= 2 && fx <= 4 && fy >= 2 && fy <= 4);
              })()
            : Math.random() < 0.48;
          if (on) cells += `<rect x="${x + 2}" y="${y + 2}" width="1" height="1"/>`;
        }
      img.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${n + 4} ${n + 4}" shape-rendering="crispEdges"><rect width="100%" height="100%" fill="#fff"/><g fill="#000">${cells}</g></svg>`;
    }
    const code = root.querySelector('[data-testid="pair-qr-code"]');
    if (code) code.textContent = 'K7Q2-9MXA';
  });
}

test('« Appairer un téléphone » shows its QR code ten minutes with its code, its peer waiting for it', async ({ app }) => {
  const { page } = app;
  if (!process.env.ESCOUADE_E2E_VIEWPORT) await page.setViewportSize({ width: 1440, height: 900 });
  const dialog = await openSettingsPage(page, 'Téléphone');
  // Off: no QR code offered.
  await expect(dialog.getByRole('button', { name: /Appairer un téléphone/ })).toHaveCount(0);
  await dialog.getByRole('spinbutton', { name: 'Port UDP' }).fill(String(UDP));
  await dialog.getByRole('button', { name: 'Appliquer' }).click();
  await dialog.getByRole('switch', { name: 'Accès depuis le téléphone' }).click();
  await expect(dialog.getByTestId('phone-state')).toContainText(`Prêt sur le port UDP ${UDP}`);
  await expect(dialog.getByTestId('phone-public')).toHaveText('203.0.113.7', { timeout: 20_000 });

  await dialog.getByRole('button', { name: /Appairer un téléphone/ }).click();
  const shown = dialog.getByRole('region', { name: 'QR code d’appairage' });
  await expect(shown).toBeVisible();
  await expect(shown.locator('.code-img svg')).toBeVisible();
  await expect(shown.getByTestId('pair-qr-code')).toHaveText(/^[A-Z2-9]{4}-[A-Z2-9]{4}$/);
  await expect(shown.getByTestId('pair-qr-left')).toContainText(/Valable encore (9|10) min/);
  await expect(shown).toContainText(`Le téléphone joindra 203.0.113.7:${UDP}.`);
  // Its peer waits for it, said so; the code waits ten minutes.
  await expect(dialog.getByTestId('phone-row')).toContainText('En attente de son QR code');
  const waiting = JSON.parse(fs.readFileSync(path.join(app.data, 'remote-pairing.json'), 'utf8')) as {
    expiresAt: number;
    scope: string;
    peer: string;
    hash: string;
  };
  expect(waiting.scope).toBe('phone');
  expect(waiting.expiresAt - Date.now()).toBeGreaterThan(9 * 60_000);
  expect(waiting.expiresAt - Date.now()).toBeLessThanOrEqual(10 * 60_000);
  // The file holds the code's hash, never the code nor the key.
  const code = (await shown.getByTestId('pair-qr-code').textContent()) ?? '';
  expect(JSON.stringify(waiting)).not.toContain(code.replace('-', ''));
  await shown.scrollIntoViewIfNeeded();
  await fictionalQr(dialog);
  await shoot(page, dialog, 'qr');
  // A new one takes the place of the first, and of its peer.
  await shown.getByRole('button', { name: 'Masquer' }).click();
  await dialog.getByRole('button', { name: /Appairer un téléphone/ }).click();
  await expect(dialog.getByTestId('phone-row')).toHaveCount(1);
});
