# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Make the API key on ClickUp

1. In Escouade, open Settings › Connected services. Under "Ticket sources", in the "Connected accounts" group, click "Connect…" on the ClickUp row.
2. Click "Create a ClickUp API key". Your browser opens `https://app.clickup.com/settings/apps`, the "Apps" page of your ClickUp settings. Generate your personal API key (it starts with `pk_`) and copy it.
3. Paste it into the "API key" field.

The key acts on your behalf on every workspace your account can reach.

Connecting a ClickUp app through OAuth is not offered: the personal key is enough.

## 3. Connect

Click "Connect". The button says "Checking…" while Escouade asks ClickUp who you are. If ClickUp accepts, the row says "Connected · " followed by your ClickUp name (your email if there is none). If ClickUp refuses, its answer shows in red under the form (see **Troubleshooting**). An empty field is reported before any call: "Give the token.".

The connection is kept at once: it does not go through "Save".

## 4. Link a list to a project

In Settings › Projects › Imported tickets, the "Sources linked to `<project>`" group offers the "ClickUp list" menu. It sorts the lists of all your workspaces by space and folder ("Product › Sprints"), as ClickUp's sidebar does: lists outside a folder first, archived ones left out. The workspace's name comes first when you have several. With the menu open, typing the first letters of a name goes to it.

Pick a list. Escouade loads its statuses and fills in the "Status mapping" (see **Use**). Save the settings.

You can also start from the Kanban: "Import" with no linked source shows "No source linked to this project" and offers "Link a source", which opens that same tab.

## What Escouade keeps, and where

- The API key goes to the system keychain: Credential Manager on Windows, Keychain on macOS. It is stored under the plugin's name, `plugin:dev.escouade.clickup:account:clickup`, and under the former name `clickup`, which a version of Escouade from before plugins reads.
- On Linux, which has no supported keychain, or when the keychain refuses the write, it stays in `~/.escouade/integrations.json`, readable by you alone. The account says so: "System keychain unavailable: the token stays in ~/.escouade/integrations.json."
- The rest of the account (your ClickUp name and user id) is in `~/.escouade/integrations.json`. The linked list and the status mapping are in Escouade's settings, with the project's.

"Disconnect" forgets the account and its key. Projects keep their linked list, unused until you connect again.
