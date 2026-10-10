// The MCP server as the window knows it: whether it runs, where it stands in each account's Claude Code, and its activity
// log (« Activité MCP »). Fed by the backend's events from the window's start, read in full when the settings show it.

import { api } from './ipc';
import type { McpActivityEntry, McpDeclaration, McpStatus, UiEvent } from './types';

/** How many entries of the log the window keeps (as many as the backend does). */
const KEPT = 200;

/** The token of a command, hidden on screen (the real command is what « Copier la commande » copies). */
export function maskToken(command: string): string {
  return command.replace(/(Bearer )[^\s"']+/g, '$1••••••••');
}

const same = (a: McpActivityEntry, b: McpActivityEntry) => JSON.stringify(a) === JSON.stringify(b);

class McpState {
  status = $state<McpStatus>({ running: false, port: 0, error: null });
  declared = $state<McpDeclaration[]>([]);
  /** The log, oldest first as the backend keeps it. */
  activity = $state<McpActivityEntry[]>([]);
  /** An event brought the status (or the declarations) since the window last read them: newer than a reading in flight. */
  private fresh = { status: false, declared: false };

  /** Reads what the backend has. What the events brought meanwhile is kept: it may be newer. */
  async load() {
    this.fresh = { status: false, declared: false };
    const [status, declared, activity] = await Promise.all([api.mcpStatus(), api.mcpDeclareStatus(), api.mcpActivity()]);
    if (status && !this.fresh.status) this.status = status;
    if (declared && !this.fresh.declared) this.declared = declared;
    if (activity) {
      const extra = this.activity.filter((e) => !activity.some((a) => same(a, e)));
      this.activity = [...activity, ...extra].sort((a, b) => a.at - b.at).slice(-KEPT);
    }
  }

  take(e: UiEvent) {
    switch (e.type) {
      case 'mcpStatus':
        this.status = e.status;
        this.fresh.status = true;
        break;
      case 'mcpDeclared':
        this.declared = e.declared;
        this.fresh.declared = true;
        break;
      case 'mcpActivity':
        this.activity = [...this.activity, e.entry].slice(-KEPT);
        break;
    }
  }

  /** « Effacer »: the window forgets the log, and the backend. */
  async clear() {
    this.activity = [];
    await api.mcpClearActivity();
  }

  /** Back to the start (tests). */
  reset() {
    this.status = { running: false, port: 0, error: null };
    this.declared = [];
    this.activity = [];
    this.fresh = { status: false, declared: false };
  }
}

export const mcp = new McpState();
