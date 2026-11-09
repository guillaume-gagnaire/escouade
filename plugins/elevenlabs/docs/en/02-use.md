# Use

What ElevenLabs does once its key is saved (see the Install tab). How dictation and conversation work is on the site: [Settings › Voice](https://escouade.dev/en/docs/settings-voice) and [the conversation](https://escouade.dev/en/docs/chat).

## Dictation

With “Transcribed by” on “Online” and “Service” on “ElevenLabs”, each dictation is transcribed by ElevenLabs:

1. Start the dictation: the microphone button beside a field, or the shortcut (`Ctrl+Shift+Space`, `⇧⌘Space` on macOS).
2. Speak. While it transcribes, the pill says “Transcribing with ElevenLabs…”.
3. Finish the dictation (a second press, or release the shortcut): the text lands in the field. `Escape` cancels it.

The “Spoken language” of the settings is given to ElevenLabs; “Automatic detection” lets it find the language. A dictation stops by itself after 10 minutes. A dictation under a tenth of a second is not sent.

### The dictation’s model

The “Model” menu lists the ElevenLabs models that transcribe. By default `scribe_v2`. With no key, it is its documentation’s list: `scribe_v2` (90 languages), `scribe_v2_medical` (medical vocabulary). Its API does not list its transcription models: the list is always its documentation’s, shipped in the plugin. “Refresh the list” reads it again; the line says when it was read. The model chosen is kept for ElevenLabs. A model ElevenLabs no longer offers is replaced by the default one, and the line says so.

## Real-time transcription

Under “Model”, the “Real-time transcription” box is ticked by default. The sound then goes to ElevenLabs while you speak, and the text comes back as you go: it is written in the dictation’s pill, then the final text goes in the field a moment after the end. In a voice conversation, it is written under the orb.

- The real-time model: `scribe_v2_realtime`. It is real time’s own: the “Model” chosen serves dictations sent whole.
- One stream per dictation, and in a conversation one stream per sentence: nothing goes between two sentences.
- The stream opens within 5 seconds, and the final text comes within 5 seconds after the end; otherwise, or if the stream drops (a proxy that refuses WebSockets, for example), the dictation goes whole, by request, and nothing is lost. In a conversation, the next sentences then go whole until the conversation ends.
- The box is kept for ElevenLabs. Unticked, each dictation and each sentence goes whole, once finished.
- The line reminds you: “In real time, ElevenLabs charges more per minute.”

## The agent’s voice

With “Voice made by” on “Online” and “Voice service” on “ElevenLabs”, ElevenLabs reads the answers during a voice conversation, and “▶ Listen” reads a test sentence. Passages are read sentence by sentence, the first asked for at once, the next ones while it plays, and the sound plays as it comes.

- **The model** (“Voice model”): by default `eleven_flash_v2_5`. With no key, its documentation’s list: `eleven_flash_v2_5` (the fastest, 32 languages), `eleven_v4_turbo` (expressive, real time), `eleven_v4` (the most expressive, slower), `eleven_v3_conversational` (expressive, real time), `eleven_v3` (expressive), `eleven_multilingual_v2` (steady, slower). With a key, its API’s.
- **The voice**: by default George (`JBFqnCBsd6RMkjVDRZzb`), in French and in English; with an old `…_v2` model that is not multilingual, in English only. Empty field: the default voice.
- Its API lists its voices (`GET /v2/voices`): the “Voice in …” field **completes**. Type a few letters: the list filters by name, id, gender, accent or description, among the voices that read the language with this model; your account’s come first, marked “your account”. If the list could not be read, the field stays free.
- **The speed**: 0.7× to 1.2×: the speed of the settings is brought within these bounds.
- **The length**: 5,000 characters at most per request; a longer passage is cut between its sentences.

A voice ElevenLabs does not know is replaced by its default voice, and the conversation says so. If ElevenLabs does not answer (no key or a refused one, quota, outage, network), the conversation says it once, then the local voice (Piper) reads on if it is downloaded; otherwise the conversation pauses, and “Resume” asks ElevenLabs again. The “Volume” applies on your machine.
