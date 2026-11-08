# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Make the token on the instance

1. In Escouade, open Settings › Connected services › "Code hosts", then "Connect a code host…". A repository whose remote is on an instance with no account also offers "Connect" followed by its host.
2. "Code host": Gitea / Forgejo. For an address the app does not know, it finds the code host type on its own ("Finding out the code host type…").
3. "Address": the instance's (`https://codeberg.org`, `https://gitea.com`, `https://git.acme.dev`). Only https is accepted (http only to this machine).
4. "Create a token" opens `<instance>/user/settings/applications`. Make an access token with the scopes the form reminds: `write:repository`, `write:issue`, `read:user`. Copy it.

The token acts on your behalf: what it opens or publishes is under your account.

## 3. Connect

Paste the token into "Token" and click "Connect". Escouade asks the instance who you are. The row says "Connected · @your-login" and "No tickets: PRs and CI only".

Gitea and Forgejo have no CLI whose token Escouade would take: the field must be filled in.

Pull requests have nothing to link: each project finds its account through its remote.

## What Escouade keeps, and where

- The token goes to the system keychain: Credential Manager on Windows, Keychain on macOS. On Linux, which has no supported keychain, or when the keychain refuses the write, it stays in `~/.escouade/integrations.json`, readable by you alone, and the row says so.
- The instance's address and your login are in `~/.escouade/integrations.json`.

"Disconnect" forgets the account and its token. With no account, nothing is blocked: approving a ticket pushes its branch and opens the pull request's creation page in your browser, filled in.
