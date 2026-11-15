// The Overseer as the window knows it (ESC-462): whether it runs, the user's instructions (« consignes ») with where
// they come from, its journal of actions with « Annuler », and its discussion panel, open or not. Fed by the backend's
// events from the window's start; its conversation is its agent's (`conversationOf`).

import { api } from './ipc';
import type { Outgoing, OverseerDirective, OverseerView } from './types';
import { t } from './i18n';
import { fDate } from './format';

const EMPTY: OverseerView = {
  enabled: false,
  running: false,
  needsMcp: false,
  agentId: null,
  directives: [],
  actions: [],
  messages: [],
};

/** The header the backend puts before the user's message it sends the agent: the panel shows the user's words alone. */
const USER_HEADER = /^\[(?:Message de l’utilisateur|User’s message) · (m-\d+) · [^\]]+\]\n/;

/** The user's words of a message sent to the Overseer, and its id; null for what the user did not write (a wake-up). */
export function userWords(text: string): { id: string; text: string } | null {
  const m = USER_HEADER.exec(text);
  return m ? { id: m[1], text: text.slice(m[0].length) } : null;
}

/** A wake-up of Escouade (its events), not a message of the user. */
export function isWake(text: string): boolean {
  return /^\[Escouade · /.test(text);
}

/** The actions of an instruction, as a sentence says them. */
export function actionsLabel(actions: Outgoing[]): string {
  return actions.map((a) => t(`overseer.actions.${a}`)).join(', ');
}

/** Where an instruction holds: its project, its ticket, its release, its end, once. */
export function scopeLabel(d: OverseerDirective): string {
  const parts: string[] = [];
  if (d.project) parts.push(t('overseer.directive.project', { name: d.project }));
  if (d.ticket) parts.push(t('overseer.directive.ticket', { key: d.ticket }));
  if (d.scope.release) parts.push(t('overseer.directive.release', { name: d.scope.release }));
  if (d.scope.until) parts.push(t('overseer.directive.until', { date: fDate(d.scope.until) }));
  if (d.scope.once) parts.push(t('overseer.directive.once'));
  return parts.length ? parts.join(' · ') : t('overseer.directive.everywhere');
}

class OverseerState {
  view = $state<OverseerView>(EMPTY);
  /** Its discussion panel is open. */
  open = $state(false);
  /** A message on its way. */
  sending = $state(false);
  /** Why the last gesture failed. */
  error = $state<string | null>(null);

  take(view: OverseerView) {
    this.view = view;
  }

  async load() {
    try {
      this.view = await api.overseerView();
    } catch {
      // A window without its backend (a test): nothing to read.
    }
  }

  /** A message of the user, typed or said. */
  async send(text: string, said = false): Promise<boolean> {
    const words = text.trim();
    if (!words || this.sending) return false;
    this.sending = true;
    this.error = null;
    try {
      await api.overseerSend(words, said);
      return true;
    } catch (e) {
      this.error = String(e);
      return false;
    } finally {
      this.sending = false;
    }
  }

  async undo(id: string) {
    this.error = null;
    try {
      await api.overseerUndo(id);
    } catch (e) {
      this.error = String(e);
    }
  }

  async revoke(id: string) {
    this.error = null;
    try {
      await api.overseerRevoke(id);
    } catch (e) {
      this.error = String(e);
    }
  }

  reset() {
    this.view = EMPTY;
    this.open = false;
    this.sending = false;
    this.error = null;
  }
}

export const overseer = new OverseerState();
