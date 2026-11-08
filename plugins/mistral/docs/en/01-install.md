# Install

## 1. Get the plugin

Escouade installs it by itself: it is in the app’s installer, with the other plugins of its own hand, and installs with no network when the app starts. You see it in the Plugins view, named “Mistral”.

- **On**, it serves Mistral in Settings › Voice and the Mistral Vibe agents. It starts at the first call that needs it.
- **Off** (its switch, in the Plugins view), it serves nothing: a dictation or a voice that goes through Mistral fails, and the message says so (see the Troubleshooting tab).
- **Uninstalled**, Escouade does not put it back without you: the Plugins view offers it under “Offered by Escouade”, with “Install”. You can also install it from the store, like any other plugin.

## 2. Create a key at Mistral

1. Sign in on console.mistral.ai, then open “API Keys” (https://console.mistral.ai/api-keys).
2. Create a key and copy it whole.
3. The calls are billed to the Mistral organization of that key.

## 3. Give it to Escouade

1. Open the settings (`Ctrl+,`, `⌘,` on macOS), “Voice” tab.
2. In the “Transcription” group, set “Transcribed by” to “Online”, then “Service” to “Mistral”.
3. On the “Mistral key” line, paste your key in the field (“Paste your API key”), then “Save the key” or `Enter`. The line then says “Saved in the keychain”, with “Replace” and “Delete the key”.
4. Click “Save” to keep the choice of service: the key itself is kept at once.

For Mistral to read your agents’ answers too, go to the “Conversation” group of the same tab: “Voice made by” to “Online”, then “Voice service” to “Mistral”. **One key per service**: saved for dictation, it already serves the voice.

### The Mistral account, shared with Mistral Vibe

The key of Settings › Voice is **the Mistral account**: the voice and Mistral Vibe share it. Its line says so (“The Mistral account, shared by the voice and Mistral Vibe…”).

- With no key here, the voice takes the one of the first active Vibe account that has one, and the line says which: “The one of the Vibe account “Team” · …”, with “Give another key”.
- A Vibe account added with no key takes this one (see below).

## 4. Set up Mistral Vibe

1. Install Mistral Vibe: `uv tool install mistral-vibe` (you need [uv](https://docs.astral.sh/uv/)). Escouade finds `vibe-acp` on the `PATH`, or where `uv` puts it.
2. Give it your key with `vibe --setup`, in a terminal: that is **the machine’s account**. Vibe keeps it on its side; Escouade never reads it.
3. For another Mistral account, open Settings › Accounts and harnesses, “Add an account…”, harness Mistral Vibe, “A new account”: its API key goes in the field (“From console.mistral.ai, API Keys”). Left empty, the account takes the Mistral key of Settings › Voice (“Empty: the Mistral key of Settings › Voice (…)”). Two Mistral organizations stay two accounts, each billed on its own.
4. Start an agent with the “Mistral Vibe” harness.

The site’s [Harnesses](https://escouade.dev/en/docs/harness#mistral-vibe) page says the rest: modes, cost, terminal view.

## What Escouade keeps, and where

- **The key**: in the system keychain (Credential Manager on Windows, Keychain on macOS), service `dev.gagnaire.escouade`, entry `plugin:dev.escouade.mistral:account:mistral`, and under its former name, `dictation-mistral`, read first. Never in a file of Escouade’s, nor in its log, and it is never shown again. On Linux, Escouade has no keychain: the key is not saved, and the app says so.
- **A Vibe account’s key**: in the same keychain, entry `plugin:dev.escouade.mistral:account:vibe-<id>`, and under its former name `vibe:<id>`, read first.
- **Your choices** (service, models, voices, real time): in the app’s settings.
- **The list of models and voices** read from Mistral: in `voice/online-catalog.json`, under Escouade’s [data folder](https://escouade.dev/en/docs/data), until you read it again.
- **The plugin keeps nothing**: it receives the key with each call, and has no disk nor secret of its own.
