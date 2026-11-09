# Utiliser

Ce que fait OpenAI une fois sa clé enregistrée (voir l’onglet Installer). L’usage de la dictée et de la conversation est sur le site : [Réglages › Voix](https://escouade.dev/docs/settings-voice) et [la conversation](https://escouade.dev/docs/chat#dicter-un-message).

## La dictée

Avec « Transcrite par » sur « En ligne » et « Service » sur « OpenAI », chaque dictée est transcrite par OpenAI :

1. Commence la dictée : le bouton du micro à côté d’un champ, ou le raccourci (`Ctrl+Maj+Espace`, `⇧⌘Espace` sous macOS).
2. Parle. Pendant la transcription, la pastille dit « Transcription par OpenAI… ».
3. Finis la dictée (un second appui, ou relâche le raccourci) : le texte arrive dans le champ. `Échap` l’annule.

La « Langue parlée » des réglages est donnée à OpenAI ; « Détection automatique » le laisse trouver la langue. Une dictée s’arrête seule au bout de 10 minutes. Une dictée de moins d’un dixième de seconde n’est pas envoyée.

### Le modèle de la dictée

Le menu « Modèle » liste les modèles d’OpenAI qui transcrivent. Par défaut `gpt-4o-mini-transcribe`. Sans clé, c’est la liste de sa documentation : `gpt-4o-mini-transcribe` (rapide), `gpt-4o-transcribe` (plus précis), `whisper-1`. Avec une clé enregistrée, la liste est celle de son API (`GET /v1/models`) : les modèles dont le nom contient `transcribe` (sauf `diarize`) ou commence par `whisper`. « Actualiser la liste » la relit ; la ligne dit quand elle a été lue. Le modèle choisi est gardé pour OpenAI. Un modèle qu’OpenAI ne propose plus est remplacé par celui par défaut, et la ligne le dit.

## La transcription en temps réel

Sous « Modèle », la case « Transcription en temps réel » est cochée par défaut. Le son part alors à OpenAI pendant que tu parles, et le texte revient au fil de ta parole : il s’écrit dans la pastille de la dictée, puis le texte définitif va dans le champ, un instant après la fin. En conversation vocale, il s’écrit sous l’orbe.

- Le modèle du temps réel : `gpt-live-transcribe`. Il est propre au temps réel : le « Modèle » choisi sert aux dictées envoyées entières.
- Un flux par dictée, et en conversation un flux par phrase : rien ne part entre deux phrases.
- Le flux s’ouvre dans les 5 secondes, et le texte définitif arrive dans les 5 secondes après la fin ; sinon, ou si le flux tombe (un proxy qui refuse les WebSockets, par exemple), la dictée part entière, par requête, sans rien perdre. En conversation, les phrases suivantes partent alors entières jusqu’à la fin de la conversation.
- La case est gardée pour OpenAI. Décochée, chaque dictée et chaque phrase partent entières, une fois finies.
- La ligne le rappelle : « En temps réel, OpenAI facture la minute plus cher. »

## La voix de l’agent

Avec « Voix générée par » sur « En ligne » et « Service de la voix » sur « OpenAI », OpenAI lit les réponses pendant une conversation vocale, et « ▶ Écouter » lit une phrase d’essai. Les passages sont lus phrase par phrase, la première demandée tout de suite, les suivantes pendant qu’elle joue, et le son joue à mesure qu’il arrive.

- **Le modèle** (« Modèle de la voix ») : par défaut `gpt-4o-mini-tts`. Sans clé, la liste de sa documentation : `gpt-4o-mini-tts` (le plus naturel), `tts-1` (rapide), `tts-1-hd` (qualité). Avec une clé, celle de son API.
- **La voix** : par défaut `marin` en français comme en anglais (`alloy` pour `tts-1` et `tts-1-hd`, qui n’ont pas `marin`). Champ vide : la voix par défaut.
- Son API ne liste pas ses voix : le champ « Voix en … » est **libre**. Écris le nom de la voix tel qu’OpenAI l’attend (par exemple marin, cedar, coral, alloy…) ; « Voir ses voix » ouvre https://platform.openai.com/docs/guides/text-to-speech#voice-options.
- **La vitesse** : de 0,25× à 4× : la vitesse des réglages est ramenée dans ces bornes.
- **La longueur** : 4 000 caractères au plus par requête ; un passage plus long est coupé entre ses phrases.

Une voix qu’OpenAI ne connaît pas est remplacée par sa voix par défaut, et la conversation le dit. Si OpenAI ne répond pas (clé absente ou refusée, quota, panne, réseau), la conversation le dit une fois, puis la voix locale (Piper) lit la suite si elle est téléchargée ; sinon la conversation se met en pause, et « Reprendre » redemande à OpenAI. Le « Volume » s’applique sur ta machine.
