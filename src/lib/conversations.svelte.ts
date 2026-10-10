// Per-agent conversation items, kept in sync with the backend through append/patch/delta ops.

import { api } from './ipc';
import type { ConvItem, ConvOp, UserItem } from './types';

/** Where the reader left a conversation, to put the view back there when the agent is opened again. */
export interface ReadingPlace {
  /** The view was at the bottom, following the conversation. */
  stick: boolean;
  /** How far it was scrolled, in pixels. */
  top: number;
  /**
   * The item whose block was at the top of the view and where its top edge was relative to the view's. Messages off
   * screen are not rendered at their real height (content-visibility): the same distance from the top would be another
   * message, so the place is told by the message. By its id, not its rank: the items drawn above it change.
   */
  anchor: { item: string; offset: number } | null;
  /** The first item the view drew, the older ones left out; null at the bottom, which draws the latest ones. */
  from: string | null;
}

let held = 0;

export class Conversation {
  items = $state<ConvItem[]>([]);
  loaded = $state(false);
  error = $state<string | null>(null);
  /** Messages sent while the worktree is being prepared: the backend records them once it is over, they show until then. */
  waiting = $state<UserItem[]>([]);
  /** Not reactive: only read when the view opens. Goes with the conversation, which goes with its agent. */
  place: ReadingPlace | null = null;
  /**
   * The item to bring into view (a search result), by its id: kept until the view shows it, which may be made, or
   * the conversation loaded, after the request. It wins over `place`.
   */
  jump = $state<string | null>(null);
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

  /** Asks the view to bring the item `id` into view and highlight it. */
  reveal(id: string) {
    this.jump = id;
  }

  /**
   * The item the conversation shows `id` in: itself, or the call of the subagent that wrote it (its first ancestor
   * without a parent: a subagent's items are drawn inside it). Null for an item it does not have.
   */
  shownAs(id: string): string | null {
    let item = this.itemOf(id);
    // A depth limit rather than trust: a damaged log could loop.
    for (let depth = 0; item?.parent && depth < 16; depth++) item = this.itemOf(item.parent);
    return item && !item.parent ? item.id : null;
  }

  private itemOf(id: string): ConvItem | undefined {
    const i = this.index.get(id);
    return i === undefined ? undefined : this.items[i];
  }

  /** Shows a message that waits for the worktree's preparation; returns what takes it off (sent, or refused). */
  holdMessage(message: Pick<UserItem, 'text' | 'images' | 'files'>): () => void {
    const item: UserItem = { ...message, kind: 'user', id: `waiting-${++held}`, ts: Date.now(), queued: false };
    this.waiting.push(item);
    return () => {
      const i = this.waiting.findIndex((w) => w.id === item.id);
      if (i >= 0) this.waiting.splice(i, 1);
    };
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
