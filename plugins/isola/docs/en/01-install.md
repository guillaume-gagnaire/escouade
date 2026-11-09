# Installing isola

The plugin comes with Escouade, you do not install it from the store. In the Plugins view, **Installed**, it is listed:

- under **Shipped with Escouade** while the app gives it itself: signed with it, updated with it, its version is Escouade’s. You can turn it off, not uninstall it;
- under **Offered by Escouade** once the installer holds its store version: Escouade installs it with no network at the first start, then it is a plugin like any other, which you can turn off, uninstall and update.

Its WASM module is not yet in Escouade’s installers: it comes there with its store version. If its page says it stopped for want of a module, see Troubleshooting.

To be of use, it needs two things: the `isola` program on the machine, and an `.isola.toml` in the project.

## 1. Install the isola program

Install [isola](https://github.com/cyucelen/isola) as its documentation says, and check it is on your PATH: `isola --help` must answer in a terminal.

- Escouade looks for `isola` in the absolute folders of the PATH, at most every 30 seconds: installed while Escouade runs, it is found soon after, without restarting the app.
- On Windows, Escouade only starts an `isola.exe` or an `isola.com`, never an `isola.cmd` or an `isola.bat` (they only run through a shell).

## 2. Add an `.isola.toml` to the project

The file goes at the root of the project and is committed: its presence in a worktree is what says that its services are isola’s. Its shape is the one isola documents.

A worktree made from a branch that has the file committed is isola’s from the start: the agent gets no block of reserved ports, and has no launch recipe to write. It only gives the address that shows the feature.

## 3. Choose the projects

In the Plugins view, open **isola**, **Projects** tab:

- it is on in every project that is not private;
- a private project is off by default: turn it on there if you want isola there too;
- the switch on its row, in **Installed**, turns it off for the whole app.

Off for the app or for the project, without the `isola` program or without the file, a worktree is nobody’s: the ticket starts as without isola, with a recipe on the reserved ports.

## What Escouade keeps, and where

- Your choices (on or off for the app, for each project): in `~/.escouade/plugins.json`.
- The `.isola.toml` you approved and the address to open: with the agent, in Escouade’s data (`~/.escouade`). A file changed afterwards is shown to you again.
- The plugin’s journal: `~/.escouade/plugin-logs/dev.escouade.isola.log`, and its last lines in the **Journal** tab.

isola keeps its own data (its services, their databases) where it puts them: Escouade does not touch it, except through `isola destroy`.
