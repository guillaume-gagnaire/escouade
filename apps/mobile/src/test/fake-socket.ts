// A socket of the phone's tests (MO-1): what the app sent a space, and what the space answers, as the remote API does.
import type { Socket } from '$shared/lib/transport';

export class FakeSocket implements Socket {
  binaryType = 'blob';
  onopen: (() => void) | null = null;
  onmessage: ((e: { data: unknown }) => void) | null = null;
  onclose: ((e: { code: number; reason: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  sent: string[] = [];
  closed = false;
  constructor(public url: string) {}
  send(data: string | ArrayBuffer | Uint8Array) {
    if (typeof data === 'string') this.sent.push(data);
  }
  close(code = 1000, reason = '') {
    if (this.closed) return;
    this.closed = true;
    this.onclose?.({ code, reason });
  }
  json(m: unknown) {
    this.onmessage?.({ data: JSON.stringify(m) });
  }
  texts(): Record<string, any>[] {
    return this.sent.map((m) => JSON.parse(m));
  }
  calls(): Record<string, any>[] {
    return this.texts().filter((m) => m.t === 'call');
  }
  /** The handshake passed. */
  welcome() {
    this.onopen?.();
    this.json({ t: 'welcome', version: '2.3.0', client: 'c1', device: 'phone', session: 's1', resumed: false, desk: ['open_url'] });
  }
  /** Each `space_glance` asked so far answered with `glance`. */
  answerGlances(glance: unknown) {
    for (const c of this.calls().filter((c) => c.cmd === 'space_glance')) this.json({ t: 'reply', id: c.id, ok: glance });
  }
}

export const tick = () => new Promise((r) => setTimeout(r, 0));

/** A glance (`glance::Glance`) with what is given. */
export function glance(over: Record<string, unknown> = {}) {
  return {
    machine: { host: 'vps', user: 'escouade' },
    version: '2.3.0',
    waiting: 0,
    running: 0,
    countsOnly: false,
    contentless: false,
    colors: [],
    projects: [],
    agents: [],
    asks: [],
    ...over,
  };
}
