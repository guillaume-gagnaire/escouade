# Troubleshooting

The messages below are the plugin's and Escouade's, as they show: under the connection form, in the import window, or when you click the ⚠ of a card whose sync failed. When ClickUp gives a reason, it follows the message.

## When connecting

**"Give the token."**
The "API key" field is empty. Fill it in; nothing went to ClickUp.

**"ClickUp refuses these credentials (401)"**
The key is wrong, or it was generated again since. Copy your current key from "Create a ClickUp API key".

**"ClickUp can’t be reached: …"**
The call did not arrive: no network, a proxy or a firewall blocking `api.clickup.com`, or more than 15 seconds without an answer. Check your connection and Settings › Advanced › Network.

## When importing and syncing

**"The ClickUp list 901… no longer exists or can’t be reached any more: choose another one in Settings › Projects › Imported tickets"**
The linked list was deleted, or your key, still good, no longer reaches it. Pick another list for the project.

**"The ClickUp list “Sprint 12” has no “review” status: ABC-12 stays where it is; choose another status in Settings › Projects › Imported tickets"**
The status chosen for the column was renamed or removed from the list. The task does not move. Pick one of the list's current statuses in the status mapping. Escouade does not try again: nothing would change.

**"ClickUp limits the requests (429): try again in a moment"**
ClickUp limits the number of calls per minute. The plugin already waited for the end of the limit (one minute at most) and asked once more. A sync that fails this way is tried again on its own.

**"ClickUp denies access (403)"**, **"ClickUp: not found (404)"**
Your key does not reach this task or this space, or the task no longer exists.

**"ClickUp: error 500"** (or another error of the service)
ClickUp answered with an error. A sync is tried again on its own, after 1, 5, 15 then 60 minutes, for 24 hours; "Resync" sends it again at once.

**"ClickUp: unreadable answer"**
ClickUp answered something other than JSON, often a proxy's error page. Try again; if it lasts, look at Settings › Advanced › Network.

**"ClickUp: no token for … Connect the account again in Settings › Connected services."**
Escouade could not lend a key to the plugin: the account has none any more. Disconnect then connect ClickUp again.

**"System keychain unreadable: restart Escouade or reconnect the account."**
At start, the keychain did not give back the key. No call leaves while it is so. Restart Escouade, or connect the account again.

## When the plugin does not answer

**"The plugin dev.escouade.clickup is not answering: …"**
The plugin is switched off (Plugins view, for the app or for this project), stopped after crashes, or too slow. Nothing is lost: the sync goes again once it answers. Look at its page in the Plugins view: its state, and its "Journal" tab.

**"Unreadable answer from the plugin dev.escouade.clickup: …"**
The plugin answered something Escouade does not understand. Restart it from its "Journal" tab; if it comes back, report it.

## A task cannot be found

- Its status is of the done or closed type: it is not offered.
- It is in another list: an id only finds a task in the linked list.
- A search by name reads at most 5 pages of 100 tasks before showing what it found: "Show more" goes on.
