# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Choose a token

Two ways:

- **The `gh` CLI.** If `gh` is installed and logged in (`gh auth login`), leave the token field empty: Escouade asks `gh` for its token (`gh auth token`) when needed, and keeps none. The account then shows "gh token".
- **A personal token.** Make it on GitHub:
  - a "classic" token with the `repo` scope;
  - or a "fine-grained" token on the repositories you want, with write access to Contents, Pull requests and Issues, and read access to Checks, Commit statuses, Actions and Deployments. To run jobs again, Actions must be in write.

The token acts on your behalf: what it opens or publishes is under your account.

## 3. Connect

1. In Escouade, open Settings › Connected services › "Code hosts", then "Connect a code host…". A repository whose remote is on an instance with no account also offers "Connect" followed by its host.
2. "Code host": GitHub.
3. "Address": empty for github.com; otherwise your instance's (`https://acme.ghe.com`, `https://github.acme.dev`). Only https is accepted (http only to this machine).
4. "Token (empty: the gh one)": paste your token, or leave it empty for `gh`'s. The form reminds "Permissions to give the token: repo" and "Create a token" opens the instance's tokens page.
5. Click "Connect". Escouade asks GitHub who you are. The row says "Connected · @your-login". For a classic token, GitHub tells its scopes: if `repo` is missing, the row says "Missing permissions: repo".

The github.com account also serves GitHub Issues: its row says "Also used for GitHub Issues". A GitHub Enterprise serves pull requests and CI only.

You can also connect GitHub Issues from Settings › Connected services › "Ticket sources": its row has a "Token (empty: the one from gh)" field and the "Create a GitHub token" link. It is the same account as github.com's in "Code hosts".

## 4. Link a repository to a project for its issues

In the "Sources linked to `<project>`" group, the "GitHub Issues repository" menu offers your repositories; the project's, when its remote is on github.com, comes first. Pick it, then save the settings. From the Kanban, "Import" with no linked source offers "Link acme/api (GitHub Issues)" directly.

Pull requests have nothing to link: each project finds its account through its remote.

## What Escouade keeps, and where

- Your token goes to the system keychain: Credential Manager on Windows, Keychain on macOS. On Linux, which has no supported keychain, or when the keychain refuses the write, it stays in `~/.escouade/integrations.json`, readable by you alone, and the row says so.
- For an account that goes through `gh`, Escouade keeps no token: it asks `gh` again when needed.
- The rest of the account (the instance's address, your login) is in `~/.escouade/integrations.json`.

"Disconnect" forgets the account and its token. An instance with no account blocks nothing: approving a ticket pushes its branch and opens the pull request's creation page in your browser, filled in.
