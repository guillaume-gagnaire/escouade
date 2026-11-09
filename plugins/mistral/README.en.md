# Mistral

Mistral in Escouade, on one Mistral account: its voice service, and Mistral Vibe as a harness. With your Mistral key, Escouade hands it dictation, real-time transcription, and the voice that reads your agents’ answers. The same plugin runs your Mistral Vibe agents too: Escouade starts the `vibe-acp` program, and the plugin turns what it says into the conversation.

It is a plugin of Escouade’s own hand (`dev.escouade.mistral`): it has its own version, and is turned off or uninstalled like any other. Who it is for: you, if you want to dictate or talk to your agents with Mistral rather than with Whisper and Piper, which run on your machine with no account, or have Mistral Vibe agents work.

## What it needs

- Escouade 2.2.0 or later.
- A Mistral account and an API key (https://console.mistral.ai/api-keys). Each call is billed to that account.
- For Mistral Vibe: the program itself, installed with `uv tool install mistral-vibe`.

## What it can do

- reach `api.mistral.ai`, and no other host;
- receive your Mistral key with each call, without keeping it: it stays in the system keychain, and Escouade gives it to this plugin alone (and to Mistral Vibe, for a Vibe account with no key of its own);
- send Mistral the sound of what you dictate, and, if you choose its voice, the text the conversation reads aloud;
- tell Escouade how to start Mistral Vibe, and translate what Vibe writes: Escouade starts it, never the plugin;
- accept by itself, for a Vibe agent, a write in the agent’s folder, and refuse at once what touches a private project;
- nothing else: no files, no commands, no secret of its own. It sees what Vibe writes, to translate it, but its only network stays `api.mistral.ai`, and it sends only the voice there.

## The tabs

- **Install**: get the plugin, create the key, give it to Escouade, and set up Mistral Vibe.
- **Use**: dictation, real time, the agent’s voice, models and voices, Vibe agents.
- **Permissions**: what goes to Mistral, when, and what never does.
- **Troubleshooting**: the messages it shows, and what to do.
- **Develop**: its folder, its tests, running it alone.
