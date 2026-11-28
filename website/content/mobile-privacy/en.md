# Mobile app privacy

Escouade’s mobile app talks to your own machines and to nobody else, except to wake your phone when an agent is waiting for you, if you accept it. This page says what it sends, to whom, and how to stop it.

## In short

- The app has **no account, no statistics, no ads and no tracking**. It contains no analytics nor crash reporting tool.
- Everything you read and send (your agents’ requests, your answers, your messages, your photos, your dictation) goes **directly between the phone and your computer or server**, through an encrypted WireGuard tunnel. Escouade’s publisher never receives it.
- **The only thing that goes through the publisher** is the push notification, and only if you accept it at pairing: a sealed wake-up, whose text is end-to-end encrypted. The relay cannot read it.

## What stays on the phone

- **Each pairing’s keys** (the tunnel’s key, the space’s token, the notifications’ key): in Android’s Keystore or iOS’s Keychain, bound to the device, left out of backups.
- **What each space said last**, to show it offline, and your drafts. No conversation nor file is kept on disk.
- **The system’s permissions**, asked when they are needed: the camera (reading the pairing QR code, attaching a photo), the microphone and speech recognition (dictation, done by the phone’s system), biometrics (the lock before an answer that makes an agent act), notifications.

## What goes to your machines

Your answers, messages, photos and the text of your dictation go to the Escouade computer or server you paired, through the tunnel. It keeps them, like everything Escouade does: see [Escouade’s privacy](/en/privacy/).

## Notifications

When you accept notifications at pairing:

| Who | What it receives | Why |
|---|---|---|
| Your computer or server | The notification’s text, which it seals with the pairing’s key | It knows what is waiting for you |
| The publisher’s relay (`push.escouade.dev`) | A sealed wake-up, a random pairing number, the phone’s push token and its platform | Hand the wake-up to Google or Apple |
| Google (Firebase Cloud Messaging, on Android) | The push token, Firebase’s installation ID, the app’s ID and version, what delivery needs of the device, and the message’s metadata (size, time, priority) | Deliver the wake-up to the phone |
| Apple (APNs, on iOS) | The push token and the message’s metadata | Deliver the wake-up to the phone |

Nobody but your phone reads the text. Of Firebase, the app only uses message delivery (FCM): no Analytics, no Crashlytics, no database, and nothing goes to Google before you accept notifications. On iOS, the app contains no Firebase SDK.

## Stop and erase

- **Turn notifications off**: in the app’s or the phone’s settings. Without them, nothing goes through the publisher, Google or Apple.
- **Revoke a phone** from your computer (Settings › Phone): its tunnel, its token and its relay registration are erased. Removing a space from the app does the same.
- **Uninstalling the app** erases what it keeps on the phone.

## Your rights and contact

The relay only keeps the table of the pairings that accepted notifications (the random number, the platform, the push token), erased at revocation. For any question about your data, or to exercise your rights (access, erasure, objection), write to the address given in the [legal notice](/en/legal/). You can also complain to the CNIL, the French data protection authority.
