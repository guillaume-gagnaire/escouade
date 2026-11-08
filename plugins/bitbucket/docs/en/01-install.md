# Install

## 1. Have it in Escouade

Open the Plugins view (its icon is in the projects bar), then "Installed". Escouade gives this plugin with the app: depending on your version, it is listed under "Shipped with Escouade" or under "Offered by Escouade" (it is in the installer, nothing to download). If it says "Not installed yet", or if you uninstalled it, "Install" puts it back.

Its page, in the same view, has a switch for the whole app and a "Projects" tab to turn it on or off project by project.

## 2. Make the token on Atlassian

Bitbucket Cloud connects with an Atlassian API token, with chosen scopes, and the email of the Atlassian account it belongs to.

1. In Escouade, open Settings › Connected services › "Code hosts", then "Connect a code host…", and pick Bitbucket in "Code host".
2. "Create a token" opens `https://id.atlassian.com/manage-profile/security/api-tokens`. Make an API token **with scopes**, for Bitbucket, and give it those the form reminds: `read:user:bitbucket`, `read:repository:bitbucket`, `read:pullrequest:bitbucket`, `write:pullrequest:bitbucket`, `read:pipeline:bitbucket`.
3. Copy the token.

The token acts on your behalf: what it opens or publishes is under your account.

## 3. Connect

1. "Address": empty, for bitbucket.org.
2. "Email": your Atlassian account's email.
3. "Token": the token you just made.
4. Click "Connect". Escouade asks Bitbucket who you are. The row says "Connected · @your-name" and "No tickets: PRs and CI only".

A repository whose remote is on bitbucket.org with no account also offers "Connect" followed by its host, in "Code hosts".

Pull requests have nothing to link: each project finds its account through its remote.

## What Escouade keeps, and where

- The token goes to the system keychain: Credential Manager on Windows, Keychain on macOS. On Linux, which has no supported keychain, or when the keychain refuses the write, it stays in `~/.escouade/integrations.json`, readable by you alone, and the row says so.
- The email, the address and your name are in `~/.escouade/integrations.json`.

"Disconnect" forgets the account and its token. With no account, nothing is blocked: approving a ticket pushes its branch and opens the pull request's creation page in your browser, filled in.
