# Permissions

Ce que le plugin peut faire, ce qu’il envoie à ElevenLabs et quand, et ce qui n’y part jamais. Tout ce qui suit est ce que fait son code (la crate `escouade-voice`) et ce que le cœur d’Escouade lui donne.

## Ce que son manifeste déclare

- **Réseau** : `api.elevenlabs.io`, seul. Le plugin est un module WASM : il n’atteint le réseau que par l’hôte d’Escouade, qui refuse tout autre hôte, et passe par le proxy de [Réglages › Avancé › Réseau](https://escouade.dev/docs/settings-network).
- **Activation** : `voice:elevenlabs`, quand une dictée, une voix ou les réglages ont besoin d’ElevenLabs.
- **Rien d’autre** : aucun secret à lui (pas de `permissions.secrets`), aucune commande, aucun événement, aucun fichier.

## Ta clé

Escouade la garde dans le trousseau du système (voir l’onglet Installer). À chaque appel, le cœur la lit et la joint à la demande qu’il fait au plugin. Le plugin la met dans l’en-tête qu’ElevenLabs attend (`xi-api-key: <clé>`), et ne l’écrit nulle part. Ce qu’ElevenLabs répond est écrit au journal du plugin la clé retirée (`***`).

Escouade ne donne cette clé qu’au plugin `dev.escouade.elevenlabs` : ce nom est réservé aux plugins de la main d’Escouade, et le store le refuse à tout autre auteur.

## Ce qui part chez ElevenLabs, et quand

Tout part vers `https://api.elevenlabs.io`, en HTTPS (en WebSocket chiffré pour le temps réel), avec ta clé.

| Quand | Ce qui part |
| --- | --- |
| Tu finis une dictée (second appui, ou le raccourci relâché), « Transcrite par » sur « En ligne », sans temps réel | Le son de la dictée (mono, 16 kHz) : une requête `POST /v1/speech-to-text` (multipart) : le son en fichier WAV (`dictation.wav`), le modèle (`model_id`), `tag_audio_events=false` et la langue parlée (`language_code`) quand elle est choisie. |
| Pendant une dictée, avec « Transcription en temps réel » | Le son au fil de ta parole, dès le début de la dictée (ce que le micro a entendu pendant qu’il s’ouvrait compris), jusqu’à sa fin : un WebSocket `wss://api.elevenlabs.io/v1/speech-to-text/realtime` (le modèle, `audio_format=pcm_16000`, `commit_strategy=manual`, la langue quand elle est choisie) : le son en PCM 16 kHz par morceaux de 100 ms (`input_audio_chunk`), le dernier avec `commit`. |
| En conversation vocale, après chaque phrase que tu dis | Le son de la phrase, de la même façon : entière après son silence, ou au fil de ta parole avec le temps réel, de ses premières syllabes à son silence de fin. |
| Un flux qui ne s’ouvre pas, ou tombe | La même dictée ou la même phrase, à nouveau, entière par requête : avec le temps réel, son son peut donc partir deux fois. |
| La conversation lit à voix haute, « Voix générée par » sur « En ligne » | Le texte de chaque passage, phrase par phrase, avec le modèle, la voix et la langue : une requête `POST /v1/text-to-speech/<voix>/stream?output_format=pcm_24000` : la phrase, le modèle, la langue pour un modèle qui la prend, et la vitesse quand elle n’est pas 1. Ce sont le passage « Lu à voix haute » de chaque réponse, les questions de l’agent, ses demandes de permission (« … demande la permission : <ce qu’il demande>. Tu autorises ? », avec jusqu’à 80 caractères de la commande), et les phrases d’Escouade (« C’est fait. La réponse est dans la conversation. »). |
| Tu cliques « ▶ Écouter » sur une voix d’ElevenLabs | La phrase d’essai : « Bonjour ! C’est la voix qui te lira les réponses de tes agents dans Escouade. » (ou sa version anglaise). |
| Réglages › Voix montre ElevenLabs avec une clé et aucune liste encore lue, ou tu cliques « Actualiser la liste » | La clé seule, pour lire ses listes : `GET /v1/models` (ses modèles de voix) et `GET /v2/voices` (ses voix, celles de ton compte comprises). |

## Ce qui n’y part jamais

- Le son du micro quand tu ne dictes pas. En conversation vocale, ni le silence avant une phrase, ni ce que le micro entend pendant que l’agent parle ou travaille : ce son-là sert seulement à savoir si tu lui coupes la parole, sur ta machine.
- Une dictée annulée par `Échap` avant sa fin, sans temps réel. Avec le temps réel, ce que tu as dit avant `Échap` est déjà parti.
- Ton projet, ton code, tes fichiers, le reste de la conversation avec l’agent : seul ce que la conversation lit à voix haute part, et seulement si la voix est en ligne.
- Le son du micro vers la voix : la voix ne reçoit que du texte.
- Ta clé vers un autre hôte que `api.elevenlabs.io`.

ElevenLabs traite ce qu’il reçoit selon ses propres conditions : lis-les chez lui.

## Ce que le plugin ne peut pas faire

- Joindre un autre hôte que `api.elevenlabs.io`.
- Lire le disque, ton projet, tes fichiers ou le trousseau, lancer un programme, recevoir des événements d’Escouade.
- Garder ta clé : il n’a ni disque ni secret à lui, et la reçoit à chaque appel.
- Écouter le micro : c’est Escouade qui l’ouvre, sur ton geste, et lui passe le son.
