# Permissions

What the plugin can do, what it sends to Mistral and when, and what never goes there. Everything below is what its code does (the `escouade-voice` crate and, for Vibe, `escouade-acp`) and what Escouade’s core gives it.

## What its manifest declares

- **Network**: `api.mistral.ai`, alone. The plugin is a WASM module: it reaches the network only through Escouade’s host, which refuses any other host, and goes through the proxy of [Settings › Advanced › Network](https://escouade.dev/en/docs/settings-network).
- **Activation**: `voice:mistral`, when a dictation, a voice or the settings need Mistral.
- **An account**: `mistral`, the Mistral account of the voice and of Vibe, and **a harness**: `vibe`, Mistral Vibe.
- **Nothing else**: no secret of its own (no `permissions.secrets`), no commands, no events, no files.

## Your key

Escouade keeps it in the system keychain (see the Install tab). With each call, the core reads it and attaches it to what it asks the plugin. The plugin puts it in the header Mistral expects (`Authorization: Bearer <key>`), and writes it nowhere. What Mistral answers is written to the plugin’s log with the key taken out (`***`).

Escouade gives this key to the `dev.escouade.mistral` plugin alone: that name is reserved for the plugins of Escouade’s own hand, and the store refuses it to any other author. The Mistral account’s key also serves the Mistral Vibe accounts that have no key of their own: the core then gives it to Mistral Vibe by `MISTRAL_API_KEY` (see below), never to the plugin.

## What goes to Mistral, and when

Everything goes to `https://api.mistral.ai`, over HTTPS (an encrypted WebSocket for real time), with your key.

| When | What goes |
| --- | --- |
| You finish a dictation (second press, or the shortcut released), “Transcribed by” on “Online”, without real time | The dictation’s sound (mono, 16 kHz): a `POST /v1/audio/transcriptions` request (multipart): the sound as a WAV file (`dictation.wav`), the model, and the spoken language when one is chosen. |
| During a dictation, with “Real-time transcription” | The sound as you speak, from the dictation’s start (what the microphone heard while it opened included) to its end: a WebSocket `wss://api.mistral.ai/v1/audio/transcriptions/realtime` (the model): the sound’s format (`session.update`, 16 kHz PCM), the sound in 100 ms pieces (`input_audio.append`), then `input_audio.flush` and `input_audio.end`. |
| In a voice conversation, after each sentence you say | The sentence’s sound, the same way: whole after its silence, or as you speak with real time, from its first syllables to its closing silence. |
| A stream that does not open, or drops | The same dictation or sentence, again, whole by request: with real time, its sound may therefore go twice. |
| The conversation reads aloud, “Voice made by” on “Online” | The text of each passage, sentence by sentence, with the model, the voice and the language: a `POST /v1/audio/speech` request: the model, the sentence, the voice (`voice_id`), `response_format=pcm`, and `stream=true`. These are the “Read aloud” passage of each answer, the agent’s questions, its permission requests (“… asks for permission: <what it asks>. Do you allow it?”, with up to 80 characters of the command), and Escouade’s own sentences (“Done. The answer is in the conversation.”). |
| You click “▶ Listen” on a Mistral voice | The test sentence: “Hello! This is the voice that will read your agents’ answers to you in Escouade.” (or its French version). |
| Settings › Voice shows Mistral with a key and no list read yet, or you click “Refresh the list” | The key alone, to read its lists: `GET /v1/models` (its models) and `GET /v1/audio/voices` (its voices). |

## What never goes there

- The microphone’s sound when you are not dictating. In a voice conversation, neither the silence before a sentence, nor what the microphone hears while the agent speaks or works: that sound only tells, on your machine, whether you talk over it.
- A dictation cancelled with `Escape` before its end, without real time. With real time, what you said before `Escape` has already gone.
- Your project, your code, your files, the rest of the conversation with the agent: only what the conversation reads aloud goes, and only if the voice is online.
- The microphone’s sound to the voice: the voice only receives text.
- Your key to any host other than `api.mistral.ai`.

Mistral handles what it receives under its own terms: read them on its side.

## Mistral Vibe

The harness is another matter: **Mistral Vibe is a separate program**, `vibe-acp`, which talks to Mistral itself with its account’s key. What it sends Mistral (your messages, the files it reads, its commands’ output, what its tools return) goes over its own network: neither Escouade nor this plugin sees it go, nor filters it.

- **Who starts it**: Escouade’s core, never the plugin. The plugin describes the launch (`vibe-acp`, `VIBE_ENABLE_TELEMETRY=false`); the core checks it, also turns off Vibe’s updates and their checks, gives each agent its own `VIBE_HOME`, and an account’s key by `MISTRAL_API_KEY`, never in a file nor on a command line.
- **What the plugin sees**: what Vibe writes (the agent’s conversation, its tool calls, its requests), which it translates for the conversation; and the token of Escouade’s MCP server, which it puts in the launch it describes. It never receives a Vibe account’s key: it is only told that an account goes by a key. Its only network stays `api.mistral.ai`, and its code sends only the voice there.
- **What it decides alone**: it accepts, in “Ask” and “Accept edits”, a write or a delete whose paths are all in the agent’s folder, outside `.git`; in “Bypass”, everything. It refuses at once a request that names a path of a private project. The rest goes through a card, which waits for you.
- **What it cannot do**: start Vibe or a command, read or write a file, reach another host.
- **Vibe has no sandbox**: a command you accept can do anything, with your rights.

## What the plugin cannot do

- Reach any host other than `api.mistral.ai`.
- Read the disk, your project, your files or the keychain, start a program (Vibe included), receive Escouade’s events.
- Keep your key: it has no disk nor secret of its own, and receives it with each call.
- Listen to the microphone: Escouade opens it, on your gesture, and hands it the sound.
