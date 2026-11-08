# Troubleshooting

The messages you can read with Groq, as the app writes them, what they mean and what to do. Dictation’s show in place of the text; the voice’s, once, in the voice conversation.

## Dictation and voice

**“No Groq key is saved. Add it in Settings › Voice, or go back to local.”**
The keychain has no Groq key. Paste it in Settings › Voice (Install tab), or set “Transcribed by” to “Local (Whisper)”. On Linux, the key cannot be saved: stay local.

**“Groq refuses the key. Check it in Settings › Voice.”**
Groq answered 401 or 403: the key is wrong, revoked, or lacks the right asked for. Create another one at Groq, then “Replace”.

**“Groq refuses for now: too many requests or the quota is spent (…). Wait a little, or check the plan of your Groq account.”**
Groq answered 429. In brackets, what it said. Wait, or look at your plan and credit at Groq.

**“Groq does not answer: … Check the network and the proxy (Settings › Advanced › Network), or go back to local.”**
The request did not get through. What follows the colon says why:

- a network or proxy error: check [Settings › Advanced › Network](https://escouade.dev/en/docs/settings-network) and that `api.groq.com` can be reached;
- “timed out”: Groq did not answer in time;
- “this service’s plugin is gone”: the Groq plugin is off or uninstalled. Turn it back on in the Plugins view, or install it (Install tab);
- “The plugin dev.escouade.groq is not answering: …”: the plugin is not running or stopped. Its Log tab, in the Plugins view, says why;
- “Unreadable answer from the plugin dev.escouade.groq: …”: report it, with its log.

**“Groq did not transcribe the dictation (<code>): …”**
Groq answered something other than a text: its HTTP code and what it said (an unknown model, a refused sound, an outage). “an answer without text”: it answered with no transcript. Fix what it says on your account, change the model, or go back to local.

**“Groq did not read the passage (<code>): …”**
Groq refused to read a sentence: its HTTP code and what it said. “an answer without sound”: it answered with no audio. Check the model and the voice in the “Conversation” group.

### In the voice conversation

Groq’s message is followed by what the conversation does:

- “… The conversation goes on with the local voice (Piper).”: a Piper voice is downloaded, it reads on.
- “… The conversation is paused: fix it, or download a local voice in Settings › Voice, then resume.”: no Piper voice is there. Fix the cause, then “Resume”.
- “… The voice “<voice>” is not one of Groq: its default voice, “<default voice>”, stands in.”: Groq does not know the voice chosen (400, 404 or 422). Choose another, or empty the field.
- “Groq (<model>) does not read French: the local voice (Piper) reads the conversation.”: this model does not read the conversation’s language (Orpheus).
- “Groq no longer offers the model “<model>”: “<default>”, its default model, stands in.”

## In Settings › Voice

- **“No key saved: the service needs one.”**: paste your key (Install tab).
- **“The key was not saved: The key is empty.”**: the field was empty.
- **“The key was not saved: The system keychain refuses the key (…): it is not kept, and never in a file.”**: the keychain refused it, or there is none (Linux: “no system keychain here”).
- **“The key was not saved: unknown service groq”**: the Groq plugin is off or uninstalled. Turn it back on, then try again.
- **“The list could not be read: …”**: reading the models at Groq failed, with Groq’s message (above). Its documentation’s list stays.
- **“List of the Groq documentation: save your key to read its own.”**: no key yet.
- **“Groq no longer offers “<model>”: its default model, “<default>”, stands in.”**: choose another model, or keep the default one.
- **“The voice could not be played: …”**: “▶ Listen” failed; what follows is Groq’s message.

## The log

- **The plugin’s**: Plugins view, “Groq” page, Log tab. What Groq answered is noted there, the key taken out: “voice: Groq refuses the key: …”, “voice: Groq answered 429: …”, “speech: Groq answered <code>: …”.
- **The app’s**: `~/.escouade/app.log` (see the site’s [troubleshooting](https://escouade.dev/en/docs/troubleshooting)). It never holds your key, nor the text read.
