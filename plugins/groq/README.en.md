# Groq

The Groq voice service in Escouade. With your Groq key, Escouade hands it dictation and the voice that reads your agents’ answers (no real-time transcription: Groq does not offer it).

It is a plugin of Escouade’s own hand (`dev.escouade.groq`): it has its own version, and is turned off or uninstalled like any other. Who it is for: you, if you want to dictate or talk to your agents with Groq rather than with Whisper and Piper, which run on your machine with no account.

## What it needs

- Escouade 2.2.0 or later.
- A Groq account and an API key (https://console.groq.com/keys). Each call is billed to that account.
- Nothing else to install.

## What it can do

- reach `api.groq.com`, and no other host;
- receive your Groq key with each call, without keeping it: it stays in the system keychain, and Escouade gives it to this plugin alone;
- send Groq the sound of what you dictate, and, if you choose its voice, the text the conversation reads aloud;
- nothing else: no files, no commands, no secret of its own, neither your project nor your code.

## The tabs

- **Install**: get the plugin, create the key, give it to Escouade.
- **Use**: dictation, the agent’s voice, models and voices.
- **Permissions**: what goes to Groq, when, and what never does.
- **Troubleshooting**: the messages it shows, and what to do.
- **Develop**: its folder, its tests, running it alone.
