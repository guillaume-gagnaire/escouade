# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Make the key and the token on Trello

Trello asks for two things: an API key, then a token given to that key.

1. In Escouade, open Settings › Connected services. Under "Ticket sources", in the "Connected accounts" group, click "Connect…" on the Trello row.
2. Click "Get an API key". Your browser opens `https://trello.com/power-ups/admin`. Make a Power-Up if you have none (its name does not matter), then generate its API key. Paste it into the "API key" field.
3. Click "Get a token for this key" (the link stays greyed out while the key is empty). Trello opens its authorization page in the name of "Escouade": read and write access to your account, with no expiry. Accept, then paste the token it shows into the "Token" field.

The token acts on your behalf on every board your account can reach. You can revoke it at any time from your Trello account settings.

## 3. Connect

Click "Connect". The button says "Checking…" while Escouade asks Trello who you are. If Trello accepts, the row says "Connected · @your-username". If Trello refuses, its answer shows in red under the form (see **Troubleshooting**). An empty field is reported before any call: "Give the API key." or "Give the token.".

The connection is kept at once: it does not go through "Save".

## 4. Link a board to a project

In Settings › Projects › Imported tickets, the "Sources linked to `<project>`" group offers the "Trello board" list: your open boards. Pick one. Escouade then loads its lists and fills in the "Status mapping" (see **Use**). Save the settings.

You can also start from the Kanban: "Import" with no linked source shows "No source linked to this project" and offers "Link a source", which opens that same tab.

## What Escouade keeps, and where

- The key and the token go to the system keychain: Credential Manager on Windows, Keychain on macOS. They are stored under the plugin's name, `plugin:dev.escouade.trello:account:trello`, and under the former name `trello`, which a version of Escouade from before plugins reads.
- On Linux, which has no supported keychain, or when the keychain refuses the write, they stay in `~/.escouade/integrations.json`, readable by you alone. The account says so: "System keychain unavailable: the token stays in ~/.escouade/integrations.json."
- The rest of the account (your Trello username) is in `~/.escouade/integrations.json`. The linked board and the status mapping are in Escouade's settings, with the project's.

"Disconnect" forgets the account and its secrets. Projects keep their linked board, unused until you connect again.
