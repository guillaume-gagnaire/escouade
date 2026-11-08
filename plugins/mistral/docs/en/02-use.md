# Use

What Mistral does once its key is saved (see the Install tab). How dictation and conversation work is on the site: [Settings › Voice](https://escouade.dev/en/docs/settings-voice) and [the conversation](https://escouade.dev/en/docs/chat).

## Dictation

With “Transcribed by” on “Online” and “Service” on “Mistral”, each dictation is transcribed by Mistral:

1. Start the dictation: the microphone button beside a field, or the shortcut (`Ctrl+Shift+Space`, `⇧⌘Space` on macOS).
2. Speak. While it transcribes, the pill says “Transcribing with Mistral…”.
3. Finish the dictation (a second press, or release the shortcut): the text lands in the field. `Escape` cancels it.

The “Spoken language” of the settings is given to Mistral; “Automatic detection” lets it find the language. A dictation stops by itself after 10 minutes. A dictation under a tenth of a second is not sent.

### The dictation’s model

The “Model” menu lists the Mistral models that transcribe. By default `voxtral-mini-latest`. With no key, it is its documentation’s list: `voxtral-mini-latest`. With a key saved, the list is its API’s (`GET /v1/models`): its Voxtral Mini models that transcribe a file. “Refresh the list” reads it again; the line says when it was read. The model chosen is kept for Mistral. A model Mistral no longer offers is replaced by the default one, and the line says so.

## Real-time transcription

Under “Model”, the “Real-time transcription” box is ticked by default. The sound then goes to Mistral while you speak, and the text comes back as you go: it is written in the dictation’s pill, then the final text goes in the field a moment after the end. In a voice conversation, it is written under the orb.

- The real-time model: `voxtral-mini-transcribe-realtime-2602`. It is real time’s own: the “Model” chosen serves dictations sent whole.
- One stream per dictation, and in a conversation one stream per sentence: nothing goes between two sentences.
- The stream opens within 5 seconds, and the final text comes within 5 seconds after the end; otherwise, or if the stream drops (a proxy that refuses WebSockets, for example), the dictation goes whole, by request, and nothing is lost. In a conversation, the next sentences then go whole until the conversation ends.
- The box is kept for Mistral. Unticked, each dictation and each sentence goes whole, once finished.
- The line reminds you: “In real time, Mistral charges more per minute.”

## The agent’s voice

With “Voice made by” on “Online” and “Voice service” on “Mistral”, Mistral reads the answers during a voice conversation, and “▶ Listen” reads a test sentence. Passages are read sentence by sentence, the first asked for at once, the next ones while it plays, and the sound plays as it comes.

- **The model** (“Voice model”): by default `voxtral-mini-tts-2603`. With no key, its documentation’s list: `voxtral-mini-tts-2603`. With a key, its API’s.
- **The voice**: by default `fr_marie_neutral` in French, `en_paul_neutral` in English. Empty field: the default voice.
- Its API lists its voices (`GET /v1/audio/voices`): the “Voice in …” field **completes**. Type a few letters: the list filters by name, id, gender, accent or description, among the voices that read the language with this model; your account’s come first, marked “your account”. If the list could not be read, the field stays free.
- **The speed**: Mistral reads at its own pace, the speed of the settings does not apply.
- **The length**: 4,000 characters at most per request; a longer passage is cut between its sentences.

A voice Mistral does not know is replaced by its default voice, and the conversation says so. If Mistral does not answer (no key or a refused one, quota, outage, network), the conversation says it once, then the local voice (Piper) reads on if it is downloaded; otherwise the conversation pauses, and “Resume” asks Mistral again. The “Volume” applies on your machine.

## Mistral Vibe agents

The plugin also serves the “Mistral Vibe” harness: an agent started with it works through `vibe-acp`, a separate program Escouade starts and watches, on the account chosen (Settings › Accounts and harnesses).

- **Modes**: “Ask” is its `ask` agent, “Accept edits” its `accept-edits` agent, “Bypass” its `auto-approve` agent. Vibe has no Plan mode: Escouade plays it (it studies, its writes are refused, its last words are the plan to approve on its card, then it carries it out).
- **Model and effort**: the “Model” menu lists its session’s (`mistral-medium-3.5` by default). “Low” turns its thinking off, “High” keeps it.
- **Requests**: “Allow”, “For this agent” (kept in the agent’s `VIBE_HOME`) and “Deny”. A denial stops the turn: Vibe would otherwise try the same command again in other forms.
- **Accepted by the plugin itself**: in “Ask” and “Accept edits”, a write or a delete whose paths are all in the agent’s folder, outside `.git`; in “Bypass”, everything. The rest goes through a card.
- **Cost**: Vibe gives each turn’s cost and tokens, no quota. Its rate limit stops the turn on an amber card; the agent resumes by itself a minute later.
- **A discussion of the Docs tab** never goes to Vibe: it has no mode that changes nothing.
- **Attachments**: images and text files, no PDFs.

The site’s [Harnesses](https://escouade.dev/en/docs/harness#mistral-vibe) page details all this, and the “Mistral Vibe view”.
