# Utiliser

Ce que fait Deepgram une fois sa clé enregistrée (voir l’onglet Installer). L’usage de la dictée et de la conversation est sur le site : [Réglages › Voix](https://escouade.dev/docs/settings-voice) et [la conversation](https://escouade.dev/docs/chat#dicter-un-message).

## La dictée

Avec « Transcrite par » sur « En ligne » et « Service » sur « Deepgram », chaque dictée est transcrite par Deepgram :

1. Commence la dictée : le bouton du micro à côté d’un champ, ou le raccourci (`Ctrl+Maj+Espace`, `⇧⌘Espace` sous macOS).
2. Parle. Pendant la transcription, la pastille dit « Transcription par Deepgram… ».
3. Finis la dictée (un second appui, ou relâche le raccourci) : le texte arrive dans le champ. `Échap` l’annule.

La « Langue parlée » des réglages est donnée à Deepgram ; « Détection automatique » le laisse trouver la langue. Une dictée s’arrête seule au bout de 10 minutes. Une dictée de moins d’un dixième de seconde n’est pas envoyée.

### Le modèle de la dictée

Le menu « Modèle » liste les modèles de Deepgram qui transcrivent. Par défaut `nova-3`. Sans clé, c’est la liste de sa documentation : `nova-3`, `nova-2`. Avec une clé enregistrée, la liste est celle de son API (`GET /v1/models`) : ses modèles de transcription qui transcrivent un fichier. « Actualiser la liste » la relit ; la ligne dit quand elle a été lue. Le modèle choisi est gardé pour Deepgram. Un modèle que Deepgram ne propose plus est remplacé par celui par défaut, et la ligne le dit.

## La transcription en temps réel

Sous « Modèle », la case « Transcription en temps réel » est cochée par défaut. Le son part alors à Deepgram pendant que tu parles, et le texte revient au fil de ta parole : il s’écrit dans la pastille de la dictée, puis le texte définitif va dans le champ, un instant après la fin. En conversation vocale, il s’écrit sous l’orbe.

- Le modèle du temps réel : le modèle choisi dans « Modèle ».
- Un flux par dictée, et en conversation un flux par phrase : rien ne part entre deux phrases.
- Le flux s’ouvre dans les 5 secondes, et le texte définitif arrive dans les 5 secondes après la fin ; sinon, ou si le flux tombe (un proxy qui refuse les WebSockets, par exemple), la dictée part entière, par requête, sans rien perdre. En conversation, les phrases suivantes partent alors entières jusqu’à la fin de la conversation.
- La case est gardée pour Deepgram. Décochée, chaque dictée et chaque phrase partent entières, une fois finies.
- La ligne le rappelle : « En temps réel, Deepgram facture la minute plus cher. »

## La voix de l’agent

Avec « Voix générée par » sur « En ligne » et « Service de la voix » sur « Deepgram », Deepgram lit les réponses pendant une conversation vocale, et « ▶ Écouter » lit une phrase d’essai. Les passages sont lus phrase par phrase, la première demandée tout de suite, les suivantes pendant qu’elle joue, et le son joue à mesure qu’il arrive.

- **Le modèle** (« Modèle de la voix ») : par défaut `aura-2`. Sans clé, la liste de sa documentation : `aura-2`, `aura` (anglais seulement). Avec une clé, celle de son API.
- **La voix** : par défaut `aura-2-agathe-fr` en français, `aura-2-thalia-en` en anglais ; avec `aura`, `aura-asteria-en`, en anglais seulement. Chez Deepgram, une voix est un modèle : la voix choisie est gardée par modèle. Champ vide : la voix par défaut.
- Son API liste ses voix (`GET /v1/models`) : le champ « Voix en … » **se complète**. Tape quelques lettres : la liste filtre par nom, identifiant, genre, accent ou description, parmi les voix qui lisent la langue avec ce modèle. Si la liste n’a pas pu être lue, le champ reste libre.
- **La vitesse** : de 0,7× à 1,5×, en anglais seulement : en français, Deepgram lit à son rythme.
- **La longueur** : 2 000 caractères au plus par requête ; un passage plus long est coupé entre ses phrases.

Une voix que Deepgram ne connaît pas est remplacée par sa voix par défaut, et la conversation le dit. Si Deepgram ne répond pas (clé absente ou refusée, quota, panne, réseau), la conversation le dit une fois, puis la voix locale (Piper) lit la suite si elle est téléchargée ; sinon la conversation se met en pause, et « Reprendre » redemande à Deepgram. Le « Volume » s’applique sur ta machine.
