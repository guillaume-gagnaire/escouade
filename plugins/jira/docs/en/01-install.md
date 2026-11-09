# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Make the API token on Atlassian

1. In Escouade, open Settings › Connected services. Under "Ticket sources", in the "Connected accounts" group, click "Connect…" on the Jira row.
2. Click "Create an Atlassian API token". Your browser opens `https://id.atlassian.com/manage-profile/security/api-tokens`. Make an API token and paste it into the "API token" field.

The token acts on your behalf, with your account's rights on the site: it sees the projects and issues you see.

## 3. Fill in the form

| Field | What to give |
| --- | --- |
| "Site" | The address of your Jira Cloud site, for example `atlas.atlassian.net`. Without `https://`, Escouade adds it. Only https is accepted (http only to this machine). |
| "Email" | The email of the Atlassian account the token belongs to, for example `ada@atlas.dev`. |
| "API token" | The token you just made. |

## 4. Connect

Click "Connect". The button says "Checking…" while Escouade asks Jira who you are. If Jira accepts, the row says "Connected · ada@atlas.dev · atlas.atlassian.net". If Jira refuses, its answer shows in red under the form (see **Troubleshooting**). An empty field is reported before any call: "Give the site’s address.", "Give the email." or "Give the token.".

The connection is kept at once: it does not go through "Save".

## 5. Link a Jira project to an Escouade project

In Settings › Projects › Imported tickets, the "Sources linked to `<project>`" group offers the "Jira project" menu: the projects of your site, sorted by key, with their name. Pick one. Escouade loads its statuses and fills in the "Status mapping" (see **Use**). Save the settings.

You can also start from the Kanban: "Import" with no linked source shows "No source linked to this project" and offers "Link a source", which opens that same tab.

## What Escouade keeps, and where

- The token goes to the system keychain: Credential Manager on Windows, Keychain on macOS. It is stored under the plugin's name, `plugin:dev.escouade.jira:account:jira`, and under the former name `jira`, which a version of Escouade from before plugins reads.
- On Linux, which has no supported keychain, or when the keychain refuses the write, it stays in `~/.escouade/integrations.json`, readable by you alone. The account says so: "System keychain unavailable: the token stays in ~/.escouade/integrations.json."
- The site, the email and your Jira user id are in `~/.escouade/integrations.json`. The linked project and the status mapping are in Escouade's settings, with the project's.

"Disconnect" forgets the account and its token. Projects keep their linked Jira project, unused until you connect again.
