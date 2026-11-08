# High contrast’s permissions

## What it is

A **declarative** plugin: a manifest, `escouade-plugin.json`, that holds a theme (`contributes.themes`), and nothing else. It has no program: nothing of it runs on your machine.

## What it asks for

Nothing. The install window and the **Permissions** tab say so: “It asks for nothing: its contributions are read, nothing of it runs.”

- It gets no event, decides no permission.
- It reads and writes no file.
- It keeps no secret.
- It reaches no host: nothing leaves your machine because of it.

## What Escouade does with its colors

Escouade reads its values and filters each before setting it on the window:

- only Escouade’s color tokens are taken (`--text`, `--line`, `--run`…); another one is ignored;
- a value must be a color (`#rrggbb`, `color-mix()`, `var()` of another token…), 160 characters at most;
- never `url()`, another function, quotes, a backslash, `;`, a brace or `!important`.

So a theme can load nothing, write no CSS of its own, and hide no request of Escouade (a permission, a question). What does not pass is left out, and Settings › General says how many values were.

## What it cannot do

- Change the shape of the interface: fonts, radii, widths stay Escouade’s.
- Act when it is not picked: installed, it changes nothing until you pick it in **Colors**.
- Touch your projects, your agents or your files.
