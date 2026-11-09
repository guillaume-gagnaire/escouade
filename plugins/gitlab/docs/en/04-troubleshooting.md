# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, on the account's row in Settings › Connected services, on an agent's card or in its MR's tab, or when you click the ⚠ of a card whose sync failed. When GitLab gives a reason, it follows the message.

## When connecting

**"Give the instance’s address."**
The address is empty and the code host has no default instance. For gitlab.com, keep GitLab chosen and the address empty.

**"Code host type not found: pick it."**
The app did not recognise the instance by its address. Pick GitLab in "Code host".

**"GitLab: give a token, or log the glab CLI in to git.acme.dev (glab auth login)"**
The token field is empty and `glab` gave no token for this instance. Run `glab auth login`, or paste a token.

**"The address must use https (http only on this machine)."**
The instance's address starts with `http://`. Give its `https://` address.

**"GitLab refuses these credentials (401)"**
The token is wrong, expired or revoked. Make another one with "Create a token".

**"GitLab can’t be reached: …"**
The call did not arrive: a mistyped address, no network, a proxy or a firewall, or more than 15 seconds without an answer. Check the address, your connection and Settings › Advanced › Network.

## On the account's row

**"Token expired or revoked · @ada"**, **"Unreachable · @ada"**
The last check was refused, or did not reach the instance. Replace the token, or "Check" to try again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the token. No call leaves while it is so.

**"The plugin GitLab is switched off (Settings › Plugins): this account serves nothing, its PRs are no longer followed"**
You switched the plugin off. Switch it back on in the Plugins view.

**"The plugin GitLab stopped (…): its PRs are no longer followed until it starts again"**
The plugin stopped on its own, the reason in brackets. Its page in the Plugins view says its state; its "Journal" tab restarts it.

## On a merge request

**"Code host stopped: MRs no longer followed"**
On an agent's card: the plugin is switched off or stopped. The card keeps the MR as last read; the first reading that works clears the message.

**"GitLab: auto-merge was not enabled"**
GitLab neither scheduled the merge nor merged the MR: the project does not allow it, or the MR is not in a state for it (conflict, draft, no pipeline). Look at the MR in GitLab.

**"This code host can’t do it: rebase merge"**, **"This code host can’t do it: semi-linear merge"**
GitLab merges by merge commit or squash; rebasing before the merge is a setting of the project. Pick merge or squash.

**"GitLab: merge request !12 not found"**
The MR does not exist in this project, or your token does not see it.

**"GitLab denies access (403)"**
Your account lacks the right asked in this project (merge, rerun a job, approve), or the token lacks the `api` scope.

**"GitLab limits the requests (429): try again in a moment"**
GitLab refused a call for its limit, twice in a row. The plugin slows down on its own; wait a little.

**"GitLab: not found (404)"**, **"GitLab: error 500"** (or another error of the service), **"GitLab: unreadable answer"**, **"GitLab: too many redirects"**
The project or the object no longer exists or is not visible, the instance answered with an error, something other than JSON (often a proxy or a login page), or more than ten redirects. Check the instance's address and Settings › Advanced › Network.

**"GitLab: no token for git.acme.dev. Connect the account again in Settings › Connected services."**
Escouade could not lend a token to the plugin. Connect the account again.

## For GitLab Issues

**"No GitLab account connected for git.acme.dev"**
The linked project is on an instance whose account was disconnected. Connect the instance again in "Code hosts".

**"The plugin dev.escouade.gitlab is not answering: …"**
An issue's sync got no answer: the plugin is switched off, stopped or too slow. Nothing is lost: it goes again once it answers.

**"Unreadable answer from the plugin dev.escouade.gitlab: …"**
The plugin answered something Escouade does not understand. Restart it from its "Journal" tab; if it comes back, report it.

The other messages are those above. A sync that fails is tried again on its own, after 1, 5, 15 then 60 minutes, for 24 hours; "Resync" sends it again at once.
