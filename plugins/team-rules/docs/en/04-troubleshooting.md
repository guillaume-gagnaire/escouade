# Troubleshooting Team rules

Each decision is written in its journal: **Journal** tab of its page, “Decision” filter (the whole file: `~/.escouade/plugin-logs/dev.escouade.team-rules.log`).

## The journal’s lines

**`allowed Bash npm test (1 ms)`**
Allowed by its rules: the agent ran the command without asking you.

**``refused Bash rm -rf build (2 ms): The team's rules forbid `rm -rf`.``**
Denied: the end is the message the agent got.

**`left Bash cd app && npm test to the user (1 ms)`**
Its rules did not decide: the request was asked of you. That is the case of any command they do not name, of a line with a redirection, a substitution, a backslash or `--output`, and of anything that is not a command (a file edit, a web page).

**`did not decide Bash cargo build: no answer in time; the user is asked`**
It did not answer within 500 ms (or 3 seconds at its first start). The request was asked of you. If it happens again, look at its state at the head of the journal.

**`did not decide …: {reason}; the user is asked`**
It crashed during the decision, or its answer was not one. The request was asked of you, and Escouade starts it again as usual.

## A command you thought denied is asked of you

A rule judges the start of a command, word for word: `git push origin main --force` does not start with `git push --force`, `sudo rm -rf` not with `rm -rf`. The plugin does not allow these lines: it leaves them to you. To deny them, you need rules of your own (Use tab).

## A command you thought allowed is asked of you

- Another command of the same line is not in `allow` (`cd app && npm test`).
- The line has a `>`, a `<`, a `$`, a backquote, a backslash or `--output`: never allowed.
- The word is not the same (`npm testing` is not `npm test`).
- The plugin is not on for that project, or you took back “Decides your agents’ permissions”: it is not asked, and nothing is written in the journal.

## The plugin is turned off

Its row in **Installed** says why:

**“Its files changed since it was installed (fingerprint …): install it again.”**
A file of `~/.escouade/plugins/dev.escouade.team-rules` was changed, `rules.json` most often. Install it again from the store. For other rules, make your own copy (Use tab).

**“Stopped after {n} crashes”**
Its program stopped several times in a row: all your requests come back to you. Read the last lines of the journal, then **Restart**. If it happens again, report it on its store page.
