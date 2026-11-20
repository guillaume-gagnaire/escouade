// The views of the box « À toi » as the window last read them (ESC-513): read again, a moment after, when what they
// show moved (the box changed, a ticket, an agent's requests, the docs to share, a list of pull requests), never at
// each token an agent writes. The MCP server off, there is none.
import type { UiEvent } from '../types';
import { readViews, requestsMark, type InboxViews } from './views';

/** What the window waits before reading again: the events of one change come together. */
export const READ_AFTER_MS = 120;

const NONE: InboxViews = { enabled: false, views: [], blocking: 0, total: 0 };

class Views {
  /** The views of every project, and the counts. */
  data = $state<InboxViews>(NONE);
  /** Set once read: the counts are known. */
  loaded = $state(false);
  private timer: ReturnType<typeof setTimeout> | null = null;
  /** What each agent last said of its requests (`requestsMark`). */
  private marks = new Map<string, string>();

  /** Reads them now. A failure keeps what was shown. */
  async load(): Promise<void> {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    try {
      const v = await readViews(null);
      // A backend that says nothing (an older one, a test that does not know the views): none.
      this.data = v && Array.isArray(v.views) ? v : NONE;
      this.loaded = true;
    } catch {
      // The next event reads them again.
    }
  }

  /** Reads them a moment from now. */
  soon(): void {
    if (this.timer) return;
    this.timer = setTimeout(() => void this.load(), READ_AFTER_MS);
  }

  /** What an event says of the views: read again when it moved them. */
  take(e: UiEvent): void {
    switch (e.type) {
      case 'inbox':
      case 'ticket':
      case 'ticketRemoved':
      case 'agentRemoved':
      case 'docsShare':
      case 'prsRead':
        this.soon();
        break;
      case 'agent': {
        const mark = requestsMark(e.agent);
        if (this.marks.get(e.agent.id) !== mark) {
          this.marks.set(e.agent.id, mark);
          this.soon();
        }
        break;
      }
    }
  }

  /** The views of one project, or of all. */
  of(projectId: string | null) {
    return projectId ? this.data.views.filter((v) => v.projectId === projectId) : this.data.views;
  }

  /** For the tests: as at the start. */
  reset(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.data = NONE;
    this.loaded = false;
    this.marks.clear();
  }
}

export const inboxViews = new Views();
