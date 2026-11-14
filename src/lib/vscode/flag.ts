// ESC-444 (study): the development setting that puts the VS Code view in place of CodeMirror in the editor. Read once
// per page: the workbench's services start once, so a change takes effect when the window is loaded again.

export const VSCODE_FLAG = 'escouade.dev.vscodeEditor';

function read(): boolean {
  try {
    return localStorage.getItem(VSCODE_FLAG) === '1';
  } catch {
    return false;
  }
}

/** The setting as the page started with it. */
export const vscodeEditor = read();

export function setVscodeEditor(on: boolean) {
  try {
    if (on) localStorage.setItem(VSCODE_FLAG, '1');
    else localStorage.removeItem(VSCODE_FLAG);
  } catch {
    // A private window: the setting stays off.
  }
}

export const VSCODE_NODE_FLAG = 'escouade.dev.vscodeNodeHost';

/** The Node extension host of VS Code, started by the core with the system's Node (`vscode_host_start`). */
export const vscodeNodeHost = (() => {
  try {
    return localStorage.getItem(VSCODE_NODE_FLAG) === '1';
  } catch {
    return false;
  }
})();
