# Privacy

Escouade works locally, on your machine. The only thing that leaves it on its own, and only if you accepted, is usage statistics, with nothing that identifies you. This page says what is sent, what never is, where it goes, and how to stop it.

## In short

- The statistics are **optional**. Escouade asks at the first start (existing installs too, after an update from a version that had none) and sends nothing until you answer “Share statistics”.
- They contain **nothing that identifies you**: an id drawn at random, never derived from your machine or from you, and no name, path, code, conversation or Git data. Because this id lasts, the GDPR treats them as pseudonymous personal data: so you have rights, below.
- They go to a server **of ours**, hosted in France. No third party receives them.
- You can stop them at any time, and the id is then deleted.

## What is sent

Every event carries its name, its time (UTC, to the millisecond), a session id, the install id, and the system information described below.

### The events

| Event | When | Props added |
|---|---|---|
| `app_started` | At each start | The interface language (`fr` or `en`), the number of Claude accounts, the number of projects, whether “Agents can drive Escouade” (the MCP server) is on (yes or no), whether a project is in WSL (yes or no) |
| `app_active` | Once a day while the app stays open | — |
| `project_created` | A project is created | — |
| `agent_created` | An agent is created | `worktree`: with a worktree or not (yes or no) |
| `message_sent` | A message is sent to an agent | — |
| `ticket_created` | A ticket is created | — |
| `ticket_started` | A ticket is started | `by`: `user` or `autopilot` |
| `ticket_approved` | A ticket is approved | — |
| `committed` | A commit is made | — |
| `merged` | A branch is merged | — |
| `git_sync` | A Git sync | `op`: `fetch`, `pull` or `push` |
| `branch_created` | A branch is created | — |
| `terminal_opened` | A terminal is opened | — |
| `launch_started` | A launch command starts | — |
| `test_run_started` | A test starts | — |
| `claude_code_view` | An agent switches to the Claude Code view | — |
| `file_saved` | A file is saved in the editor | — |
| `code_searched` | A search in the code | — |
| `mcp_enabled` | “Agents can drive Escouade” is turned on | — |
| `integration_connected` | A service is connected | `service`: `jira`, `trello`, `github`, `linear` or `clickup` |
| `tickets_imported` | Tickets are imported | `service`: `jira`, `trello`, `github`, `linear` or `clickup` |
| `account_created` | A Claude account is added | — |
| `remote_control_enabled` | Remote control is turned on, by hand or by a setting | `from`: `agent` (right-click on an agent), `project` (a project’s setting) or `global` (the setting for all agents) |
| `dictation_used` | A dictation gave text | `target`: `composer` (the message field), `terminal` (the Claude Code view) or `ticket` (a ticket’s description); `model`: the Whisper model (`tiny`, `base`, `small`…), or the service of online dictation (`openai`, `groq`, `mistral`, `elevenlabs`, `deepgram`); `online_model`: online, the service’s model (`gpt-4o-mini-transcribe`…) |
| `voice_conversation` | A voice conversation starts | `lang`: the conversation’s language (`fr`, `en`); `voice`: the voice that reads (`fr_FR-siwis-medium`…, or the service’s); `engine`: what makes it (`piper`, or the online service); `model`: its model (`piper`, or the service’s); never what is said or read |

An event says that a feature was used, not on what: never the name of the project, the agent or the branch, nor anything that was written.

### The system and the install

Every event also carries:

| Information | Example or detail |
|---|---|
| Install id | A random UUID v4 drawn when you agree, kept in `~/.escouade/telemetry-id`, deleted when you say no |
| Session id | A random number, renewed after 4 hours without any event |
| Escouade’s version | For example `1.8.0` |
| Development build | Whether the app is a development build (yes or no) |
| System and its version | For example Windows 10.0.26200, macOS 15.1 |
| System locale | For example `fr-FR` (10 characters at most) |
| Web engine and its version | WebView2, WebKit or WebKitGTK |

## What is never sent

- Names: yours, and those of your projects, agents, branches and accounts.
- E-mail addresses.
- Paths and file names.
- Your code.
- The content of conversations and of the instructions given to agents.
- The titles and texts of your tickets.
- Git data: commits, remotes.
- Tokens and secrets.
- Anything you type.
- What you dictate: the microphone’s sound and the text Whisper makes of it stay on your machine. The transcription happens there; only the model is downloaded, once, from Hugging Face (`huggingface.co`), through the proxy of the settings. Online dictation, off by default, is apart: see “Online dictation, if you turn it on”, below.
- What you say to an agent in a voice conversation: the transcription follows the dictation’s: by Whisper on your machine, or, when dictation is online, by its service (see “Online dictation, if you turn it on”, below). What it answers aloud is read by Piper on your machine, unless you choose an online voice: see “The conversation’s online voice, if you turn it on”, below. Only Piper’s voices are downloaded, once each, from Hugging Face, through the proxy of the settings.

## The tool and where the data lives

The statistics are received by [Aptabase](https://aptabase.com), an open source analytics tool ([its source](https://github.com/aptabase/aptabase)). It is **self-hosted by Escouade’s author**, at `aptabase.gagnaire.dev`, on a Scaleway server in a datacenter in Paris (France). No third party receives this data, and nothing goes through Aptabase’s own cloud.

On the server side, Aptabase does not store the IP address (the one this site’s server keeps in its logs is explained below). It uses it on reception to derive the country and, when it can, the region, and, combined with a salt that changes every day, to compute a daily visitor hash.

## How events leave

- Events wait in memory, never on disk.
- They are sent every minute, in batches of 25, and when you quit the app (3 seconds at most), over HTTPS to `https://aptabase.gagnaire.dev/api/v0/events`.
- Offline, up to 1,000 events wait in memory, the oldest being dropped first. If you quit the app offline, they are lost.

## How long

The server deletes events automatically after 25 months.

## Saying yes, no, or changing your mind

- **At the first start**, a “Help improve Escouade?” dialog offers “Share statistics” or “No thanks”, with a “What is sent, in detail” link to this page. If you close it without answering (with `Esc`), it comes back at the next start.
- **In the settings**, “About” tab, the “Usage statistics” line has a switch and an “In detail” button that opens this page. It is off until you accept in the first-start dialog.
- **When you turn it off**, the install id is forgotten, its `telemetry-id` file is deleted, and events still waiting are dropped. If you turn it on again, a new id is drawn: the install counts as a new one.

See also [the “About” settings](settings-about).

## Tests and demos

An instance started with a separate data folder (the `ESCOUADE_DATA_DIR` variable, used by tests and demos) never sends anything. See [local data](data).

## Why, and on what basis

To measure the use of Escouade: how many people use it (users active over 30 days, counted by install id), which versions are still in use (to know when an old one can stop being supported), on which systems, and which features are used, in order to decide what to improve.

The legal basis is your **consent** (article 6.1.a of the GDPR, and article 82 of the French Informatique et Libertés law for the id stored on your machine). You can withdraw it at any time, in Settings › About, without affecting what was done before.

## Pseudonymous data

The id is drawn at random. It is derived from nothing about your machine or about you, and the statistics contain neither your name, nor your e-mail address, nor your projects. But because it stays the same from one launch to the next, it can recognize an install: the GDPR therefore treats these statistics as pseudonymous personal data. That is why this page also gives the controller, the legal basis, the recipients and your rights.

## The controller

GAGNAIRE DEVELOPPEMENT, SAS, 2 rue du Taillis, 33620 Cavignac, France, the publisher of Escouade (see the [legal notice](/en/legal/)). Contact: [guillaume@gagnaire.dev](mailto:guillaume@gagnaire.dev). A data protection officer is not required for this processing: there is none.

## Who receives the data

Only GAGNAIRE DEVELOPPEMENT. Its processor is Scaleway SAS, which hosts the server in Paris. No data leaves the European Union.

## Server logs

The server that receives the statistics and the one that serves this site sit behind a reverse proxy (Traefik, run by Coolify). It writes an access log of every request it receives, including the statistics sent by the app and the visits to this site: IP address, date, address requested, browser (User-Agent) and referring page (referer).

- **Why**: the security and operation of the server. The legal basis is legitimate interest (article 6.1.f of the GDPR).
- **Where and how long**: these logs stay on the server, are not used for anything else, and are not analysed. They are rotated every day and kept one year at most: past that, they are deleted.
- The site’s own web server (nginx) keeps no access log.

## Your rights

You have the right to access your data, to have it rectified or erased, to restrict its processing, to object to it, to take it with you (portability), and to withdraw your consent.

To exercise them, write to [guillaume@gagnaire.dev](mailto:guillaume@gagnaire.dev) giving the id in your `telemetry-id` file (in `~/.escouade/`), or open a [GitHub issue](https://github.com/guillaume-gagnaire/escouade/issues). The answer comes within one month.

If you think your rights are not respected, you can lodge a complaint with the CNIL: [www.cnil.fr/fr/plaintes](https://www.cnil.fr/fr/plaintes), 3 place de Fontenoy, TSA 80715, 75334 Paris Cedex 07, France.

## The code

The code that sends the statistics is open: [`telemetry.rs`](https://github.com/guillaume-gagnaire/escouade/blob/main/src-tauri/src/telemetry.rs). What this page says has to be true there.

## What leaves Escouade besides the statistics

Escouade has no account or server of its own for your projects, conversations or settings: they stay in `~/.escouade/` ([local data](data)). It does talk to a few services, and only for what you ask of it:

- **Claude Code**, which it runs on your machine, sends its requests to Anthropic as it would in a terminal. Escouade also queries Anthropic’s usage service to show your quotas ([quotas](usage)).
- **Updates**: Escouade reads the `latest.json` file of the latest release on GitHub, then downloads the new version ([updates](updates)).
- **The integrations** you connect (Jira, Trello, GitHub, Linear, ClickUp, GitLab, Azure Boards): Escouade talks to those services with your token ([integrations](integrations)).
- **Online dictation**, if you turn it on: see just below.

## Online dictation, if you turn it on

By default, dictation happens on your machine and nothing you say leaves it. In Settings › Voice › “Transcribed by”, you may choose “Online” and a service, with **your own** API key: it is off until you choose it.

- **What leaves**: the sound of each dictation (a WAV file, mono, 16 kHz), once you end it, with the spoken language of the settings and your key. Nothing else: neither the project, nor the code, nor the conversation with the agent, nor the text of tickets. In a voice conversation, the sound of each sentence you say leaves the same way, once its end is found by the silence.
- **In real time**: with “Real-time transcription” (ticked by default, but with Groq), the sound leaves as you speak, through an encrypted stream (WebSocket) to the same service, with your key and through the same proxy, rather than once the dictation or the sentence is over. In a voice conversation, only while you speak: one stream per sentence, opened when your voice starts, closed a moment after your last word; never the silence between two sentences, nor the agent’s voice, nor what the microphone hears while it works. For the dictation, what the microphone hears while it lasts, as before. The real-time models: `gpt-live-transcribe` (OpenAI), `voxtral-mini-transcribe-realtime-2602` (Mistral), `scribe_v2_realtime` (ElevenLabs), the model chosen (Deepgram).
- **Where**: to the one service chosen, through the proxy of the network settings, with the model chosen in Settings › Voice (by default these):

| Service | Address | Dictation model | Voice model |
| --- | --- | --- | --- |
| OpenAI | `api.openai.com` | `gpt-4o-mini-transcribe` | `gpt-4o-mini-tts` |
| Groq | `api.groq.com` | `whisper-large-v3-turbo` | `canopylabs/orpheus-v1-english` |
| Mistral | `api.mistral.ai` | `voxtral-mini-latest` | `voxtral-mini-tts-2603` |
| ElevenLabs | `api.elevenlabs.io` | `scribe_v2` | `eleven_flash_v2_5` |
| Deepgram | `api.deepgram.com` | `nova-3` | `aura-2` |

- **What it does with it**: the service sends the text back, which Escouade puts in the field dictated into. It handles the sound under its own terms, those of your account with it: that is between it and you, Escouade receives none of it.
- **Your key**: kept in the system keychain (Windows’ Credential Manager, macOS’ Keychain), never in Escouade’s files nor in its log. On Linux, where Escouade has no keychain, it is not saved. One key per service: the same serves dictation and the conversation’s voice.
- **The list of models and voices**: with your key saved, Escouade asks the service for its models (and its voices, your account’s included, when its API lists them); only the key leaves. The list is kept on your machine (`~/.escouade/voice/online-catalog.json`) and read again when you refresh it.

## The conversation’s online voice, if you turn it on

By default, the voice conversation’s answers are read by Piper, on your machine. In Settings › Voice › Conversation › “Voice made by”, you may choose “Online”, a service of the table above and one of its voices, with the key of its dictation: it is off until you choose it.

- **What leaves**: the text of each passage read, sentence by sentence, with the model, the voice, the conversation’s language, the speed when the service takes it, and your key. Not the sound of the microphone: what you say is transcribed as your dictation is (above). Neither the project, nor the code, nor the rest of the conversation.
- **Where**: to the one service chosen, through the proxy of the network settings.
- **What it does with it**: it sends the sound back, played on your machine. It handles the text under its own terms, those of your account with it.
- **If the service does not answer**: the conversation says so, and Piper reads on if one of its voices is downloaded; else it pauses. Escouade’s log never holds your key nor the text read.

## The store

[store.escouade.dev](https://store.escouade.dev), Escouade’s plugin marketplace, has accounts for plugin **authors**, and for them only: the app’s users have no account there and give it nothing.

- **Signing in** goes through GitHub, by the “Escouade Store” GitHub App, read-only, on the repositories the author chooses only. The store gets from GitHub your numeric id, your login, your display name and your avatar’s address, read again at each sign-in. GitHub, Inc. is in the United States: this information comes from it at your request, when you sign in, under the EU–US Data Privacy Framework GitHub relies on; the store sends it nothing it does not already know.
- **What is kept, and how long**:

| Data | Why | How long |
|---|---|---|
| GitHub id, login, display name, avatar address | The account, the author’s page | Until the account is deleted, then 30 days in the backups |
| E-mail, optional, typed by you and checked by a link | The notices (rejection, review done, report) | Same; erased as soon as you remove it |
| Session (the hash of a random token, its date) | Staying signed in | 30 days |
| The session’s GitHub access token, encrypted | Reading the App’s installations during the session | 8 hours at most; erased and revoked at GitHub on sign-out |
| Plugins, versions, checks, review summaries | The service | As long as the plugin exists |
| Journal of actions (sign-in, address, export, deletion, the App’s installations) | Security, disputes | 1 year |
| Reports | Moderation | 1 year after they are closed |

- **The legal bases**: performing the [authors’ terms](https://store.escouade.dev/en/terms) (article 6.1.b of the GDPR) for the account; legitimate interest (article 6.1.f) for the journal of actions and the reports.
- **One cookie only**, the session’s (`__Host-store_session`, `HttpOnly`, `Secure`, `SameSite=Lax`), strictly needed: no banner, no analytics. The avatars are drawn by the store (your initials): a visitor loads nothing from GitHub.
- **Where**: on Scaleway’s server in Paris, as this site; the backups in Scaleway’s object storage in Paris. The reverse proxy keeps its access log as for the site (see [server logs](#server-logs)).
- **Your rights**: your account’s page exports your data (a JSON file) and deletes your account; your name and avatar are corrected at GitHub and read again at your next sign-in. For the rest, and to object to the journal, write to [guillaume@gagnaire.dev](mailto:guillaume@gagnaire.dev).

## This site

This site’s code includes no cookie and no analytics tool. It only keeps your language choice in your browser’s local storage (the `escouade-lang` key), to leave you on the right language at your next visit. That value stays in your browser and is sent nowhere. As with the statistics, the server’s reverse proxy keeps an access log of your visits: see [server logs](#server-logs).
