# Using Team rules

When an agent of a project the plugin is on in asks to run a command, Escouade asks the plugin before bothering you. The plugin answers by its rules: allow, deny, or leave it to you.

## What it is asked

- **Commands** only: the `Bash` tool (or `PowerShell`), whatever the agent’s harness. The plugin reads the command line as the agent wrote it.
- A file edit, a read, a web page, an MCP tool: it does not judge them, they are asked of you.
- A plan to approve and a question of the agent never go to plugins.

## Its rules

Those of the `rules.json` published with it:

```json
{
  "deny": [
    "rm -rf",
    "git reset --hard",
    { "command": "git push --force", "message": "No force push in this team: push a new commit instead." },
    { "command": "git push -f", "message": "No force push in this team: push a new commit instead." }
  ],
  "allow": ["npm test", "npm run lint", "cargo test", "cargo nextest", "git status", "git diff", "git log"]
}
```

- `deny`: the commands denied, by their start. A rule is a text, or a text and the message the agent gets. Without a message, the agent gets ``The team's rules forbid `rm -rf`.``
- `allow`: the commands allowed, by their start.

## What they decide

The plugin cuts the line into commands, at each `&&`, `||`, `;`, `|`, `&` and line break, then judges each by its **start, word for word** (extra spaces do not count).

1. **Denied** if a single command of the line starts with a `deny` rule. A denial wins over everything.
2. **Allowed** if each command of the line starts with an `allow` rule, and the line has no substitution (`$(…)`, backquotes, `$VAR`), no redirection (`>`, `<`), no backslash and no `--output` option.
3. **Else, yours**: the request comes to its card as without the plugin.

| Line | Decision |
| --- | --- |
| `npm test` | allowed |
| `npm test -- --watch` | allowed (starts with `npm test`) |
| `git status && git diff` | allowed (both commands are) |
| `npm testing` | yours (`npm testing` is not `npm test`) |
| `cd app && npm test` | yours (`cd app` is not allowed) |
| `rm -rf build` | denied |
| `npm test && rm -rf dist` | denied (one denied command is enough) |
| `git push --force` | denied, with its message |
| `git diff > out.txt` | yours (a redirection) |
| `git log --output=x` | yours (`--output` writes a file) |
| `git diff \| less` | yours (`less` is not allowed) |

## Its limits

A rule judges the **start** of a command, not its meaning:

- `git push origin main --force` is not denied: it does not start with `git push --force`. It is asked of you.
- `sudo rm -rf /` and `rm -fr x` are not denied either: they do not start with `rm -rf`. They are asked of you.
- What the plugin does not deny is not allowed for all that: whatever its rules do not name comes back to you. So it does not replace your reading, it spares you part of it.

An allowance holds for that request alone. Another plugin that denies wins over it.

## What you see, what the agent sees

The request’s card, in the conversation, says who answered: “Allowed by Team rules via Escouade”, or “Denied by Team rules via Escouade” with the reason. Escouade only notifies you if no plugin decided. You can always answer yourself on the card; the first answer to arrive is kept.

The agent learns who refused, never that it was you: “Refused by Team rules through Escouade: No force push in this team: push a new commit instead.”. It can look for another way.

## Having your own rules

The rules are a file of the plugin: they are not changed in the app. Escouade pins an installed plugin by the fingerprint of its files; a `rules.json` changed in `~/.escouade/plugins/dev.escouade.team-rules` turns it off (“Its files changed since it was installed”).

For rules of your own:

1. Copy the `plugins/team-rules` folder of Escouade’s repository.
2. Give it an id of your own in `escouade-plugin.json` (`com.my-team.rules`): `dev.escouade.*` is kept for Escouade’s plugins.
3. Write your `rules.json`, in the same shape.
4. Install it from the Plugins view, **From a folder…** or **From a git repository…** (so that the team reviews its rules like its code). It will be “Not verified”: nobody else read it.

Then turn `dev.escouade.team-rules` off if you only want your rules: both decide together, and a denial of either wins.
