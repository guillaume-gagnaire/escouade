# Versions

How a plugin says which Escouade it is written for, what happens when either one changes, and how the store picks the version to offer.

## The plugin’s version

`version`, in the manifest, is three numbers (`1.2.0`, `0.3.0-beta.1`), the same as the version’s git tag (`v1.2.0`, or `plugins/<folder>/v1.2.0` in a repository of several plugins). The store refuses a version that is not greater than the last one published; a published version is frozen, and its number never comes back.

## For which Escouade

`escouade` says three things:

- `api`: the major version of the [protocol](plugins/api) the plugin speaks, `"1"`. Escouade serves version 1 today.
- `min`: the oldest Escouade it was tried with (`2.1.0`, or `2.1`).
- `max`, optional: the newest. `2.4` covers every 2.4.x.

Outside that range, or for another API, the plugin stays installed but disabled, and Escouade says why: “This plugin needs Escouade 2.3.0 or newer; you have 2.1.0. Update Escouade.”

## What does not break

- **An unknown field of the manifest is ignored**: a manifest written for a newer Escouade still loads, and the ignored fields are listed.
- **An unknown method** answers the error `-32601`, both ways: a plugin that does not serve a call does not crash, and Escouade applies its default answer.
- **A new fact** is only sent to the plugin that asks for it in `permissions.events`.

## What breaks

- **Another version of the protocol**: at `initialize`, a plugin that does not speak Escouade’s is stopped, with the reason (“it speaks version 2 of the protocol; this Escouade speaks version 1”).
- **A manifest invalid** for this Escouade: it does not load, and the reason is told field by field.

## Updates

- **Escouade** offers **the latest version it can run**: a version written for a newer Escouade is skipped, a version withdrawn by its author too. An update is offered, never installed on its own (see [installing a plugin](plugins/installer#updates)).
- **A version that asks for more** permissions asks for the user’s consent again.
- **The plugins shipped with Escouade** have its version and change with it.

## Withdrawing a version

- **Its author** withdraws it from the store from their account: it is no longer offered, and those who have it keep it, marked “withdrawn by the author” (see [publishing on the store](plugins/publishing)).
- **Escouade** can withdraw a dangerous version through its revocation list: it is then stopped and disabled for everyone, with the reason (see [withdrawn by Escouade](plugins/installer#withdrawn-by-escouade)).
