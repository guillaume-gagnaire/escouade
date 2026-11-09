# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, in the import window, or when you click the ⚠ of a card whose sync failed. When Trello gives a reason, it follows the message.

## When connecting

**"Give the API key."**, **"Give the token."**
A field is empty. Fill it in; nothing went to Trello.

**"Trello refuses these credentials (401)"**
The key or the token is wrong, or the token was revoked, or it was made for another key. Make the token again with "Get a token for this key", after pasting the right key.

**"Trello can’t be reached: …"**
The call did not arrive: no network, a proxy or a firewall blocking `api.trello.com`, or more than 15 seconds without an answer. Check your connection and Settings › Advanced › Network.

## When importing and syncing

**"Trello denies access (403)"**
Your account no longer reaches the board or the card. Check on Trello that you are still a member, or link another board.

**"Trello: not found (404)"**
The board, the list or the card no longer exists (deleted, or moved out of your reach). For a board, pick another one in Settings › Projects › Imported tickets; for a list, change the status mapping.

**"Trello limits the requests (429): try again in a moment"**
Trello limits the number of calls per key and token. Wait a little: a sync that fails this way is tried again on its own.

**"Trello: error 500"** (or another error of the service)
Trello answered with an error. A sync is tried again on its own, after 1, 5, 15 then 60 minutes, for 24 hours; "Resync" sends it again at once.

**"Trello: unreadable answer"**
Trello answered something other than JSON, often a proxy's error page. Try again; if it lasts, look at Settings › Advanced › Network.

**"Trello: no token for … Connect the account again in Settings › Connected services."**
Escouade could not lend a token to the plugin: the account has none any more. Disconnect then connect Trello again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the key and the token. No call leaves while it is so. Restart Escouade, or connect the account again.

## When the plugin does not answer

**"The plugin dev.escouade.trello is not answering: …"**
The plugin is switched off (Plugins view, for the app or for this project), stopped after crashes, or too slow. Nothing is lost: the sync goes again once it answers. Look at its page in the Plugins view: its state, and its "Journal" tab.

**"Unreadable answer from the plugin dev.escouade.trello: …"**
The plugin answered something Escouade does not understand. Restart it from its "Journal" tab; if it comes back, report it.

## Nothing moves on Trello

- The column has no list in the status mapping ("unchanged"): the card stays where it is.
- "Update the external status" is off in the "Sync" group.
- The ticket's card has a ⚠: click it to read the reason, then "Resync".
