# Permissions

What the plugin can do, what it sends to Deepgram and when, and what never goes there. Everything below is what its code does (the `escouade-voice` crate) and what Escouade’s core gives it.

## What its manifest declares

- **Network**: `api.deepgram.com`, alone. The plugin is a WASM module: it reaches the network only through Escouade’s host, which refuses any other host, and goes through the proxy of [Settings › Advanced › Network](https://escouade.dev/en/docs/settings-network).
- **Activation**: `voice:deepgram`, when a dictation, a voice or the settings need Deepgram.
- **Nothing else**: no secret of its own (no `permissions.secrets`), no commands, no events, no files.

## Your key

Escouade keeps it in the system keychain (see the Install tab). With each call, the core reads it and attaches it to what it asks the plugin. The plugin puts it in the header Deepgram expects (`Authorization: Token <key>`), and writes it nowhere. What Deepgram answers is written to the plugin’s log with the key taken out (`***`).

Escouade gives this key to the `dev.escouade.deepgram` plugin alone: that name is reserved for the plugins of Escouade’s own hand, and the store refuses it to any other author.

## What goes to Deepgram, and when

Everything goes to `https://api.deepgram.com`, over HTTPS (an encrypted WebSocket for real time), with your key.

| When | What goes |
| --- | --- |
| You finish a dictation (second press, or the shortcut released), “Transcribed by” on “Online”, without real time | The dictation’s sound (mono, 16 kHz): a `POST /v1/listen?model=…&smart_format=true` request with the spoken language (`language`), or `detect_language=true` in automatic detection: the body is the sound itself, a WAV file. |
| During a dictation, with “Real-time transcription” | The sound as you speak, from the dictation’s start (what the microphone heard while it opened included) to its end: a WebSocket `wss://api.deepgram.com/v1/listen` (the model, 16 kHz mono PCM, `interim_results=true`, `smart_format=true`, the language, or `multi` with Nova-3 in automatic detection): the sound in 100 ms binary frames, then `Finalize` and `CloseStream`. |
| In a voice conversation, after each sentence you say | The sentence’s sound, the same way: whole after its silence, or as you speak with real time, from its first syllables to its closing silence. |
| A stream that does not open, or drops | The same dictation or sentence, again, whole by request: with real time, its sound may therefore go twice. |
| The conversation reads aloud, “Voice made by” on “Online” | The text of each passage, sentence by sentence, with the model, the voice and the language: a `POST /v1/speak?model=<voice>&encoding=linear16&sample_rate=24000&container=none` request (and `speed` in English, when it is not 1): the sentence. These are the “Read aloud” passage of each answer, the agent’s questions, its permission requests (“… asks for permission: <what it asks>. Do you allow it?”, with up to 80 characters of the command), and Escouade’s own sentences (“Done. The answer is in the conversation.”). |
| You click “▶ Listen” on a Deepgram voice | The test sentence: “Hello! This is the voice that will read your agents’ answers to you in Escouade.” (or its French version). |
| Settings › Voice shows Deepgram with a key and no list read yet, or you click “Refresh the list” | The key alone, to read its lists: `GET /v1/models` (its transcription models and its voices). |

## What never goes there

- The microphone’s sound when you are not dictating. In a voice conversation, neither the silence before a sentence, nor what the microphone hears while the agent speaks or works: that sound only tells, on your machine, whether you talk over it.
- A dictation cancelled with `Escape` before its end, without real time. With real time, what you said before `Escape` has already gone.
- Your project, your code, your files, the rest of the conversation with the agent: only what the conversation reads aloud goes, and only if the voice is online.
- The microphone’s sound to the voice: the voice only receives text.
- Your key to any host other than `api.deepgram.com`.

Deepgram handles what it receives under its own terms: read them on its side.

## What the plugin cannot do

- Reach any host other than `api.deepgram.com`.
- Read the disk, your project, your files or the keychain, start a program, receive Escouade’s events.
- Keep your key: it has no disk nor secret of its own, and receives it with each call.
- Listen to the microphone: Escouade opens it, on your gesture, and hands it the sound.
