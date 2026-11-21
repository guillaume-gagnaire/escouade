# Changelog

## 0.1.5

The kit of Escouade 2.3, on escouade-plugin-protocol 0.1.6: what came after the 0.1.4 published from Escouade 2.2.5, which it keeps whole. Its changes were written for a 0.1.3 and a 0.1.4 of the 2.3 that were never published.

- `Forge::review` (« Ta relecture », ESC-494): a `ReviewLine` also says its range (`start_line`) and its side (`side`, the base's for a line removed), a `ReviewDraft` the commit read (`commit`), on escouade-plugin-protocol 0.1.6; a plugin that ignores them publishes as before. `errors::said` also reads a list of texts in `errors` (GitHub's 422: `["Line could not be resolved"]`).

- `Forge`: the control of a pull request (ESC-488), `unsupported` by default: `control` (`forge/control`, a `PrControl`), `candidates` (`forge/candidates`, `Candidates`), `set_draft`, `request_reviewers`, `remove_reviewer`, `add_labels`, `remove_label`, `update_branch` (`UpdateMethod`), `rerun_check`, `cancel_auto_merge` and `delete_branch`, on the types of escouade-plugin-protocol 0.1.6.

- `Forge`: two writes more (`unsupported` by default), `unresolve` (`forge/unresolve`, a thread opened again) and `line_comment` (`forge/lineComment`, a comment on lines of the diff, `LineComment` of escouade-plugin-protocol 0.1.6); `resolve` takes the thread's id of a host that reads threads.

- `Forge`: a pull request read whole, four calls more a host serves or refuses (`unsupported` by default): `description`, `commits`, `files` and `threads` (`forge/description`, `forge/commits`, `forge/files`, `forge/threads`), on the types of escouade-plugin-protocol 0.1.6 (`PrDescription`, `PrCommit`, `PrFile`, `ReviewThread` and its `ThreadComment`s).

## 0.1.4

0.1.3 was never published: its changes come with this one. It needs Escouade 2.2.5 for what follows (`escouade_plugin_protocol::ESCOUADE_MIN`, the `escouade.min` of a plugin that uses it).

- Profiles (`contributes.profiles`): `secret_of(profile, key)` asks a secret field of a profile, `profiles(kind)` the profiles of a kind, `profile(kind, id)` the simple fields of one, `profile_of(project, setting)` the profile a project chose.
- `settings(project)`: the settings, those of a project by their ranks (this machine's value for the project, its `.escouade.yml`, the default).
- `spawn(params)` runs a program and gives its `proc` and its `pid` (`Spawned`); `spawn_typed(params)`, for an entry with jokers, wants the pid.
- `Cx::require(Capability::Profiles | Capability::TypedCommands)` refuses (`-32601`) before the call when Escouade did not announce it at `initialize`: an older Escouade would ignore the profile of `secrets/get` and answer another secret. `secret_of`, `profiles`, `profile`, `profile_of` and `spawn_typed` check it themselves. `Cx::greeting()` is what Escouade said.
- Takes `escouade-plugin-protocol` 0.1.4: `greeting().typed_commands` says whether Escouade serves the jokers of `permissions.commands` (`{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}`, `{path:plugin}`), and `proc/spawn` answers the program's `pid` with its `proc`.
- In a module, a big line (a dictation of 10 minutes is one of 25 MB) is read and written with one copy instead of three, and never doubled at its end: such a module crashed at random for lack of memory, by where its allocator stood. A plugin rebuilt on it has more than 32 MB to spare on that dictation.

## 0.1.2

- `Hosts`: a source served from several accounts. An account after its first names its secret (`account:<name>`), and each one keeps its own client.
- Its refusals send to the settings pages of Escouade 2.2: Connected services, and Advanced › Network.

## 0.1.1

- In a module, a task woken by another goes on in the same turn. It used to sleep until the next line, for ever when none came: a call waiting for `proc/exit`, for instance. A turn is bounded, and tasks still awake at its end get the next one.
- In a module, the client's `send` follows redirections as reqwest does: ten hops, a GET after 301, 302 and 303, the same call after 307 and 308. Credentials stay on the service, and a body never leaves it.
- `http::redirect`, for a client that has its say at each hop.
- `host`: a module's turns for a plugin that reads `net/stream`, keeps `ws/*` sockets or starts tasks no call began.
- In a module, a body that is not UTF-8 reads with its replacement characters instead of coming empty.

## 0.1.0

First version.
