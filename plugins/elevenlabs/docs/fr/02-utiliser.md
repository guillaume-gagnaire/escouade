# Utiliser

Ce que fait ElevenLabs une fois sa clé enregistrée (voir l’onglet Installer). L’usage de la dictée et de la conversation est sur le site : [Réglages › Voix](https://escouade.dev/docs/settings-voice) et [la conversation](https://escouade.dev/docs/chat#dicter-un-message).

## La dictée

Avec « Transcrite par » sur « En ligne » et « Service » sur « ElevenLabs », chaque dictée est transcrite par ElevenLabs :

1. Commence la dictée : le bouton du micro à côté d’un champ, ou le raccourci (`Ctrl+Maj+Espace`, `⇧⌘Espace` sous macOS).
2. Parle. Pendant la transcription, la pastille dit « Transcription par ElevenLabs… ».
3. Finis la dictée (un second appui, ou relâche le raccourci) : le texte arrive dans le champ. `Échap` l’annule.

La « Langue parlée » des réglages est donnée à ElevenLabs ; « Détection automatique » le laisse trouver la langue. Une dictée s’arrête seule au bout de 10 minutes. Une dictée de moins d’un dixième de seconde n’est pas envoyée.

### Le modèle de la dictée

Le menu « Modèle » liste les modèles d’ElevenLabs qui transcrivent. Par défaut `scribe_v2`. Sans clé, c’est la liste de sa documentation : `scribe_v2` (90 langues), `scribe_v2_medical` (vocabulaire médical). Son API ne liste pas ses modèles de transcription : la liste est toujours celle de sa documentation, livrée dans le plugin. « Actualiser la liste » la relit ; la ligne dit quand elle a été lue. Le modèle choisi est gardé pour ElevenLabs. Un modèle qu’ElevenLabs ne propose plus est remplacé par celui par défaut, et la ligne le dit.

## La transcription en temps réel

Sous « Modèle », la case « Transcription en temps réel » est cochée par défaut. Le son part alors à ElevenLabs pendant que tu parles, et le texte revient au fil de ta parole : il s’écrit dans la pastille de la dictée, puis le texte définitif va dans le champ, un instant après la fin. En conversation vocale, il s’écrit sous l’orbe.

- Le modèle du temps réel : `scribe_v2_realtime`. Il est propre au temps réel : le « Modèle » choisi sert aux dictées envoyées entières.
- Un flux par dictée, et en conversation un flux par phrase : rien ne part entre deux phrases.
- Le flux s’ouvre dans les 5 secondes, et le texte définitif arrive dans les 5 secondes après la fin ; sinon, ou si le flux tombe (un proxy qui refuse les WebSockets, par exemple), la dictée part entière, par requête, sans rien perdre. En conversation, les phrases suivantes partent alors entières jusqu’à la fin de la conversation.
- La case est gardée pour ElevenLabs. Décochée, chaque dictée et chaque phrase partent entières, une fois finies.
- La ligne le rappelle : « En temps réel, ElevenLabs facture la minute plus cher. »

## La voix de l’agent

Avec « Voix générée par » sur « En ligne » et « Service de la voix » sur « ElevenLabs », ElevenLabs lit les réponses pendant une conversation vocale, et « ▶ Écouter » lit une phrase d’essai. Les passages sont lus phrase par phrase, la première demandée tout de suite, les suivantes pendant qu’elle joue, et le son joue à mesure qu’il arrive.

- **Le modèle** (« Modèle de la voix ») : par défaut `eleven_flash_v2_5`. Sans clé, la liste de sa documentation : `eleven_flash_v2_5` (le plus rapide, 32 langues), `eleven_v4_turbo` (expressif, en temps réel), `eleven_v4` (le plus expressif, plus lent), `eleven_v3_conversational` (expressif, en temps réel), `eleven_v3` (expressif), `eleven_multilingual_v2` (stable, plus lent). Avec une clé, celle de son API.
- **La voix** : par défaut George (`JBFqnCBsd6RMkjVDRZzb`), en français comme en anglais ; avec un ancien modèle `…_v2` qui n’est pas multilingue, en anglais seulement. Champ vide : la voix par défaut.
- Son API liste ses voix (`GET /v2/voices`) : le champ « Voix en … » **se complète**. Tape quelques lettres : la liste filtre par nom, identifiant, genre, accent ou description, parmi les voix qui lisent la langue avec ce modèle ; celles de ton compte viennent en premier, marquées « ton compte ». Si la liste n’a pas pu être lue, le champ reste libre.
- **La vitesse** : de 0,7× à 1,2× : la vitesse des réglages est ramenée dans ces bornes.
- **La longueur** : 5 000 caractères au plus par requête ; un passage plus long est coupé entre ses phrases.

Une voix qu’ElevenLabs ne connaît pas est remplacée par sa voix par défaut, et la conversation le dit. Si ElevenLabs ne répond pas (clé absente ou refusée, quota, panne, réseau), la conversation le dit une fois, puis la voix locale (Piper) lit la suite si elle est téléchargée ; sinon la conversation se met en pause, et « Reprendre » redemande à ElevenLabs. Le « Volume » s’applique sur ta machine.
