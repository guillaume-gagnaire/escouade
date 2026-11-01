# Changelog

## 1.1.1 (2026-11-01)

- A line with `--output` is never allowed: with it, `git diff` and `git log` write a file of the agent's choosing,
  a redirection by another name. It goes to the user, quoted or not.

## 1.1.0 (2026-10-31)

- A plugin of Escouade's own hand: its id is `dev.escouade.team-rules` (it was `dev.exemple.team-rules`), and it lives in `plugins/team-rules/` of
  Escouade's repository, submitted to the store from that folder (its tags `plugins/team-rules/v1.2.3`). Nothing else
  changes; the old id leaves the store: install this one from the Plugins view.

## 1.0.1 (2026-10-30)

- A command allowed no longer lets another one through behind it: `&` cuts the line too, and a line with a
  substitution, a redirection or an escape is never allowed (it goes to the user).

## 1.0.0 (2026-10-30)

- Its first version.
