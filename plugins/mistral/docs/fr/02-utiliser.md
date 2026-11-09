# Utiliser

Ce que fait Mistral une fois sa clé enregistrée (voir l’onglet Installer). L’usage de la dictée et de la conversation est sur le site : [Réglages › Voix](https://escouade.dev/docs/settings-voice) et [la conversation](https://escouade.dev/docs/chat#dicter-un-message).

## La dictée

Avec « Transcrite par » sur « En ligne » et « Service » sur « Mistral », chaque dictée est transcrite par Mistral :

1. Commence la dictée : le bouton du micro à côté d’un champ, ou le raccourci (`Ctrl+Maj+Espace`, `⇧⌘Espace` sous macOS).
2. Parle. Pendant la transcription, la pastille dit « Transcription par Mistral… ».
3. Finis la dictée (un second appui, ou relâche le raccourci) : le texte arrive dans le champ. `Échap` l’annule.

La « Langue parlée » des réglages est donnée à Mistral ; « Détection automatique » le laisse trouver la langue. Une dictée s’arrête seule au bout de 10 minutes. Une dictée de moins d’un dixième de seconde n’est pas envoyée.

### Le modèle de la dictée

Le menu « Modèle » liste les modèles de Mistral qui transcrivent. Par défaut `voxtral-mini-latest`. Sans clé, c’est la liste de sa documentation : `voxtral-mini-latest`. Avec une clé enregistrée, la liste est celle de son API (`GET /v1/models`) : ses modèles Voxtral Mini qui transcrivent un fichier. « Actualiser la liste » la relit ; la ligne dit quand elle a été lue. Le modèle choisi est gardé pour Mistral. Un modèle que Mistral ne propose plus est remplacé par celui par défaut, et la ligne le dit.

## La transcription en temps réel

Sous « Modèle », la case « Transcription en temps réel » est cochée par défaut. Le son part alors à Mistral pendant que tu parles, et le texte revient au fil de ta parole : il s’écrit dans la pastille de la dictée, puis le texte définitif va dans le champ, un instant après la fin. En conversation vocale, il s’écrit sous l’orbe.

- Le modèle du temps réel : `voxtral-mini-transcribe-realtime-2602`. Il est propre au temps réel : le « Modèle » choisi sert aux dictées envoyées entières.
- Un flux par dictée, et en conversation un flux par phrase : rien ne part entre deux phrases.
- Le flux s’ouvre dans les 5 secondes, et le texte définitif arrive dans les 5 secondes après la fin ; sinon, ou si le flux tombe (un proxy qui refuse les WebSockets, par exemple), la dictée part entière, par requête, sans rien perdre. En conversation, les phrases suivantes partent alors entières jusqu’à la fin de la conversation.
- La case est gardée pour Mistral. Décochée, chaque dictée et chaque phrase partent entières, une fois finies.
- La ligne le rappelle : « En temps réel, Mistral facture la minute plus cher. »

## La voix de l’agent

Avec « Voix générée par » sur « En ligne » et « Service de la voix » sur « Mistral », Mistral lit les réponses pendant une conversation vocale, et « ▶ Écouter » lit une phrase d’essai. Les passages sont lus phrase par phrase, la première demandée tout de suite, les suivantes pendant qu’elle joue, et le son joue à mesure qu’il arrive.

- **Le modèle** (« Modèle de la voix ») : par défaut `voxtral-mini-tts-2603`. Sans clé, la liste de sa documentation : `voxtral-mini-tts-2603`. Avec une clé, celle de son API.
- **La voix** : par défaut `fr_marie_neutral` en français, `en_paul_neutral` en anglais. Champ vide : la voix par défaut.
- Son API liste ses voix (`GET /v1/audio/voices`) : le champ « Voix en … » **se complète**. Tape quelques lettres : la liste filtre par nom, identifiant, genre, accent ou description, parmi les voix qui lisent la langue avec ce modèle ; celles de ton compte viennent en premier, marquées « ton compte ». Si la liste n’a pas pu être lue, le champ reste libre.
- **La vitesse** : Mistral lit à son propre rythme, la vitesse des réglages ne s’applique pas.
- **La longueur** : 4 000 caractères au plus par requête ; un passage plus long est coupé entre ses phrases.

Une voix que Mistral ne connaît pas est remplacée par sa voix par défaut, et la conversation le dit. Si Mistral ne répond pas (clé absente ou refusée, quota, panne, réseau), la conversation le dit une fois, puis la voix locale (Piper) lit la suite si elle est téléchargée ; sinon la conversation se met en pause, et « Reprendre » redemande à Mistral. Le « Volume » s’applique sur ta machine.

## Les agents Mistral Vibe

Le plugin sert aussi le harness « Mistral Vibe » : un agent lancé avec lui travaille par `vibe-acp`, un programme à part qu’Escouade lance et surveille, sur le compte choisi (Réglages › Comptes et harness).

- **Les modes** : « Demander » est son agent `ask`, « Édits auto » son agent `accept-edits`, « Bypass » son agent `auto-approve`. Vibe n’a pas de mode Plan : Escouade le joue (il étudie, ses écritures sont refusées, ses derniers mots sont le plan à approuver sur sa carte, puis il le met en œuvre).
- **Le modèle et l’effort** : le menu « Modèle » liste ceux de sa session (`mistral-medium-3.5` par défaut). « Bas » coupe sa réflexion, « Élevé » la garde.
- **Les demandes** : « Autoriser », « Pour cet agent » (gardé dans le `VIBE_HOME` de l’agent) et « Refuser ». Un refus arrête le tour : Vibe réessaierait sinon la même commande sous d’autres formes.
- **Accepté seul par le plugin** : en « Demander » et en « Édits auto », une écriture ou un effacement dont tous les chemins sont dans le dossier de l’agent, hors de `.git` ; en « Bypass », tout. Le reste passe par une carte.
- **Le coût** : Vibe donne le coût et les tokens de chaque tour, pas de quota. Sa limite de débit arrête le tour sur une carte ambre ; l’agent reprend seul une minute plus tard.
- **Une discussion de l’onglet Docs** ne va jamais à Vibe : il n’a pas de mode qui ne change rien.
- **Les pièces jointes** : des images et des fichiers texte, pas de PDF.

La page [Harness](https://escouade.dev/docs/harness#mistral-vibe) du site détaille tout cela, et la « Vue Mistral Vibe ».
