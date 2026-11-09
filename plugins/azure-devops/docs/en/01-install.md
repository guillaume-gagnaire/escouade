# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Choose a token

Two ways:

- **The `az` CLI**, on dev.azure.com only. If `az` is installed and logged in (`az login`), leave the token field empty: Escouade asks `az` for an Entra ID token and asks again before it expires. An Azure DevOps Server does not know those tokens: it needs a personal token.
- **A personal access token (PAT)**, made on your organization's tokens page (`<organization>/_usersSettings/tokens`, which "Create a token" opens), with these scopes:
  - Code: read and write;
  - Build: read and execute;
  - Work Items: read and write, for Azure Boards.

Azure does not tell a token's scopes: Escouade cannot check them when connecting. A token without Work Items shows at Azure Boards' first refusal, and the message says where to make another one.

The token acts on your behalf: what it opens or publishes is under your account.

## 3. Connect

1. In Escouade, open Settings › Connected services › "Code hosts", then "Connect a code host…". A repository whose remote is on an organization with no account also offers "Connect" followed by its host.
2. "Code host": Azure DevOps.
3. "Address": your organization's, `dev.azure.com/<organization>` (a former `<organization>.visualstudio.com` address is the same), or for Azure DevOps Server its collection's, such as `https://server/tfs/DefaultCollection`. Only https is accepted (http only to this machine).
4. "Token (empty: the az one)": paste your token, or leave it empty for `az`'s. The form reminds the scopes: `vso.code_write`, `vso.build_execute`, `vso.work_write`.
5. Click "Connect". The row says "Connected · " followed by your account and the organization ("ada@acme.dev · acme"), and "Also used for Azure Boards".

Under "Ticket sources", Azure Boards has no account of its own: its row says "The Azure DevOps code host’s account: it is connected and disconnected in Code hosts.", reminds "Its token needs the Work Items right (read and write)." and offers "Connect Azure DevOps…".

## 4. Link an Azure Boards project

In the "Sources linked to `<project>`" group, the "Azure Boards project" menu offers your organizations' projects, sorted by organization; the project's, when its remote is on the connected organization, comes first. Pick it: Escouade loads the states of its work item types and fills in the status mapping. Save the settings. From the Kanban, "Import" with no linked source offers "Link … (Azure Boards)" directly.

Pull requests have nothing to link: each project finds its account through its remote, in all its forms (`https://dev.azure.com/<organization>/<project>/_git/<repository>`, `<organization>.visualstudio.com`, ssh `git@ssh.dev.azure.com:v3/…`, and a Server's).

## What Escouade keeps, and where

- Your personal token goes to the system keychain: Credential Manager on Windows, Keychain on macOS. On Linux, which has no supported keychain, or when the keychain refuses the write, it stays in `~/.escouade/integrations.json`, readable by you alone, and the row says so.
- For an account that goes through `az`, Escouade asks `az` for its token again when needed.
- The rest of the account (the organization's address, your identity) is in `~/.escouade/integrations.json`.

"Disconnect" forgets the account and its token. An organization with no account blocks nothing: approving a ticket pushes its branch and opens the pull request's creation page in your browser, filled in.
