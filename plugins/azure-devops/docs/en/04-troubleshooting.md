# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, on the account's row in Settings › Connected services, on an agent's card or in its pull request's tab, or when you click the ⚠ of a card whose sync failed. When Azure gives a reason, it follows the message.

## When connecting

**"Give the instance’s address."**
Give your organization's address (`dev.azure.com/acme`) or your collection's.

**"Azure DevOps: give a token, or log the az CLI in to dev.azure.com (az login)"**
The token field is empty and `az` gave no token: it is not installed, or not logged in. Run `az login`, or paste a personal token.

**"Azure DevOps: give a token."**
The token field is empty for an Azure DevOps Server, which does not know `az`'s tokens. Paste a personal token.

**"The address must use https (http only on this machine)."**
The address starts with `http://`. Give its `https://` address.

**"Azure DevOps refuses these credentials (401)"**
The token is wrong, expired or revoked, or it does not reach this organization. Azure also answers a refused token with a sign-in page: the plugin reads it as that refusal. Make a token with "Create a token".

**"Azure DevOps can’t be reached: …"**
The call did not arrive: a mistyped address, no network, a proxy or a firewall, or more than 15 seconds without an answer. Check the address, your connection and Settings › Advanced › Network.

## On the account's row

**"Token expired or revoked · ada@acme.dev"**, **"Unreachable · ada@acme.dev"**
The last check was refused, or did not reach the organization. Replace the token (or `az login`), or "Check" to try again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the token. No call leaves while it is so.

**"The plugin Azure DevOps is switched off (Settings › Plugins): this account serves nothing, its PRs are no longer followed"**
You switched the plugin off. Switch it back on in the Plugins view.

**"The plugin Azure DevOps stopped (…): its PRs are no longer followed until it starts again"**
The plugin stopped on its own, the reason in brackets. Its page in the Plugins view says its state; its "Journal" tab restarts it.

## On a pull request

**"Code host stopped: PRs no longer followed"**
On an agent's card: the plugin is switched off or stopped. The card keeps the pull request as last read; the first reading that works clears the message.

**"Azure DevOps: the target branch doesn’t allow this merge (…); it allows: …"**
The target's branch policy does not allow the strategy asked; the message says those it allows. Pick one. Escouade does not try again: nothing would change.

**"Azure DevOps: auto-complete was not set"**
Azure did not record the auto-complete. Look at the pull request in Azure DevOps (draft, rights on the branch).

**"Azure DevOps: this build has no log"**
The build has no log to read (yet): it did not start, or was cancelled before.

**"Azure DevOps: unknown comment (…)"**
The reply or the resolution aims at a comment Azure no longer knows.

**"Azure DevOps denies access (403)"**
The token lacks the scope asked (Code in write to open or merge, Build in execute to retry), or your account lacks this right on the repository.

**"Azure DevOps limits the requests (429): try again in a moment"**
Azure refused a call for its limit, twice in a row. The plugin slows down on its own; wait a little.

**"Azure DevOps: not found (404)"**, **"Azure DevOps: error 500"** (or another error of the service), **"Azure DevOps: unreadable answer"**, **"Azure DevOps: too many redirects"**
The repository or the object no longer exists or is not visible, Azure answered with an error, something other than JSON, or more than ten redirects. Try again; if it lasts, look at Settings › Advanced › Network.

**"Azure DevOps: no token for dev.azure.com. Connect the account again in Settings › Connected services."**
Escouade could not lend a token to the plugin. Connect the account again.

## For Azure Boards

**"Azure DevOps refuses these credentials (401). The Azure DevOps token needs the Work Items right (read and write): make one on https://dev.azure.com/acme/_usersSettings/tokens, then replace it in Settings › Connected services › Code hosts."**
The token serves pull requests but lacks the Work Items scope. A 403 refusal says the same. Make a token with Work Items in read and write on the page the message names, then replace the old one.

**"No Azure DevOps account connected for the acme organization"**, **"No Azure DevOps account connected"**
The linked project is in an organization whose account was disconnected. Connect the organization again in "Code hosts".

**"Azure Boards: no token for … Connect the account again in Settings › Connected services."**
Escouade could not lend a token to the plugin. Connect the account again.

**"The plugin dev.escouade.azure-devops is not answering: …"**
An item's sync got no answer: the plugin is switched off, stopped or too slow. Nothing is lost: it goes again once it answers.

**"Unreadable answer from the plugin dev.escouade.azure-devops: …"**
The plugin answered something Escouade does not understand. Restart it from its "Journal" tab; if it comes back, report it.

A sync that fails is tried again on its own, after 1, 5, 15 then 60 minutes, for 24 hours; "Resync" sends it again at once.
