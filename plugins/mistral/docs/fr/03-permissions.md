# Permissions

Ce que le plugin peut faire, ce qu’il envoie à Mistral et quand, et ce qui n’y part jamais. Tout ce qui suit est ce que fait son code (la crate `escouade-voice` et, pour Vibe, `escouade-acp`) et ce que le cœur d’Escouade lui donne.

## Ce que son manifeste déclare

- **Réseau** : `api.mistral.ai`, seul. Le plugin est un module WASM : il n’atteint le réseau que par l’hôte d’Escouade, qui refuse tout autre hôte, et passe par le proxy de [Réglages › Avancé › Réseau](https://escouade.dev/docs/settings-network).
- **Activation** : `voice:mistral`, quand une dictée, une voix ou les réglages ont besoin de Mistral.
- **Un compte** : `mistral`, le compte Mistral de la voix et de Vibe, et **un harness** : `vibe`, Mistral Vibe.
- **Rien d’autre** : aucun secret à lui (pas de `permissions.secrets`), aucune commande, aucun événement, aucun fichier.

## Ta clé

Escouade la garde dans le trousseau du système (voir l’onglet Installer). À chaque appel, le cœur la lit et la joint à la demande qu’il fait au plugin. Le plugin la met dans l’en-tête que Mistral attend (`Authorization: Bearer <clé>`), et ne l’écrit nulle part. Ce que Mistral répond est écrit au journal du plugin la clé retirée (`***`).

Escouade ne donne cette clé qu’au plugin `dev.escouade.mistral` : ce nom est réservé aux plugins de la main d’Escouade, et le store le refuse à tout autre auteur. La clé du compte Mistral sert aussi aux comptes Mistral Vibe qui n’ont pas de clé à eux : le cœur la donne alors à Mistral Vibe par `MISTRAL_API_KEY` (voir plus bas), jamais au plugin.

## Ce qui part chez Mistral, et quand

Tout part vers `https://api.mistral.ai`, en HTTPS (en WebSocket chiffré pour le temps réel), avec ta clé.

| Quand | Ce qui part |
| --- | --- |
| Tu finis une dictée (second appui, ou le raccourci relâché), « Transcrite par » sur « En ligne », sans temps réel | Le son de la dictée (mono, 16 kHz) : une requête `POST /v1/audio/transcriptions` (multipart) : le son en fichier WAV (`dictation.wav`), le modèle et la langue parlée quand elle est choisie. |
| Pendant une dictée, avec « Transcription en temps réel » | Le son au fil de ta parole, dès le début de la dictée (ce que le micro a entendu pendant qu’il s’ouvrait compris), jusqu’à sa fin : un WebSocket `wss://api.mistral.ai/v1/audio/transcriptions/realtime` (le modèle) : le format du son (`session.update`, PCM 16 kHz), le son par morceaux de 100 ms (`input_audio.append`), puis `input_audio.flush` et `input_audio.end`. |
| En conversation vocale, après chaque phrase que tu dis | Le son de la phrase, de la même façon : entière après son silence, ou au fil de ta parole avec le temps réel, de ses premières syllabes à son silence de fin. |
| Un flux qui ne s’ouvre pas, ou tombe | La même dictée ou la même phrase, à nouveau, entière par requête : avec le temps réel, son son peut donc partir deux fois. |
| La conversation lit à voix haute, « Voix générée par » sur « En ligne » | Le texte de chaque passage, phrase par phrase, avec le modèle, la voix et la langue : une requête `POST /v1/audio/speech` : le modèle, la phrase, la voix (`voice_id`), `response_format=pcm` et `stream=true`. Ce sont le passage « Lu à voix haute » de chaque réponse, les questions de l’agent, ses demandes de permission (« … demande la permission : <ce qu’il demande>. Tu autorises ? », avec jusqu’à 80 caractères de la commande), et les phrases d’Escouade (« C’est fait. La réponse est dans la conversation. »). |
| Tu cliques « ▶ Écouter » sur une voix de Mistral | La phrase d’essai : « Bonjour ! C’est la voix qui te lira les réponses de tes agents dans Escouade. » (ou sa version anglaise). |
| Réglages › Voix montre Mistral avec une clé et aucune liste encore lue, ou tu cliques « Actualiser la liste » | La clé seule, pour lire ses listes : `GET /v1/models` (ses modèles) et `GET /v1/audio/voices` (ses voix). |

## Ce qui n’y part jamais

- Le son du micro quand tu ne dictes pas. En conversation vocale, ni le silence avant une phrase, ni ce que le micro entend pendant que l’agent parle ou travaille : ce son-là sert seulement à savoir si tu lui coupes la parole, sur ta machine.
- Une dictée annulée par `Échap` avant sa fin, sans temps réel. Avec le temps réel, ce que tu as dit avant `Échap` est déjà parti.
- Ton projet, ton code, tes fichiers, le reste de la conversation avec l’agent : seul ce que la conversation lit à voix haute part, et seulement si la voix est en ligne.
- Le son du micro vers la voix : la voix ne reçoit que du texte.
- Ta clé vers un autre hôte que `api.mistral.ai`.

Mistral traite ce qu’il reçoit selon ses propres conditions : lis-les chez lui.

## Mistral Vibe

Le harness est une autre affaire : **Mistral Vibe est un programme à part**, `vibe-acp`, qui parle lui-même à Mistral avec la clé de son compte. Ce qu’il envoie à Mistral (tes messages, les fichiers qu’il lit, la sortie de ses commandes, ce que ses outils lui rendent) passe par son propre réseau : ni Escouade ni ce plugin ne le voient passer, ni ne le filtrent.

- **Qui le lance** : le cœur d’Escouade, jamais le plugin. Le plugin décrit le lancement (`vibe-acp`, `VIBE_ENABLE_TELEMETRY=false`) ; le cœur le vérifie, coupe aussi les mises à jour et leurs vérifications de Vibe, donne à chaque agent son propre `VIBE_HOME`, et la clé d’un compte par `MISTRAL_API_KEY`, jamais dans un fichier ni sur une ligne de commande.
- **Ce que voit le plugin** : ce que Vibe écrit (la conversation de l’agent, ses appels d’outils, ses demandes), qu’il traduit pour la conversation ; et le jeton du serveur MCP d’Escouade, qu’il met dans le lancement qu’il décrit. Il ne reçoit jamais la clé d’un compte Vibe : on lui dit seulement qu’un compte passe par une clé. Son seul réseau reste `api.mistral.ai`, et son code n’y envoie que la voix.
- **Ce qu’il décide seul** : il accepte, en « Demander » et en « Édits auto », une écriture ou un effacement dont tous les chemins sont dans le dossier de l’agent, hors de `.git` ; en « Bypass », tout. Il refuse d’office une demande qui nomme un chemin d’un projet privé. Le reste passe par une carte, qui t’attend.
- **Ce qu’il ne peut pas** : lancer Vibe ou une commande, lire ou écrire un fichier, joindre un autre hôte.
- **Vibe n’a pas de bac à sable** : une commande que tu acceptes peut tout faire, avec tes droits.

## Ce que le plugin ne peut pas faire

- Joindre un autre hôte que `api.mistral.ai`.
- Lire le disque, ton projet, tes fichiers ou le trousseau, lancer un programme (Vibe compris), recevoir des événements d’Escouade.
- Garder ta clé : il n’a ni disque ni secret à lui, et la reçoit à chaque appel.
- Écouter le micro : c’est Escouade qui l’ouvre, sur ton geste, et lui passe le son.
