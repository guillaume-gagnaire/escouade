/**
 * What an agent stopped for, whole (MO-5, M4): the command of a permission, line by line, never cut; the plan of a
 * plan; what the agent wrote just before. The box's view carries the request (2000 characters a field), the glance a
 * shorter one (400): when either says it was cut, the agent's conversation is read and its card's input taken whole.
 * When it cannot be had whole, it says so: « Autoriser » never arms on a command not read to its end.
 */

import type { ConvItem, PendingRequest, PermissionItem, TextItem } from '$shared/lib/types';

/** Beyond so many lines at the phone's width, the command is read in full screen. */
export const COMMAND_LINES = 12;

export interface Detail {
  /** The command (a shell command), or what the tool acts on. */
  text: string;
  /** A shell command: shown after « $ ». */
  shell: boolean;
  /** The text is whole: « Autoriser » may arm once it was read. */
  whole: boolean;
  /** A plan's text (Markdown), for « Approuver le plan ». */
  plan: string | null;
  /** What the agent wrote last before it stopped, a few lines of it. */
  context: string | null;
}

/** What a permission's card shows of its input, whole. */
export function inputText(item: Pick<PermissionItem, 'toolName' | 'input'>): { text: string; shell: boolean } {
  const input = item.input ?? {};
  if (typeof input.command === 'string') return { text: input.command, shell: true };
  const path = input.file_path ?? input.notebook_path ?? input.path;
  if (typeof path === 'string') {
    const change = typeof input.content === 'string' ? input.content : typeof input.new_string === 'string' ? input.new_string : '';
    return { text: change ? `${path}\n\n${change}` : path, shell: false };
  }
  if (typeof input.url === 'string') return { text: input.url, shell: false };
  return { text: JSON.stringify(input, null, 2), shell: false };
}

/** The last thing the agent wrote before the request `id`, cut to a few sentences. */
export function contextBefore(items: ConvItem[], id: string): string | null {
  const at = items.findIndex((i) => i.id === id);
  const before = (at < 0 ? items : items.slice(0, at)).filter((i): i is TextItem => i.kind === 'text' && !i.parent);
  const text = before.at(-1)?.text.trim();
  if (!text) return null;
  return text.length > 280 ? `${text.slice(0, 279).trimEnd()}…` : text;
}

/** The detail of `request`, from what the line carries and, when needed or given, the conversation. */
export function detailOf(request: PendingRequest | null, items: ConvItem[] | null): Detail {
  const card = items?.find((i): i is PermissionItem => i.kind === 'permission' && i.id === request?.id) ?? null;
  const plan = request?.tool === 'ExitPlanMode' ? (typeof card?.input?.plan === 'string' ? card.input.plan : null) : null;
  const context = items && request ? contextBefore(items, request.id) : null;
  if (card && request?.tool !== 'ExitPlanMode') return { ...inputText(card), whole: true, plan, context };
  const shell = request?.tool === 'Bash';
  return {
    text: request?.arg ?? '',
    shell,
    whole: !!request && !request.cut,
    plan,
    context,
  };
}
