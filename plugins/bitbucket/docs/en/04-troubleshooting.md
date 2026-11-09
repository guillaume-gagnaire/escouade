# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, on the account's row in Settings › Connected services, on an agent's card or in its pull request's tab. When Bitbucket gives a reason, it follows the message.

## When connecting

**"Give the email."**
Bitbucket connects with the email of the token's Atlassian account. Fill it in.

**"Bitbucket: give a token."**
The "Token" field is empty. Bitbucket has no CLI whose token Escouade would take: make one with "Create a token".

**"Bitbucket refuses these credentials (401)"**
The token is wrong, expired or revoked, or the email is not that of its Atlassian account, or it is a token without scopes. Make an API token with the Bitbucket scopes the form reminds.

**"Bitbucket can’t be reached: …"**
The call did not arrive: no network, a proxy or a firewall blocking `api.bitbucket.org`, or more than 15 seconds without an answer. Check your connection and Settings › Advanced › Network.

## On the account's row

**"Token expired or revoked · @ada"**, **"Unreachable · @ada"**
The last check was refused, or did not reach Bitbucket. Replace the token, or "Check" to try again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the token. No call leaves while it is so.

**"The plugin Bitbucket is switched off (Settings › Plugins): this account serves nothing, its PRs are no longer followed"**
You switched the plugin off. Switch it back on in the Plugins view.

**"The plugin Bitbucket stopped (…): its PRs are no longer followed until it starts again"**
The plugin stopped on its own, the reason in brackets. Its page in the Plugins view says its state; its "Journal" tab restarts it.

## On a pull request

**"Code host stopped: PRs no longer followed"**
On an agent's card: the plugin is switched off or stopped. The card keeps the pull request as last read; the first reading that works clears the message.

**"Bitbucket: the merge failed (…)"**
Bitbucket merged in the background and its task failed, its state in brackets. Look at the pull request in Bitbucket (conflict, branch rule).

**"Bitbucket: the merge is not done yet"**
Bitbucket is still merging in the background after about thirty seconds. The merge may succeed on its own: the pull request will say so at the next reading.

**"Bitbucket: pull request #42 not found"**
The pull request does not exist in this repository, or your token does not see it.

**"Bitbucket: unknown comment (…)"**
The reply or the resolution aims at a comment Bitbucket no longer knows (deleted).

**"This code host can’t do it: …"**
What is asked does not exist on Bitbucket in this plugin (auto-merge, rerunning jobs). Escouade usually knows it beforehand and does not offer it; this message says so if it happens.

**"Bitbucket denies access (403)"**
Your token lacks the scope asked (one missing among the form's), or your account lacks this right on the repository (merge, approve).

**"Bitbucket limits the requests (429): try again in a moment"**
The hour's budget is spent: the plugin already waited and asked once more. Wait a little.

**"Bitbucket: not found (404)"**, **"Bitbucket: error 500"** (or another error of the service), **"Bitbucket: unreadable answer"**, **"Bitbucket: too many redirects"**
The repository or the object no longer exists or is not visible, Bitbucket answered with an error, something other than JSON (often a proxy), or more than ten redirects. Try again; if it lasts, look at Settings › Advanced › Network.

**"Bitbucket: no token for bitbucket.org. Connect the account again in Settings › Connected services."**
Escouade could not lend a token to the plugin. Connect the account again.

**"The plugin dev.escouade.bitbucket is not answering: …"**
The plugin is switched off, stopped or too slow. Nothing is lost: pull requests are read again once it answers.
