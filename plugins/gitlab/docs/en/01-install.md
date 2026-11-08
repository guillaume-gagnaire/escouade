# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Choose a token

Two ways:

- **The `glab` CLI.** If `glab` is installed and logged in to the instance (`glab auth login`), leave the token field empty: Escouade asks `glab` for its token when needed. The account then shows "glab token".
- **A personal access token**, made in your GitLab account's settings, with the `api` scope. "Create a token", in the form, opens the tokens page of the instance typed in "Address" (gitlab.com when it is empty), with the name "Escouade" and the `api` scope already chosen.

The token acts on your behalf: what it opens or publishes is under your account.

## 3. Connect

1. In Escouade, open Settings › Connected services › "Code hosts", then "Connect a code host…". A repository whose remote is on an instance with no account also offers "Connect" followed by its host.
2. "Code host": GitLab. For an address the app does not know, it finds the code host type on its own ("Finding out the code host type…") by asking the instance for `/api/v4/version`.
3. "Address": empty for gitlab.com; otherwise your instance's, path included if it has one (`https://acme.dev/gitlab`). Only https is accepted (http only to this machine).
4. "Token (empty: the glab one)": paste your token, or leave it empty for `glab`'s. The form reminds "Permissions to give the token: api".
5. Click "Connect". Escouade asks GitLab who you are. The row says "Connected · @your-login".

Each GitLab account also serves GitLab Issues: its row says "Also used for GitLab Issues". Under "Ticket sources", GitLab Issues has no account of its own: its row says "The GitLab code host’s account: it is connected and disconnected in Code hosts." and offers "Connect GitLab…".

## 4. Link a GitLab project for its issues

In the "Sources linked to `<project>`" group, the "GitLab Issues project" menu offers your projects, sorted by instance, with their groups and subgroups in the path ("acme/back/api"), the most recently active first; the project's, when its remote is on a connected instance, comes first. Pick it, then save the settings. From the Kanban, "Import" with no linked source offers "Link acme/api (GitLab Issues)" directly.

Merge requests have nothing to link: each project finds its account through its remote.

## What Escouade keeps, and where

- Your token goes to the system keychain: Credential Manager on Windows, Keychain on macOS. On Linux, which has no supported keychain, or when the keychain refuses the write, it stays in `~/.escouade/integrations.json`, readable by you alone, and the row says so.
- For an account that goes through `glab`, Escouade asks `glab` for its token again when needed.
- The rest of the account (the instance's address, your login) is in `~/.escouade/integrations.json`.

"Disconnect" forgets the account and its token. An instance with no account blocks nothing: approving a ticket pushes its branch and opens the merge request's creation page in your browser, filled in.
