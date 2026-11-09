# Utiliser

Ce que fait Groq une fois sa clé enregistrée (voir l’onglet Installer). L’usage de la dictée et de la conversation est sur le site : [Réglages › Voix](https://escouade.dev/docs/settings-voice) et [la conversation](https://escouade.dev/docs/chat#dicter-un-message).

## La dictée

Avec « Transcrite par » sur « En ligne » et « Service » sur « Groq », chaque dictée est transcrite par Groq :

1. Commence la dictée : le bouton du micro à côté d’un champ, ou le raccourci (`Ctrl+Maj+Espace`, `⇧⌘Espace` sous macOS).
2. Parle. Pendant la transcription, la pastille dit « Transcription par Groq… ».
3. Finis la dictée (un second appui, ou relâche le raccourci) : le texte arrive dans le champ. `Échap` l’annule.

La « Langue parlée » des réglages est donnée à Groq ; « Détection automatique » le laisse trouver la langue. Une dictée s’arrête seule au bout de 10 minutes. Une dictée de moins d’un dixième de seconde n’est pas envoyée.

### Le modèle de la dictée

Le menu « Modèle » liste les modèles de Groq qui transcrivent. Par défaut `whisper-large-v3-turbo`. Sans clé, c’est la liste de sa documentation : `whisper-large-v3-turbo` (rapide), `whisper-large-v3` (plus précis). Avec une clé enregistrée, la liste est celle de son API (`GET /openai/v1/models`) : les modèles dont le nom commence par `whisper`. « Actualiser la liste » la relit ; la ligne dit quand elle a été lue. Le modèle choisi est gardé pour Groq. Un modèle que Groq ne propose plus est remplacé par celui par défaut, et la ligne le dit.

## Pas de temps réel

Groq ne transcrit pas au fil de la parole : la case « Transcription en temps réel » ne s’affiche pas. Chaque dictée part entière quand tu la finis, et en conversation chaque phrase part entière après son silence.

## La voix de l’agent

Avec « Voix générée par » sur « En ligne » et « Service de la voix » sur « Groq », Groq lit les réponses pendant une conversation vocale, et « ▶ Écouter » lit une phrase d’essai. Les passages sont lus phrase par phrase, la première demandée tout de suite, les suivantes pendant qu’elle joue, et le son joue à mesure qu’il arrive.

- **Le modèle** (« Modèle de la voix ») : par défaut `canopylabs/orpheus-v1-english`. Sans clé, la liste de sa documentation : `canopylabs/orpheus-v1-english` (anglais seulement). Avec une clé, celle de son API.
- **La voix** : par défaut `hannah`, en anglais seulement : Orpheus ne lit pas le français, et la voix locale (Piper) lit alors la conversation en français. Champ vide : la voix par défaut.
- Son API ne liste pas ses voix : le champ « Voix en … » est **libre**. Écris le nom de la voix tel que Groq l’attend (par exemple hannah, troy, austin, diana…) ; « Voir ses voix » ouvre https://console.groq.com/docs/text-to-speech/orpheus.
- **La vitesse** : Groq lit à son propre rythme, la vitesse des réglages ne s’applique pas.
- **La longueur** : 200 caractères au plus par requête ; un passage plus long est coupé entre ses phrases.

Une voix que Groq ne connaît pas est remplacée par sa voix par défaut, et la conversation le dit. Si Groq ne répond pas (clé absente ou refusée, quota, panne, réseau), la conversation le dit une fois, puis la voix locale (Piper) lit la suite si elle est téléchargée ; sinon la conversation se met en pause, et « Reprendre » redemande à Groq. Le « Volume » s’applique sur ta machine.
