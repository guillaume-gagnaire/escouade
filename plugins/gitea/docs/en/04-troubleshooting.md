# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, on the account's row in Settings › Connected services, on an agent's card or in its pull request's tab. The plugin calls itself "Gitea" in its messages, for Forgejo too. When the instance gives a reason, it follows the message.

## When connecting

**"Give the instance’s address."**
Gitea / Forgejo has no default instance: give its address (`https://codeberg.org`).

**"Code host type not found: pick it."**
The app did not recognise the instance by its address. Pick Gitea / Forgejo in "Code host".

**"Gitea: give a token."**
The "Token" field is empty. There is no CLI whose token Escouade would take: make one with "Create a token".

**"The address must use https (http only on this machine)."**
The instance's address starts with `http://`. Give its `https://` address.

**"Gitea refuses these credentials (401)"**
The token is wrong or was deleted. Make another one with the scopes the form reminds.

**"Gitea can’t be reached: …"**
The call did not arrive: a mistyped address, no network, a proxy or a firewall, or more than 15 seconds without an answer. Check the address, your connection and Settings › Advanced › Network.

## On the account's row

**"Token expired or revoked · @ada"**, **"Unreachable · @ada"**
The last check was refused, or did not reach the instance. Replace the token, or "Check" to try again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the token. No call leaves while it is so.

**"The plugin Gitea / Forgejo is switched off (Settings › Plugins): this account serves nothing, its PRs are no longer followed"**
You switched the plugin off. Switch it back on in the Plugins view.

**"The plugin Gitea / Forgejo stopped (…): its PRs are no longer followed until it starts again"**
The plugin stopped on its own, the reason in brackets. Its page in the Plugins view says its state; its "Journal" tab restarts it.

## On a pull request

**"Code host stopped: PRs no longer followed"**
On an agent's card: the plugin is switched off or stopped. The card keeps the pull request as last read; the first reading that works clears the message.

**"Gitea: auto-merge exists only on a protected branch, and main is not one"**
The instance only schedules auto-merge on a protected branch. Protect the target branch in the repository's settings, or merge when it is green from Escouade.

**"This code host can’t do it: job_log"**, **"…: rerun_failed"**, **"…: rerun_failed_of_branch"**, **"…: reply"**, **"…: resolve"**
The instance's version lacks the call: a job's log (Gitea before 1.25, Forgejo before 12), rerunning jobs (Gitea before 1.26, any Forgejo), replying in a thread or resolving it (Gitea before 1.26, any Forgejo). See the table in **Use**.

**"Gitea: pull request without commit"**, **"Gitea: branch main without commit"**
The instance gave no last commit for the pull request or the branch, whose CI therefore cannot be read. Try again after a push.

**"Gitea denies access (403)"**
The token lacks the right asked (one of the three scopes is missing), or your account lacks this right on the repository (merge, review).

**"Gitea limits the requests (429): try again in a moment"**
The instance limits the calls. Wait a little.

**"Gitea: not found (404)"**, **"Gitea: error 500"** (or another error of the service), **"Gitea: unreadable answer"**, **"Gitea: too many redirects"**
The repository or the object no longer exists or is not visible, the instance answered with an error, something other than JSON (often a proxy or a login page), or more than ten redirects. Check the instance's address and Settings › Advanced › Network.

**"Gitea / Forgejo: no token for git.acme.dev. Connect the account again in Settings › Connected services."**
Escouade could not lend a token to the plugin. Connect the account again.

**"The plugin dev.escouade.gitea is not answering: …"**
The plugin is switched off, stopped or too slow. Nothing is lost: pull requests are read again once it answers.
