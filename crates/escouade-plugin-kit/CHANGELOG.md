# Changelog

## 0.1.1

- In a module, a task woken by another goes on in the same turn. It used to sleep until the next line, for ever when none came: a call waiting for `proc/exit`, for instance. A turn is bounded, and tasks still awake at its end get the next one.
- In a module, the client's `send` follows redirections as reqwest does: ten hops, a GET after 301, 302 and 303, the same call after 307 and 308. Credentials stay on the service, and a body never leaves it.
- `http::redirect`, for a client that has its say at each hop.
- `host`: a module's turns for a plugin that reads `net/stream`, keeps `ws/*` sockets or starts tasks no call began.
- In a module, a body that is not UTF-8 reads with its replacement characters instead of coming empty.

## 0.1.0

First version.
