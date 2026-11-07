# Use

What OpenAI does once its key is saved (see the Install tab). How dictation and conversation work is on the site: [Settings › Voice](https://escouade.dev/en/docs/settings-voice) and [the conversation](https://escouade.dev/en/docs/chat).

## Dictation

With “Transcribed by” on “Online” and “Service” on “OpenAI”, each dictation is transcribed by OpenAI:

1. Start the dictation: the microphone button beside a field, or the shortcut (`Ctrl+Shift+Space`, `⇧⌘Space` on macOS).
2. Speak. While it transcribes, the pill says “Transcribing with OpenAI…”.
3. Finish the dictation (a second press, or release the shortcut): the text lands in the field. `Escape` cancels it.

The “Spoken language” of the settings is given to OpenAI; “Automatic detection” lets it find the language. A dictation stops by itself after 10 minutes. A dictation under a tenth of a second is not sent.

### The dictation’s model

The “Model” menu lists the OpenAI models that transcribe. By default `gpt-4o-mini-transcribe`. With no key, it is its documentation’s list: `gpt-4o-mini-transcribe` (fast), `gpt-4o-transcribe` (more accurate), `whisper-1`. With a key saved, the list is its API’s (`GET /v1/models`): the models whose name holds `transcribe` (but not `diarize`) or starts with `whisper`. “Refresh the list” reads it again; the line says when it was read. The model chosen is kept for OpenAI. A model OpenAI no longer offers is replaced by the default one, and the line says so.

## Real-time transcription

Under “Model”, the “Real-time transcription” box is ticked by default. The sound then goes to OpenAI while you speak, and the text comes back as you go: it is written in the dictation’s pill, then the final text goes in the field a moment after the end. In a voice conversation, it is written under the orb.

- The real-time model: `gpt-live-transcribe`. It is real time’s own: the “Model” chosen serves dictations sent whole.
- One stream per dictation, and in a conversation one stream per sentence: nothing goes between two sentences.
- The stream opens within 5 seconds, and the final text comes within 5 seconds after the end; otherwise, or if the stream drops (a proxy that refuses WebSockets, for example), the dictation goes whole, by request, and nothing is lost. In a conversation, the next sentences then go whole until the conversation ends.
- The box is kept for OpenAI. Unticked, each dictation and each sentence goes whole, once finished.
- The line reminds you: “In real time, OpenAI charges more per minute.”

## The agent’s voice

With “Voice made by” on “Online” and “Voice service” on “OpenAI”, OpenAI reads the answers during a voice conversation, and “▶ Listen” reads a test sentence. Passages are read sentence by sentence, the first asked for at once, the next ones while it plays, and the sound plays as it comes.

- **The model** (“Voice model”): by default `gpt-4o-mini-tts`. With no key, its documentation’s list: `gpt-4o-mini-tts` (the most natural), `tts-1` (fast), `tts-1-hd` (quality). With a key, its API’s.
- **The voice**: by default `marin` in French and in English (`alloy` for `tts-1` and `tts-1-hd`, which have no `marin`). Empty field: the default voice.
- Its API does not list its voices: the “Voice in …” field is **free**. Write the voice’s name as OpenAI expects it (for example marin, cedar, coral, alloy…); “See its voices” opens https://platform.openai.com/docs/guides/text-to-speech#voice-options.
- **The speed**: 0.25× to 4×: the speed of the settings is brought within these bounds.
- **The length**: 4,000 characters at most per request; a longer passage is cut between its sentences.

A voice OpenAI does not know is replaced by its default voice, and the conversation says so. If OpenAI does not answer (no key or a refused one, quota, outage, network), the conversation says it once, then the local voice (Piper) reads on if it is downloaded; otherwise the conversation pauses, and “Resume” asks OpenAI again. The “Volume” applies on your machine.
