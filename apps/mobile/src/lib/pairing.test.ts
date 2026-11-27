// The pairing of a space and its end (MO-4, ESC-570): the lock asked first, the QR code handed over unread, the code
// typed with an address, the phone's key given with the code and the token never on the page's side, each failure's
// words, « Retirer » that asks the core to revoke then forgets; and the one function of the lock (M3).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '$shared/lib/i18n';
import { confirmByLock, useLock } from './lock';
import { keptSpaces } from './paired';
import { openPairing, pairing, pairNow, refusalText, removeSpace, scanQr, typeCode, usePairingDeps } from './pairing.svelte';
import { FakeSocket, tick } from '../test/fake-socket';

const QR = JSON.stringify({
  escouade: 'pair/1',
  name: 'DESKTOP-7QK2',
  here: true,
  version: '2.3.0',
  code: 'ABCD-EFGH',
  expiresAt: Date.now() + 600_000,
  tunnel: { privateKey: 'k', peerPublicKey: 'p', endpoint: '203.0.113.7:51820', address: '10.77.0.2/32', api: '10.77.0.1:80' },
});

function fakePlugin(over: Record<string, unknown> = {}) {
  return {
    readPairing: vi.fn(async (o: { qr?: string; api?: string }) => {
      if (o.qr !== undefined) {
        const r = JSON.parse(o.qr);
        return {
          kind: 'tunnel' as const,
          name: r.name,
          here: r.here,
          version: r.version,
          code: r.code,
          expiresAt: r.expiresAt,
          endpoint: r.tunnel.endpoint,
        };
      }
      return { kind: 'direct' as const, endpoint: o.api };
    }),
    pairingKey: vi.fn(async () => ({ publicKey: 'cGhvbmUtcHVibGljLWtleQ==' })),
    finishPairing: vi.fn(async () => {}),
    startTunnel: vi.fn(async (o: { space: string }) => ({
      space: o.space,
      state: 'up' as const,
      proxyUrl: 'http://127.0.0.1:5000/s',
      websocketUrl: 'ws://127.0.0.1:5000/s/remote',
      txBytes: 0,
      rxBytes: 0,
      connections: 0,
    })),
    tunnelStatus: vi.fn(),
    stopTunnel: vi.fn(async () => {}),
    forgetSpace: vi.fn(async () => {}),
    ...over,
  };
}

describe('the pairing of a space', () => {
  let sockets: FakeSocket[];
  let used = 0;
  let plugin: ReturnType<typeof fakePlugin>;
  let lock: { lockState: ReturnType<typeof vi.fn>; unlock: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    setLang('fr');
    localStorage.clear();
    sockets = [];
    used = 0;
    plugin = fakePlugin();
    lock = { lockState: vi.fn(async () => ({ screenLock: true, biometry: 'face' })), unlock: vi.fn(async () => {}) };
    useLock(lock as never);
    usePairingDeps({
      plugin,
      scan: async () => QR,
      open: (url) => {
        const s = new FakeSocket(url);
        sockets.push(s);
        return s;
      },
      waitMs: 2_000,
    });
    openPairing();
  });
  afterEach(() => {
    usePairingDeps(null);
    useLock(null);
  });

  /** The socket of the pairing opened, the message it sent, and the core's answer. */
  async function answer(m: Record<string, unknown>): Promise<Record<string, any>> {
    await vi.waitFor(() => expect(sockets.length).toBeGreaterThan(used));
    const s = sockets[used++];
    s.onopen?.();
    const sent = s.texts()[0];
    s.json(m);
    return sent;
  }

  it('refuses a phone without a screen lock before anything', async () => {
    lock.lockState.mockResolvedValue({ screenLock: false, biometry: 'none' });
    await scanQr();
    expect(pairing.step).toBe('nolock');
    expect(plugin.readPairing).not.toHaveBeenCalled();
    await typeCode('ABCD-EFGH', '100.64.1.2:7878');
    expect(pairing.step).toBe('nolock');
    expect(plugin.readPairing).not.toHaveBeenCalled();
  });

  it('hands the QR over unread, says what it found, gives the code with the phone’s key and keeps no token', async () => {
    await scanQr();
    expect(plugin.readPairing).toHaveBeenCalledWith({ space: expect.stringMatching(/^s-/), qr: QR });
    expect(pairing.step).toBe('found');
    expect(pairing.found).toMatchObject({ kind: 'tunnel', name: 'DESKTOP-7QK2', here: true, version: '2.3.0' });
    pairing.name = 'Pixel 8';
    const done = pairNow();
    const sent = await answer({ t: 'paired', device: { id: 'd1', name: 'Pixel 8' }, token: '', key: 'Y29yZS1wdWJsaWMta2V5' });
    expect(sent).toEqual({ t: 'pair', code: 'ABCD-EFGH', name: 'Pixel 8', key: 'cGhvbmUtcHVibGljLWtleQ==' });
    const kept = await done;
    expect(plugin.finishPairing).toHaveBeenCalledWith({ space: kept!.id, coreKey: 'Y29yZS1wdWJsaWMta2V5' });
    expect(keptSpaces()).toEqual([{ id: kept!.id, name: 'Ce poste', here: true, kind: 'tunnel' }]);
    // Nothing of a key or a token in what the page keeps.
    expect(JSON.stringify(localStorage)).not.toMatch(/token|key|ABCD/i);
    expect(pairing.open).toBe(false);
  });

  it('says each refusal in its words, and refuses a core that would send the token', async () => {
    for (const [reason, words] of [
      ['expired', 'Ce QR code a expiré'],
      ['none', 'a déjà servi'],
      ['wrong', 'n’est pas le bon'],
      ['wait', 'Trop d’essais faux'],
    ]) {
      expect(refusalText({ code: 'pairing', reason })).toContain(words);
    }
    expect(refusalText({ code: 'version', update: 'server' })).toContain('Mets Escouade à jour');
    await scanQr();
    const done = pairNow();
    await answer({ t: 'refused', code: 'pairing', reason: 'expired', error: 'x' });
    await done;
    expect(pairing.step).toBe('failed');
    expect(pairing.error).toContain('Ce QR code a expiré');
    expect(pairing.again).toBe('scan');
    // A core of before: the token in its answer, no key of its own.
    await scanQr();
    const old = pairNow();
    await answer({ t: 'paired', device: { id: 'd' }, token: 'secret' });
    expect(await old).toBeNull();
    expect(pairing.error).toContain('Mets Escouade à jour');
    expect(plugin.finishPairing).not.toHaveBeenCalled();
  });

  it('pairs by the code typed with an address, and says a tunnel that does not come up', async () => {
    await typeCode('abcd efgh', 'ws://100.64.1.2:7878/remote');
    expect(plugin.readPairing).toHaveBeenCalledWith({ space: expect.any(String), api: '100.64.1.2:7878' });
    const done = pairNow();
    const sent = await answer({ t: 'paired', device: { id: 'd2' }, token: '', key: 'a2V5' });
    expect(sent.code).toBe('ABCD-EFGH');
    expect((await done)?.kind).toBe('direct');
    await typeCode('ABC', '100.64.1.2:7878');
    expect(pairing.error).toContain('8 caractères');
    // A tunnel that stays « connecting »: its words, the endpoint said.
    plugin.startTunnel.mockResolvedValue({
      space: 'x',
      state: 'connecting',
      proxyUrl: '',
      websocketUrl: '',
      txBytes: 0,
      rxBytes: 0,
      connections: 0,
    } as never);
    plugin.tunnelStatus.mockResolvedValue({
      space: 'x',
      state: 'connecting',
      proxyUrl: '',
      websocketUrl: '',
      txBytes: 0,
      rxBytes: 0,
      connections: 0,
    });
    usePairingDeps({ plugin, scan: async () => QR, open: (u) => new FakeSocket(u), waitMs: 300 });
    await scanQr();
    await pairNow();
    expect(pairing.error).toContain('Le poste ne répond pas');
    expect(pairing.error).toContain('203.0.113.7:51820');
  });

  it('« Retirer » asks the core to revoke this phone, then forgets its keys, whatever it answers', async () => {
    localStorage.setItem('escouade.spaces', JSON.stringify([{ id: 's-1', name: 'Travail', here: false, kind: 'tunnel' }]));
    const done = removeSpace('s-1', true);
    const sent = await answer({ t: 'unpaired' });
    await done;
    // The token is the proxy's to write: the page gives none.
    expect(sent).toEqual({ t: 'unpair', token: '' });
    expect(plugin.forgetSpace).toHaveBeenCalledWith({ space: 's-1' });
    expect(keptSpaces()).toEqual([]);
    // Revoked already: nothing asked of the core.
    await removeSpace('s-2', false);
    await tick();
    expect(sockets).toHaveLength(1);
  });
});

describe('the lock before a gesture that acts (M3)', () => {
  afterEach(() => useLock(null));

  it('is one function: true once it passed, false when cancelled or failed, never a throw', async () => {
    const unlock = vi.fn(async () => {});
    useLock({ unlock, lockState: vi.fn() });
    expect(await confirmByLock('Autoriser la commande de tests-e2e ? · Escouade · Travail')).toBe(true);
    expect(unlock).toHaveBeenCalledWith({ reason: 'Autoriser la commande de tests-e2e ? · Escouade · Travail' });
    unlock.mockRejectedValueOnce(Object.assign(new Error('cancelled'), { code: 'Cancelled' }));
    expect(await confirmByLock('x')).toBe(false);
    unlock.mockRejectedValueOnce(Object.assign(new Error('no lock'), { code: 'NoLock' }));
    expect(await confirmByLock('x')).toBe(false);
  });
});
