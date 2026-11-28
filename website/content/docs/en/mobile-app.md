# The mobile app

Escouade’s mobile app, for Android and iPhone, shows what is waiting for you on your computer and your servers and lets you answer it. This page covers installing it, pairing it, answering from it and revoking it.

## Installing it

The app comes first as a closed test on Google Play and in TestFlight, then on both stores. It needs Escouade 2.3 or later on the computer or server it reaches, and a phone with a screen lock. The iPad has no layout of its own.

## Pairing it

1. On your computer: Settings › Phone, turn on “Access from the phone”, then “Pair a phone” shows a QR code. On a server: `escouade-server pair --qr`.
2. In the app: “Scan the QR code”, or type the 8-character code beside it with the space’s address.
3. Give this phone a name, then “Pair this phone”.

The QR code lasts 10 minutes, for one phone. The phone then reaches the machine through a WireGuard tunnel built into Escouade: one UDP port open in the router, and the API is served inside the tunnel alone. The keys stay in Android’s Keystore or iOS’s Keychain. Without a screen lock, the app refuses to pair.

Each paired machine is a [space](spaces): “Everywhere” gathers what is waiting in each, and one chip per space filters the thread.

## The three tabs

- **Your turn**: stopped agents (a permission, a question, a plan), oldest first, then the requests of the [“Your turn” inbox](inbox). A line unfolds in place.
- **Agents**: those waiting for your answer, then those working, with what they are doing. An agent opens its conversation, where you write or dictate a message, with a photo.
- **Kanban**: a project’s [Kanban](board), its columns as segments. A ticket in “To review” is approved or sent back to its agent.

The “+” at the top creates an agent, a dictated ticket or a project on a server.

## Answering under the lock

A command to allow reads in full before the button. Every answer that makes an agent act (allowing a command, approving a plan, approving a ticket, giving a go-ahead) goes through the phone’s lock: fingerprint, face or passcode. Canceling changes nothing: the request stays open.

“Always allow”, permission rules and Bypass mode are never set from the phone: they are set on the computer, with the whole context in view.

## Offline

When a space is cut off, its chip says so, a band offers “Try again”, and the app keeps what it said last, dated. Answering waits for the connection: an answer never leaves from an outdated screen. In the background, the app closes its connections and reopens them when you come back.

## Notifications

At pairing, the app offers notifications. Their text is sealed by your computer with the pairing’s key: the publisher’s relay, Google and Apple only see a wake-up. Tapping one reopens what it announces, or says the request was settled in the meantime. Without them, nothing goes through the publisher.

## Revoking a phone

On the computer, Settings › Phone › “Phones”: “Remove” revokes its tunnel, its token and its notification registration at once. The phone says so at its next connection (“This phone is no longer paired with…”) and erases what it knew of that space; the others do not move. From the app, “Your spaces” in the “+” sheet removes a space.

What the app sends, and to whom: [the mobile app’s privacy](https://escouade.dev/en/mobile/privacy/).
