# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, on the account's row in Settings › Connected services, on an agent's card or in its pull request's tab, or when you click the ⚠ of a card whose sync failed. When GitHub gives a reason, it follows the message.

## When connecting

**"GitHub: give a token, or log the gh CLI in to github.com (gh auth login)"**
The token field is empty and `gh` gave no token: it is not installed, or not logged in to this instance. Run `gh auth login` (with `--hostname` for an Enterprise), or paste a token.

**"The address must use https (http only on this machine)."**
The instance's address starts with `http://`. Give its `https://` address.

**"GitHub refuses these credentials (401)"**
The token is wrong, expired or revoked. Make another one with "Create a token".

**"GitHub can’t be reached: …"**
The call did not arrive: a mistyped address, no network, a proxy or a firewall, or more than 15 seconds without an answer. Check the address, your connection and Settings › Advanced › Network.

## On the account's row

**"Missing permissions: repo"**
A classic token without the `repo` scope: it can neither read private repositories nor open pull requests. Make a token with `repo`.

**"Token expired or revoked · @ada"**
GitHub refused the token at the last check. Replace it, or log `gh` in again (`gh auth login`).

**"Unreachable · @ada"**
The last check did not reach the instance. "Check" runs it again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the token. No call leaves while it is so.

**"The plugin GitHub is switched off (Settings › Plugins): this account serves nothing, its PRs are no longer followed"**
You switched the plugin off. Switch it back on in the Plugins view.

**"The plugin GitHub stopped (…): its PRs are no longer followed until it starts again"**
The plugin stopped on its own, the reason in brackets. Its page in the Plugins view says its state; its "Journal" tab restarts it.

## On a pull request

**"Code host stopped: PRs no longer followed"**
On an agent's card: the plugin is switched off or stopped. The card keeps the pull request as last read; the first reading that works clears the message.

**"GitHub: auto-merge was not enabled"**
GitHub did not take the auto-merge: the repository does not allow it ("Allow auto-merge" off), or the pull request is not in a state for it. Turn it on in the repository's settings, or merge by hand.

**"This code host can’t do it: semi-linear merge"**
GitHub has no semi-linear merge. Pick merge, squash or rebase.

**"GitHub: pull request #42 not found"**
The pull request does not exist in this repository, or your token does not see it.

**"GitHub: pull request without commit"**
GitHub gave no last commit for the pull request, whose CI therefore cannot be read. Try again after a push.

**"GitHub (GraphQL) : …"**
GitHub's GraphQL API refused the request; its reason follows. Often a token that does not see the repository.

**"GitHub denies access (403)"**
The token lacks the right asked on this repository (a fine-grained token without this repository or this permission, an organization that requires the token's SSO authorization), or GitHub limits the requests. Check the token's permissions.

**"GitHub limits the requests (429): try again in a moment"**
The API's limit is reached. The plugin slows down on its own; wait for the end of the window.

**"GitHub: not found (404)"**, **"GitHub: error 500"** (or another error of the service), **"GitHub: unreadable answer"**, **"GitHub: too many redirects"**
The repository or the object no longer exists or is not visible, GitHub answered with an error, something other than JSON (often a proxy), or more than ten redirects. Try again; if it lasts, look at Settings › Advanced › Network.

**"GitHub: no token for github.com. Connect the account again in Settings › Connected services."**
Escouade could not lend a token to the plugin. Connect the account again.

## For GitHub Issues

**"The plugin dev.escouade.github is not answering: …"**
An issue's sync got no answer: the plugin is switched off, stopped or too slow. Nothing is lost: it goes again once it answers.

**"Unreadable answer from the plugin dev.escouade.github: …"**
The plugin answered something Escouade does not understand. Restart it from its "Journal" tab; if it comes back, report it.

The other messages are those above ("GitHub refuses these credentials (401)"…). A sync that fails is tried again on its own, after 1, 5, 15 then 60 minutes, for 24 hours; "Resync" sends it again at once.

## A job's log does not show

On a GitHub Enterprise Server that stores its Actions logs on a host other than itself, the plugin cannot read them: that host is not in its permissions.
