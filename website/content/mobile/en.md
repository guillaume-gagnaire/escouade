# The mobile app

Escouade on your phone, for Android and iPhone: what is waiting for your answer on your computer and your servers, in one thread, and what you need to unblock an agent in seconds, with no cloud in between.

![The “Your turn” thread: stopped agents, then the inbox](/images/en/mobile-thread.jpg)

## What it does

- **What is waiting, everywhere.** Stopped agents first (a permission, a question, a plan), then the “Your turn” inbox (a choice between renders, a go-ahead, an answer, a ticket to approve). Each request shows its space, its project and its agent.
- **Answer, knowing what you accept.** A command reads in full before you allow it. Every answer that makes an agent act goes through your phone’s lock: fingerprint, Face ID or passcode. “Always allow” is never set from the phone.
- **Talk to your agents.** A typed or dictated message, with a photo; their conversation; a project’s Kanban, where you approve a ticket in “To review” or send it back.
- **Create.** An agent and its first message, a dictated ticket, a project on a server.
- **Get notified.** A notification when an agent is waiting for you, end-to-end encrypted: the relay only sees a wake-up, never the text.

![A permission unfolded, the whole command](/images/en/mobile-authorization.jpg)

## How it connects

The phone pairs with your computer or an Escouade server through a QR code, then talks to it directly through an encrypted WireGuard tunnel. No Escouade account, no data with us: the keys stay in the phone’s Keystore or Keychain, and a lost phone is revoked from the computer. The [app’s documentation](/en/docs/mobile-app/) covers pairing, spaces and the lock.

## Getting it

The app comes first as a closed test on Google Play and in TestFlight, then on both stores. It needs Escouade 2.3 or later on the computer or server. What it sends, and to whom, is in its [privacy policy](/en/mobile/privacy/).
