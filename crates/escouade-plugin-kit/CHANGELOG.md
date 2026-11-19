# Changelog

## 0.1.3

- Profiles (`contributes.profiles`, Escouade 2.2.5): `secret_of(profile, key)` asks a secret field of a profile, `profiles(kind)` the profiles of a kind, `profile(kind, id)` the simple fields of one; `greeting().profiles` says whether Escouade serves them.
- Takes `escouade-plugin-protocol` 0.1.4: `greeting().typed_commands` says whether Escouade serves the jokers of `permissions.commands` (`{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}`, `{path:plugin}`), and `proc/spawn` answers the program's `pid` with its `proc`.
- In a module, a big line (a dictation of 10 minutes is one of 25 MB) is read and written with one copy instead of three, and never doubled at its end: such a module crashed at random for lack of memory, by where its allocator stood. A plugin rebuilt on 0.1.4 has more than 32 MB to spare on that dictation.

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
