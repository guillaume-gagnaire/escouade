# Changelog

## 2.3.0

- Controlling a pull request, on the user's click in Escouade 2.3 (ESC-488): where it stands for its merge (`forge/control`), who may review and the repository's labels (`forge/candidates`), a draft made ready or back (`forge/setDraft`), reviewers and labels asked and taken back (`forge/requestReviewers`, `forge/removeReviewer`, `forge/addLabels`, `forge/removeLabel`), its branch updated by a merge or a rebase (`forge/updateBranch`), one job run again (`forge/rerunCheck`), auto-merge turned off (`forge/cancelAutoMerge`) and its branch deleted once merged (`forge/deleteBranch`). GraphQL's refusals say their status (403, 404, 422).

- Writing on a pull request, on the user's click in Escouade 2.3 (ESC-474): a comment on lines of its diff, one or a range, on the commit read (`forge/lineComment`), a thread resolved (`forge/resolve`, by its id) and opened again (`forge/unresolve`), beside the answers and comments it already wrote. A thread deleted meanwhile says 404.

- A pull request read whole, for Escouade 2.3 (ESC-452): its description (`forge/description`, with its tip's commit and when it last changed), its commits (`forge/commits`), its files and their counts (`forge/files`; their lines are git's, read by Escouade) and its threads (`forge/threads`), with their lines, their answers and their state: open, resolved or outdated. It stands on escouade-plugin-kit 0.1.3.

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Connected services and Advanced › Network (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: install, use, permissions, troubleshooting and develop.

## 2.2.1

- A next page outside the API is not followed: the token stays on it.
- Its page names the hosts of a job's log, reached without the token.

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It answers as it did. It has its own version, is
  published on the store and can be uninstalled. It may reach the instances its accounts name (`{account}`), as it did as a process.
- Moved to `plugins/github/` of Escouade's repository (it was `plugins/builtin/dev.escouade.github/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
