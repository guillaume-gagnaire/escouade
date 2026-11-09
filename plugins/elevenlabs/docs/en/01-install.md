# Install

## 1. Get the plugin

Escouade installs it by itself: it is in the app’s installer, with the other plugins of its own hand, and installs with no network when the app starts. You see it in the Plugins view, named “ElevenLabs”.

- **On**, it serves ElevenLabs in Settings › Voice. It starts at the first call that needs it.
- **Off** (its switch, in the Plugins view), it serves nothing: a dictation or a voice that goes through ElevenLabs fails, and the message says so (see the Troubleshooting tab).
- **Uninstalled**, Escouade does not put it back without you: the Plugins view offers it under “Offered by Escouade”, with “Install”. You can also install it from the store, like any other plugin.

## 2. Create a key at ElevenLabs

1. Sign in on elevenlabs.io, then open your account’s API keys (https://elevenlabs.io/app/settings/api-keys).
2. Create a key and copy it. If you restrict it, leave it speech to text, text to speech and reading the models and voices: without reading the voices, the voice field stays free.
3. Dictation and voice count against your account’s credits or plan.

## 3. Give it to Escouade

1. Open the settings (`Ctrl+,`, `⌘,` on macOS), “Voice” tab.
2. In the “Transcription” group, set “Transcribed by” to “Online”, then “Service” to “ElevenLabs”.
3. On the “ElevenLabs key” line, paste your key in the field (“Paste your API key”), then “Save the key” or `Enter`. The line then says “Saved in the keychain”, with “Replace” and “Delete the key”.
4. Click “Save” to keep the choice of service: the key itself is kept at once.

For ElevenLabs to read your agents’ answers too, go to the “Conversation” group of the same tab: “Voice made by” to “Online”, then “Voice service” to “ElevenLabs”. **One key per service**: saved for dictation, it already serves the voice.

## What Escouade keeps, and where

- **The key**: in the system keychain (Credential Manager on Windows, Keychain on macOS), service `dev.gagnaire.escouade`, entry `dictation-elevenlabs`. Never in a file of Escouade’s, nor in its log, and it is never shown again. On Linux, Escouade has no keychain: the key is not saved, and the app says so.
- **Your choices** (service, models, voices, real time): in the app’s settings.
- **The list of models and voices** read from ElevenLabs: in `voice/online-catalog.json`, under Escouade’s [data folder](https://escouade.dev/en/docs/data), until you read it again.
- **The plugin keeps nothing**: it receives the key with each call, and has no disk nor secret of its own.
