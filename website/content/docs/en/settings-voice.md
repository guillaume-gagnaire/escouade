# Settings: Voice

The “Voice” page sets up dictation (the microphone, who transcribes what you say, the spoken language and the shortcut) and the voice conversation (what makes the voice, the voice of each language, its speed, its volume and the silence that sends what you said).

## Opening the page

Open the settings with `Ctrl+,` (`⌘,` on macOS), then choose “Voice” (“Dictating messages: microphone, transcription model, language and shortcut”). These settings are the app’s own, shared by all projects.

The page’s choices apply as you set them, like the other settings, and three actions too: downloading or deleting a Whisper model, downloading or deleting a voice, and saving or deleting the key of an online service.

Using dictation and the conversation is described in [the conversation](chat#dictating-a-message).

## Microphone

| Setting | Effect | Default |
|---|---|---|
| “Microphone used” | The microphone that dictation and the conversation listen to: “The system’s” (with its name in parentheses when the system gives it), or one of the microphones plugged in. A microphone chosen then unplugged stays in the list under its id; while it is missing, the system’s is used. | “The system’s” |

With no microphone at all, the row says “No microphone found: plug one in, then open the settings again.” The messages for a microphone the system refuses are in [troubleshooting](troubleshooting#dictation-or-the-conversation-hears-nothing).

## Transcription

This group chooses who transcribes a dictation, and in which language. Under its title, the app recalls what leaves your machine: with Whisper, “Whisper transcribes on your machine: nothing you say is sent. Only the model is downloaded, through the proxy of the network settings, and its checksum checked.”; online, what leaves and to which service (see [Online dictation](#online-dictation)).

| Setting | Effect | Default |
|---|---|---|
| “Transcribed by” | “Local (Whisper)”: the dictation is transcribed on your machine. “Online”: the sound of each dictation is sent to the service chosen, with your key, for a machine too slow for Whisper or a better quality. The voice conversation follows this choice: online, each sentence you say goes to the same service. | “Local (Whisper)” |
| “Model” (local) | The Whisper model. A bigger one understands better and transcribes more slowly. The one chosen downloads at the first dictation, or from its row. It also serves the voice conversation. | base |
| “Service” (online) | The service that transcribes: OpenAI, Groq, Mistral, ElevenLabs or Deepgram, paid per use on your account with it. | OpenAI |
| “Model” (online) | The service’s model that transcribes, kept for each service. | The one of the table below |
| “Real-time transcription” (online, but Groq) | The text is written while you speak, and the sentence is ready a moment after your last word. Kept for each service; unticked, each dictation and each sentence go whole, once over. See [Real-time transcription](#real-time-transcription). | Ticked |
| “<service> key” (online) | Your API key with that service, the same for dictation and for the conversation’s voice. | None |
| “Spoken language” | The language Whisper (or the service) expects: “The interface’s”, “Automatic detection”, or a given language (Français, English, Deutsch, Español, Italiano, Português, Nederlands). Naming the language makes the transcription more accurate; automatic detection suits those who speak several. | “The interface’s” |

### Whisper’s models

Each model has its row, with its size and two marks out of 5, like “Speed 4/5 · quality 2/5”:

| Model | Size | Speed | Quality |
|---|---|---|---|
| `tiny` | 74 MB | 5/5 | 1/5 |
| `base` | 141 MB | 4/5 | 2/5 |
| `small` | 465 MB | 3/5 | 3/5 |
| `large-v3-turbo-q5_0` | 547 MB | 2/5 | 4/5 |
| `medium` | 1.4 GB | 1/5 | 4/5 |
| `large-v3-turbo` | 1.5 GB | 2/5 | 5/5 |

A model not yet downloaded has a “Download” button; while it downloads, the row shows the percentage and “Cancel”. A downloaded model is marked “Downloaded”, with “Delete”: its file leaves the disk, and it downloads again at the next dictation that needs it. Models come from Hugging Face (`huggingface.co`), through the proxy of the [network settings](settings-network), and their checksum is checked before they are kept.

### Online dictation

“Online” shows three rows in place of Whisper’s models: “Service”, “Model” and “<service> key”. The services and their default model:

| Service | Address | Default model |
|---|---|---|
| OpenAI | `api.openai.com` | `gpt-4o-mini-transcribe` |
| Groq | `api.groq.com` | `whisper-large-v3-turbo` |
| Mistral | `api.mistral.ai` | `voxtral-mini-latest` |
| ElevenLabs | `api.elevenlabs.io` | `scribe_v2` |
| Deepgram | `api.deepgram.com` | `nova-3` |

Paste the key in the field (“Paste your API key”), then “Save the key” or `Enter`. It goes straight to the system keychain, one entry per service, never to Escouade’s files nor to its log, and is never shown again: the row says “Saved in the keychain”, with “Replace” and “Delete the key”. Without a key, the row says “No key saved: the service needs one.” On Linux, where Escouade has no keychain, the key cannot be saved. **One key per service**: saved once, here or in the “Conversation” group, it serves dictation and the [online voice](#the-online-voice).

The “Model” menu lists the service’s models that transcribe, with what sets them apart (“fast”, “more accurate”), the default one first. With a key saved, the list is the one the service’s API gives, read with your key and kept on your machine; “Refresh the list” reads it again, and the row says when it was read. Without a key, or for a service whose API does not list its transcription models (ElevenLabs), it is the list of its documentation, shipped with the app. Until you choose another one, it is the default model: dictation stays as it was. The model chosen is kept per service: switching service and back finds it again. A model the service no longer offers is replaced by the default one, and the row says so (“Groq no longer offers “…”: its default model, “whisper-large-v3-turbo”, stands in.”).

These five services are plugins shipped with Escouade (`dev.escouade.openai`, `dev.escouade.groq`, `dev.escouade.mistral`, `dev.escouade.elevenlabs`, `dev.escouade.deepgram`): the key stays in the app’s keychain, which gives it to its service’s plugin alone at each call. **Mistral shares its account with Mistral Vibe**: the key given here also serves the Vibe accounts without a key of their own, and with no key here, the voice takes the first active Vibe account’s, which the row says (“The one of the Vibe account “Team” · key …wxyz”, with “Give another key”). Switched off in the Plugins view, a service leaves this page and dictation goes back to local; a plugin that does not start lets Escouade answer in its place, and its journal says why (see [the plugins shipped with Escouade](plugins/installer#the-plugins-shipped-with-escouade)).

### Real-time transcription

Under “Model”, the “Real-time transcription” box is ticked by default for the services that offer it: OpenAI, Mistral, ElevenLabs and Deepgram (not Groq). The sound then goes to the service while you speak, through an encrypted stream, and the text comes back as you speak: it is written in the dictation’s panel, then the final text goes into the field, a moment after the dictation ends; in a voice conversation, it is written under the orb, and the sentence is ready a few hundred milliseconds after your last word. The row says the model taken and that, with this service, a minute in real time costs more:

| Service | Real-time model |
|---|---|
| OpenAI | `gpt-live-transcribe` |
| Mistral | `voxtral-mini-transcribe-realtime-2602` |
| ElevenLabs | `scribe_v2_realtime` |
| Deepgram | The model chosen |

The box is kept for each service. Unticked, each dictation and each sentence of the conversation go whole, once over, as before. If the stream does not get through (a corporate proxy that refuses WebSockets, for instance), or drops, the sentence goes whole, by request: nothing is lost, and the conversation does not try the stream again until it ends.

Under the group’s title, the app says what leaves. With “Real-time transcription”, the sound of what you say, as you say it, to the service chosen, with your key, through the proxy of the network settings; in a voice conversation, only while you speak: neither the silence, nor the agent’s voice. Otherwise, the sound of each dictation (a WAV file), once you end it, to the service chosen, with your key, through the proxy of the network settings. Nothing else: neither the project, nor the code, nor the conversation with the agent. The text comes back into the field dictated into; the service handles the sound under its own terms. The details are on the [Privacy](/privacy/) page.

## Use

| Setting | Effect | Default |
|---|---|---|
| “Shortcut” | The dictation shortcut: a short press starts it and another ends it; held down, it dictates while it is pressed. It dictates into the field that has the focus: the message, a ticket’s description, the terminal of the Claude Code view. | `Ctrl+Shift+Space` (`⇧⌘Space` on macOS) |

“Change” waits for the new shortcut (“Press the shortcut…”): it needs `Ctrl`, `Alt` or `Cmd`, or a key `F1` to `F24`; `Esc` keeps the current one. A changed shortcut has a “Default” button, which puts back `Ctrl+Shift+Space`.

## Conversation

This group sets up the voice that reads the agent’s answers during a [voice conversation](chat#talking-with-the-agent-voice-conversation): Piper by default, on your machine, or an online service (see [The online voice](#the-online-voice)). Under its title, the app says, with Piper: “Answers are read on your machine by Piper: nothing leaves your machine.” The voices offered are those whose license allows it: free model and dataset, no voice of a character, a known person or a brand, but for one exception, marked as such.

| Setting | Effect | Default |
|---|---|---|
| “Voice made by” | “Local (Piper)”: the answers are read on your machine. “Online”: by the service chosen, with the key of its dictation. | “Local (Piper)” |
| “Voice in Français” | The voice that reads the answers when the conversation is held in French, out of 7. With an “Online” voice, these lines go away: a single one, “If ElevenLabs doesn’t answer” for instance, says which downloaded local voice takes over (or that the conversation pauses). | Siwis, medium quality (60 MB) |
| “Voice in English” | The same in English, out of 29. | Ljspeech, medium quality (61 MB) |
| “Speaker” | For a voice with several speakers or styles, the one that reads: its name when the voice gives one, else its number. | The first |
| “Speed” | From 0.5× to 2×, in steps of 0.1: 1 is the voice’s own pace, higher it reads faster. Online, brought within the service’s bounds; the row says when it takes none. | 1.0× |
| “Volume” | From 0 to 100 %, applied on your machine, whatever the service. | 100 % |
| “Delay before sending” | The silence after which what you said goes to the agent, from 0.5 to 3 s, in steps of 0.1 s. | 1.2 s |

The conversation is held in the “Spoken language” when it is French or English, else in the interface’s language.

### The voices

Each language’s menu names each voice with its quality and size, like “Siwis - French (FR) Medium · medium quality · 60 MB”. Under the menu:

- “Download (60 MB)” for a voice not yet downloaded, then the percentage and “Cancel”;
- “▶ Listen” (“Playing…” while it reads a test sentence) and “Delete” for a downloaded voice;
- its license and its dataset’s (“Model license: mit · Dataset: CC-BY 4.0”, or “Model license: none declared” for Fuze), and the credit it asks for (“Credit: …”).

**Fuze**, in French, of high quality (74 MB), is the only voice offered as an exception, which its card says: its model declares no license, and its dataset, under CC BY 4.0, was generated by a synthetic voice (ElevenLabs v4). It has 14 styles, chosen with “Speaker”: normal (the default), joyeuse, excited, colere, triste, surprise, interrogative, storytelling, moqueur, peur, chuchote, determine, fier and degoute.

The credits of Piper, eSpeak NG, ONNX Runtime, whisper.cpp and each voice are in [Settings › About](settings-about), “Credits and licenses”.

### The online voice

“Online”, in “Voice made by”, has the passages read by a service, with **the key of its dictation** (one keychain entry per service: saved for dictation, it already serves here). The group then shows “Service”, “Voice model”, “<service> key”, then “Voice in Français” and “Voice in English”. The services, their default model and voice:

| Service | Default model | Default voice (fr / en) | Speed |
|---|---|---|---|
| OpenAI | `gpt-4o-mini-tts` | `marin` / `marin` | 0.25× to 4× |
| Groq | `canopylabs/orpheus-v1-english` | English only: `hannah` | Its own pace |
| Mistral | `voxtral-mini-tts-2603` | `fr_marie_neutral` / `en_paul_neutral` | Its own pace |
| ElevenLabs | `eleven_flash_v2_5` | George (`JBFqnCBsd6RMkjVDRZzb`) | 0.7× to 1.2× |
| Deepgram | `aura-2` | `aura-2-agathe-fr` / `aura-2-thalia-en` | 0.7× to 1.5×, in English only |

The “Voice model” menu works as the dictation’s does: the list of the service’s API, read with your key (for instance ElevenLabs’ models: Flash v2.5, v4 Turbo, v4, v3…), else the one of its documentation, the default model until you choose another one, kept per service.

The “Voice in …” field depends on the service:

- when its API lists its voices (ElevenLabs, Mistral, Deepgram), **it completes**: type a few letters, the list filters by name, id, gender, accent or description, among the voices that read the language with this model; the voices made or cloned on your account come first, marked “your account”. Pick with the mouse, or with the arrows and `Enter`; `Esc` closes the list;
- else (OpenAI, Groq), it is a **free field**: write the voice’s name or id as the service expects it (the row gives an example), “See its voices” opens the service’s page of voices.

Empty, it is the service’s default voice. The voice chosen is kept per service and per language (and per model with Deepgram, whose voices are models). “▶ Listen” reads a sentence with the service, model and voice chosen: a voice the service refuses is said at once, with its answer. A model that does not read a language (Groq’s Orpheus in French): the row says so, and Piper reads the conversation in that language.

During the conversation, nothing changes for you: the passages are read sentence by sentence, the first one asked at once and the next ones while it plays, the sound played as it comes, with the same subtitles, the same volume, the same barge-in, pause and stop. Under the group’s title, the app says what leaves: the text of each passage read, sentence by sentence, to the service, with your key, through the proxy of the [network settings](settings-network), not the sound of the microphone, which follows “Transcribed by”. The details are on the [Privacy](/privacy/) page.

If the service does not answer (key missing or refused, quota spent, service down, network or proxy), the conversation says so once, with the service and what to do, and **Piper reads on** if a local voice is downloaded; else it **pauses**, and “Resume” asks the service again. A voice the service no longer knows: it is said, and its default voice reads. Escouade’s log never holds your key nor the text read.

### What is read aloud

During a conversation, each message said to the agent asks it to end its answer with a short passage written to be read: two or three sentences, with no code, no file path and no technical detail. Escouade shows it apart under the answer (“Read aloud”) and reads it with the language’s voice; the whole answer stays in the conversation. This holds for every harness (Claude Code, Codex, OpenCode, Copilot CLI). An agent that gives no such passage makes it say “Done. The answer is in the conversation.” The agent’s questions and permission requests are read too.

### When the voice is not available

Piper runs the voices with ONNX Runtime, which ships with the app. If it is missing, the group says “Speech is not available: the voice’s engine (ONNX Runtime) is missing from the app.”, the voices’ buttons are grayed out, and the conversation does not start: reinstall Escouade.

## What downloads, and where

Nothing downloads until you dictate or talk with an agent. At the first dictation, then at the first conversation, Escouade offers each missing file, with its size, and downloads it only after “Download”. Everything goes through the proxy of the [network settings](settings-network), from Hugging Face, and each file is checked against its checksum.

| What | When | Where, under the [data folder](data) |
|---|---|---|
| The Whisper model chosen | At the first dictation or conversation in local | `voice/models/ggml-<model>.bin` |
| The language’s voice | At the first conversation in that language | `voice/voices/<voice>.onnx` and `<voice>.onnx.json` |
| eSpeak NG’s data (how French and English are pronounced) | Written by the app at the first reading, without a download, and written again after an update | `voice/espeak-ng-data/` |
| ONNX Runtime | Never: it ships with the app | The app’s folder |

Online dictation and the online voice download nothing. “Delete”, on the row of a model or a voice, frees its room on the disk.
