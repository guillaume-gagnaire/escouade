# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, in the import window, or when you click the ⚠ of a card whose sync failed. When Linear gives a reason, it follows the message.

Linear answers with error codes of its own; the plugin reads them as the HTTP errors they amount to: a refused key as a 401, a refused access as a 403, a limit as a 429, any other refused request as a 400.

## When connecting

**"Give the token."**
The "API key" field is empty. Fill it in; nothing went to Linear.

**"Linear refuses these credentials (401)"**
The key is wrong or was revoked. Make a new key with "Create a Linear API key".

**"Linear can’t be reached: …"**
The call did not arrive: no network, a proxy or a firewall blocking `api.linear.app`, or more than 15 seconds without an answer. Check your connection and Settings › Advanced › Network.

## When importing and syncing

**"Linear refuses to move ENG-42 to “In Review”"**
Linear did not accept the change of state. Check that the state still exists in the team, and pick another one in the status mapping. Escouade does not try again: nothing would change.

**"Linear refuses the comment on ENG-42"**
Linear did not accept the comment. Check that the issue still exists and that your key may write on it.

**"Linear denies access (403)"**
Your key may not read or write what is asked, or you are no longer in the team.

**"Linear limits the requests (429): try again in a moment"**
Linear limits the number of calls per key. A sync that fails this way is tried again on its own.

**"Linear: error 400"**
Linear refused the request as it is. Escouade sets it aside without trying again; "Resync" sends it again once you have fixed what Linear says.

**"Linear: error 500"** (or another error of the service)
Linear answered with an error. A sync is tried again on its own, after 1, 5, 15 then 60 minutes, for 24 hours; "Resync" sends it again at once.

**"Linear: unreadable answer"**
Linear answered with no data, or something other than JSON, often a proxy's error page. Try again; if it lasts, look at Settings › Advanced › Network.

**"Linear: no token for … Connect the account again in Settings › Connected services."**
Escouade could not lend a key to the plugin: the account has none any more. Disconnect then connect Linear again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the key. No call leaves while it is so. Restart Escouade, or connect the account again.

## When the plugin does not answer

**"The plugin dev.escouade.linear is not answering: …"**
The plugin is switched off (Plugins view, for the app or for this project), stopped after crashes, or too slow. Nothing is lost: the sync goes again once it answers. Look at its page in the Plugins view: its state, and its "Journal" tab.

**"Unreadable answer from the plugin dev.escouade.linear: …"**
The plugin answered something Escouade does not understand. Restart it from its "Journal" tab; if it comes back, report it.

## An issue cannot be found

- It is completed or canceled: it is not offered.
- It is in another team: a number only finds an issue in the linked team, even if you type another team's key (`OPS-42` looks for issue 42 of the linked team).
