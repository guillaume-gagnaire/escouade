# Troubleshooting isola

The messages the plugin and Escouade show for it, what they mean and what to do. They are in the interface’s language; `{…}` stands for the command, the reason or the value of the moment.

## “▶ Test” does not offer isola

The ticket starts with a recipe on the reserved ports, as without isola. A worktree is isola’s only when all three hold:

- the plugin is on for the app and for the project (**Projects** tab; a private project is off by default);
- the worktree has an `.isola.toml` at its root;
- `isola` is found on the PATH (on Windows, an `isola.exe` or `isola.com`).

Escouade reads the PATH again at most every 30 seconds: once isola is installed, wait a little. If Escouade was started before you changed the PATH, restart it.

## During a test

**“isola doesn’t run this worktree (isola not found, or no .isola.toml).”**
The program or the file went since the window opened. Check both, then run “▶ Test” again.

**“isola’s configuration (.isola.toml) hasn’t been approved: run “▶ Test” to read it before it runs.”**
The file changed since you agreed to it, or you never approved it. Run “▶ Test” again, read it, then **Run**.

**“.isola.toml is too large to be read before the launch ({n} bytes).”**
Escouade only shows files of 256 KiB at most, whole: it never cuts one. Make the file smaller.

**“.isola.toml isn’t in UTF-8.”** / **“.isola.toml can’t be read: {reason}”**
The file could not be read as it is. Save it again in UTF-8, check its rights.

**“isola up failed (code {code})”**
`isola up` ended with an error. Click **View log**: it is isola’s output, in the test’s terminal. The cause is most often in the `.isola.toml` or in the tool isola uses.

**“isola ls --json failed: {last line}”**, **“isola down failed: …”**, **“isola destroy failed: …”**
isola returned an error code. The end of the message is the last non-empty line it wrote on its error output. Run the same command in the worktree, in a terminal, to see the rest.

**“isola {command} didn’t answer in 60 s”**
isola did not finish in time: Escouade stopped it. Run the command by hand in the worktree to see where it hangs.

**“isola {command} couldn’t start: {reason}”**
Escouade could not start the program. The possible reasons:
- `no program isola on this machine`: `isola` is no longer on the PATH (on Windows, only an `.exe` or a `.com` counts);
- `4 programs at once at most`: the plugin already has four commands under way; try again once they are done;
- `its instance has ended`: the plugin was stopping; try again.

**“isola’s answer can’t be read: {reason}”**
`isola ls --json` did not write JSON. Your version of isola may not give the same output: run `isola ls --json` in the worktree and look at what it writes.

**“No response from {address} after 3 min”**
A service runs but does not answer on its port. Look at its output with isola’s tools, and check the port the `.isola.toml` gives it.

**“{service} isn’t running”**
isola lists the service, but not as `running`. Its status is written on its line.

**“The plugin dev.escouade.isola is not answering: {reason}”**
The plugin’s module could not be called (stopped, turned off, or crashed). Look at the **Journal** tab of its page, then **Restart**.

**Its state or its journal says `its module …plugin.wasm: …`**
This version of Escouade does not have isola’s module: it comes in the installers with its store version. Meanwhile, turn the plugin off (for the app or for the project): tickets start again with a recipe on the reserved ports.

## When a worktree is removed

**“Agent deleted, but isola destroy failed: {last line}”**
The agent and its worktree are deleted, but isola did not tear its services down. Run `isola ls` to see what is left, then tear it down with isola’s commands.

An `isola down` run in the background (end of a test, project closed) that fails is not shown: it is written in the app’s log.

## The journal

The **Journal** tab of the plugin’s page shows each start (`proc/spawn: ran isola down in …`), each refusal and each end of a program. The whole file is `~/.escouade/plugin-logs/dev.escouade.isola.log`.
