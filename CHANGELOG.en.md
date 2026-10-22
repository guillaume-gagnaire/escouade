# Changelog

Escouade's visible changes, version by version, in English. The notes of each GitHub release and the English Discord channel take the section of its version; the French notes, those of the in-app update included, are in [CHANGELOG.md](CHANGELOG.md), which also holds the versions before 1.8.2.

To write an entry: one or two short sentences, for a human reader. What they can now see or do, and why it helps; the details (edge cases, settings, file names, tools, ticket numbers) go in the documentation, not here. Each entry of CHANGELOG.md has its English one here, under the same version and the same heading (`## [Unreleased]` for `## [Non publié]`).

## [Unreleased]

### Fixes

- Local dictation and voice conversation (Whisper) are much faster, above all on Windows: a sentence is transcribed there in 2.5 s with large-v3-turbo instead of over a minute, and in a quarter of a second with base.

## [1.8.8] — 2026-10-20

### Fixes

- With live transcription, the voice conversation no longer cuts the starts and ends of your sentences, nor the words said more softly after a pause: the whole sentence goes to the service, and its text comes a moment later.

## [1.8.7] — 2026-10-20

### New

- With online transcription, the text writes itself as you speak, in dictation as in the voice conversation, and the sentence is ready a moment after your last word. A box in Settings › Voice turns it off for each service.

## [1.8.6] — 2026-10-20

### Fixes

- With online transcription, the voice conversation also sends what you say to the chosen service (ElevenLabs…), as dictation does: on Windows, each sentence no longer takes tens of seconds to be understood.

## [1.8.5] — 2026-10-19

### Fixes

- Stopping an agent can no longer cut an update of Claude Code half-way, which could make the `claude` command disappear: Escouade updates it itself, every day (a switch in Settings › Harness). A Claude Code that cannot be found is reported with the command that reinstalls it.

## [1.8.4] — 2026-10-19

### Fixes

- With an online voice, Settings › Voice no longer shows Piper's settings: a single line says which local voice takes over if the service does not answer.

## [1.8.3] — 2026-10-19

### New

- The voice conversation can be read by an online service (OpenAI, ElevenLabs, Mistral, Deepgram, Groq), with the same key as its dictation, and the voice of your choice, your account's own included. For dictation as for the voice, you also choose each service's model.

## [1.8.2] — 2026-10-19

### New

- In a voice conversation, the agent asks its questions aloud, one at a time, even when it has several. The choices show on screen: you answer by saying the choice, its number or your own answer.

### Changes

- Settings › Harness no longer has a "Sign in…" button: each account signs in from Settings › Accounts.
