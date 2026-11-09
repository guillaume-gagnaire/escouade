# Troubleshooting

The messages you can read with OpenAI, as the app writes them, what they mean and what to do. Dictation’s show in place of the text; the voice’s, once, in the voice conversation.

## Dictation and voice

**“No OpenAI key is saved. Add it in Settings › Voice, or go back to local.”**
The keychain has no OpenAI key. Paste it in Settings › Voice (Install tab), or set “Transcribed by” to “Local (Whisper)”. On Linux, the key cannot be saved: stay local.

**“OpenAI refuses the key. Check it in Settings › Voice.”**
OpenAI answered 401 or 403: the key is wrong, revoked, or lacks the right asked for. Create another one at OpenAI, then “Replace”.

**“OpenAI refuses for now: too many requests or the quota is spent (…). Wait a little, or check the plan of your OpenAI account.”**
OpenAI answered 429. In brackets, what it said. Wait, or look at your plan and credit at OpenAI.

**“OpenAI does not answer: … Check the network and the proxy (Settings › Advanced › Network), or go back to local.”**
The request did not get through. What follows the colon says why:

- a network or proxy error: check [Settings › Advanced › Network](https://escouade.dev/en/docs/settings-network) and that `api.openai.com` can be reached;
- “timed out”: OpenAI did not answer in time;
- “this service’s plugin is gone”: the OpenAI plugin is off or uninstalled. Turn it back on in the Plugins view, or install it (Install tab);
- “The plugin dev.escouade.openai is not answering: …”: the plugin is not running or stopped. Its Log tab, in the Plugins view, says why;
- “Unreadable answer from the plugin dev.escouade.openai: …”: report it, with its log.

**“OpenAI did not transcribe the dictation (<code>): …”**
OpenAI answered something other than a text: its HTTP code and what it said (an unknown model, a refused sound, an outage). “an answer without text”: it answered with no transcript. Fix what it says on your account, change the model, or go back to local.

**“OpenAI did not read the passage (<code>): …”**
OpenAI refused to read a sentence: its HTTP code and what it said. “an answer without sound”: it answered with no audio. Check the model and the voice in the “Conversation” group.

### In the voice conversation

OpenAI’s message is followed by what the conversation does:

- “… The conversation goes on with the local voice (Piper).”: a Piper voice is downloaded, it reads on.
- “… The conversation is paused: fix it, or download a local voice in Settings › Voice, then resume.”: no Piper voice is there. Fix the cause, then “Resume”.
- “… The voice “<voice>” is not one of OpenAI: its default voice, “<default voice>”, stands in.”: OpenAI does not know the voice chosen (400, 404 or 422). Choose another, or empty the field.
- “OpenAI no longer offers the model “<model>”: “<default>”, its default model, stands in.”

## In Settings › Voice

- **“No key saved: the service needs one.”**: paste your key (Install tab).
- **“The key was not saved: The key is empty.”**: the field was empty.
- **“The key was not saved: The system keychain refuses the key (…): it is not kept, and never in a file.”**: the keychain refused it, or there is none (Linux: “no system keychain here”).
- **“The key was not saved: unknown service openai”**: the OpenAI plugin is off or uninstalled. Turn it back on, then try again.
- **“The list could not be read: …”**: reading the models at OpenAI failed, with OpenAI’s message (above). Its documentation’s list stays.
- **“List of the OpenAI documentation: save your key to read its own.”**: no key yet.
- **“OpenAI no longer offers “<model>”: its default model, “<default>”, stands in.”**: choose another model, or keep the default one.
- **“The voice could not be played: …”**: “▶ Listen” failed; what follows is OpenAI’s message.

## Real time does not show

The text appears only at the end, though “Real-time transcription” is ticked: the stream did not open, or dropped, and the dictation went whole. Nothing shows; the app’s log (`~/.escouade/app.log`) says it: “voice: the live transcription by OpenAI failed (…): the dictation goes whole, by request”. A proxy that refuses WebSockets is the most common cause.

## The log

- **The plugin’s**: Plugins view, “OpenAI” page, Log tab. What OpenAI answered is noted there, the key taken out: “voice: OpenAI refuses the key: …”, “voice: OpenAI answered 429: …”, “speech: OpenAI answered <code>: …”.
- **The app’s**: `~/.escouade/app.log` (see the site’s [troubleshooting](https://escouade.dev/en/docs/troubleshooting)). It never holds your key, nor the text read.
