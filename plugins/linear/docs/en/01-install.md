# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Make the API key on Linear

1. In Escouade, open Settings › Connected services. Under "Ticket sources", in the "Connected accounts" group, click "Connect…" on the Linear row.
2. Click "Create a Linear API key". Your browser opens `https://linear.app/settings/account/security`. Make a personal API key and copy it.
3. Paste it into the "API key" field.

The key acts on your behalf: it sees the teams and issues your account sees.

## 3. Connect

Click "Connect". The button says "Checking…" while Escouade asks Linear who you are. If Linear accepts, the row says "Connected · " followed by your Linear email. If Linear refuses, its answer shows in red under the form (see **Troubleshooting**). An empty field is reported before any call: "Give the token.".

The connection is kept at once: it does not go through "Save".

## 4. Link a team to a project

In Settings › Projects › Imported tickets, the "Sources linked to `<project>`" group offers the "Linear team" menu: your teams, with their key ("Engine (ENG)"). Pick one. Escouade loads its workflow states and fills in the "Status mapping" (see **Use**). Save the settings.

You can also start from the Kanban: "Import" with no linked source shows "No source linked to this project" and offers "Link a source", which opens that same tab.

## What Escouade keeps, and where

- The API key goes to the system keychain: Credential Manager on Windows, Keychain on macOS. It is stored under the plugin's name, `plugin:dev.escouade.linear:account:linear`, and under the former name `linear`, which a version of Escouade from before plugins reads.
- On Linux, which has no supported keychain, or when the keychain refuses the write, it stays in `~/.escouade/integrations.json`, readable by you alone. The account says so: "System keychain unavailable: the token stays in ~/.escouade/integrations.json."
- The rest of the account (your Linear email and user id) is in `~/.escouade/integrations.json`. The linked team and the status mapping are in Escouade's settings, with the project's.

"Disconnect" forgets the account and its key. Projects keep their linked team, unused until you connect again.
