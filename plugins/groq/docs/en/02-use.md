# Use

What Groq does once its key is saved (see the Install tab). How dictation and conversation work is on the site: [Settings › Voice](https://escouade.dev/en/docs/settings-voice) and [the conversation](https://escouade.dev/en/docs/chat).

## Dictation

With “Transcribed by” on “Online” and “Service” on “Groq”, each dictation is transcribed by Groq:

1. Start the dictation: the microphone button beside a field, or the shortcut (`Ctrl+Shift+Space`, `⇧⌘Space` on macOS).
2. Speak. While it transcribes, the pill says “Transcribing with Groq…”.
3. Finish the dictation (a second press, or release the shortcut): the text lands in the field. `Escape` cancels it.

The “Spoken language” of the settings is given to Groq; “Automatic detection” lets it find the language. A dictation stops by itself after 10 minutes. A dictation under a tenth of a second is not sent.

### The dictation’s model

The “Model” menu lists the Groq models that transcribe. By default `whisper-large-v3-turbo`. With no key, it is its documentation’s list: `whisper-large-v3-turbo` (fast), `whisper-large-v3` (more accurate). With a key saved, the list is its API’s (`GET /openai/v1/models`): the models whose name starts with `whisper`. “Refresh the list” reads it again; the line says when it was read. The model chosen is kept for Groq. A model Groq no longer offers is replaced by the default one, and the line says so.

## No real time

Groq does not transcribe as you speak: the “Real-time transcription” box does not show. Each dictation goes whole when you finish it, and in a conversation each sentence goes whole after its silence.

## The agent’s voice

With “Voice made by” on “Online” and “Voice service” on “Groq”, Groq reads the answers during a voice conversation, and “▶ Listen” reads a test sentence. Passages are read sentence by sentence, the first asked for at once, the next ones while it plays, and the sound plays as it comes.

- **The model** (“Voice model”): by default `canopylabs/orpheus-v1-english`. With no key, its documentation’s list: `canopylabs/orpheus-v1-english` (English only). With a key, its API’s.
- **The voice**: by default `hannah`, in English only: Orpheus does not read French, and the local voice (Piper) then reads the conversation in French. Empty field: the default voice.
- Its API does not list its voices: the “Voice in …” field is **free**. Write the voice’s name as Groq expects it (for example hannah, troy, austin, diana…); “See its voices” opens https://console.groq.com/docs/text-to-speech/orpheus.
- **The speed**: Groq reads at its own pace, the speed of the settings does not apply.
- **The length**: 200 characters at most per request; a longer passage is cut between its sentences.

A voice Groq does not know is replaced by its default voice, and the conversation says so. If Groq does not answer (no key or a refused one, quota, outage, network), the conversation says it once, then the local voice (Piper) reads on if it is downloaded; otherwise the conversation pauses, and “Resume” asks Groq again. The “Volume” applies on your machine.
