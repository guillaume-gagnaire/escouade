# The conversation

The conversation is where you talk to an agent and read what it does. This page describes the message field, attachments, autocompletion, how messages and tool calls are displayed, questions and permissions, and long conversations.

![The conversation of the agent “pagination-users”: a code block, the diff of an Edit tool call, and the message field](/images/en/chat.jpg)

On macOS, `Ctrl` becomes `⌘` for shortcuts.

## The header

The conversation header shows the agent’s name (a double-click renames it) and its status in a pill, then, below, its harness (when several are installed), `project / branch`, the number of its pull requests and, for a Codex agent, “Sandbox” (the tooltip says what its mode lets it do). Then:

- the agent’s figures, the label above the value: “Context” (for example “76.2k / 200k”, with a thin gauge under it: how much of the model’s window is already filled, with the percentage in the tooltip; the value takes on a warning color from 80%), “Tokens”, “Cost” (preceded by “≈” during a turn: it’s an estimate), “Files” (in the classic layout, a click shows the uncommitted files, see [git](git)), and “Duration”;
- a single action button: “Open a PR” while the agent’s branch has commits without a pull request (see [pull requests](git)), otherwise “Test” or “Prepare launch”, depending on the agent, which launches its worktree or asks the agent how to launch it (see [testing](testing));
- the two views, “Escouade” and “Claude Code”, as segments: the second runs the agent in Claude Code’s own interface (see [the Claude Code view](claude-code-view)). They don’t exist for a ticket agent or an archived agent;
- “…”, the agent’s menu, the same as the right click on its card: rename, duplicate, open the [editor](editor) or a [terminal](terminals) in its worktree, launch, open a PR, archive, delete…;
- two layout buttons: classic, or the conversation and the files side by side (`Ctrl+Shift+L`, see [the interface](interface)).

When the conversation narrows (side-by-side layout, small window), the header first drops “Tokens” and “Files”, then “Duration” and the text of its buttons, then its figures.

## Writing a message

### Sending

`Enter` sends the message, `Shift+Enter` starts a new line. The field grows with the text, up to 40% of the window’s height. An empty message with no attached file isn’t sent. The “Send” button does the same.

While the conversation is empty, the field says “Describe the task to give to Claude…”; after that, “Send a message to” followed by the agent’s name.

### While Claude works

You can write to Claude while it works. The message appears in the conversation with the note “sent during the turn”: Claude takes it into account at its next step, as in Claude Code. Writing to an agent also cancels its auto-resume after the usage limit.

To stop it, press `Esc` in the message field, or click “Stop” next to “Send”. The turn ends with “Interrupted”, the agent goes to “Done”, and no notification is sent. When a suggestions list is open, `Esc` closes it first.

### Answering with the field

When Claude is waiting for an answer to a question or a permission, the field’s text changes and what you write answers it:

- for a question, “Answer the question or write a free answer…”: your text is the answer;
- for a permission, “Tell Claude what to do instead (this denies the request)…”: your text denies the request and tells Claude what to do instead.

### Attachments

You can attach any file: an image, a PDF or a text file, but also an archive, a spreadsheet, a document, a video, an audio file… There are three ways to attach it: paste it (`Ctrl+V`, for example a copied screenshot), drag it onto the field, or click the paperclip (“Attach a file”). The file dialog has no filter: it shows every file. Each file appears above the field: a thumbnail for an image, otherwise a chip with an icon for its type (archive, spreadsheet, document, video, audio, code, or generic), its name, its size, and a cross that removes it. While a file is being copied (see below), its chip shows a percentage and a progress bar, and “Send” stays grayed out.

Escouade hands a file to Claude in one of two ways. What Claude reads directly stays **in the message**, within these limits:

| Type | Formats | Maximum size |
|---|---|---|
| Image | PNG, JPEG, GIF, WebP | 5 MB |
| PDF | `.pdf` | 18 MB |
| Text | Code, configuration, logs, CSV, Markdown, JSON… and files with no known extension such as `Dockerfile` or `.env.local` | 256 KB |

The files of one message don’t exceed 18 MB in all.

Everything else goes **by path**: any other format, a readable file that is too big for the message (an image over 5 MB, a PDF over 18 MB, a text file over 256 KB), or whatever would push the message past 18 MB. The message then gives Claude the file’s path: one more line per file, after your text (or alone, if you wrote nothing). Claude reads the file with its tools.

```text
Attached file: report.xlsx, C:\…\report.xlsx
```

The line follows the language of the texts written by the agents (`Fichier joint : …` in French, see [Application settings](settings-application)). In WSL, the path is the one Claude Code reads, for example `/mnt/c/…`.

A file is rejected if it exceeds the maximum size you set (none by default), if it’s a text file over 256 KB and you chose to refuse those, or if Escouade can’t read or copy it. It is named in a red message, and the other files are attached anyway. In the conversation, the message shows “2 images” and the names of the files; the lines of the files sent by path are there too, with your text. If Escouade can no longer find a file when you send (moved or deleted in the meantime), the message is refused, with the reason, and goes back to the field.

Attached files don’t go with an answer to a question or a permission: they stay ready for your next message. In the Claude Code view, Escouade’s message field isn’t shown: a file is attached by pasting it into the terminal or dropping it there, and it is always copied; see [the Claude Code view](claude-code-view). In a ticket’s window, files are always copied too; see [the Kanban](board).

### Copy or original file

A file sent by path is either the original file or a copy.

- **The original file** (the default, thanks to the “Attach the original file” setting): nothing is copied, Claude reads the file where it is. Escouade has to know its path, and it only does on Windows: the app itself reads the path of a file on disk dropped or pasted from Explorer, and the window can’t give it any other. A pasted screenshot isn’t a file on disk: it has no path. A path the app didn’t read itself is refused, when attaching as when sending the message, and a file in Escouade’s data folder is never attached by its path: it is refused. On Linux and macOS, there is no original path: files are copied. For a dropped file, Escouade doesn’t give Claude its folder: Claude Code asks for permission to read a file that is in none of its folders, and Escouade answers “allowed” on its own, for that file alone, at its original path, and only to read it (never to write to it). Any other request comes back to you as usual, including one for a tool other than reading (such as `Bash`). Claude reads the file as it is at that moment: if you changed it, it reads the changed version; if you moved or deleted it, it is no longer found. The chip of an original file carries a marker and the tooltip “Original file, read where it is”, so that you don’t take it for a copy.
- **A copy**: Escouade copies the file under its real name, in chunks of 4 MB at most, into its data folder (`attachments/<agent>/`, see [local data](data)), outside your repository. Claude Code reads that folder without asking. The copy is made when the original path is unknown (on Linux and macOS, and for a screenshot pasted from the clipboard, which isn’t a file on disk), when you turn off “Attach the original file”, for tickets (their agent reads the file later, when the original may have moved or vanished), and for the Claude Code view (its process is launched once, with no way to open one more folder for it).

> [!NOTE]
> That Claude Code reads the copies’ folder without asking has been checked with the real Claude Code (2.1.301). Escouade allowing an original file has been checked with a fake Claude Code in the automatic tests (the real one does send the message’s exact path in its request); the path translated under WSL has not been tried for real.

### Size, large text files, and settings

- **Size.** No limit by default. A file over 200 MB carries a warning (a “!” mark on its chip, whose tooltip says “Big file: 250 MB, copied to the disk”, or “… attached where it is, not copied” for an original file), but it is attached all the same. So is an empty file. You can set a maximum size: past it, the file is rejected.
- **Text file over 256 KB.** By default, it goes by path like the other files. You can have it rejected, as before, by setting “Text over 256 KB” to “Refused”; that holds in a ticket’s window and in the Claude Code view too.
- **Settings.** The five settings of the “Attachments” group (“Largest attached file”, “Warn above”, “Attach the original file”, “Text over 256 KB”, and “Purge copies after”) are in [Settings › Advanced › Attachments](settings-claude-code#attachments), with their defaults.

### Deleting attached files

- The cross of a chip deletes the copy, or stops the one in progress. For an original file, it only removes the chip.
- An agent’s copies go with it when you delete it. A project’s copies go when you delete it for good: closing a project keeps them with its history, and the size of that history (in Settings › All projects) counts them too, like the images of its tickets. A ticket’s copies go with the ticket.
- If you set the purge, Escouade deletes the copies older than the number of days you chose, at startup and then every hour. By default, nothing is purged.
- Escouade never deletes an original file.
- For each agent, Escouade keeps, with it, the list of original files it has allowed that agent to read (200 at most, the most recent). The interface doesn’t show this list yet and doesn’t let you remove a file from it; deleting the agent erases it.

### Dictating a message

The microphone, next to the paperclip, dictates the message instead of typing it. Two ways to talk:

- **one click** starts the dictation, **another** ends it;
- **a long press** dictates while the button is held down: letting go ends it.

The shortcut `Ctrl+Shift+Space` (`⇧⌘Space` on macOS) does the same, short or held, in the field that has the focus. While it listens, a panel shows the microphone’s level and the time; `Esc` cancels without writing anything.

The dictated text is **inserted at the cursor** (it replaces the selection, if any), spaced from the words around it. It is **never sent by itself**: you read it, fix it, then send it with `Enter`. A ticket’s description is dictated the same way (the microphone is next to “Attach”), and so is a harness’s prompt in its terminal view (the [Claude Code view](claude-code-view)): the microphone is at the top right of the terminal, and the text is typed there as a paste, not submitted.

The transcription happens **on your machine**, by Whisper (whisper.cpp), on a thread of its own: nothing you say leaves your machine. At the first dictation, Escouade offers to download the “base” model (141 MB), once, from Hugging Face, through the proxy of the [network settings](settings-network); its checksum is checked before it is kept, in `~/.escouade/voice/models`. After that, a sentence is transcribed in one or two seconds.

**Online, if you choose it.** For a machine too slow for Whisper, or a better quality, Settings › Voice › “Transcribed by” › “Online” sends the sound of each dictation, once you end it, to the service chosen: OpenAI, Groq, Mistral (Voxtral), ElevenLabs (Scribe) or Deepgram, with **your own** API key, paid per use on your account with it. It is off by default. Only the sound of the dictation leaves, through the proxy of the [network settings](settings-network); no model is to be downloaded. The key is kept in the system keychain (not on Linux, where it cannot be saved), never in Escouade’s files nor in its log. With “Real-time transcription” (ticked by default, but with Groq), the sound goes while you speak: the text is written as you speak in the dictation’s panel, and the final text goes into the field a moment after the end; if the stream does not get through, the dictation goes whole, by request. When the key is refused or the service cannot be reached, the dictation says so, and nothing is written. The voice conversation follows this choice. The details are on the [Privacy](/en/privacy/) page.

[Settings › Voice](settings-voice) chooses the microphone, who transcribes (“Local (Whisper)” and its model, from `tiny`, 74 MB, to `large-v3-turbo`, 1.5 GB, or “Online” and its service), the “Spoken language” and the “Shortcut”.

When the microphone is refused or missing, Escouade says so, with your system’s steps (macOS, Windows or Linux): see [troubleshooting](troubleshooting#dictation-or-the-conversation-hears-nothing).

The dictation stops by itself after ten minutes, when the microphone is unplugged (what was said is transcribed), when the window is closed or reloaded, and when the app quits.

### Talking with the agent (voice conversation)

The **Conversation** button, left of the microphone in the message field of the Escouade view, holds a spoken conversation with the agent, hands off the keyboard. It works with every harness (Claude Code, Codex, OpenCode, Copilot CLI); a harness’s terminal view only has the dictation’s microphone. A turn goes like this:

1. you speak; when you stop, what you said **goes by itself** to the agent after a short silence (“Delay before sending”, 1.2 s by default);
2. the agent works and answers as usual, in the conversation;
3. it ends its answer with **a short passage written to be read**, less wordy and less technical: Escouade shows it apart under the answer (“Read aloud”) and **reads it aloud**. Without that passage, Escouade says “Done. The answer is in the conversation.”;
4. then it listens again.

During the conversation, **it fills Escouade’s window**: in the middle, a glowing orb that follows the sound, the agent’s voice when it speaks (its edge breathes and its halo grows with the level of the sound read) and your microphone when it listens, with an arc turning while it transcribes and while the agent works; below, the state (“Listening”, “Transcribing on your machine…”, “Alice is working…”, “Alice is answering”). At the bottom, **subtitles** as in a cinema: what you said, as soon as it is transcribed, then what the agent says, sentence by sentence, as it is read. The whole history stays in the conversation. `Esc` or “Stop” leaves the mode and closes the microphone.

- **Cutting in**: speak while it reads, the reading stops at once and Escouade listens. The microphone does not take the agent’s voice for a sentence of yours: while it reads, it only tells whether you speak over it, above the echo of the speakers. Headphones remain the most comfortable.
- **Permissions**: when the agent asks for a permission, Escouade reads it, then your spoken answer answers it: “yes”, “okay” or “go ahead” allow, “no” denies, “always” always allows. If Escouade does not understand, the conversation **pauses** and says so: answer in the window, then “Resume”.
- **Questions, one at a time**: when the agent asks questions (Claude Code often asks up to four at once), Escouade asks them **one at a time**: the voice says the question (“Question 2 of 3. Which tests? Several answers possible.”), and **its choices show** under the orb, numbered, with their description; the voice does not read them. Answer aloud: an option by its name (near enough: “Postgres” does for “PostgreSQL”) or by its rank (“the second”, “2”, “the last one”); for several choices, several (“the first and the third”, “both”, “all of them”, “none”); anything else you say is your **free answer**. You can also click a choice, or type its number (for several choices, tick them, then “Confirm” or `Enter`). What was understood lights up a moment (a free answer: “Your answer: …”) and shows ticked in the card of the question, then comes the next question; after the last one, all the answers go to the agent together. “Repeat” asks the question again; “cancel” or “never mind” stops the questions and leaves the card to be answered in the window. An answer not understood (two options for a single choice, a rank that does not exist): Escouade says so and asks the question again, twice at most, then pauses; “Resume” asks it again, the answers already given stay ticked.

By default, everything happens **on your machine**: the transcription by Whisper, with the dictation’s model, and the voice by **Piper**, linked into Escouade; unless you have the voice made by an online service (OpenAI, ElevenLabs…), with the key of its dictation: only the text of the passages read then goes to it, and if it does not answer, Piper takes over or the conversation pauses. The transcription follows the dictation’s: with “Transcribed by” › “Online”, each sentence you say goes to the dictation’s service, with its key, and no Whisper model is asked for; with “Real-time transcription”, it goes while you say it, is written under the orb as you speak, and is ready a few hundred milliseconds after your last word (nothing goes during the silence, nor while the agent speaks or works); if the service refuses the key or does not answer, the conversation says so, as the dictation does, and listens again. At the first conversation, Escouade offers to download what is missing: the Whisper model (local), then the voice (60 MB for the default one), once, from Hugging Face, their checksum checked. The conversation is held in French or English: the spoken language of the settings, else the interface’s.

[Settings › Voice](settings-voice#conversation), “Conversation” group, chooses what makes the voice (“Voice made by”: Piper, or an online service with its model and voice), the voice of each language among those whose license allows it (7 in French, Fuze and its 14 styles among them, 29 in English), its speaker, the speed, the volume and the “Delay before sending”.

### Mentioning a file with @

Type `@` at the start of a message or after a space, then part of the name: Escouade lists up to 12 files from the agent’s folder (its worktree if it has one), with their folder. `↑` and `↓` choose, `Enter` or `Tab` (or a click) insert `@path/to/file`, `Esc` closes the list. The search is approximate: a few letters of the name are enough. It covers the files git knows (tracked, or untracked but not ignored); outside a git repository, the folder is walked without dependency and build folders.

### Running a command with /

At the very start of a message, `/` opens the list of commands that Claude Code announces for this agent. Those whose name starts with what you type come first, then those that contain it; the list shows 12 at most, and each line gives the argument hint and the description. The keys are the same as for `@`, and `Enter` inserts `/command` followed by a space. If the agent’s process is stopped, it is restarted to get the list.

### Recalling the last message

`↑` in an empty field puts your last message back: you can send it again as is or edit it. The original stays in the conversation.

### Drafts

What you write is kept separately for each agent: switching to another agent and coming back finds your text. The text also survives a restart, an update, or a reload of the window (it is saved a moment after your last keystroke). Attachments stay in memory: their chips disappear on restart (the copy of a file sent by path stays on disk, with the agent’s other copies). A draft is cleared when the message goes out, or when the agent is deleted. If sending fails, the message comes back into the field and an error is shown.

### Model, effort, mode, and account

The bar under the field sets the agent’s model, effort, and permission mode, and its account when you have several. Everything is explained on [the agents page](managing-agents#model-effort-and-permission-mode).

### While a worktree is being set up

When a new agent’s worktree is being set up, the conversation shows “Setting up the worktree” with the current step (“Show output” shows what it writes). Your message is put on hold (“Waiting for the setup…”) and goes out when the setup is done; only one message can wait at a time.

## Reading the conversation

### Messages

Your messages are in bubbles on the right, Claude’s on the left with a “C”. Claude’s text appears as it is written, with “Claude is working…” while the turn lasts. It’s rendered as Markdown: headings, lists, tables, quotes, links (a click on an `http`, `https`, or `mailto` link opens it in your browser). HTML written in a reply is displayed as text. A Markdown image (`![caption](address)`) whose address is a path of the machine (absolute, or relative to the agent’s folder) shows: Escouade reads the file as a file sent (an image recognised by its content, 10 MB at most, never of a private project), and a click opens it in the viewer. An image of the web never loads by itself: an agent that read a rigged page or file could write a secret in its address, which loading it would send. It stays a label, its caption and its domain (“image from example.com”), with “Show image”: on that click only, Escouade downloads it (`https` only, 10 MB at most, an image recognised by its content, without a cookie or anything that tells who asks, through the proxy of the settings) and shows it. The rest (an `http` address, a path that is no image or no longer there) keeps its caption, as quiet text.

Code blocks have a header with their language (“text” when none is given) and a “Copy” button that copies the code, then shows “Copied ✓”. Once the text is complete, the code is highlighted for common languages: TypeScript, JavaScript, JSON, Rust, Python, Bash, PowerShell, CSS, HTML, Svelte, Vue, Markdown, YAML, TOML, SQL, Go, Java, Kotlin, C, C++, C#, PHP, Ruby, Swift, diff, Dockerfile, GraphQL… Others stay plain text.

### Thinking

When Claude thinks, a “Thinking” line, preceded by a chevron, appears, with “…” while it goes on. A click unfolds it.

### Tool calls

The tools of a turn form a thread, tied by a line, with no box around each one. Each tool Claude uses takes one line: an icon of what it does (read, edit, search, a command, a subagent, Escouade), the tool’s name (`Read`, `Edit`, `Bash`, `Grep`…), what it acts on (the command, the file’s path relative to the agent’s folder, the pattern searched, the address), and on the right a short result: for a command, the last line of its output (or a check mark if there is none); for a read, the number of lines; “14 results” or “no results” for a search; “+24 −3” for an edit; “interrupted” or “error”. A circle spins while the tool runs. Hover over the line to read its full text.

A click on the line (or `Enter` when it has focus) unfolds it:

- **File edit** (`Edit`, `Write`, `MultiEdit`): the diff, with numbered lines, in green and red. It’s unified or side by side depending on the choice you made in the diff viewer (“Unified” or “Side by side”, see [git](git)). Past 400 lines, it stops at “… N more lines”. The file’s path is a link: it opens the file in the editor, at the first changed line.
- **Command** (`Bash`, `PowerShell`): the command after `$`, then its output.
- **Task list** (`TodoWrite`): the tasks, each with a circle, ticked once it is done.
- **Subagent** (`Agent`): what it was asked, its own tool calls (nested lines, which you can unfold too), and its last text. The line says how many tools it used, and its button at the end of the line opens the subagent’s conversation in full size, in place of the agent’s (see [a subagent’s conversation](plan#the-conversation-of-a-subagent)).
- **Other tools**: their result, otherwise the call’s parameters.

Tools that only reread the task list (`TaskList`, `TaskGet`) are displayed in softer shades. The tools of Escouade’s MCP server carry an “Escouade · …” badge (see [the MCP server](mcp)).

### Files the agent sends

Claude Code can send files to the user (its `SendUserFile` tool): a screenshot, a render, a report. Escouade shows them in the conversation, in a “Sent by Claude” card that says how many images and files there are:

- the **caption** the agent wrote, first;
- the **images** (PNG, JPEG, GIF, WebP, SVG) as thumbnails, on a row that wraps. A click on a thumbnail opens it full size in a viewer: “Previous image” and “Next image” (or the arrow keys) go from one to the next, `Esc` or a click beside it closes it. When the agent asked to attach them rather than show them, they are listed as files, with a small thumbnail that opens the viewer too;
- the **other files** (PDF, HTML, text, archive…) as rows, with their name, their size, “Open” (with the system’s default application) and “Show in folder”. A program or a script (`.exe`, `.bat`, `.ps1`, `.msi`, `.lnk`, `.sh`, `.app`, `.dmg`…, or a file marked executable on macOS and Linux) only has “Show in folder”: Escouade never runs what an agent sends.

Escouade reads the files, never the window. An image is recognised by its content, not its extension: a `.png` file that is no image is a file like any other. The images are copied when they are sent into Escouade’s data folder, with the agent’s attached files, since they often live in a temporary folder of its worktree: the card still shows them once that folder is gone, and after a restart. The copy goes with the agent, or with the other copies after “Purge copies after”. An image over 10 MB has no preview: it is listed as a file and not copied; one sending copies 50 MB of images at most, the next ones are read where they are. The other files stay where they are: moved or deleted since, a file is marked “File not found”, without its buttons. A file of a private project (see [private projects](mcp#private-projects)) is neither read nor opened: the card says “File of a private project, not shown”.

When the agent sends files on its own, without you asking, you get a notification, as for a question. While it sends them, its card in the list says for example “Sent 2 images and 1 file”.

The other harnesses (Codex, Copilot CLI, OpenCode, Cursor CLI, Mistral Vibe, Antigravity CLI) have no tool to send a file to the user: their images go through the Markdown of their messages, below.

### Subagents, background tasks, and other sessions

What Claude Code passes on to Claude by itself isn’t mistaken for a message from you: a “Background task completed” line (or “failed”, “stopped”) with its summary, a “Report from the subagent” block (with the label of its task) that holds its report in Markdown, or “Message from another Claude session”. A message sent from claude.ai or the Claude app (see [remote control](remote-control)) is in a bubble like yours, with the note “from claude.ai”.

### The end of a turn and its cost

Between two turns, a line gives the duration, the tokens, and the cost: “1m 20s · 12k tokens · $0.14”. An interrupted turn says so (“Interrupted”). When the agent is “Done”, the last turn is shown as a card: “Task done”, with the duration, the tokens, the cost, and “N files edited”, then the list of those files with their added and removed lines (a click opens the file in the editor).

A turn that fails shows a red card, “The turn ended with an error”, with the message. When the cause is the usage limit, the card announces the auto-resume (“Auto-resume at 3:00 PM”) with a “Cancel auto-resume” button, and offers to resume on another account, for example “Resume on Pro” (see [accounts](accounts)).

### Kanban and test cards

A ticket agent, or an agent you asked to prepare its launch, ends its turns with a block that the conversation shows as a card rather than as text: “Criteria report” (the ticket’s criteria met, for example `2/3`, with a note if any), “What was done” (up to eight lines), and “Test launch” (the setup commands and the processes, with a “▶ Test” button when the worktree can be launched). See [the Kanban](board) and [testing](testing).

### Notices

Short lines, outside the messages, report what happens to the session: “Context compacted”, “New Claude conversation (context cleared)”, “Claude session not found: a new session will start with your next message.”, or, in red, “Claude Code stopped (code 1)”. When the agent is in error, a banner adds: “The agent stopped. Send a message to restart Claude on the same session.”

## Questions and permissions

When Claude needs you, the conversation shows an amber card, the agent goes to “Question”, and you’re notified (see [notifications](notifications)). You answer with the mouse, the keyboard, or by writing in the field.

When the request comes from a subagent, the card is headed by “Request of the subagent” and its name, with an “Open its conversation” button. You can answer it here or in the subagent’s conversation, where the same card appears (see [a subagent’s conversation](plan#the-conversation-of-a-subagent)); the answer holds on both sides.

### Questions

The “Claude is waiting for your answer” card shows the question and one button per option. Hover over an option to read its description; it’s also written under the buttons.

- A single-choice question is answered with one click.
- When there are several questions, or a multiple-choice question (circles to tick), the answer is confirmed with the “Submit” button, enabled once every question has an answer.
- You can also answer freely: what you write in the field (“or answer freely in the field below”) is the answer to every question on the card.

Once answered, the card says “Claude asked you a question”, the question and its choices, the one taken marked in the project’s color (a free answer is shown alone), then “Answered”.

### Permissions

The “Claude is asking for permission” card shows the tool and what it wants to do (the command, the file, the address…), with the description and the reason Claude Code gives.

| Button | Effect |
|---|---|
| Allow | Allows this action. |
| Always allow | Allows and applies the rule Claude Code suggests with the request, so this kind of request is no longer asked. Absent when Claude Code suggests none. |
| Deny | Denies. Claude receives “The user refused this action.” To deny with an explanation, write in the field instead. |

When Claude Code would deny the request by default, the “Deny” button is the main button, and only a click on “Allow” allows it: no keyboard shortcut does. For a tool of Escouade’s MCP server, the card lists every parameter (up to 2,000 characters each), so you read what goes out.

In Plan mode, the request is “Claude proposes a plan”: the plan is shown as Markdown, and the buttons are “Approve the plan”, “Approve and accept edits” (the plan is approved and the file edits that follow are accepted), and “Keep planning”. The last one tells Claude to keep planning; if you’d rather say what’s wrong, write it in the field.

A handled request becomes a quieter card: the tool and its target (“Bash npm test”), then “Allowed”, “Denied”, “Allowed for this session” or “Always allowed”, with a green tick or a red cross. When another agent answered, through the [MCP server](mcp), the card says so: “by chef-escouade” and the “via Escouade” pill.

### With the keyboard

| Keys | Effect |
|---|---|
| `Ctrl+Enter` | Allows the pending request. For a question, confirms the answer when it’s complete, otherwise moves on to the next question that has no answer. |
| `Ctrl+Shift+Enter` | “Always allow” (or “Approve and accept edits” for a plan). |
| `Alt+1` … `Alt+9` | Picks the nth option of the question (ticks or unticks for a multiple choice). The keys are shown on the buttons. |
| Some text, then `Enter` | Answers the question freely, or denies the permission with that text. |

These keys work when the focus is in the conversation or nowhere; they leave their place to a terminal, the editor, and an open window. When several requests are waiting, the keyboard answers the first. A half-second guard protects against wrong answers: for 500 ms after a request has become the one the keyboard answers, these keys (and `Enter` in the field) do nothing, so that a `Ctrl+Enter` meant to send your message doesn’t allow a request you haven’t read; your text stays in the field.

You can also allow or deny without opening the agent, from [the overview](overview).

## Long conversations

A long conversation first shows only its last 80 items (messages, tool calls, cards). A “Show the previous 80 messages” button at the top adds the 80 before; the view stays on what you were reading. The conversation follows the bottom as long as you haven’t scrolled up; once you have, a “↓ New messages” button takes you back to the bottom when there is something new. Each agent reopens where you left it.

A result of [the search in conversations](overview#searching-conversations) opens the agent on the message found, even an old one, and highlights it for a moment.

An archived agent opens read-only: an “Agent archived.” banner and a “Restore” button replace the message field.
