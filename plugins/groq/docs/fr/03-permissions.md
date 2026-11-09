# Permissions

Ce que le plugin peut faire, ce qu’il envoie à Groq et quand, et ce qui n’y part jamais. Tout ce qui suit est ce que fait son code (la crate `escouade-voice`) et ce que le cœur d’Escouade lui donne.

## Ce que son manifeste déclare

- **Réseau** : `api.groq.com`, seul. Le plugin est un module WASM : il n’atteint le réseau que par l’hôte d’Escouade, qui refuse tout autre hôte, et passe par le proxy de [Réglages › Avancé › Réseau](https://escouade.dev/docs/settings-network).
- **Activation** : `voice:groq`, quand une dictée, une voix ou les réglages ont besoin de Groq.
- **Rien d’autre** : aucun secret à lui (pas de `permissions.secrets`), aucune commande, aucun événement, aucun fichier.

## Ta clé

Escouade la garde dans le trousseau du système (voir l’onglet Installer). À chaque appel, le cœur la lit et la joint à la demande qu’il fait au plugin. Le plugin la met dans l’en-tête que Groq attend (`Authorization: Bearer <clé>`), et ne l’écrit nulle part. Ce que Groq répond est écrit au journal du plugin la clé retirée (`***`).

Escouade ne donne cette clé qu’au plugin `dev.escouade.groq` : ce nom est réservé aux plugins de la main d’Escouade, et le store le refuse à tout autre auteur.

## Ce qui part chez Groq, et quand

Tout part vers `https://api.groq.com/openai`, en HTTPS, avec ta clé.

| Quand | Ce qui part |
| --- | --- |
| Tu finis une dictée (second appui, ou le raccourci relâché), « Transcrite par » sur « En ligne » | Le son de la dictée (mono, 16 kHz) : une requête `POST /openai/v1/audio/transcriptions` (multipart) : le son en fichier WAV (`dictation.wav`), le modèle, `response_format=json` et la langue parlée quand elle est choisie. |
| En conversation vocale, après chaque phrase que tu dis | Le son de la phrase, de la même façon, entière après son silence. |
| La conversation lit à voix haute, « Voix générée par » sur « En ligne » | Le texte de chaque passage, phrase par phrase, avec le modèle, la voix et la langue : une requête `POST /openai/v1/audio/speech` : le modèle, la phrase, la voix et `response_format=wav`. Ce sont le passage « Lu à voix haute » de chaque réponse, les questions de l’agent, ses demandes de permission (« … demande la permission : <ce qu’il demande>. Tu autorises ? », avec jusqu’à 80 caractères de la commande), et les phrases d’Escouade (« C’est fait. La réponse est dans la conversation. »). |
| Tu cliques « ▶ Écouter » sur une voix de Groq | La phrase d’essai : « Bonjour ! C’est la voix qui te lira les réponses de tes agents dans Escouade. » (ou sa version anglaise). |
| Réglages › Voix montre Groq avec une clé et aucune liste encore lue, ou tu cliques « Actualiser la liste » | La clé seule, pour lire ses listes : `GET /openai/v1/models`. |

## Ce qui n’y part jamais

- Le son du micro quand tu ne dictes pas. En conversation vocale, ni le silence avant une phrase, ni ce que le micro entend pendant que l’agent parle ou travaille : ce son-là sert seulement à savoir si tu lui coupes la parole, sur ta machine.
- Une dictée annulée par `Échap` avant sa fin.
- Ton projet, ton code, tes fichiers, le reste de la conversation avec l’agent : seul ce que la conversation lit à voix haute part, et seulement si la voix est en ligne.
- Le son du micro vers la voix : la voix ne reçoit que du texte.
- Ta clé vers un autre hôte que `api.groq.com`.

Groq traite ce qu’il reçoit selon ses propres conditions : lis-les chez lui.

## Ce que le plugin ne peut pas faire

- Joindre un autre hôte que `api.groq.com`.
- Lire le disque, ton projet, tes fichiers ou le trousseau, lancer un programme, recevoir des événements d’Escouade.
- Garder ta clé : il n’a ni disque ni secret à lui, et la reçoit à chaque appel.
- Écouter le micro : c’est Escouade qui l’ouvre, sur ton geste, et lui passe le son.
