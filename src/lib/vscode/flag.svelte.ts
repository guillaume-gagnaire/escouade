// ESC-444 (study): the development settings that put the VS Code view in place of CodeMirror in the editor, and start
// the Node extension host of VS Code with it. Read at the page's start: the workbench's services start once, so a change
// takes effect when the window is loaded again (or before the first editor is opened, as the study's bench does).

export const VSCODE_FLAG = 'escouade.dev.vscodeEditor';
export const VSCODE_NODE_FLAG = 'escouade.dev.vscodeNodeHost';

function read(key: string): boolean {
  try {
    return localStorage.getItem(key) === '1';
  } catch {
    return false;
  }
}

export const vscodeFlags = $state({
  /** The VS Code view in place of CodeMirror. */
  editor: read(VSCODE_FLAG),
  /** The Node extension host of VS Code, started by the core with the system's Node (`vscode_host_start`). */
  nodeHost: read(VSCODE_NODE_FLAG),
});
