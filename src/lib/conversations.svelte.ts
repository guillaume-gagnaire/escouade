// Per-agent conversation items, kept in sync with the backend through append/patch/delta ops.

import { api } from './ipc';
import type { ConvItem, ConvOp } from './types';

/** Where the reader left a conversation, to put the view back there when the agent is opened again. */
export interface ReadingPlace {
  /** The view was at the bottom, following the conversation. */
  stick: boolean;
  /** How far it was scrolled, in pixels. */
  top: number;
  /**
   * The block of the conversation at the top of the view (its rank among the blocks) and where its top edge was
   * relative to the view's. Messages off screen are not rendered at their real height (content-visibility): the
   * same distance from the top would be another message, so the place is told by the message.
   */
  anchor: { index: number; offset: number } | null;
}

export class Conversation {
  items = $state<ConvItem[]>([]);
  loaded = $state(false);
  error = $state<string | null>(null);
  /** Not reactive: only read when the view opens. Goes with the conversation, which goes with its agent. */
  place: ReadingPlace | null = null;
  private index = new Map<string, number>();
  private buffer: ConvOp[] | null = null;

  constructor(public agentId: string) {}

  async load() {
    this.buffer = [];
    this.error = null;
    try {
      const items = await api.getConversation(this.agentId);
      this.items = items;
      this.index = new Map(items.map((it, i) => [it.id, i]));
    } catch (e) {
      this.error = String(e);
    }
    const buffered = this.buffer ?? [];
    this.buffer = null;
    // Streaming deltas may already be included in the snapshot; final patches restore the text.
    for (const op of buffered) if (op.op !== 'delta') this.apply(op);
    this.loaded = true;
  }

  apply(op: ConvOp) {
    if (this.buffer) {
      this.buffer.push(op);
      return;
    }
    switch (op.op) {
      case 'append': {
        const i = this.index.get(op.item.id);
        if (i !== undefined) this.items[i] = op.item;
        else {
          this.index.set(op.item.id, this.items.length);
          this.items.push(op.item);
        }
        break;
      }
      case 'patch': {
        const i = this.index.get(op.id);
        if (i !== undefined) Object.assign(this.items[i], op.patch);
        break;
      }
      case 'delta': {
        const i = this.index.get(op.id);
        if (i !== undefined) {
          const it = this.items[i] as { text?: string };
          it.text = (it.text ?? '') + op.text;
        }
        break;
      }
    }
  }
}

const conversations = new Map<string, Conversation>();

export function conversationOf(agentId: string): Conversation {
  let c = conversations.get(agentId);
  if (!c) {
    c = new Conversation(agentId);
    conversations.set(agentId, c);
    c.load();
  }
  return c;
}

export function applyConvOps(agentId: string, ops: ConvOp[]) {
  const c = conversations.get(agentId);
  if (c) for (const op of ops) c.apply(op);
}

export function dropConversation(agentId: string) {
  conversations.delete(agentId);
}
