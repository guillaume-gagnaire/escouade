# Troubleshooting

The messages you can read with Mistral, as the app writes them, what they mean and what to do. Dictation’s show in place of the text; the voice’s, once, in the voice conversation.

## Dictation and voice

**“No Mistral key is saved. Add it in Settings › Voice, or go back to local.”**
The keychain has no Mistral key, and no active Vibe account lends its own. Paste it in Settings › Voice (Install tab), or set “Transcribed by” to “Local (Whisper)”. On Linux, the key cannot be saved: stay local.

**“Mistral refuses the key. Check it in Settings › Voice.”**
Mistral answered 401 or 403: the key is wrong, revoked, or lacks the right asked for. Create another one at Mistral, then “Replace”.

**“Mistral refuses for now: too many requests or the quota is spent (…). Wait a little, or check the plan of your Mistral account.”**
Mistral answered 429. In brackets, what it said. Wait, or look at your plan and credit at Mistral.

**“Mistral does not answer: … Check the network and the proxy (Settings › Advanced › Network), or go back to local.”**
The request did not get through. What follows the colon says why:

- a network or proxy error: check [Settings › Advanced › Network](https://escouade.dev/en/docs/settings-network) and that `api.mistral.ai` can be reached;
- “timed out”: Mistral did not answer in time;
- “this service’s plugin is gone”: the Mistral plugin is off or uninstalled. Turn it back on in the Plugins view, or install it (Install tab);
- “The plugin dev.escouade.mistral is not answering: …”: the plugin is not running or stopped. Its Log tab, in the Plugins view, says why;
- “Unreadable answer from the plugin dev.escouade.mistral: …”: report it, with its log.

**“Mistral did not transcribe the dictation (<code>): …”**
Mistral answered something other than a text: its HTTP code and what it said (an unknown model, a refused sound, an outage). “an answer without text”: it answered with no transcript. Fix what it says on your account, change the model, or go back to local.

**“Mistral did not read the passage (<code>): …”**
Mistral refused to read a sentence: its HTTP code and what it said. “an answer without sound”: it answered with no audio. Check the model and the voice in the “Conversation” group.

### In the voice conversation

Mistral’s message is followed by what the conversation does:

- “… The conversation goes on with the local voice (Piper).”: a Piper voice is downloaded, it reads on.
- “… The conversation is paused: fix it, or download a local voice in Settings › Voice, then resume.”: no Piper voice is there. Fix the cause, then “Resume”.
- “… The voice “<voice>” is not one of Mistral: its default voice, “<default voice>”, stands in.”: Mistral does not know the voice chosen (400, 404 or 422). Choose another, or empty the field.
- “Mistral no longer offers the model “<model>”: “<default>”, its default model, stands in.”

## In Settings › Voice

- **“No key saved: the service needs one.”**: paste your key (Install tab).
- **“The key was not saved: The key is empty.”**: the field was empty.
- **“The key was not saved: The system keychain refuses the key (…): it is not kept, and never in a file.”**: the keychain refused it, or there is none (Linux: “no system keychain here”).
- **“The key was not saved: unknown service mistral”**: the Mistral plugin is off or uninstalled. Turn it back on, then try again.
- **“The list could not be read: …”**: reading the models at Mistral failed, with Mistral’s message (above). Its documentation’s list stays.
- **“List of the Mistral documentation: save your key to read its own.”**: no key yet.
- **“Mistral no longer offers “<model>”: its default model, “<default>”, stands in.”**: choose another model, or keep the default one.
- **“The voice could not be played: …”**: “▶ Listen” failed; what follows is Mistral’s message.
- **The voice field is free, with no list**: Mistral did not give its voices to your key. The plugin’s log says “voice: the voices of Mistral not listed: …”.

## Real time does not show

The text appears only at the end, though “Real-time transcription” is ticked: the stream did not open, or dropped, and the dictation went whole. Nothing shows; the app’s log (`~/.escouade/app.log`) says it: “voice: the live transcription by Mistral failed (…): the dictation goes whole, by request”. A proxy that refuses WebSockets is the most common cause.

## Mistral Vibe

**“The Mistral plugin (dev.escouade.mistral) is switched off or not loaded: turn it back on in the Plugins view to start a Mistral Vibe agent.”**
Turn it back on: the agent keeps its session.

**“The Mistral plugin (dev.escouade.mistral) is no longer installed: the Plugins view offers to install it, to start a Mistral Vibe agent.”**
Install it from the Plugins view.

**“Mistral Vibe not found. Install it or give its path in the settings.”**
Install it: `uv tool install mistral-vibe`.

**“Mistral Vibe didn’t open the agent’s session: … If it has no API key, run vibe --setup in a terminal.”**
For the machine’s account, `vibe --setup`; for another account, its key in Settings › Accounts and harnesses.

**“The Mistral Vibe account “<account>” has no API key: give it in Settings › Accounts and harnesses, “Change the key…”.”**
Neither the account nor Settings › Voice has a key. Give one to either.

**“That isn’t a Mistral API key: copy it whole from console.mistral.ai, API Keys.”**
The key pasted does not have its shape.

**“Mistral Vibe speaks ACP <version>, Escouade ACP <version>: update Mistral Vibe (uv tool upgrade mistral-vibe).”**
Update it.

**“Mistral’s rate limit reached: …”**
The turn stops on an amber card; the agent resumes by itself a minute later.

**“The Mistral Vibe session couldn’t be resumed: new session (context cleared)”**
Vibe did not find the session: the agent starts again from a new one.

**“Mistral Vibe has no configuration folder where to refuse the private projects’ paths: the agent doesn’t start.”** and **“The private projects’ paths couldn’t be refused to Mistral Vibe (…): the agent doesn’t start.”**
Escouade could not write the fence of private projects in the agent’s `VIBE_HOME`: the agent does not start rather than go past it. The reason is in brackets.

**““<file>” (<type>) can’t be attached: Mistral Vibe reads images and text files, not PDFs nor other files.”**
Attach an image or a text file.

The agent also reads “Refused by Escouade: <path> is in a private project, out of this agent's reach.” when it asks for a path of a private project: that is the fence holding.

## The log

- **The plugin’s**: Plugins view, “Mistral” page, Log tab. What Mistral answered is noted there, the key taken out: “voice: Mistral refuses the key: …”, “voice: Mistral answered 429: …”, “speech: Mistral answered <code>: …”.
- **The app’s**: `~/.escouade/app.log` (see the site’s [troubleshooting](https://escouade.dev/en/docs/troubleshooting)). It never holds your key, nor the text read.
