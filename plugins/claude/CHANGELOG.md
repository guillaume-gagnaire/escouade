# Changelog

## 2.2.6

- Its manifest declares the environment variables its launches set (`permissions.env`: `CLAUDE_CODE_ARTIFACT`), shown before it is installed: from Escouade 2.2, a plugin sets no other (ESC-424).

## 2.2.5

- A turn a safeguard of Anthropic refused (`stop_reason: "refusal"` on Claude Code's synthetic API error, which `error: "invalid_request"` alone does not tell from others) is told as one (`Item::Refused`, protocol 0.1.2), in the stream and in the « Vue Claude Code », and its turn ends on « Refus d'un safeguard d'Anthropic » / « Refused by an Anthropic safeguard » instead of Claude Code's English text. Escouade 2.2.x leaves the item aside; from the next version, the agent's next start leaves the refused turn out of its history (ESC-416).

## 2.2.4

- Its agents create artifacts as in a terminal: each launch sets `CLAUDE_CODE_ARTIFACT=1`, without which Claude Code in stream-json (an Agent SDK session) has no `Artifact` tool and says « Unknown skill: artifact-design » (ESC-400).

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Accounts and harnesses and a project’s Project page (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: README.fr.md, README.en.md and five pages in docs/.

## 2.2.1

- A discussion refuses `PowerShell`, `Monitor` and `EnterWorktree`, as it refuses `Bash`, instead of asking on a card.

## 2.2.0

- Its « Vue Claude Code » is read by the module (`view/translate`): the core no longer links this crate.
- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It translates what `claude` says as it did. It has
  its own version, is published on the store and can be uninstalled.
- No more `tokio` nor `signin.rs`: what Claude Code keeps of a sign-in is read by the core alone.
- Moved to `plugins/claude/` of Escouade's repository (it was `plugins/builtin/dev.escouade.claude-code/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id, its code and what it does do not change; its version is still the app's.
