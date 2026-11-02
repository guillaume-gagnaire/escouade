# Plugins

A plugin adds to Escouade what it does not do on its own: a theme, a harness, notifications, rules for your agents’ permissions. This page says what a plugin can do, the forms it comes in, where it comes from and where to start.

## What a plugin brings

A plugin describes itself in one file, its [manifest](plugins/manifest), and declares there what it adds:

- **themes**: the window’s colors, dark and light, and a palette for projects ([themes](plugins/themes));
- **reactions to events**: a webhook or a command when a ticket moves to “To review”, when an agent waits, when a turn ends ([events](plugins/events));
- **permission decisions**: allowing or denying what an agent asks for, before the question reaches you ([permission rules](plugins/permission-rules));
- **harnesses**: an agent that speaks ACP, like Goose, described without code ([a described ACP harness](plugins/harnesses));
- **a workflow reader**, for a workflow Escouade does not read on its own;
- **its settings and its secrets**, which you fill in on its page.

Some of Escouade’s integrations are plugins themselves, shipped with the app: GitHub, GitLab, Bitbucket, Gitea and Forgejo, Trello, the ACP harnesses, isola and the online voice services (see [the plugins shipped with Escouade](plugins/installer#the-plugins-shipped-with-escouade)).

## Three forms

| Form | What it is | What the install window says |
| --- | --- | --- |
| Declarative | The manifest alone: a theme, webhooks, a described harness. Nothing runs. | Its permissions, with no program |
| Program in JS or TypeScript | A bundle (`run.js`) that Escouade runs inside the app, in WASM, with no Node on your machine. It reaches the network only through Escouade, to the hosts it declares. | “Runs inside Escouade, isolated” |
| Program | An executable (`run.command`) started in a process, with your rights. This is the form of a plugin that must start a program of your computer. | “Runs a program on your computer, with your rights” |

A program talks to Escouade through [the protocol](plugins/api); [the SDK](plugins/sdk) writes it for you in TypeScript.

## What a plugin cannot do

- **Read another one’s secrets**: a plugin has only its own, never those of an account, a harness or another plugin.
- **Act in a private project** without your consent: a plugin you install is off there from the start (only the plugins shipped with Escouade are on there, like the rest of the app).
- **Install itself**: a project can propose a plugin in its [`.escouade.yml`](escouade-yml#plugins), never install it; an `escouade://` link opens its page, never the install.
- **Receive what it did not ask for**: a fact outside `permissions.events` is never sent to it, a secret outside `permissions.secrets` is refused (see [permissions](plugins/permissions)).
- **Change the interface** beyond what its manifest declares: no view of its own.

## Where it comes from

- **Shipped with Escouade**: signed and updated with the app; it is switched off, never uninstalled.
- **The store**, [store.escouade.dev](https://store.escouade.dev): each version passes eleven checks there; those Escouade reviewed carry the “Verified by Escouade” seal, the others are “Unverified”.
- **A folder or a git repository**: allowed, always “Unverified”: nobody reviewed it.
- **A folder in development**: loaded in place, read again when it changes, to write it.

## Where to start

- **Using one**: [installing a plugin](plugins/installer), then [permissions](plugins/permissions).
- **Writing one**: [a first plugin](plugins/quick-start) in ten minutes, then [the manifest](plugins/manifest), [the SDK](plugins/sdk) and [the example plugins](plugins/examples).
- **Publishing it**: [testing a plugin](plugins/testing), [publishing on the store](plugins/publishing), [plugin security](plugins/security) and [versions](plugins/versioning).
- **A question**: [the FAQ](plugins/faq).
