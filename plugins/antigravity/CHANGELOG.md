# Changelog

## 2.2.2

- What the plugin accepts by its rules no longer opens more than the call judged, even if `agy` reads a rule as a mere start of a text (ESC-376, the review of 2.2.1).
- A write of the agent’s folder itself (`C:\wt`, `C:\wt\`, `C:\wt\.`, `.`, an empty path), or of a name `.git`, `.husky`, `.githooks`, `.agents` or `.gemini` starts with, is a card without “For this agent” in Ask and Accept edits.
- A call is accepted by the rules only when it names its path already absolute and normal; a relative path, `..`, `.` or a final separator is a card.
- No command is accepted by the rules until the real trial of `agy`: `git add` and `git commit` in Accept edits, and each command in Bypass with a private project, are cards; `agentRepo` leaves `permissions.files`.
- A word of a command that, resolved against the agent’s folder, falls in the fence has it refused (`type ..\prive\secret.txt`).
- A resumption that carries a write also refuses writing in the kept folders at the root of the agent’s folder and under the allowed path followed by `\..` and `/..`.
- The card says “Always” gives its rule again each time the call comes back, and the conversation says what the rules accepted; brackets and braces never go in a rule.
- The plan is read by the root `agentHome` alone; the `dirs` dependency is gone; `real_agy_rules` has the cases the review found missing.

## 2.2.1

- Its page has its documentation, in French and in English: README.fr.md, README.en.md and five pages in docs/.
- No rule of `agy` guards anything alone (ESC-371): outside Bypass without a fence, the plugin writes no `allow` rule of its own, and judges each call `agy` refused at the end of its turn.
- A write in `.git` (any case, `GIT~1`, a worktree’s `.git` file), `.husky`, `.githooks`, `.agents` or `.gemini` is a card in Ask and Accept edits, without “For this agent”; one elsewhere in the agent’s folder is accepted by the plugin.
- Accept edits is no longer `agy`’s `accept-edits` mode, and `git add` and `git commit` are no longer prefix rules: a command is accepted alone only when `escouade_acp::git::accepts_script` accepts it and the host says its git protected (`agentRepo` read in `permissions.files`).
- In Bypass with a private project, no `command(*)` nor any other wide rule: each refusal is judged, what names the fence refused.
- A rule is never made of a text with `*`, `?`, a parenthesis or a line break; the card says “Allow” holds for the next turn as a start of a text.
- “For this agent” is no longer offered to a fenced agent.
- The fence and the agent’s folder are told by the resolved path (`..`, a relative path); a call refused again on the rule that allowed it is not allowed again in a loop.
- Its rules are written again when it is attached again, at each end of turn and on “Stop”, and `agy` is launched again before the next message after a change of mode, a restart or a rule taken back.
- “For this agent” is kept by the plugin, no longer in `escouade-always.json`; `toolPermission` and `agentMode` are taken out of `settings.json`, and only Escouade’s server is kept in `mcp_config.json`.
- Read-only is told only when its rules are written; a permission answer to a card of questions no longer takes it away; its page says the plan is read at the path the model gave.

## 2.2.0

- Its « Vue Antigravity » is read by the module (`view/translate`): the core no longer links this crate.
- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It says what it said on the recorded streams
  of `agy`. It reads the agent's home by the host (`permissions.files`, `agentHome`, in reading), and only the last 4 MiB of a
  transcript. A transcript written late no longer holds the plugin: what follows is held and sent once it is read. It has its own
  version, is published on the store and can be uninstalled.
- Moved to `plugins/antigravity/` of Escouade's repository (it was `plugins/builtin/dev.escouade.antigravity-cli/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id, its code and what it does do not change; its version is still the app's.
