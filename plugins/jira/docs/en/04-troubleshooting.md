# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, in the import window, or when you click the ⚠ of a card whose sync failed. When Jira gives a reason, it follows the message.

## When connecting

**"Give the site’s address."**, **"Give the email."**, **"Give the token."**
A field is empty. Fill it in; nothing went to Jira.

**"The Jira site must use https (http only on this machine)."**
The site's address starts with `http://`. Type it without a scheme (`atlas.atlassian.net`) or with `https://`.

**"Jira refuses these credentials (401)"**
The token is wrong or revoked, or the email is not that of the token's Atlassian account. Check the email, and make a new token with "Create an Atlassian API token".

**"Jira can’t be reached: …"**
The call did not arrive: a mistyped site, no network, a proxy or a firewall blocking the site, or more than 15 seconds without an answer. Check the site's address, your connection and Settings › Advanced › Network.

## When importing and syncing

**"Jira: no transition from “In Progress” to “Done” for ATL-1294"**
The issue's workflow does not lead from its current status to the status chosen for the column. Escouade forces nothing and does not try again. Pick in the status mapping a status the workflow reaches from there, or add the transition in Jira, then "Resync".

**"Jira denies access (403)"**
Your account may not do what is asked in this project (read, take a transition, comment). Check its permissions in Jira.

**"Jira: not found (404)"**
The project or the issue no longer exists, or your account no longer sees it. For a project, pick another one in Settings › Projects › Imported tickets.

**"Jira limits the requests (429): try again in a moment"**
Jira limits the number of calls. A sync that fails this way is tried again on its own.

**"Jira: error 500"** (or another error of the service)
Jira answered with an error. A sync is tried again on its own, after 1, 5, 15 then 60 minutes, for 24 hours; "Resync" sends it again at once.

**"Jira: unreadable answer"**
Jira answered something other than JSON, often a proxy's error page or a login page. Check the site's address and Settings › Advanced › Network.

**"Jira: this account has no site. Connect it again in Settings › Connected services."**
The kept account has no site address. Disconnect then connect Jira again with its site.

**"Jira: no token for jira. Connect the account again in Settings › Connected services."**
Escouade could not lend a token to the plugin: the account has none any more. Disconnect then connect Jira again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the token. No call leaves while it is so. Restart Escouade, or connect the account again.

## When the plugin does not answer

**"The plugin dev.escouade.jira is not answering: …"**
The plugin is switched off (Plugins view, for the app or for this project), stopped after crashes, or too slow. Nothing is lost: the sync goes again once it answers. Look at its page in the Plugins view: its state, and its "Journal" tab.

**"Unreadable answer from the plugin dev.escouade.jira: …"**
The plugin answered something Escouade does not understand. Restart it from its "Journal" tab; if it comes back, report it.

## An issue cannot be found

- It is in the "Done" category: it is not offered.
- A chip hides it: "Active sprint" and "To do" add up with the search.
- It is in another project: another project's key finds nothing in the linked project.
