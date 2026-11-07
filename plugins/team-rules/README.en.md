# Team rules

Allows or denies your agents’ commands by rules written in a file, `rules.json`, before they reach you. A forbidden command is denied with a message to the agent, a safe one goes through, and the rest is asked of you as without the plugin.

**Who it is for**: you or your team, when the same permission requests keep coming back (`npm test`, `git status`) and you want to forbid others (`rm -rf`, `git push --force`), without putting your agents in Bypass mode.

**What it needs**: nothing to set. Its rules are those of its `rules.json`. For rules of your own, you need your own copy of the plugin (see the Use tab).

**What it can do**:

- **decide your agents’ permissions**: the strongest permission a plugin has, shown in red at the install;
- run its program inside Escouade, isolated (in WASM): no Node to install.

It connects to nothing, keeps no secret, reads no file but its `rules.json`. It judges commands only; a file edit, a web page or an MCP tool is always asked of you.

The rest is in the documentation tabs:

1. Install: install it and choose its projects.
2. Use: the rules, and what they decide.
3. Permissions: what it sees, what it decides, what it cannot do.
4. Troubleshooting: its journal and its messages.
5. Develop: its folder, its tests, its build.
