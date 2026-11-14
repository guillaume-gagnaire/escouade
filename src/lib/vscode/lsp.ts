// ESC-444 (study): a language server started by the core (`lsp_start`: rust-analyzer, typescript-language-server, in
// the project's folder), joined to the VS Code view by `monaco-languageclient`. Its messages go both ways over the
// window's IPC: `lsp_send` to it, the event `vscode-study-lsp` from it. The page never starts a program.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { MonacoLanguageClient } from 'monaco-languageclient';
import { CloseAction, ErrorAction } from 'vscode-languageclient/browser';
import {
  AbstractMessageReader,
  AbstractMessageWriter,
  type DataCallback,
  type Disposable,
  type Message,
  type MessageReader,
  type MessageWriter,
} from 'vscode-jsonrpc/browser';
import * as vscode from 'vscode';

type Server = 'rust' | 'typescript';

const readers = new Map<number, CoreReader>();
let listening: Promise<unknown> | null = null;

function listenOnce() {
  listening ??= listen<{ id: number; msg: string | null }>('vscode-study-lsp', (e) => {
    const r = readers.get(e.payload.id);
    if (!r) return;
    if (e.payload.msg === null) r.ended();
    else r.push(JSON.parse(e.payload.msg) as Message);
  });
  return listening;
}

class CoreReader extends AbstractMessageReader implements MessageReader {
  private callback: DataCallback | null = null;
  private queue: Message[] = [];
  /** Messages received, for the study. */
  count = 0;
  listen(callback: DataCallback): Disposable {
    this.callback = callback;
    for (const m of this.queue.splice(0)) callback(m);
    return { dispose: () => (this.callback = null) };
  }
  push(m: Message) {
    this.count++;
    if (this.callback) this.callback(m);
    else this.queue.push(m);
  }
  ended() {
    this.fireClose();
  }
}

class CoreWriter extends AbstractMessageWriter implements MessageWriter {
  constructor(private readonly id: number) {
    super();
  }
  async write(msg: Message): Promise<void> {
    await invoke('lsp_send', { id: this.id, msg: JSON.stringify(msg) });
  }
  end() {}
}

const LANGUAGES: Record<Server, string[]> = { rust: ['rust'], typescript: ['typescript', 'javascript'] };

export interface Running {
  client: MonacoLanguageClient;
  id: number;
  reader: CoreReader;
}

/** Starts `server` for the project whose folder is `rootPath` (its URI path) and joins it to the view. */
export async function startLanguageServer(projectId: string, server: Server, rootPath: string): Promise<Running> {
  await listenOnce();
  const id = await invoke<number>('lsp_start', { projectId, server });
  const reader = new CoreReader();
  readers.set(id, reader);
  const writer = new CoreWriter(id);
  const root = vscode.Uri.from({ scheme: 'file', path: rootPath });
  const client = new MonacoLanguageClient({
    name: `${server} (Escouade)`,
    id: `escouade-${server}`,
    clientOptions: {
      documentSelector: LANGUAGES[server].map((language) => ({ scheme: 'file', language })),
      workspaceFolder: { index: 0, name: rootPath.split('/').pop() ?? 'root', uri: root },
      errorHandler: {
        error: () => ({ action: ErrorAction.Continue }),
        closed: () => ({ action: CloseAction.DoNotRestart }),
      },
    },
    messageTransports: { reader, writer },
  });
  reader.onClose(() => readers.delete(id));
  await client.start();
  return { client, id, reader };
}

export async function stopLanguageServer(r: Running) {
  await r.client.stop().catch(() => {});
  await invoke('lsp_stop', { id: r.id });
}
